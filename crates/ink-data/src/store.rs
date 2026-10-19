//! Versioned key-value entries in `ink-store.db`, served by one connection on one thread per process.
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Instant,
};

use ink_runtime::{NativeCall, Responder};
use serde_json::json;

use crate::{Cancellation, DatabaseBackend};

struct Job {
    session: u64,
    call: NativeCall,
    responder: Responder,
}

pub(crate) struct Store {
    backend: Arc<dyn DatabaseBackend>,
    jobs: Mutex<Option<mpsc::Sender<Job>>>,
    sessions: Mutex<Vec<(u64, Responder)>>,
}

impl Store {
    pub(crate) fn new(backend: Arc<dyn DatabaseBackend>) -> Arc<Self> {
        Arc::new(Self {
            backend,
            jobs: Mutex::new(None),
            sessions: Mutex::new(Vec::new()),
        })
    }

    pub(crate) fn register(&self, session: u64, responder: Responder) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|(_, responder)| responder.is_open());
        sessions.push((session, responder));
    }

    pub(crate) fn call(self: &Arc<Self>, session: u64, call: NativeCall, responder: Responder) {
        let id = call.id;
        let job = Job {
            session,
            call,
            responder: responder.clone(),
        };
        let mut jobs = self.jobs.lock().unwrap();
        let sender = jobs.get_or_insert_with(|| {
            let (sender, receiver) = mpsc::channel();
            let store = self.clone();
            thread::Builder::new()
                .name("ink-store".into())
                .spawn(move || store.run(receiver))
                .expect("start store thread");
            sender
        });
        if sender.send(job).is_err() {
            responder.reject(id, "unavailable", "Store is unavailable", true);
        }
    }

    fn run(&self, jobs: mpsc::Receiver<Job>) {
        let mut connection: Option<Box<dyn crate::DatabaseConnection>> = None;
        let cancellation = Cancellation::default();
        for job in jobs {
            if !job.responder.is_open() {
                continue;
            }
            let id = job.call.id;
            let result = match &mut connection {
                Some(connection) => connection.execute(
                    &job.call,
                    &cancellation,
                    Instant::now() + job.call.timeout,
                ),
                None => match self.backend.connect(true) {
                    Ok(opened) => connection.insert(opened).execute(
                        &job.call,
                        &cancellation,
                        Instant::now() + job.call.timeout,
                    ),
                    Err(error) => Err(error),
                },
            };
            match result {
                Ok(value) => {
                    job.responder.resolve(id, &value);
                    if matches!(job.call.operation.as_str(), "write" | "replace")
                        && serde_json::from_str::<serde_json::Value>(&value)
                            .is_ok_and(|result| result["committed"] == true)
                        && let Some(key) = job
                            .call
                            .payload
                            .get("key")
                            .and_then(serde_json::Value::as_str)
                    {
                        self.notify(job.session, key);
                    }
                }
                Err(failure) => {
                    failure.send(&job.responder, id);
                }
            }
        }
    }

    fn notify(&self, writer: u64, key: &str) {
        let message = json!({"type": "store-changed", "key": key}).to_string();
        let mut sessions = self.sessions.lock().unwrap();
        sessions
            .retain(|(session, responder)| *session == writer || responder.send(message.clone()));
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) mod embedded {
    use crate::{
        Cancellation, DatabaseConnection, Failure, MAX_SAFE_INTEGER, integer, string, utf16_length,
    };
    use ink_runtime::NativeCall;
    use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
    use serde_json::json;
    use std::{
        path::Path,
        time::{Duration, Instant},
    };
    const MAX_RESULT_BYTES: usize = 1024 * 1024;
    pub(crate) struct Store(Connection);
    impl Store {
        pub(crate) fn new(path: &Path) -> Result<Box<dyn DatabaseConnection>, Failure> {
            open(path)
                .map(|connection| Box::new(Self(connection)) as Box<dyn DatabaseConnection>)
                .map_err(|error| Failure::unexpected(format!("Could not open store: {error}")))
        }
    }
    impl DatabaseConnection for Store {
        fn execute(
            &mut self,
            call: &NativeCall,
            _: &Cancellation,
            _: Instant,
        ) -> Result<String, Failure> {
            execute(&mut self.0, call)
        }
        fn discard(&mut self, _: &str) {}
    }
    fn open(path: &Path) -> rusqlite::Result<Connection> {
        if let Some(directory) = path.parent() {
            let _ = std::fs::create_dir_all(directory);
        }
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        // WAL with full sync keeps committed writes durable while needing fewer syncs than a rollback journal.
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        match version {
            0 => connection.execute_batch(
                "BEGIN; CREATE TABLE IF NOT EXISTS entries (key TEXT PRIMARY KEY, revision INTEGER NOT NULL, version INTEGER NOT NULL, value TEXT NOT NULL); PRAGMA user_version = 1; COMMIT;",
            )?,
            1 => {}
            _ => return Err(rusqlite::Error::InvalidQuery),
        }
        Ok(connection)
    }

    fn execute(connection: &mut Connection, call: &NativeCall) -> Result<String, Failure> {
        let key = string(&call.payload, "key")?;
        if key.is_empty() || utf16_length(key) > 256 {
            return Err(Failure::unexpected("Invalid store key"));
        }
        match call.operation.as_str() {
            "get" => {
                let entry = connection
                    .query_row("SELECT revision, version, value FROM entries WHERE key = ?1", [key], |row| {
                        Ok(json!({"revision": row.get::<_, i64>(0)?, "version": row.get::<_, i64>(1)?, "value": row.get::<_, String>(2)?}))
                    })
                    .optional()
                    .map_err(Failure::unexpected)?;
                Ok(entry.map_or_else(|| "null".to_owned(), |entry| entry.to_string()))
            }
            operation @ ("write" | "replace") => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(Failure::unexpected)?;
                let current = transaction
                    .query_row(
                        "SELECT revision, version FROM entries WHERE key = ?1",
                        [key],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .optional()
                    .map_err(Failure::unexpected)?;
                let revision = current.map_or(0, |(revision, _)| revision);
                if operation == "write" && revision != integer(&call.payload, "revision")? {
                    return Ok(json!({"committed": false}).to_string());
                }
                let version = integer(&call.payload, "version")?;
                if !(1..=MAX_SAFE_INTEGER).contains(&version) {
                    return Err(Failure::unexpected("Invalid store version"));
                }
                if current.map_or(0, |(_, version)| version) > version {
                    return Err(Failure::unexpected(format!(
                        "Store {key} uses a newer version"
                    )));
                }
                if revision >= MAX_SAFE_INTEGER {
                    return Err(Failure::unexpected("Store revision is exhausted"));
                }
                let value = string(&call.payload, "value")?;
                // Reserve the widest revision so later writes cannot make this value unreadable.
                let widest =
                    json!({"revision": MAX_SAFE_INTEGER, "version": version, "value": value})
                        .to_string();
                let envelope =
                    json!({"type": "result", "id": MAX_SAFE_INTEGER, "value": widest}).to_string();
                if envelope.len() > MAX_RESULT_BYTES {
                    return Err(Failure::unexpected("Store value is too large"));
                }
                transaction
                    .execute(
                        "INSERT OR REPLACE INTO entries (key, revision, version, value) VALUES (?1, ?2, ?3, ?4)",
                        params![key, revision + 1, version, value],
                    )
                    .map_err(Failure::unexpected)?;
                transaction.commit().map_err(Failure::unexpected)?;
                Ok(json!({"committed": true}).to_string())
            }
            operation => Err(Failure::protocol(format!(
                "Unknown store operation: {operation}"
            ))),
        }
    }
}
