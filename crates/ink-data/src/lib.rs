//! Store, SQLite and authenticated encryption modules answered by Rust instead of the platform UI thread.
#[cfg(feature = "store")]
use std::sync::atomic::{AtomicU64, Ordering};
use std::{io::Read, path::PathBuf, sync::Arc};

use ink_runtime::{NativeCall, NativeHost, NativeSession, Responder};
#[cfg(all(feature = "store", not(target_os = "android")))]
use serde_json::Value;

#[cfg(feature = "store")]
mod backend;
#[cfg(feature = "store")]
pub use backend::{Cancellation, DatabaseBackend, DatabaseConnection};

#[cfg(feature = "crypto")]
mod crypto;
#[cfg(feature = "store")]
mod sqlite;
#[cfg(feature = "store")]
mod store;

/// Opens a packaged asset such as `ink-assets/<hash>.db`.
pub type AssetOpener = Arc<dyn Fn(&str) -> std::io::Result<Box<dyn Read>> + Send + Sync>;

pub struct Config {
    /// The key-value store database, shared by every runtime in the process.
    pub store_path: PathBuf,
    /// Directory where imported database assets are installed before opening.
    pub files_directory: PathBuf,
    pub assets: AssetOpener,
}

/// Serves Rust native modules to every runtime in the process.
pub struct DataHost {
    #[cfg(feature = "store")]
    store: Arc<store::Store>,
    #[cfg(feature = "store")]
    backend: Arc<dyn DatabaseBackend>,
    #[cfg(feature = "store")]
    sessions: AtomicU64,
}

impl DataHost {
    #[cfg(not(target_os = "android"))]
    pub fn new(config: Config) -> Arc<Self> {
        #[cfg(feature = "store")]
        return Self::with_backend(Arc::new(EmbeddedBackend(config)));
        #[cfg(not(feature = "store"))]
        {
            let _ = config;
            Arc::new(Self {})
        }
    }

    #[cfg(feature = "store")]
    pub fn with_backend(backend: Arc<dyn DatabaseBackend>) -> Arc<Self> {
        Arc::new(Self {
            store: store::Store::new(backend.clone()),
            backend,
            sessions: AtomicU64::new(0),
        })
    }

    #[cfg(all(target_os = "android", not(feature = "store")))]
    pub fn crypto_only() -> Arc<Self> {
        Arc::new(Self {})
    }
}

#[cfg(all(feature = "store", not(target_os = "android")))]
struct EmbeddedBackend(Config);

#[cfg(all(feature = "store", not(target_os = "android")))]
impl DatabaseBackend for EmbeddedBackend {
    fn connect(&self, store: bool) -> Result<Box<dyn DatabaseConnection>, Failure> {
        if store {
            store::embedded::Store::new(&self.0.store_path)
        } else {
            Ok(sqlite::embedded::Databases::new(
                self.0.files_directory.clone(),
                self.0.assets.clone(),
            ))
        }
    }
}

impl NativeHost for DataHost {
    fn connect(&self, responder: Responder) -> Box<dyn NativeSession> {
        #[cfg(feature = "store")]
        let id = self.sessions.fetch_add(1, Ordering::Relaxed) + 1;
        #[cfg(feature = "store")]
        self.store.register(id, responder.clone());
        Box::new(Session {
            #[cfg(feature = "store")]
            id,
            #[cfg(feature = "store")]
            store: self.store.clone(),
            #[cfg(feature = "store")]
            sqlite: sqlite::Sqlite::new(self.backend.clone(), responder.clone()),
            responder,
        })
    }
}

struct Session {
    #[cfg(feature = "store")]
    id: u64,
    #[cfg(feature = "store")]
    store: Arc<store::Store>,
    #[cfg(feature = "store")]
    sqlite: sqlite::Sqlite,
    #[cfg_attr(not(any(feature = "store", feature = "crypto")), allow(dead_code))]
    responder: Responder,
}

impl NativeSession for Session {
    fn call(&mut self, call: NativeCall) -> bool {
        match call.module.as_str() {
            #[cfg(feature = "store")]
            "store" => {
                self.store.call(self.id, call, self.responder.clone());
                true
            }
            #[cfg(feature = "store")]
            "sqlite" => {
                self.sqlite.call(call);
                true
            }
            #[cfg(feature = "crypto")]
            "crypto" => {
                crypto::call(&call, &self.responder);
                true
            }
            _ => false,
        }
    }

    fn cancel(&mut self, _id: u64) -> bool {
        #[cfg(feature = "store")]
        return self.sqlite.cancel(_id);
        #[cfg(not(feature = "store"))]
        false
    }
}

/// A rejected request: error kind, message and whether retrying may succeed.
#[cfg_attr(not(any(feature = "store", feature = "crypto")), allow(dead_code))]
pub struct Failure(pub &'static str, pub String, pub bool);

#[cfg_attr(not(any(feature = "store", feature = "crypto")), allow(dead_code))]
impl Failure {
    pub fn unexpected(message: impl ToString) -> Self {
        Self("unexpected", message.to_string(), false)
    }
    pub fn protocol(message: impl ToString) -> Self {
        Self("protocol", message.to_string(), false)
    }
    fn send(self, responder: &Responder, id: u64) -> bool {
        responder.reject(id, self.0, &self.1, self.2)
    }
}

#[cfg(all(feature = "store", not(target_os = "android")))]
fn string<'a>(payload: &'a Value, field: &str) -> Result<&'a str, Failure> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| Failure::unexpected(format!("No string value for {field}")))
}

#[cfg(all(feature = "store", not(target_os = "android")))]
fn integer(payload: &Value, field: &str) -> Result<i64, Failure> {
    payload
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| Failure::unexpected(format!("No integer value for {field}")))
}

/// Length in UTF-16 code units, matching JavaScript and Java string lengths.
#[cfg(all(feature = "store", not(target_os = "android")))]
fn utf16_length(value: &str) -> usize {
    value.encode_utf16().count()
}

#[cfg(all(feature = "store", not(target_os = "android")))]
pub(crate) const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
