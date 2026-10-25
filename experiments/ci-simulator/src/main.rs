use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use ci_simulator::{CallbackDispatch, RequestKey, SimulatorError, SimulatorStore, SubmitRequest};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:8787")]
    listen: SocketAddr,
    #[arg(long)]
    db: PathBuf,
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    dedup: bool,
}

#[derive(Clone)]
struct AppState {
    store: Arc<SimulatorStore>,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct SubmitControl {
    #[serde(default)]
    drop_response: bool,
}

#[derive(Deserialize)]
struct ScheduleCallbackInput {
    callback_id: String,
    run_id: String,
    target_url: String,
}

#[derive(Deserialize)]
struct DeliverCallbackInput {
    callback_id: String,
}

#[derive(Deserialize)]
struct CallbackQuery {
    callback_id: String,
}

#[derive(Serialize)]
struct CallbackEvent {
    callback_id: String,
    attempt: u32,
    run_id: String,
    key: RequestKey,
    result: String,
}

#[derive(Deserialize)]
struct OutcomeInput {
    outcome_id: String,
    key: RequestKey,
    outcome: String,
}

#[derive(Deserialize)]
struct KeyQuery {
    repo_id: String,
    operation_id: String,
    check_name: String,
    candidate_id: String,
}

#[derive(Debug, thiserror::Error)]
enum HandlerError {
    #[error(transparent)]
    Simulator(#[from] SimulatorError),
    #[error("blocking simulator operation failed: {0}")]
    Join(String),
}

async fn store_call<T, F>(store: Arc<SimulatorStore>, operation: F) -> Result<T, HandlerError>
where
    T: Send + 'static,
    F: FnOnce(Arc<SimulatorStore>) -> Result<T, SimulatorError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || operation(store))
        .await
        .map_err(|error| HandlerError::Join(error.to_string()))?
        .map_err(HandlerError::Simulator)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let store = Arc::new(
        tokio::task::spawn_blocking(move || SimulatorStore::open(args.db, args.dedup)).await??,
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    let app = Router::new()
        .route("/health/ready", get(|| async { "ready" }))
        .route("/v1/requests", post(submit))
        .route("/v1/reconcile", get(reconcile))
        .route("/v1/callbacks/schedule", post(schedule_callback))
        .route("/v1/callbacks/deliver", post(deliver_callback))
        .route("/v1/callbacks/attempts", get(callback_attempts))
        .route("/v1/outcomes/ack", post(acknowledge_outcome))
        .route("/v1/counts", get(counts))
        .with_state(AppState { store, client });
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn submit(
    State(state): State<AppState>,
    Query(control): Query<SubmitControl>,
    Json(request): Json<SubmitRequest>,
) -> Response<Body> {
    match store_call(state.store, move |store| store.submit(&request, false)).await {
        Ok(_disposition) if control.drop_response => Response::builder()
            .status(StatusCode::ACCEPTED)
            .header("content-type", "application/json")
            .body(Body::from_stream(futures_util::stream::iter([
                Ok::<_, std::io::Error>("accepted"),
                Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "simulated response loss after durable accept",
                )),
            ])))
            .unwrap_or_else(|_| {
                StatusCode::INTERNAL_SERVER_ERROR
                    .into_response()
                    .map(Body::new)
            }),
        Ok(disposition) => (
            StatusCode::ACCEPTED,
            Json(
                serde_json::to_value(disposition)
                    .unwrap_or_else(|_| serde_json::json!({"kind":"serialization_error"})),
            ),
        )
            .into_response(),
        Err(HandlerError::Simulator(SimulatorError::PayloadConflict)) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"kind":"payload_conflict"})),
        )
            .into_response(),
        Err(error) => internal_error(error).into_response(),
    }
}

async fn reconcile(
    State(state): State<AppState>,
    Query(key): Query<KeyQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let key = RequestKey {
        repo_id: key.repo_id,
        operation_id: key.operation_id,
        check_name: key.check_name,
        candidate_id: key.candidate_id,
    };
    match store_call(state.store, move |store| store.reconcile(&key)).await {
        Ok(result) => (
            StatusCode::OK,
            Json(
                serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"kind":"serialization_error"})),
            ),
        ),
        Err(error) => internal_error(error),
    }
}

async fn schedule_callback(
    State(state): State<AppState>,
    Json(input): Json<ScheduleCallbackInput>,
) -> (StatusCode, Json<serde_json::Value>) {
    let callback_id = input.callback_id.clone();
    let run_id = input.run_id.clone();
    match store_call(state.store, move |store| {
        store.schedule_callback(&input.callback_id, &input.run_id, &input.target_url)
    })
    .await
    {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({"callback_id":callback_id,"run_id":run_id,"scheduled":true})),
        ),
        Err(HandlerError::Simulator(
            SimulatorError::PayloadConflict
            | SimulatorError::CallbackRunMismatch
            | SimulatorError::InvalidCallbackUrl,
        )) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"kind":"callback_schedule_rejected"})),
        ),
        Err(error) => internal_error(error),
    }
}

async fn deliver_callback(
    State(state): State<AppState>,
    Json(input): Json<DeliverCallbackInput>,
) -> (StatusCode, Json<serde_json::Value>) {
    let callback_id = input.callback_id.clone();
    let dispatch = match store_call(state.store.clone(), move |store| {
        store.begin_callback(&input.callback_id)
    })
    .await
    {
        Ok(value) => value,
        Err(HandlerError::Simulator(
            SimulatorError::CallbackNotScheduled | SimulatorError::CallbackRunMismatch,
        )) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"kind":"callback_not_scheduled"})),
            );
        }
        Err(error) => return internal_error(error),
    };
    let body = callback_event(&dispatch);
    let send_result = state
        .client
        .post(&dispatch.target_url)
        .json(&body)
        .send()
        .await;
    let (delivered, detail, response_status) = match send_result {
        Ok(response) if response.status().is_success() => {
            (true, None, Some(response.status().as_u16()))
        }
        Ok(response) => {
            let status = response.status().as_u16();
            (
                false,
                Some(format!("receiver returned HTTP {status}")),
                Some(status),
            )
        }
        Err(error) => (false, Some(format!("callback transport: {error}")), None),
    };
    let store = state.store;
    let id = dispatch.callback_id.clone();
    let attempt = dispatch.attempt;
    let recorded_detail = detail.clone();
    if let Err(error) = store_call(store, move |store| {
        store.finish_callback(&id, attempt, delivered, recorded_detail.as_deref())
    })
    .await
    {
        return internal_error(error);
    }
    if delivered {
        (
            StatusCode::OK,
            Json(
                serde_json::json!({"callback_id":callback_id,"attempt":attempt,"delivered":true,"http_status":response_status}),
            ),
        )
    } else {
        (
            StatusCode::BAD_GATEWAY,
            Json(
                serde_json::json!({"callback_id":callback_id,"attempt":attempt,"delivered":false,"http_status":response_status,"detail":detail}),
            ),
        )
    }
}

async fn callback_attempts(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    match store_call(state.store, move |store| {
        store.callback_attempts(&query.callback_id)
    })
    .await
    {
        Ok(value) => (
            StatusCode::OK,
            Json(
                serde_json::to_value(value)
                    .unwrap_or_else(|_| serde_json::json!({"kind":"serialization_error"})),
            ),
        ),
        Err(error) => internal_error(error),
    }
}

fn callback_event(dispatch: &CallbackDispatch) -> CallbackEvent {
    CallbackEvent {
        callback_id: dispatch.callback_id.clone(),
        attempt: dispatch.attempt,
        run_id: dispatch.run_id.clone(),
        key: dispatch.key.clone(),
        result: dispatch.result.clone(),
    }
}

async fn acknowledge_outcome(
    State(state): State<AppState>,
    Json(input): Json<OutcomeInput>,
) -> (StatusCode, Json<serde_json::Value>) {
    let outcome_id = input.outcome_id.clone();
    match store_call(state.store, move |store| {
        store.acknowledge_outcome(&input.outcome_id, &input.key, &input.outcome)
    })
    .await
    {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({"outcome_id":outcome_id,"acknowledged":true})),
        ),
        Err(HandlerError::Simulator(SimulatorError::PayloadConflict)) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"kind":"outcome_conflict"})),
        ),
        Err(error) => internal_error(error),
    }
}

async fn counts(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match store_call(state.store, |store| store.counts()).await {
        Ok(value) => (
            StatusCode::OK,
            Json(
                serde_json::to_value(value)
                    .unwrap_or_else(|_| serde_json::json!({"kind":"serialization_error"})),
            ),
        ),
        Err(error) => internal_error(error),
    }
}

fn internal_error(error: HandlerError) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"kind":"store_error","detail":error.to_string()})),
    )
}
