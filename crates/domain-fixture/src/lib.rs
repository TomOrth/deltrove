//! DEL-26's PostgreSQL-only transaction fixture, not a repository implementation.
//!
//! A successful command atomically advances a fixture generation, retains small
//! immutable candidate bytes, records a replayable outcome, and inserts an outbox
//! record. External CI dispatch and production object storage are separate tasks.

#![deny(missing_docs)]

use std::{
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool, Row,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use thiserror::Error;

const MAX_ATTEMPTS: u32 = 8;
const COMMAND_DEADLINE: Duration = Duration::from_secs(10);

/// A fixture-only publication command. All fields participate in replay identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublishCandidate {
    /// Repository scope, at most 128 UTF-8 bytes.
    pub repo_id: String,
    /// Fixture actor; compared with the repository's current authorized actor.
    pub actor_id: String,
    /// Stable caller-supplied key within repository and actor scope.
    pub request_key: String,
    /// Required current repository generation.
    pub expected_generation: i64,
    /// Required current recovery epoch.
    pub recovery_epoch: i64,
    /// Required current worker fencing token.
    pub fence_token: i64,
    /// Placeholder immutable content, at most 64 KiB; no production format implied.
    pub candidate_bytes: Vec<u8>,
}

/// Durable outcome returned verbatim for identical replayed commands.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Outcome {
    /// Stable fixture operation identifier, derived from the scoped request key.
    pub operation_id: String,
    /// Repository scope.
    pub repo_id: String,
    /// Fixture candidate identifier derived from placeholder bytes.
    pub candidate_id: String,
    /// Generation installed by this operation.
    pub generation: i64,
    /// Epoch under which this transition was authorized.
    pub recovery_epoch: i64,
    /// Worker token under which this transition was authorized.
    pub fence_token: i64,
}

/// Explicit failure categories for the fixture's retry/reconciliation boundary.
#[derive(Debug, Error)]
pub enum FixtureError {
    /// Input or schema name violates fixture bounds.
    #[error("invalid fixture input: {0}")]
    InvalidInput(&'static str),
    /// Repository does not exist in this isolated fixture schema.
    #[error("fixture repository not found")]
    NotFound,
    /// Current fixture policy denies this actor, including outcome replays.
    #[error("actor is not authorized")]
    PermissionDenied,
    /// An existing stable key was reused with different command fields.
    #[error("request key reused with different input")]
    KeyMismatch,
    /// Generation CAS failed or the stored generation differs from the request.
    #[error("stale generation")]
    StaleGeneration,
    /// A stale recovery epoch or worker token cannot publish.
    #[error("stale publication authority")]
    StaleAuthority,
    /// Serialization/deadlock retries reached the bounded attempt limit.
    #[error("transaction retry budget exhausted")]
    RetryExhausted,
    /// A bounded command timed out; callers must reconcile using the same key.
    #[error("command deadline exceeded; reconcile with the same request key")]
    Deadline,
    /// Commit acknowledgment was unavailable; callers must reconcile, not re-key.
    #[error("commit outcome unknown; reconcile with the same request key: {0}")]
    CommitUnknown(sqlx::Error),
    /// PostgreSQL rejected the command before a successful commit.
    #[error("database failure: {0}")]
    Database(#[from] sqlx::Error),
    /// Fixture command/outcome serialization failed.
    #[error("fixture encoding failure: {0}")]
    Encoding(#[from] serde_json::Error),
}

/// A bounded connection pool operating in one explicitly isolated fixture schema.
#[derive(Clone)]
pub struct DomainFixture {
    pool: PgPool,
    serialization_retries: Arc<AtomicU64>,
}

impl DomainFixture {
    /// Creates a new `del26_` schema and installs the fixture migration.
    ///
    /// Existing schemas are rejected rather than silently reused or overwritten.
    /// Use a disposable test database; this is not a production migration API.
    ///
    /// # Errors
    /// Returns an input error for unsafe schema names, or a PostgreSQL error.
    pub async fn create(database_url: &str, schema: &str) -> Result<Self, FixtureError> {
        validate_schema(schema)?;
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;
        let mut tx = admin.begin().await?;
        sqlx::query(&format!("CREATE SCHEMA {schema}"))
            .execute(&mut *tx)
            .await?;
        sqlx::query(&format!("SET LOCAL search_path TO {schema}, pg_catalog"))
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(include_str!("../migrations/0001_fixture.sql"))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        admin.close().await;
        Self::connect(database_url, schema).await
    }

    /// Connects to an existing fixture schema; it creates or migrates nothing.
    ///
    /// # Errors
    /// Returns invalid input or PostgreSQL connection/configuration errors.
    pub async fn connect(database_url: &str, schema: &str) -> Result<Self, FixtureError> {
        validate_schema(schema)?;
        let options = PgConnectOptions::from_str(database_url)?.application_name(schema);
        let schema = schema.to_owned();
        let pool = PgPoolOptions::new().max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(move |connection, _| {
                let search_path = format!("{schema}, pg_catalog");
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false), set_config('statement_timeout', '5s', false), set_config('lock_timeout', '2s', false)")
                        .bind(search_path).execute(connection).await?;
                    Ok(())
                })
            }).connect_with(options).await?;
        Ok(Self {
            pool,
            serialization_retries: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Exposes the isolated pool for independent fixture observers and fault tests.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Counts actual SQL serialization/deadlock retries, not logical outcomes.
    pub fn serialization_retries(&self) -> u64 {
        self.serialization_retries.load(Ordering::Relaxed)
    }

    /// Creates a tiny repository with one fixture-authorized actor and generation 0.
    ///
    /// # Errors
    /// Returns invalid input or PostgreSQL constraint/connection errors.
    pub async fn create_repository(&self, repo: &str, actor: &str) -> Result<(), FixtureError> {
        validate_id(repo)?;
        validate_id(actor)?;
        sqlx::query("INSERT INTO fixture_repositories (repo_id, authorized_actor) VALUES ($1, $2)")
            .bind(repo)
            .bind(actor)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Publishes or replays one command with bounded whole-transaction retries.
    ///
    /// Only known-aborted PostgreSQL serialization/deadlock failures are retried.
    /// A deadline or uncertain commit requires the caller to retry the SAME key.
    /// No external HTTP, CI, or filesystem effect occurs inside the SQL transaction.
    ///
    /// ```
    /// use domain_fixture::PublishCandidate;
    /// let command = PublishCandidate {
    ///     repo_id: "repo".into(), actor_id: "alice".into(), request_key: "request-1".into(),
    ///     expected_generation: 0, recovery_epoch: 1, fence_token: 1,
    ///     candidate_bytes: b"placeholder".to_vec(),
    /// };
    /// assert_eq!(command.expected_generation, 0);
    /// ```
    ///
    /// # Errors
    /// Returns typed policy, identity, generation, authority, budget, or SQL errors.
    pub async fn publish(&self, command: &PublishCandidate) -> Result<Outcome, FixtureError> {
        validate_command(command)?;
        let fingerprint = hash_bytes(&serde_json::to_vec(command)?);
        let operation_id = format!(
            "fixture-op-{}",
            hash_bytes(&serde_json::to_vec(&(
                &command.repo_id,
                &command.actor_id,
                &command.request_key
            ))?)
        );
        let candidate_id = format!("fixture-candidate-{}", hash_bytes(&command.candidate_bytes));
        tokio::time::timeout(COMMAND_DEADLINE, async {
            for attempt in 0..MAX_ATTEMPTS {
                match self
                    .publish_once(command, &fingerprint, &operation_id, &candidate_id)
                    .await
                {
                    Err(FixtureError::Database(ref error)) if retryable(error) => {
                        self.serialization_retries.fetch_add(1, Ordering::Relaxed);
                        if attempt + 1 == MAX_ATTEMPTS {
                            return Err(FixtureError::RetryExhausted);
                        }
                        tokio::time::sleep(Duration::from_millis(1 << attempt)).await;
                    }
                    result => return result,
                }
            }
            Err(FixtureError::RetryExhausted)
        })
        .await
        .map_err(|_| FixtureError::Deadline)?
    }

    async fn publish_once(
        &self,
        command: &PublishCandidate,
        fingerprint: &str,
        operation_id: &str,
        candidate_id: &str,
    ) -> Result<Outcome, FixtureError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL synchronous_commit TO on")
            .execute(&mut *tx)
            .await?;
        let repo = sqlx::query("SELECT authorized_actor, generation, recovery_epoch, fence_token FROM fixture_repositories WHERE repo_id=$1")
            .bind(&command.repo_id).fetch_optional(&mut *tx).await?.ok_or(FixtureError::NotFound)?;
        if repo.try_get::<String, _>("authorized_actor")? != command.actor_id {
            return Err(FixtureError::PermissionDenied);
        }
        if let Some(row) = sqlx::query("SELECT input_fingerprint, operation_id, repo_id, candidate_id, generation, recovery_epoch, fence_token FROM fixture_outcomes WHERE repo_id=$1 AND actor_id=$2 AND request_key=$3")
            .bind(&command.repo_id).bind(&command.actor_id).bind(&command.request_key)
            .fetch_optional(&mut *tx).await? {
            if row.try_get::<String, _>("input_fingerprint")? != fingerprint { return Err(FixtureError::KeyMismatch); }
            return Ok(Outcome {
                operation_id: row.try_get("operation_id")?, repo_id: row.try_get("repo_id")?,
                candidate_id: row.try_get("candidate_id")?, generation: row.try_get("generation")?,
                recovery_epoch: row.try_get("recovery_epoch")?, fence_token: row.try_get("fence_token")?,
            });
        }
        if repo.try_get::<i64, _>("generation")? != command.expected_generation {
            return Err(FixtureError::StaleGeneration);
        }
        if repo.try_get::<i64, _>("recovery_epoch")? != command.recovery_epoch
            || repo.try_get::<i64, _>("fence_token")? != command.fence_token
        {
            return Err(FixtureError::StaleAuthority);
        }
        let generation = command
            .expected_generation
            .checked_add(1)
            .ok_or(FixtureError::InvalidInput("generation exhausted"))?;
        let updated = sqlx::query("UPDATE fixture_repositories SET generation=$2, candidate_id=$3 WHERE repo_id=$1 AND generation=$4 AND recovery_epoch=$5 AND fence_token=$6 AND authorized_actor=$7")
            .bind(&command.repo_id).bind(generation).bind(candidate_id).bind(command.expected_generation)
            .bind(command.recovery_epoch).bind(command.fence_token).bind(&command.actor_id)
            .execute(&mut *tx).await?;
        if updated.rows_affected() != 1 {
            return Err(FixtureError::StaleGeneration);
        }
        sqlx::query("INSERT INTO fixture_candidates(candidate_id, payload) VALUES($1,$2) ON CONFLICT (candidate_id) DO NOTHING")
            .bind(candidate_id).bind(&command.candidate_bytes).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO fixture_outcomes(operation_id, repo_id, actor_id, request_key, input_fingerprint, candidate_id, generation, recovery_epoch, fence_token) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(operation_id).bind(&command.repo_id).bind(&command.actor_id).bind(&command.request_key)
            .bind(fingerprint).bind(candidate_id).bind(generation).bind(command.recovery_epoch).bind(command.fence_token)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO fixture_outbox(operation_id, repo_id, candidate_id, generation) VALUES($1,$2,$3,$4)")
            .bind(operation_id).bind(&command.repo_id).bind(candidate_id).bind(generation).execute(&mut *tx).await?;
        if let Err(error) = tx.commit().await {
            return Err(if retryable(&error) {
                FixtureError::Database(error)
            } else {
                FixtureError::CommitUnknown(error)
            });
        }
        Ok(Outcome {
            operation_id: operation_id.to_owned(),
            repo_id: command.repo_id.clone(),
            candidate_id: candidate_id.to_owned(),
            generation,
            recovery_epoch: command.recovery_epoch,
            fence_token: command.fence_token,
        })
    }
}

fn validate_schema(schema: &str) -> Result<(), FixtureError> {
    if !schema.starts_with("del26_")
        || schema.len() > 63
        || !schema
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(FixtureError::InvalidInput(
            "schema must be a del26_ identifier of at most 63 ASCII bytes",
        ));
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), FixtureError> {
    if value.is_empty() || value.len() > 128 {
        return Err(FixtureError::InvalidInput("IDs must contain 1–128 bytes"));
    }
    Ok(())
}

fn validate_command(command: &PublishCandidate) -> Result<(), FixtureError> {
    validate_id(&command.repo_id)?;
    validate_id(&command.actor_id)?;
    validate_id(&command.request_key)?;
    if command.candidate_bytes.len() > 65_536
        || command.expected_generation < 0
        || command.recovery_epoch <= 0
        || command.fence_token <= 0
    {
        return Err(FixtureError::InvalidInput(
            "invalid payload, generation, or authority bounds",
        ));
    }
    Ok(())
}

fn retryable(error: &sqlx::Error) -> bool {
    matches!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("40001" | "40P01")
    )
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_schema_injection_and_non_fixture_names() {
        for schema in ["public", "del26_x;drop schema public", "del26_upperCase"] {
            assert!(matches!(
                validate_schema(schema),
                Err(FixtureError::InvalidInput(_))
            ));
        }
        assert!(validate_schema("del26_test_123").is_ok());
    }
}
