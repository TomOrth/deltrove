use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode as AxumStatus,
    routing::{get, post},
};
use ci_simulator::{Reconciliation, RequestKey, RequestPayload, StoreCounts, SubmitRequest};
use reqwest::StatusCode;

struct Server(Child);

impl Server {
    fn start(db: &std::path::Path, dedup: bool) -> (Self, String) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral port bind");
        let address = listener.local_addr().expect("bound address");
        drop(listener);
        let mut child = Command::new(env!("CARGO_BIN_EXE_ci-simulator"))
            .arg("--listen")
            .arg(address.to_string())
            .arg("--db")
            .arg(db)
            .arg("--dedup")
            .arg(dedup.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start simulator");
        let base = format!("http://{address}");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Ok(mut stream) = TcpStream::connect(address) {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
                let _ = stream.write_all(
                    b"GET /health/ready HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                );
                let mut response = String::new();
                if stream.read_to_string(&mut response).is_ok()
                    && response.contains("200 OK")
                    && response.contains("ready")
                {
                    return (Self(child), base);
                }
            }
            thread::yield_now();
        }
        let _ = child.kill();
        let _ = child.wait();
        panic!("simulator readiness deadline expired");
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct TestDb(PathBuf);

impl Drop for TestDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(format!("{}-wal", self.0.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.0.display()));
    }
}

#[derive(Clone)]
struct ReceiverState {
    bodies: Arc<Mutex<Vec<serde_json::Value>>>,
    failures_left: Arc<AtomicUsize>,
}

struct Receiver {
    url: String,
    state: ReceiverState,
    task: tokio::task::JoinHandle<()>,
}

impl Receiver {
    fn received(&self) -> Vec<serde_json::Value> {
        self.state
            .bodies
            .lock()
            .map(|bodies| bodies.clone())
            .unwrap_or_default()
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn receive_callback(
    State(state): State<ReceiverState>,
    Json(body): Json<serde_json::Value>,
) -> AxumStatus {
    if let Ok(mut bodies) = state.bodies.lock() {
        bodies.push(body);
    }
    if state
        .failures_left
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
            value.checked_sub(1)
        })
        .is_ok()
    {
        AxumStatus::INTERNAL_SERVER_ERROR
    } else {
        AxumStatus::OK
    }
}

async fn start_receiver(client: &reqwest::Client, failures: usize) -> Receiver {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("receiver listener");
    let address = listener.local_addr().expect("receiver address");
    let state = ReceiverState {
        bodies: Arc::new(Mutex::new(Vec::new())),
        failures_left: Arc::new(AtomicUsize::new(failures)),
    };
    let app = Router::new()
        .route("/health/ready", get(|| async { "ready" }))
        .route("/callback", post(receive_callback))
        .with_state(state.clone());
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let url = format!("http://{address}");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        if client
            .get(format!("{url}/health/ready"))
            .send()
            .await
            .is_ok()
        {
            return Receiver { url, state, task };
        }
        tokio::task::yield_now().await;
    }
    task.abort();
    panic!("receiver readiness deadline expired");
}

fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("bounded test client")
}

fn fixture_request() -> SubmitRequest {
    SubmitRequest {
        key: RequestKey {
            repo_id: "repo-fixture-1".into(),
            operation_id: "op-7".into(),
            check_name: "unit".into(),
            candidate_id: "cand-3".into(),
        },
        payload: RequestPayload {
            candidate_digest: "sha256:abc".into(),
            command: "cargo test".into(),
        },
    }
}

fn temp_db(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("ci-sim-{label}-{}.sqlite", uuid::Uuid::new_v4()))
}

async fn submit_run(client: &reqwest::Client, base: &str, request: SubmitRequest) -> String {
    let response: serde_json::Value = client
        .post(format!("{base}/v1/requests"))
        .json(&request)
        .send()
        .await
        .expect("submit fixture request")
        .error_for_status()
        .expect("accepted request")
        .json()
        .await
        .expect("submit response JSON");
    response["run_id"].as_str().expect("run ID").to_owned()
}

#[tokio::test]
async fn accepted_lost_body_reconciles_and_survives_provider_restart() {
    let db = TestDb(temp_db("restart"));
    let client = test_client();
    let receiver = start_receiver(&client, 0).await;
    let (server, base) = Server::start(&db.0, true);
    let request = fixture_request();
    let result = client
        .post(format!("{base}/v1/requests?drop_response=true"))
        .json(&request)
        .send()
        .await;
    match result {
        Ok(response) => {
            assert_eq!(response.status(), StatusCode::ACCEPTED);
            assert!(
                response.bytes().await.is_err(),
                "the client must observe a body transport failure after acceptance"
            );
        }
        Err(error) => assert!(
            error.is_request(),
            "loss must be visible as a transport error"
        ),
    }
    let query = format!(
        "repo_id={}&operation_id={}&check_name={}&candidate_id={}",
        request.key.repo_id,
        request.key.operation_id,
        request.key.check_name,
        request.key.candidate_id
    );
    let found: Reconciliation = client
        .get(format!("{base}/v1/reconcile?{query}"))
        .send()
        .await
        .expect("reconcile")
        .json()
        .await
        .expect("reconciliation JSON");
    let run_id = match found {
        Reconciliation::Found { run_ids, .. } => {
            assert_eq!(run_ids.len(), 1);
            run_ids[0].clone()
        }
        Reconciliation::Unknown => panic!("accepted request must be durable"),
    };
    let duplicate: serde_json::Value = client
        .post(format!("{base}/v1/requests"))
        .json(&request)
        .send()
        .await
        .expect("dedup request")
        .json()
        .await
        .expect("dedup JSON");
    assert_eq!(duplicate["deduplicated"], true);
    let mut conflict = request.clone();
    conflict.payload.command = "cargo test --all".into();
    let response = client
        .post(format!("{base}/v1/requests"))
        .json(&conflict)
        .send()
        .await
        .expect("conflict response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let scheduled = client
        .post(format!("{base}/v1/callbacks/schedule"))
        .json(&serde_json::json!({"callback_id":"cb-1","run_id":run_id,"target_url":format!("{}/callback",receiver.url)}))
        .send()
        .await
        .expect("callback schedule");
    assert_eq!(scheduled.status(), StatusCode::ACCEPTED);
    let outcome = client
        .post(format!("{base}/v1/outcomes/ack"))
        .json(&serde_json::json!({"outcome_id":"outcome-1","key":request.key,"outcome":"check_passed"}))
        .send()
        .await
        .expect("logical outcome acknowledgement");
    assert_eq!(outcome.status(), StatusCode::ACCEPTED);
    drop(server);

    let (_restarted, base) = Server::start(&db.0, true);
    let found: Reconciliation = client
        .get(format!("{base}/v1/reconcile?{query}"))
        .send()
        .await
        .expect("reconcile after restart")
        .json()
        .await
        .expect("reconciliation JSON");
    assert!(
        matches!(found, Reconciliation::Found { ref run_ids, .. } if run_ids == std::slice::from_ref(&run_id))
    );
    for expected_attempt in [1, 2] {
        let callback = client
            .post(format!("{base}/v1/callbacks/deliver"))
            .json(&serde_json::json!({"callback_id":"cb-1"}))
            .send()
            .await
            .expect("callback delivery");
        assert_eq!(callback.status(), StatusCode::OK);
        let result: serde_json::Value = callback.json().await.expect("callback result");
        assert_eq!(result["attempt"], expected_attempt);
    }
    let counts: StoreCounts = client
        .get(format!("{base}/v1/counts"))
        .send()
        .await
        .expect("counts")
        .json()
        .await
        .expect("counts JSON");
    assert_eq!(counts.submission_attempts, 2);
    assert_eq!(counts.physical_executions, 1);
    assert_eq!(counts.callback_attempts, 2);
    assert_eq!(counts.logical_outcomes, 1);
    assert_eq!(receiver.received().len(), 2);
    assert_eq!(receiver.received()[0]["callback_id"], "cb-1");
    assert_eq!(receiver.received()[0]["run_id"], run_id);
    assert_eq!(receiver.received()[0]["attempt"], 1);
    assert_eq!(receiver.received()[1]["attempt"], 2);
}

#[tokio::test]
async fn callback_failure_is_retained_then_retried_to_independent_receiver() {
    let db = TestDb(temp_db("callback-retry"));
    let client = test_client();
    let receiver = start_receiver(&client, 1).await;
    let (_server, base) = Server::start(&db.0, true);
    let run_id = submit_run(&client, &base, fixture_request()).await;
    let scheduled = client
        .post(format!("{base}/v1/callbacks/schedule"))
        .json(&serde_json::json!({"callback_id":"cb-retry","run_id":run_id,"target_url":format!("{}/callback",receiver.url)}))
        .send().await.expect("schedule callback");
    assert_eq!(scheduled.status(), StatusCode::ACCEPTED);

    let failed = client
        .post(format!("{base}/v1/callbacks/deliver"))
        .json(&serde_json::json!({"callback_id":"cb-retry"}))
        .send()
        .await
        .expect("failed callback response");
    assert_eq!(failed.status(), StatusCode::BAD_GATEWAY);
    let failed_result: serde_json::Value = failed.json().await.expect("failed attempt response");
    assert_eq!(failed_result["attempt"], 1);
    let attempts: Vec<serde_json::Value> = client
        .get(format!("{base}/v1/callbacks/attempts?callback_id=cb-retry"))
        .send()
        .await
        .expect("read failed attempt")
        .json()
        .await
        .expect("attempt JSON");
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0]["status"], "failed");
    assert_eq!(attempts[0]["delivered"], false);

    let retried = client
        .post(format!("{base}/v1/callbacks/deliver"))
        .json(&serde_json::json!({"callback_id":"cb-retry"}))
        .send()
        .await
        .expect("retry callback");
    assert_eq!(retried.status(), StatusCode::OK);
    let attempts: Vec<serde_json::Value> = client
        .get(format!("{base}/v1/callbacks/attempts?callback_id=cb-retry"))
        .send()
        .await
        .expect("read attempts")
        .json()
        .await
        .expect("attempt JSON");
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0]["status"], "failed");
    assert_eq!(attempts[1]["status"], "delivered");
    let delivered = receiver.received();
    assert_eq!(delivered.len(), 2);
    assert_eq!(delivered[0]["callback_id"], delivered[1]["callback_id"]);
    assert_eq!(delivered[0]["run_id"], run_id);
    assert_eq!(delivered[0]["attempt"], 1);
    assert_eq!(delivered[1]["attempt"], 2);
    assert_eq!(delivered[0]["key"]["repo_id"], "repo-fixture-1");
    assert_eq!(delivered[0]["result"], "accepted");
}

#[tokio::test]
async fn callback_delivery_requires_a_matching_durable_schedule() {
    let db = TestDb(temp_db("callback-binding"));
    let client = test_client();
    let receiver = start_receiver(&client, 0).await;
    let (_server, base) = Server::start(&db.0, true);
    let run_id = submit_run(&client, &base, fixture_request()).await;

    let unscheduled = client
        .post(format!("{base}/v1/callbacks/deliver"))
        .json(&serde_json::json!({"callback_id":"never-scheduled"}))
        .send()
        .await
        .expect("unscheduled callback result");
    assert_eq!(unscheduled.status(), StatusCode::NOT_FOUND);
    let mismatched = client.post(format!("{base}/v1/callbacks/schedule"))
        .json(&serde_json::json!({"callback_id":"cb-bound","run_id":"unknown-run","target_url":format!("{}/callback",receiver.url)}))
        .send().await.expect("mismatched run schedule");
    assert_eq!(mismatched.status(), StatusCode::CONFLICT);
    let scheduled = client.post(format!("{base}/v1/callbacks/schedule"))
        .json(&serde_json::json!({"callback_id":"cb-bound","run_id":run_id,"target_url":format!("{}/callback",receiver.url)}))
        .send().await.expect("valid schedule");
    assert_eq!(scheduled.status(), StatusCode::ACCEPTED);
    let changed_binding = client.post(format!("{base}/v1/callbacks/schedule"))
        .json(&serde_json::json!({"callback_id":"cb-bound","run_id":"unknown-run","target_url":format!("{}/callback",receiver.url)}))
        .send().await.expect("changed callback binding");
    assert_eq!(changed_binding.status(), StatusCode::CONFLICT);
    assert!(receiver.received().is_empty());
    let attempts: Vec<serde_json::Value> = client
        .get(format!("{base}/v1/callbacks/attempts?callback_id=cb-bound"))
        .send()
        .await
        .expect("attempts before delivery")
        .json()
        .await
        .expect("attempt JSON");
    assert!(attempts.is_empty());
}

#[tokio::test]
async fn disabled_provider_dedup_records_multiple_physical_runs() {
    let db = TestDb(temp_db("no-dedup"));
    let (_server, base) = Server::start(&db.0, false);
    let client = test_client();
    let request = fixture_request();
    for _ in 0..2 {
        client
            .post(format!("{base}/v1/requests"))
            .json(&request)
            .send()
            .await
            .expect("submit")
            .error_for_status()
            .expect("accepted");
    }
    let counts: StoreCounts = client
        .get(format!("{base}/v1/counts"))
        .send()
        .await
        .expect("counts")
        .json()
        .await
        .expect("counts JSON");
    assert_eq!(counts.submission_attempts, 2);
    assert_eq!(counts.physical_executions, 2);
}
