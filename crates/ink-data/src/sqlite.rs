//! Read-only query workers, shared by embedded and platform SQLite backends.
use crate::{Cancellation, DatabaseBackend, DatabaseConnection, Failure};
use ink_runtime::{NativeCall, Responder};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Instant,
};

struct Job {
    call: NativeCall,
    cancellation: Cancellation,
    deadline: Instant,
}

pub(crate) struct Sqlite {
    backend: Arc<dyn DatabaseBackend>,
    responder: Responder,
    jobs: Option<mpsc::Sender<Job>>,
    pending: Arc<Mutex<HashMap<u64, Cancellation>>>,
}
impl Sqlite {
    pub(crate) fn new(backend: Arc<dyn DatabaseBackend>, responder: Responder) -> Self {
        Self {
            backend,
            responder,
            jobs: None,
            pending: Arc::default(),
        }
    }
    pub(crate) fn call(&mut self, call: NativeCall) {
        let cancellation = match self.backend.cancellation() {
            Ok(cancellation) => cancellation,
            Err(error) => {
                error.send(&self.responder, call.id);
                return;
            }
        };
        self.pending
            .lock()
            .unwrap()
            .insert(call.id, cancellation.clone());
        let deadline = Instant::now() + call.timeout;
        let jobs = self.jobs.get_or_insert_with(|| {
            let (sender, receiver) = mpsc::channel();
            let worker = Worker {
                backend: self.backend.clone(),
                responder: self.responder.clone(),
                pending: self.pending.clone(),
            };
            thread::Builder::new()
                .name("ink-sqlite".into())
                .spawn(move || worker.run(receiver))
                .expect("start SQLite thread");
            sender
        });
        let id = call.id;
        if jobs
            .send(Job {
                call,
                cancellation,
                deadline,
            })
            .is_err()
        {
            self.pending.lock().unwrap().remove(&id);
            self.responder
                .reject(id, "unavailable", "Database adapter is stopped", false);
        }
    }
    pub(crate) fn cancel(&mut self, id: u64) -> bool {
        let cancellation = self.pending.lock().unwrap().get(&id).cloned();
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
            true
        } else {
            false
        }
    }
}
impl Drop for Sqlite {
    fn drop(&mut self) {
        let pending: Vec<_> = self.pending.lock().unwrap().values().cloned().collect();
        for cancellation in pending {
            cancellation.cancel();
        }
    }
}
struct Worker {
    backend: Arc<dyn DatabaseBackend>,
    responder: Responder,
    pending: Arc<Mutex<HashMap<u64, Cancellation>>>,
}
impl Worker {
    fn run(self, jobs: mpsc::Receiver<Job>) {
        let mut connection: Option<Box<dyn DatabaseConnection>> = None;
        for job in jobs {
            let id = job.call.id;
            let result = if job.cancellation.is_cancelled() {
                None
            } else if Instant::now() >= job.deadline {
                Some(Err(Failure(
                    "timeout",
                    "Native request timed out".into(),
                    true,
                )))
            } else {
                Some(match &mut connection {
                    Some(connection) => {
                        connection.execute(&job.call, &job.cancellation, job.deadline)
                    }
                    None => match self.backend.connect(false) {
                        Ok(opened) => connection.insert(opened).execute(
                            &job.call,
                            &job.cancellation,
                            job.deadline,
                        ),
                        Err(error) => Err(error),
                    },
                })
            };
            self.pending.lock().unwrap().remove(&id);
            let opened = match result {
                Some(Ok(value)) if !job.cancellation.is_cancelled() => {
                    let delivered = self.responder.resolve(id, &value);
                    (job.call.operation == "open" && !delivered).then_some(value)
                }
                Some(Ok(value)) => (job.call.operation == "open").then_some(value),
                Some(Err(error)) if !job.cancellation.is_cancelled() => {
                    error.send(&self.responder, id);
                    None
                }
                _ => None,
            };
            if let Some(id) = opened
                && let Some(connection) = &mut connection
            {
                connection.discard(&id);
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) mod embedded {
    use crate::{
        AssetOpener, Cancellation, DatabaseConnection, Failure, MAX_SAFE_INTEGER, string,
        utf16_length,
    };
    use ink_runtime::NativeCall;
    use rusqlite::{Connection, OpenFlags, types::ValueRef};
    use serde_json::Value;
    use std::{
        collections::HashMap,
        fmt::Write as _,
        path::{Path, PathBuf},
        sync::{
            Mutex,
            atomic::{AtomicU64, Ordering},
        },
        time::Instant,
    };
    const MAX_DATABASES: usize = 8;
    const MAX_ROWS: usize = 10_000;
    const MAX_RESULT_BYTES: usize = 400_000;
    static INSTALL: Mutex<()> = Mutex::new(());
    static NEXT_DATABASE: AtomicU64 = AtomicU64::new(1);
    pub(crate) struct Databases {
        files: PathBuf,
        assets: AssetOpener,
        databases: HashMap<String, Connection>,
    }
    impl Databases {
        pub(crate) fn new(files: PathBuf, assets: AssetOpener) -> Box<dyn DatabaseConnection> {
            Box::new(Self {
                files,
                assets,
                databases: HashMap::new(),
            })
        }
    }
    impl DatabaseConnection for Databases {
        fn execute(
            &mut self,
            call: &NativeCall,
            cancellation: &Cancellation,
            deadline: Instant,
        ) -> Result<String, Failure> {
            let payload = &call.payload;
            match call.operation.as_str() {
                "open" => {
                    if self.databases.len() >= MAX_DATABASES {
                        return Err(Failure::unexpected("Too many open databases"));
                    }
                    let asset = string(payload, "asset")?;
                    let asset = asset.strip_prefix("asset://").unwrap_or(asset);
                    if !imported_database(asset) {
                        return Err(Failure::unexpected("Expected an imported database asset"));
                    }
                    let destination = self.files.join(asset);
                    install(&self.assets, asset, &destination).map_err(Failure::unexpected)?;
                    if cancellation.is_cancelled() {
                        return Err(Failure::unexpected("Cancelled"));
                    }
                    let connection = Connection::open_with_flags(
                        &destination,
                        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
                    )
                    .map_err(Failure::unexpected)?;
                    let id = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed).to_string();
                    self.databases.insert(id.clone(), connection);
                    Ok(id)
                }
                "query" => {
                    let database = string(payload, "id")?;
                    let connection = self
                        .databases
                        .get(database)
                        .ok_or_else(|| Failure::unexpected("Database is closed"))?;
                    let sql = string(payload, "sql")?;
                    if utf16_length(sql) > 65_536 {
                        return Err(Failure::unexpected("SQL is too long"));
                    }
                    if !read_only(sql) {
                        return Err(Failure::unexpected(
                            "Read-only queries must start with SELECT, WITH or EXPLAIN",
                        ));
                    }
                    let parameters = payload
                        .get("parameters")
                        .and_then(Value::as_array)
                        .ok_or_else(|| Failure::unexpected("No array value for parameters"))?;
                    let cancelled = cancellation.clone();
                    connection.progress_handler(
                        1_000,
                        Some(move || cancelled.is_cancelled() || Instant::now() >= deadline),
                    );
                    let result = query(connection, sql, parameters);
                    connection.progress_handler(0, None::<fn() -> bool>);
                    result.map_err(|failure| {
                        if Instant::now() >= deadline && !cancellation.is_cancelled() {
                            Failure("timeout", "Native request timed out".into(), true)
                        } else {
                            failure
                        }
                    })
                }
                "close" => {
                    self.databases.remove(string(payload, "id")?);
                    Ok(String::new())
                }
                _ => Err(Failure::protocol("Unknown database operation")),
            }
        }
        fn discard(&mut self, id: &str) {
            self.databases.remove(id);
        }
    }
    fn query(connection: &Connection, sql: &str, parameters: &[Value]) -> Result<String, Failure> {
        let mut statement = connection.prepare(sql).map_err(Failure::unexpected)?;
        for (index, value) in parameters.iter().enumerate() {
            let index = index + 1;
            match value {
                Value::Null => statement.raw_bind_parameter(index, rusqlite::types::Null),
                Value::String(text) => statement.raw_bind_parameter(index, text),
                Value::Number(number) => match number.as_i64() {
                    Some(integer) => statement.raw_bind_parameter(index, integer),
                    None => {
                        let real = number
                            .as_f64()
                            .filter(|real| real.is_finite())
                            .ok_or_else(|| Failure::unexpected("SQL numbers must be finite"))?;
                        statement.raw_bind_parameter(index, real)
                    }
                },
                _ => return Err(Failure::unexpected("Invalid SQL parameter")),
            }
            .map_err(Failure::unexpected)?;
        }
        let names: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(|name| serde_json::to_string(name).expect("strings serialise"))
            .collect();
        let mut rows = statement.raw_query();
        let mut output = String::from("[");
        let mut count = 0;
        let mut bytes = 0;
        while let Some(row) = rows.next().map_err(Failure::unexpected)? {
            if count == MAX_ROWS {
                return Err(Failure::unexpected(
                    "Query returns too many rows; add a LIMIT",
                ));
            }
            // Keep column order, as JavaScript objects preserve insertion order.
            let mut encoded = String::from("{");
            for (column, name) in names.iter().enumerate() {
                if column > 0 {
                    encoded.push(',');
                }
                encoded.push_str(name);
                encoded.push(':');
                match row.get_ref(column).map_err(Failure::unexpected)? {
                    ValueRef::Null => encoded.push_str("null"),
                    ValueRef::Integer(value) => {
                        if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
                            return Err(Failure::unexpected(
                                "Integer exceeds JavaScript precision; cast it to text",
                            ));
                        }
                        let _ = write!(encoded, "{value}");
                    }
                    ValueRef::Real(value) => {
                        let value = serde_json::Number::from_f64(value)
                            .ok_or_else(|| Failure::unexpected("SQL numbers must be finite"))?;
                        let _ = write!(encoded, "{value}");
                    }
                    ValueRef::Text(text) => {
                        encoded.push_str(
                            &serde_json::to_string(&String::from_utf8_lossy(text))
                                .expect("strings serialise"),
                        );
                    }
                    ValueRef::Blob(_) => {
                        return Err(Failure::unexpected(
                            "Binary SQL values are unsupported; select text or numeric columns",
                        ));
                    }
                }
            }
            encoded.push('}');
            bytes += encoded.len();
            if bytes >= MAX_RESULT_BYTES {
                return Err(Failure::unexpected(
                    "Query result is too large; add a LIMIT",
                ));
            }
            if count > 0 {
                output.push(',');
            }
            output.push_str(&encoded);
            count += 1;
        }
        output.push(']');
        Ok(output)
    }

    /// `ink-assets/<name>.db`, as produced by importing a database file.
    fn imported_database(asset: &str) -> bool {
        asset
            .strip_prefix("ink-assets/")
            .and_then(|name| name.strip_suffix(".db"))
            .is_some_and(|name| {
                !name.is_empty()
                    && name.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
            })
    }

    fn install(assets: &AssetOpener, asset: &str, destination: &Path) -> std::io::Result<()> {
        let _guard = INSTALL.lock().unwrap();
        if destination.exists() {
            return Ok(());
        }
        if let Some(directory) = destination.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let temporary = destination.with_extension("db.partial");
        let result = (|| {
            let mut source = assets(asset)?;
            let mut file = std::fs::File::create(&temporary)?;
            std::io::copy(&mut source, &mut file)?;
            file.sync_all()?;
            std::fs::rename(&temporary, destination)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }

    /// Whether `sql` begins with SELECT, WITH or EXPLAIN after whitespace and comments.
    fn read_only(sql: &str) -> bool {
        let mut rest = sql;
        loop {
            rest = rest.trim_start();
            if let Some(comment) = rest.strip_prefix("--") {
                rest = comment.split_once('\n').map_or("", |(_, after)| after);
            } else if let Some(comment) = rest.strip_prefix("/*") {
                match comment.split_once("*/") {
                    Some((_, after)) => rest = after,
                    None => return false,
                }
            } else {
                break;
            }
        }
        let word: String = rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        ["SELECT", "WITH", "EXPLAIN"]
            .iter()
            .any(|keyword| word.eq_ignore_ascii_case(keyword))
    }
}
