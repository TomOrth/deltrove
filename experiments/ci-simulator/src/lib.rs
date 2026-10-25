//! Durable fake CI provider and process fault controls for the F1 fixture.
#![deny(missing_docs)]

use std::{path::Path, sync::Mutex};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A stable logical CI identity scoped to a repository operation and candidate.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct RequestKey {
    /// Fixture repository identifier.
    pub repo_id: String,
    /// Domain operation identifier.
    pub operation_id: String,
    /// Named check.
    pub check_name: String,
    /// Immutable candidate identifier.
    pub candidate_id: String,
}

/// Immutable data sent to the provider for a logical key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestPayload {
    /// Digest of the candidate fixture content.
    pub candidate_digest: String,
    /// Command label used by the fake provider.
    pub command: String,
}

/// Submission request accepted by the simulator.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubmitRequest {
    /// Stable logical identity.
    pub key: RequestKey,
    /// Immutable provider payload.
    pub payload: RequestPayload,
}

/// Submission disposition returned to a caller.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubmitDisposition {
    /// Provider durably accepted the request or reused the existing run.
    Accepted {
        /// Stable identity of the physical execution.
        run_id: String,
        /// Whether an earlier execution was reused.
        deduplicated: bool,
    },
    /// Provider accepted the request but the simulated response was lost.
    TransportAmbiguous {
        /// Physical execution committed before the response was lost.
        run_id: String,
        /// Stable description of the simulated transport failure.
        reason: String,
    },
}

/// Durable logical lookup result. Unknown does not prove that no request was accepted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reconciliation {
    /// Durable request record exists.
    Found {
        /// Physical executions currently associated with the stable key.
        run_ids: Vec<String>,
        /// Latest durable execution status.
        status: String,
    },
    /// No row is currently visible; callers must retain uncertainty where acceptance was ambiguous.
    Unknown,
}

/// Errors returned by persistent simulator operations.
#[derive(Debug, Error)]
pub enum SimulatorError {
    /// SQLite operation failed.
    #[error("simulator store: {0}")]
    Store(#[from] rusqlite::Error),
    /// Reused logical key carries different immutable request data.
    #[error("stable request key already has a conflicting payload")]
    PayloadConflict,
    /// The database mutex was poisoned after a panic.
    #[error("simulator store lock is poisoned")]
    Poisoned,
    /// Callback delivery was requested without a durable schedule.
    #[error("callback is not scheduled")]
    CallbackNotScheduled,
    /// Callback schedule does not name a known physical execution.
    #[error("callback is not bound to a known physical run")]
    CallbackRunMismatch,
    /// Callback URL is malformed or not HTTP(S).
    #[error("callback target URL must be a valid HTTP or HTTPS URL")]
    InvalidCallbackUrl,
    /// Callback attempt was missing or was already finalized.
    #[error("callback attempt is missing or already finalized")]
    CallbackAttemptMissing,
}

/// Durable provider store with optional provider-side idempotency.
pub struct SimulatorStore {
    connection: Mutex<Connection>,
    dedup_enabled: bool,
}

impl SimulatorStore {
    /// Open or create the durable store at `path`, enabling SQLite FULL synchronous mode.
    pub fn open(path: impl AsRef<Path>, dedup_enabled: bool) -> Result<Self, SimulatorError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS submissions (
                attempt_id TEXT PRIMARY KEY, key_json TEXT NOT NULL, payload_json TEXT NOT NULL,
                run_id TEXT NOT NULL, disposition TEXT NOT NULL, submitted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS executions (
                run_id TEXT PRIMARY KEY, key_json TEXT NOT NULL, payload_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'accepted', executed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS callbacks (
                callback_id TEXT NOT NULL, attempt INTEGER NOT NULL, run_id TEXT NOT NULL,
                delivered INTEGER NOT NULL DEFAULT 0, delivered_at TEXT, status TEXT NOT NULL,
                detail TEXT,
                PRIMARY KEY(callback_id, attempt)
             );
             CREATE TABLE IF NOT EXISTS callback_queue (
                callback_id TEXT PRIMARY KEY, run_id TEXT NOT NULL, key_json TEXT NOT NULL,
                target_url TEXT NOT NULL,
                scheduled_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS client_outcomes (
                outcome_id TEXT PRIMARY KEY, key_json TEXT NOT NULL, outcome_json TEXT NOT NULL,
                acknowledged_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
            dedup_enabled,
        })
    }

    /// Submit a request and persist separate submission and physical execution records.
    pub fn submit(
        &self,
        request: &SubmitRequest,
        drop_response: bool,
    ) -> Result<SubmitDisposition, SimulatorError> {
        let mut db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let key_json =
            serde_json::to_string(&request.key).map_err(|_| SimulatorError::PayloadConflict)?;
        let payload_json =
            serde_json::to_string(&request.payload).map_err(|_| SimulatorError::PayloadConflict)?;
        let existing: Option<(String, String)> = tx.query_row(
            "SELECT payload_json, run_id FROM executions WHERE key_json = ?1 ORDER BY executed_at LIMIT 1",
            [&key_json], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        if let Some((existing_payload, _)) = &existing
            && existing_payload != &payload_json
        {
            return Err(SimulatorError::PayloadConflict);
        }
        let reused = self.dedup_enabled && existing.is_some();
        let run_id = if reused {
            existing.map(|(_, id)| id).unwrap_or_default()
        } else {
            uuid::Uuid::new_v4().to_string()
        };
        let attempt_id = uuid::Uuid::new_v4().to_string();
        let disposition = if reused { "deduplicated" } else { "accepted" };
        tx.execute("INSERT INTO submissions(attempt_id,key_json,payload_json,run_id,disposition) VALUES (?1,?2,?3,?4,?5)",
            params![attempt_id, key_json, payload_json, run_id, disposition])?;
        if !reused {
            tx.execute(
                "INSERT INTO executions(run_id,key_json,payload_json) VALUES (?1,?2,?3)",
                params![run_id, key_json, payload_json],
            )?;
        }
        tx.commit()?;
        if drop_response {
            Ok(SubmitDisposition::TransportAmbiguous {
                run_id,
                reason: "response_dropped_after_accept".into(),
            })
        } else {
            Ok(SubmitDisposition::Accepted {
                run_id,
                deduplicated: reused,
            })
        }
    }

    /// Reconcile durable provider state by exact stable key.
    pub fn reconcile(&self, key: &RequestKey) -> Result<Reconciliation, SimulatorError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let key_json = serde_json::to_string(key).map_err(|_| SimulatorError::PayloadConflict)?;
        let mut statement = db.prepare(
            "SELECT run_id,status FROM executions WHERE key_json=?1 ORDER BY executed_at",
        )?;
        let rows = statement.query_map([key_json], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let found: Vec<_> = rows.collect::<Result<_, _>>()?;
        if found.is_empty() {
            return Ok(Reconciliation::Unknown);
        }
        let run_ids = found.iter().map(|(id, _)| id.clone()).collect();
        let status = found
            .last()
            .map(|(_, status)| status.clone())
            .unwrap_or_else(|| "accepted".into());
        Ok(Reconciliation::Found { run_ids, status })
    }

    /// Return durable counts, including submission attempts, physical executions, and callback attempts.
    pub fn counts(&self) -> Result<StoreCounts, SimulatorError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let count = |table: &str| -> Result<u64, rusqlite::Error> {
            db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
        };
        Ok(StoreCounts {
            submission_attempts: count("submissions")?,
            physical_executions: count("executions")?,
            callback_attempts: count("callbacks")?,
            logical_outcomes: count("client_outcomes")?,
        })
    }

    /// Begin a callback delivery attempt after validating its durable schedule and run binding.
    pub fn begin_callback(&self, callback_id: &str) -> Result<CallbackDispatch, SimulatorError> {
        let mut db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (run_id, key_json, target_url): (String, String, String) = tx
            .query_row(
                "SELECT run_id,key_json,target_url FROM callback_queue WHERE callback_id=?1",
                [callback_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or(SimulatorError::CallbackNotScheduled)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM executions WHERE run_id=?1 AND key_json=?2)",
            params![run_id, key_json],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(SimulatorError::CallbackRunMismatch);
        }
        let attempt: u32 = tx.query_row(
            "SELECT COALESCE(MAX(attempt),0)+1 FROM callbacks WHERE callback_id=?1",
            [callback_id],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO callbacks(callback_id,attempt,run_id,delivered,status) VALUES (?1,?2,?3,0,'pending')",
            params![callback_id, attempt, run_id],
        )?;
        let status: String = tx.query_row(
            "SELECT status FROM executions WHERE run_id=?1",
            [&run_id],
            |row| row.get(0),
        )?;
        tx.commit()?;
        let key = serde_json::from_str(&key_json).map_err(|_| SimulatorError::PayloadConflict)?;
        Ok(CallbackDispatch {
            callback_id: callback_id.to_owned(),
            attempt,
            run_id,
            key,
            result: status,
            target_url,
        })
    }

    /// Finish a previously recorded delivery attempt, retaining failures for reconciliation/retry.
    pub fn finish_callback(
        &self,
        callback_id: &str,
        attempt: u32,
        delivered: bool,
        detail: Option<&str>,
    ) -> Result<(), SimulatorError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let changed = db.execute(
            "UPDATE callbacks SET delivered=?3, status=?4, detail=?5,
             delivered_at=CASE WHEN ?3=1 THEN CURRENT_TIMESTAMP ELSE NULL END
             WHERE callback_id=?1 AND attempt=?2 AND status='pending'",
            params![
                callback_id,
                attempt,
                i64::from(delivered),
                if delivered { "delivered" } else { "failed" },
                detail
            ],
        )?;
        if changed == 0 {
            return Err(SimulatorError::CallbackAttemptMissing);
        }
        Ok(())
    }

    /// Read persisted delivery attempts for one stable callback identity.
    pub fn callback_attempts(
        &self,
        callback_id: &str,
    ) -> Result<Vec<CallbackAttempt>, SimulatorError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let mut statement = db.prepare(
            "SELECT callback_id,attempt,run_id,status,delivered,detail FROM callbacks WHERE callback_id=?1 ORDER BY attempt",
        )?;
        let rows = statement.query_map([callback_id], |row| {
            Ok(CallbackAttempt {
                callback_id: row.get(0)?,
                attempt: row.get(1)?,
                run_id: row.get(2)?,
                status: row.get(3)?,
                delivered: row.get::<_, i64>(4)? == 1,
                detail: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(SimulatorError::from)
    }

    /// Durably queue a callback so tests can delay delivery across process restarts.
    pub fn schedule_callback(
        &self,
        callback_id: &str,
        run_id: &str,
        target_url: &str,
    ) -> Result<(), SimulatorError> {
        let parsed =
            reqwest::Url::parse(target_url).map_err(|_| SimulatorError::InvalidCallbackUrl)?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(SimulatorError::InvalidCallbackUrl);
        }
        let mut db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let execution: Option<String> = tx
            .query_row(
                "SELECT key_json FROM executions WHERE run_id=?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()?;
        let key_json = execution.ok_or(SimulatorError::CallbackRunMismatch)?;
        let existing: Option<(String, String, String)> = tx
            .query_row(
                "SELECT run_id,key_json,target_url FROM callback_queue WHERE callback_id=?1",
                [callback_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if existing.as_ref().is_some_and(|value| {
            value != &(run_id.to_owned(), key_json.clone(), target_url.to_owned())
        }) {
            return Err(SimulatorError::PayloadConflict);
        }
        tx.execute(
            "INSERT OR IGNORE INTO callback_queue(callback_id,run_id,key_json,target_url) VALUES (?1,?2,?3,?4)",
            params![callback_id, run_id, key_json, target_url],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Persist an independent fixture-client acknowledgment for a logical outcome.
    pub fn acknowledge_outcome(
        &self,
        outcome_id: &str,
        key: &RequestKey,
        outcome: &str,
    ) -> Result<(), SimulatorError> {
        let db = self
            .connection
            .lock()
            .map_err(|_| SimulatorError::Poisoned)?;
        let key_json = serde_json::to_string(key).map_err(|_| SimulatorError::PayloadConflict)?;
        let proposed = (key_json.clone(), outcome.to_owned());
        let existing: Option<(String, String)> = db
            .query_row(
                "SELECT key_json,outcome_json FROM client_outcomes WHERE outcome_id=?1",
                [outcome_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if existing.as_ref().is_some_and(|stored| stored != &proposed) {
            return Err(SimulatorError::PayloadConflict);
        }
        db.execute(
            "INSERT OR IGNORE INTO client_outcomes(outcome_id,key_json,outcome_json) VALUES (?1,?2,?3)",
            params![outcome_id, key_json, outcome],
        )?;
        Ok(())
    }
}

/// Persisted callback details for one delivery attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallbackDispatch {
    /// Stable callback identity across retries.
    pub callback_id: String,
    /// This delivery attempt number.
    pub attempt: u32,
    /// Physical execution that produced the callback.
    pub run_id: String,
    /// Stable logical request key.
    pub key: RequestKey,
    /// Provider result reported to the callback receiver.
    pub result: String,
    /// Persisted destination for this callback.
    pub target_url: String,
}

/// Durable status for a callback delivery attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallbackAttempt {
    /// Stable callback identity across retries.
    pub callback_id: String,
    /// Monotonic delivery attempt number for this callback identity.
    pub attempt: u32,
    /// Physical execution associated with this callback.
    pub run_id: String,
    /// `pending`, `failed`, or `delivered`.
    pub status: String,
    /// Whether a receiver returned a successful 2xx response.
    pub delivered: bool,
    /// Failure detail when the attempt did not receive a 2xx response.
    pub detail: Option<String>,
}

/// Counts derived from durable records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoreCounts {
    /// Number of persisted submission attempts.
    pub submission_attempts: u64,
    /// Number of persisted physical provider executions.
    pub physical_executions: u64,
    /// Number of persisted callback delivery attempts.
    pub callback_attempts: u64,
    /// Number of persisted fixture-client logical acknowledgments.
    pub logical_outcomes: u64,
}
