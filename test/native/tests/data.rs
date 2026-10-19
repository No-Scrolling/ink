use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use ink_data::{Config, DataHost};
use ink_runtime::{AppRuntime, Event, EventReceiver, NativeHost, Script};
use serde_json::{Value, json};

const WAIT: Duration = Duration::from_secs(10);

/// Mirrors the request and result envelopes used by `ink/native`.
const HARNESS: &str = r#"
const pending = new Map();
let next = 1;
globalThis.__inkReceive = source => {
  const message = JSON.parse(source);
  if (message.type === "result" && pending.has(message.id)) {
    pending.get(message.id)(message);
    pending.delete(message.id);
  } else __inkPost(JSON.stringify({ type: "observed", message }));
};
function call(module, operation, payload) {
  const id = next++;
  return new Promise(resolve => {
    pending.set(id, resolve);
    __inkPost(JSON.stringify({ type: "call", id, module, operation, payload, timeoutMs: 30000, binary: false }));
  });
}
function report(value) { __inkPost(JSON.stringify({ type: "report", value })); }
"#;

struct Fixture {
    directory: PathBuf,
    host: Arc<DataHost>,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let directory = std::env::temp_dir().join(format!("ink-data-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(directory.join("assets/ink-assets")).unwrap();
        let host = Self::host(&directory);
        Self { directory, host }
    }

    fn host(directory: &Path) -> Arc<DataHost> {
        let assets = directory.join("assets");
        DataHost::new(Config {
            store_path: directory.join("databases/ink-store.db"),
            files_directory: directory.join("files"),
            assets: Arc::new(move |name| Ok(Box::new(std::fs::File::open(assets.join(name))?) as Box<dyn std::io::Read>)),
        })
    }

    fn run(&self, script: &str) -> (AppRuntime, EventReceiver) {
        AppRuntime::spawn_script(
            Script::Source(format!("{HARNESS}\n{script}")),
            || {},
            None::<fn() -> anyhow::Result<Script>>,
            Some(self.host.clone() as Arc<dyn NativeHost>),
        )
        .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

/// The next posted message of `kind`; other output is ignored.
fn next(events: &EventReceiver, kind: &str) -> Value {
    let deadline = Instant::now() + WAIT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match events.recv_timeout(remaining).expect("runtime produced no message in time") {
            Event::Message(source) => {
                let value: Value = serde_json::from_str(&source).unwrap();
                if value["type"] == kind {
                    return value;
                }
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
}

#[test]
fn store_revisions_reject_stale_writes_and_persist_for_a_new_process() {
    let fixture = Fixture::new("store");
    let (_runtime, events) = fixture.run(r#"
(async () => {
  const empty = await call("store", "get", { key: "settings" });
  const first = await call("store", "write", { key: "settings", revision: 0, version: 1, value: "{\"theme\":\"dark\"}" });
  const stale = await call("store", "write", { key: "settings", revision: 0, version: 1, value: "{\"theme\":\"light\"}" });
  const read = await call("store", "get", { key: "settings" });
  const older = await call("store", "replace", { key: "settings", version: 0, value: "{}" });
  report({ empty, first, stale, read, older });
})();
"#);
    let report = &next(&events, "report")["value"];
    assert_eq!(report["empty"]["value"], "null");
    assert_eq!(serde_json::from_str::<Value>(report["first"]["value"].as_str().unwrap()).unwrap(), json!({"committed": true}));
    assert_eq!(serde_json::from_str::<Value>(report["stale"]["value"].as_str().unwrap()).unwrap(), json!({"committed": false}));
    assert_eq!(
        serde_json::from_str::<Value>(report["read"]["value"].as_str().unwrap()).unwrap(),
        json!({"revision": 1, "version": 1, "value": "{\"theme\":\"dark\"}"})
    );
    assert_eq!(report["older"]["kind"], "unexpected");
    assert_eq!(report["older"]["message"], "Invalid store version");

    // A separate connection sees the committed entry, as a relaunched app would.
    let connection = rusqlite::Connection::open(fixture.directory.join("databases/ink-store.db")).unwrap();
    let stored: (i64, String) = connection
        .query_row("SELECT revision, value FROM entries WHERE key = 'settings'", [], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    assert_eq!(stored, (1, "{\"theme\":\"dark\"}".to_owned()));
}

#[test]
fn store_reads_entries_written_by_the_android_sqlite_helper() {
    let fixture = Fixture::new("legacy");
    let path = fixture.directory.join("databases/ink-store.db");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    {
        // The schema, metadata table and user version created by the former SQLiteOpenHelper.
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch(
            "CREATE TABLE android_metadata (locale TEXT); INSERT INTO android_metadata VALUES ('en_GB');
             CREATE TABLE entries (key TEXT PRIMARY KEY, revision INTEGER NOT NULL, version INTEGER NOT NULL, value TEXT NOT NULL);
             INSERT INTO entries VALUES ('podcasts', 7, 2, '[\"a\"]'); PRAGMA user_version = 1;",
        ).unwrap();
    }
    let (_runtime, events) = fixture.run(r#"
(async () => report(await call("store", "get", { key: "podcasts" })))();
"#);
    let entry: Value = serde_json::from_str(next(&events, "report")["value"]["value"].as_str().unwrap()).unwrap();
    assert_eq!(entry, json!({"revision": 7, "version": 2, "value": "[\"a\"]"}));
}

#[test]
fn store_changes_notify_other_runtimes_but_not_the_writer() {
    let fixture = Fixture::new("notify");
    let (_listener, listening) = fixture.run("report('ready');");
    next(&listening, "report");
    let (_writer, writing) = fixture.run(r#"
(async () => {
  await call("store", "replace", { key: "queue", version: 1, value: "[]" });
  setTimeout(() => report("settled"), 300);
})();
"#);
    assert_eq!(next(&listening, "observed")["message"], json!({"type": "store-changed", "key": "queue"}));
    // Everything the writer posts before settling: its own change must not be echoed back.
    let deadline = Instant::now() + WAIT;
    loop {
        match writing.recv_timeout(deadline.saturating_duration_since(Instant::now())).unwrap() {
            Event::Message(source) => {
                let value: Value = serde_json::from_str(&source).unwrap();
                assert_ne!(value["type"], "observed", "writer received {value}");
                if value["type"] == "report" {
                    break;
                }
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
}

#[test]
fn crypto_decrypts_independently_produced_data_and_round_trips() {
    let fixture = Fixture::new("crypto");
    // Produced by Python's `cryptography` AES-GCM with the same nonce layout and associated data.
    let (_runtime, events) = fixture.run(r#"
const key = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
(async () => {
  const known = await call("crypto", "decrypt", { key, text: "ink1.ZGVmZ2hpamtsbW5vD2kd2rp2M75fFyzIkwsB1Q0211NqKfNNMWuy0zRPKw==" });
  const sealed = await call("crypto", "encrypt", { key, text: "second message" });
  const opened = await call("crypto", "decrypt", { key, text: JSON.parse(sealed.value) });
  // Change one ciphertext character after the prefix and 16-character nonce, so only the tag can catch it.
  const text = JSON.parse(sealed.value);
  const tampered = await call("crypto", "decrypt", { key, text: text.slice(0, 25) + (text[25] === "A" ? "B" : "A") + text.slice(26) });
  const shortKey = await call("crypto", "encrypt", { key: "AAEC", text: "x" });
  report({ known, sealed, opened, tampered, shortKey });
})();
"#);
    let report = &next(&events, "report")["value"];
    assert_eq!(serde_json::from_str::<String>(report["known"]["value"].as_str().unwrap()).unwrap(), "Grüße aus Ink");
    let sealed: String = serde_json::from_str(report["sealed"]["value"].as_str().unwrap()).unwrap();
    assert!(sealed.starts_with("ink1."));
    assert_eq!(serde_json::from_str::<String>(report["opened"]["value"].as_str().unwrap()).unwrap(), "second message");
    assert_eq!(report["tampered"]["kind"], "protocol");
    assert_eq!(report["tampered"]["message"], "Could not decrypt data");
    assert_eq!(report["shortKey"]["message"], "Could not encrypt data");
}

#[test]
fn sqlite_queries_imported_assets_read_only_in_column_order() {
    let fixture = Fixture::new("sqlite");
    {
        let connection = rusqlite::Connection::open(fixture.directory.join("assets/ink-assets/places.db")).unwrap();
        connection.execute_batch(
            "CREATE TABLE places (id INTEGER PRIMARY KEY, name TEXT, rating REAL);
             INSERT INTO places VALUES (1, 'Kings Cross Station', 4.5), (2, 'Euston Station', NULL), (3, 'Hyde Park', 5);",
        ).unwrap();
    }
    let (_runtime, events) = fixture.run(r#"
(async () => {
  const id = (await call("sqlite", "open", { asset: "asset://ink-assets/places.db" })).value;
  const rows = await call("sqlite", "query", { id, sql: "-- stations\nSELECT name, id, rating FROM places WHERE name LIKE ? ORDER BY id", parameters: ["%Station%"] });
  const write = await call("sqlite", "query", { id, sql: "DELETE FROM places", parameters: [] });
  const outside = await call("sqlite", "open", { asset: "asset://../secrets.db" });
  await call("sqlite", "close", { id });
  const closed = await call("sqlite", "query", { id, sql: "SELECT 1", parameters: [] });
  report({ rows, write, outside, closed });
})();
"#);
    let report = &next(&events, "report")["value"];
    let rows = report["rows"]["value"].as_str().unwrap();
    assert_eq!(rows, r#"[{"name":"Kings Cross Station","id":1,"rating":4.5},{"name":"Euston Station","id":2,"rating":null}]"#);
    assert_eq!(report["write"]["message"], "Read-only queries must start with SELECT, WITH or EXPLAIN");
    assert_eq!(report["outside"]["message"], "Expected an imported database asset");
    assert_eq!(report["closed"]["message"], "Database is closed");
    assert!(fixture.directory.join("files/ink-assets/places.db").is_file());
}

#[test]
fn cancelled_sqlite_queries_stop_and_never_resolve() {
    let fixture = Fixture::new("cancel");
    rusqlite::Connection::open(fixture.directory.join("assets/ink-assets/slow.db")).unwrap()
        .execute_batch("CREATE TABLE t (x INTEGER);").unwrap();
    let (_runtime, events) = fixture.run(r#"
(async () => {
  const id = (await call("sqlite", "open", { asset: "asset://ink-assets/slow.db" })).value;
  let settled = false;
  // An unbounded recursive query only ends when interrupted.
  call("sqlite", "query", { id, sql: "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n) SELECT count(*) FROM n", parameters: [] })
    .then(() => { settled = true; });
  __inkPost(JSON.stringify({ type: "cancel", id: next - 1 }));
  const after = await call("sqlite", "query", { id, sql: "SELECT 42 AS answer", parameters: [] });
  report({ settled, after });
})();
"#);
    let report = &next(&events, "report")["value"];
    assert_eq!(report["settled"], false);
    assert_eq!(report["after"]["value"], r#"[{"answer":42}]"#);
}

#[test]
fn modules_without_a_rust_implementation_still_reach_the_platform() {
    let fixture = Fixture::new("forward");
    let (_runtime, events) = fixture.run(r#"call("clipboard", "read", {});"#);
    let deadline = Instant::now() + WAIT;
    loop {
        match events.recv_timeout(deadline.saturating_duration_since(Instant::now())).unwrap() {
            Event::Message(source) => {
                let call: Value = serde_json::from_str(&source).unwrap();
                assert_eq!((call["type"].as_str(), call["module"].as_str()), (Some("call"), Some("clipboard")));
                break;
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
}

/// The committed flag from a store write result.
fn committed(result: &Value) -> bool {
    serde_json::from_str::<Value>(result["value"].as_str().unwrap_or_else(|| panic!("write failed: {result}")))
        .unwrap()["committed"]
        .as_bool()
        .unwrap()
}

fn entry(result: &Value) -> Value {
    serde_json::from_str(result["value"].as_str().unwrap()).unwrap()
}

#[test]
fn store_replace_ignores_revisions_and_survives_a_new_host() {
    let fixture = Fixture::new("reopen");
    let stored = "{\"text\":\"hello 😀\\n\",\"count\":9}";
    let (_runtime, events) = fixture.run(&format!(r#"
(async () => {{
  const first = await call("store", "write", {{ key: "persistence", revision: 0, version: 1, value: {stored:?} }});
  report(first);
}})();
"#));
    assert!(committed(&next(&events, "report")["value"]));
    // A host for a new process opens the same database file.
    let reopened = Fixture { directory: fixture.directory.clone(), host: Fixture::host(&fixture.directory) };
    let (_runtime, events) = reopened.run(r#"
(async () => {
  const saved = await call("store", "get", { key: "persistence" });
  const replaced = await call("store", "replace", { key: "persistence", revision: 999, version: 1, value: "replacement" });
  const after = await call("store", "get", { key: "persistence" });
  report({ saved, replaced, after });
})();
"#);
    let report = next(&events, "report")["value"].clone();
    assert_eq!(entry(&report["saved"]), json!({"revision": 1, "version": 1, "value": stored}));
    assert!(committed(&report["replaced"]));
    assert_eq!(entry(&report["after"]), json!({"revision": 2, "version": 1, "value": "replacement"}));
}

#[test]
fn store_concurrent_compare_and_set_commits_exactly_one_writer() {
    let fixture = Fixture::new("contested");
    let (_runtime, events) = fixture.run(r#"
(async () => {
  const results = await Promise.all(Array.from({ length: 12 }, (_, index) =>
    call("store", "write", { key: "contested", revision: 0, version: 1, value: `writer-${index}` })));
  report({ results, saved: await call("store", "get", { key: "contested" }) });
})();
"#);
    let report = &next(&events, "report")["value"];
    let commits = report["results"].as_array().unwrap().iter().filter(|result| committed(result)).count();
    assert_eq!(commits, 1);
    let saved = entry(&report["saved"]);
    assert_eq!(saved["revision"], 1);
    assert!((0..12).any(|index| saved["value"] == format!("writer-{index}")));
}

#[test]
fn store_skips_notifications_for_rejected_writes_and_stopped_runtimes() {
    let fixture = Fixture::new("quiet");
    let (stopped, stopping) = fixture.run("report('ready');");
    next(&stopping, "report");
    stopped.stop();
    drop(stopped);
    let (_sibling, sibling) = fixture.run("report('ready');");
    next(&sibling, "report");
    let (_writer, writing) = fixture.run(r#"
(async () => {
  const first = await call("store", "write", { key: "notification", revision: 0, version: 1, value: "initial" });
  const stale = await call("store", "write", { key: "notification", revision: 0, version: 1, value: "initial" });
  report({ first, stale });
})();
"#);
    let report = &next(&writing, "report")["value"];
    assert!(committed(&report["first"]));
    assert!(!committed(&report["stale"]));
    assert_eq!(next(&sibling, "observed")["message"], json!({"type": "store-changed", "key": "notification"}));
    // The rejected write produces no second notification.
    loop {
        match sibling.recv_timeout(Duration::from_millis(200)) {
            Ok(Event::Message(source)) => panic!("unexpected sibling message {source}"),
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

#[test]
fn store_rejects_invalid_requests_without_changing_the_entry() {
    let fixture = Fixture::new("validation");
    let (_runtime, events) = fixture.run(r#"
(async () => {
  const stable = await call("store", "write", { key: "validated", revision: 0, version: 2, value: "stable" });
  const downgrade = await call("store", "replace", { key: "validated", version: 1, value: "downgrade" });
  // Fits in a call, but its entry is escaped twice inside a result and would exceed the result limit.
  const oversized = await call("store", "replace", { key: "validated", version: 2, value: '"'.repeat(400000) });
  const saved = await call("store", "get", { key: "validated" });
  const keys = await Promise.all(["", "x".repeat(257)].map(key => call("store", "write", { key, revision: 0, version: 1, value: "x" })));
  const versions = await Promise.all([0, 9007199254740992].map(version => call("store", "write", { key: `version-${version}`, revision: 0, version, value: "x" })));
  const unknown = await call("store", "unknown", { key: "unknown" });
  report({ stable, downgrade, oversized, saved, keys, versions, unknown });
})();
"#);
    let report = &next(&events, "report")["value"];
    assert!(committed(&report["stable"]));
    assert_eq!(report["downgrade"]["message"], "Store validated uses a newer version");
    assert_eq!(report["oversized"]["message"], "Store value is too large");
    assert_eq!(entry(&report["saved"]), json!({"revision": 1, "version": 2, "value": "stable"}));
    for failure in report["keys"].as_array().unwrap().iter().chain(report["versions"].as_array().unwrap()) {
        assert_eq!(failure["kind"], "unexpected", "{failure}");
    }
    assert_eq!(report["unknown"]["kind"], "protocol");
}
