//! Explicit real-PostgreSQL gate. Run with `--ignored`; no database is mocked.

use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use domain_fixture::{DomainFixture, FixtureError, Outcome, PublishCandidate};
use sqlx::{PgPool, Row};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::timeout,
};

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;
static NEXT_SCHEMA: AtomicU64 = AtomicU64::new(0);

struct Context {
    fixture: DomainFixture,
    database_url: String,
    schema: String,
}

impl Context {
    async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL is required for the real-PostgreSQL gate")?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let schema = format!(
            "del26_{}_{now}_{}",
            std::process::id(),
            NEXT_SCHEMA.fetch_add(1, Ordering::Relaxed)
        );
        let fixture = DomainFixture::create(&database_url, &schema).await?;
        fixture.create_repository("repo", "alice").await?;
        Ok(Self {
            fixture,
            database_url,
            schema,
        })
    }

    async fn cleanup(self) -> TestResult {
        self.fixture.pool().close().await;
        let admin = PgPool::connect(&self.database_url).await?;
        // The schema is generated here and validated by DomainFixture::create.
        sqlx::query(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .execute(&admin)
            .await?;
        admin.close().await;
        Ok(())
    }
}

fn command(key: &str, bytes: &[u8]) -> PublishCandidate {
    PublishCandidate {
        repo_id: "repo".into(),
        actor_id: "alice".into(),
        request_key: key.into(),
        expected_generation: 0,
        recovery_epoch: 1,
        fence_token: 1,
        candidate_bytes: bytes.to_vec(),
    }
}

// This observer reads committed SQL directly, independently of publish/replay code.
async fn assert_oracle(
    fixture: &DomainFixture,
    expected: &[(&PublishCandidate, &Outcome)],
) -> TestResult {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM fixture_outcomes")
        .fetch_one(fixture.pool())
        .await?;
    let outbox_count: i64 = sqlx::query_scalar("SELECT count(*) FROM fixture_outbox")
        .fetch_one(fixture.pool())
        .await?;
    assert_eq!(
        count,
        expected.len() as i64,
        "unexpected logical mutation count"
    );
    assert_eq!(
        outbox_count, count,
        "one atomic outbox record per logical mutation"
    );
    let generation: i64 =
        sqlx::query_scalar("SELECT generation FROM fixture_repositories WHERE repo_id='repo'")
            .fetch_one(fixture.pool())
            .await?;
    if generation != count {
        return Err("oracle detected an unrecorded or repeated generation transition".into());
    }
    for (input, acknowledged) in expected {
        let row = sqlx::query("SELECT o.*, c.payload, b.generation AS outbox_generation, b.candidate_id AS outbox_candidate FROM fixture_outcomes o JOIN fixture_candidates c USING(candidate_id) JOIN fixture_outbox b USING(operation_id) WHERE o.operation_id=$1")
            .bind(&acknowledged.operation_id).fetch_one(fixture.pool()).await?;
        assert_eq!(row.try_get::<String, _>("repo_id")?, input.repo_id);
        assert_eq!(row.try_get::<String, _>("actor_id")?, input.actor_id);
        assert_eq!(row.try_get::<String, _>("request_key")?, input.request_key);
        assert_eq!(row.try_get::<Vec<u8>, _>("payload")?, input.candidate_bytes);
        assert_eq!(
            row.try_get::<i64, _>("generation")?,
            input.expected_generation + 1
        );
        assert_eq!(
            row.try_get::<i64, _>("generation")?,
            acknowledged.generation
        );
        assert_eq!(
            row.try_get::<i64, _>("outbox_generation")?,
            acknowledged.generation
        );
        assert_eq!(
            row.try_get::<String, _>("candidate_id")?,
            acknowledged.candidate_id
        );
        assert_eq!(
            row.try_get::<String, _>("outbox_candidate")?,
            acknowledged.candidate_id
        );
        assert_eq!(
            row.try_get::<i64, _>("recovery_epoch")?,
            input.recovery_epoch
        );
        assert_eq!(row.try_get::<i64, _>("fence_token")?, input.fence_token);
    }
    Ok(())
}

#[tokio::test]
#[ignore = "real PostgreSQL required: tools/domain-fixture/test-postgres.sh"]
async fn oracle_rejects_a_deliberately_unrecorded_transition() -> TestResult {
    let context = Context::new().await?;
    let input = command("oracle-control", b"known authorized transition");
    let outcome = context.fixture.publish(&input).await?;
    sqlx::query("UPDATE fixture_repositories SET generation=2 WHERE repo_id='repo'")
        .execute(context.fixture.pool())
        .await?;
    assert!(
        assert_oracle(&context.fixture, &[(&input, &outcome)])
            .await
            .is_err()
    );
    context.cleanup().await
}

#[tokio::test]
#[ignore = "real PostgreSQL required: tools/domain-fixture/test-postgres.sh"]
async fn replay_and_changed_input_preserve_one_authorized_transition() -> TestResult {
    let context = Context::new().await?;
    let input = command("key", b"immutable fixture");
    let first = context.fixture.publish(&input).await?;
    assert_eq!(first, context.fixture.publish(&input).await?);
    let mut different = input.clone();
    different.candidate_bytes.push(0);
    assert!(matches!(
        context.fixture.publish(&different).await,
        Err(FixtureError::KeyMismatch)
    ));
    assert_oracle(&context.fixture, &[(&input, &first)]).await?;
    context.cleanup().await
}

#[tokio::test]
#[ignore = "real PostgreSQL required: tools/domain-fixture/test-postgres.sh"]
async fn competing_writers_force_actual_serialization_retry_then_revalidate_generation()
-> TestResult {
    let context = Context::new().await?;
    let first_input = command("writer-a", b"candidate a");
    let second_input = command("writer-b", b"candidate b");
    let mut blocker = context.fixture.pool().begin().await?;
    sqlx::query("SELECT repo_id FROM fixture_repositories WHERE repo_id='repo' FOR UPDATE")
        .fetch_one(&mut *blocker)
        .await?;
    let a_fixture = context.fixture.clone();
    let a_command = first_input.clone();
    let a = tokio::spawn(async move { a_fixture.publish(&a_command).await });
    let b_fixture = context.fixture.clone();
    let b_command = second_input.clone();
    let b = tokio::spawn(async move { b_fixture.publish(&b_command).await });
    timeout(Duration::from_secs(2), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock'")
                .bind(&context.schema).fetch_one(context.fixture.pool()).await?;
            if waiting == 2 { break Ok::<(), sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    blocker.commit().await?;
    let a = timeout(Duration::from_secs(10), a).await??;
    let b = timeout(Duration::from_secs(10), b).await??;
    assert!(
        context.fixture.serialization_retries() >= 1,
        "must observe an actual SQL 40001 retry"
    );
    match (a, b) {
        (Ok(outcome), Err(FixtureError::StaleGeneration)) => {
            assert_oracle(&context.fixture, &[(&first_input, &outcome)]).await?
        }
        (Err(FixtureError::StaleGeneration), Ok(outcome)) => {
            assert_oracle(&context.fixture, &[(&second_input, &outcome)]).await?
        }
        other => panic!("expected exactly one authorized winner: {other:?}"),
    }
    context.cleanup().await
}

#[tokio::test]
#[ignore = "real PostgreSQL required: tools/domain-fixture/test-postgres.sh"]
async fn duplicate_concurrent_commands_return_the_same_durable_outcome() -> TestResult {
    let context = Context::new().await?;
    let input = command("same-key", b"one candidate");
    let (a, b) = tokio::join!(
        context.fixture.publish(&input),
        context.fixture.publish(&input)
    );
    let a = a?;
    let b = b?;
    assert_eq!(a, b);
    assert_oracle(&context.fixture, &[(&input, &a)]).await?;
    context.cleanup().await
}

#[tokio::test]
#[ignore = "real PostgreSQL required: tools/domain-fixture/test-postgres.sh"]
async fn outbox_failure_rolls_back_candidate_outcome_and_generation_together() -> TestResult {
    let context = Context::new().await?;
    sqlx::raw_sql("CREATE FUNCTION reject_outbox() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected outbox failure'; END $$; CREATE TRIGGER reject_outbox BEFORE INSERT ON fixture_outbox FOR EACH ROW EXECUTE FUNCTION reject_outbox();")
        .execute(context.fixture.pool()).await?;
    assert!(matches!(
        context
            .fixture
            .publish(&command("rollback", b"not published"))
            .await,
        Err(FixtureError::Database(_))
    ));
    assert_oracle(&context.fixture, &[]).await?;
    let candidates: i64 = sqlx::query_scalar("SELECT count(*) FROM fixture_candidates")
        .fetch_one(context.fixture.pool())
        .await?;
    assert_eq!(
        candidates, 0,
        "uncommitted candidate insert must roll back too"
    );
    context.cleanup().await
}

#[tokio::test]
#[ignore = "real PostgreSQL required: tools/domain-fixture/test-postgres.sh"]
async fn stale_authority_and_revoked_actor_cannot_publish_or_replay() -> TestResult {
    let context = Context::new().await?;
    for (epoch, fence) in [(2, 1), (1, 2)] {
        let mut stale = command("stale", b"rejected");
        stale.recovery_epoch = epoch;
        stale.fence_token = fence;
        assert!(matches!(
            context.fixture.publish(&stale).await,
            Err(FixtureError::StaleAuthority)
        ));
    }
    let input = command("accepted", b"allowed");
    let outcome = context.fixture.publish(&input).await?;
    sqlx::query("UPDATE fixture_repositories SET authorized_actor='bob' WHERE repo_id='repo'")
        .execute(context.fixture.pool())
        .await?;
    assert!(matches!(
        context.fixture.publish(&input).await,
        Err(FixtureError::PermissionDenied)
    ));
    assert_oracle(&context.fixture, &[(&input, &outcome)]).await?;
    context.cleanup().await
}

#[tokio::test]
#[ignore = "real PostgreSQL required: tools/domain-fixture/test-postgres.sh"]
async fn kill_after_commit_before_response_reconciles_with_a_durable_acknowledgment_ledger()
-> TestResult {
    let context = Context::new().await?;
    let input = command("lost-response", b"durable placeholder");
    let mut child = Command::new(env!("CARGO_BIN_EXE_domain-fixture"))
        .arg("--hold-after-commit")
        .env("DATABASE_URL", &context.database_url)
        .env("DELTROVE_FIXTURE_SCHEMA", &context.schema)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("missing stdin")?;
    stdin.write_all(&serde_json::to_vec(&input)?).await?;
    stdin.shutdown().await?;
    drop(stdin);
    let stderr = child.stderr.take().ok_or("missing stderr")?;
    let mut reader = BufReader::new(stderr);
    let mut marker = String::new();
    timeout(Duration::from_secs(10), reader.read_line(&mut marker)).await??;
    assert_eq!(marker, "DEL26_COMMITTED_RESPONSE_WITHHELD\n");
    child.kill().await?;
    let mut response = Vec::new();
    child
        .stdout
        .take()
        .ok_or("missing stdout")?
        .read_to_end(&mut response)
        .await?;
    assert!(
        response.is_empty(),
        "client must not have received an outcome before the crash"
    );
    let retry = context.fixture.publish(&input).await?;
    let ledger = std::env::temp_dir().join(format!("{}-ack.jsonl", context.schema));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ledger)?;
    writeln!(file, "{}", serde_json::to_string(&retry)?)?;
    file.sync_all()?;
    drop(file);
    let reopened = DomainFixture::connect(&context.database_url, &context.schema).await?;
    let persisted_ack: Outcome = serde_json::from_str(fs::read_to_string(&ledger)?.trim())?;
    assert_eq!(retry, reopened.publish(&input).await?);
    assert_oracle(&reopened, &[(&input, &persisted_ack)]).await?;
    reopened.pool().close().await;
    fs::remove_file(ledger)?;
    context.cleanup().await
}
