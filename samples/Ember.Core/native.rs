//! Native adapters. Network I/O and SQLite operations run on workers, never
//! while polling the returned futures on the UI thread.
use crate::{BoxFuture, HttpClient, ReaderError, Store};
use futures_channel::oneshot;
use rusqlite::{params, Connection, OptionalExtension};
use std::{io::Read, path::Path, sync::mpsc, time::Duration};

const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct NativeHttpClient {
    agent: ureq::Agent,
}
impl Default for NativeHttpClient {
    fn default() -> Self {
        Self::new()
    }
}
impl NativeHttpClient {
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(8))
                .timeout_read(Duration::from_secs(20))
                .timeout(Duration::from_secs(30))
                .user_agent("Ember-HN/0.1 (FerroUI; public read API)")
                .build(),
        }
    }
}
impl HttpClient for NativeHttpClient {
    fn get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<String, ReaderError>> {
        let agent = self.agent.clone();
        let url = url.to_owned();
        let (sender, receiver) = oneshot::channel();
        let spawned = std::thread::Builder::new()
            .name("ember-http".into())
            .spawn(move || {
                let result = (|| {
                    let response = agent
                        .get(&url)
                        .set("Accept", "application/json")
                        .call()
                        .map_err(|e| ReaderError::Network(e.to_string()))?;
                    let mut bytes = Vec::new();
                    response
                        .into_reader()
                        .take((MAX_BODY_BYTES + 1) as u64)
                        .read_to_end(&mut bytes)
                        .map_err(|e| ReaderError::Network(e.to_string()))?;
                    if bytes.len() > MAX_BODY_BYTES {
                        return Err(ReaderError::InvalidData("Response exceeds 8 MiB".into()));
                    }
                    String::from_utf8(bytes).map_err(|e| ReaderError::InvalidData(e.to_string()))
                })();
                let _ = sender.send(result);
            });
        Box::pin(async move {
            spawned
                .map_err(|e| ReaderError::Network(format!("Could not start HTTP worker: {e}")))?;
            receiver
                .await
                .map_err(|_| ReaderError::Network("HTTP worker stopped".into()))?
        })
    }
}

type DbJob = Box<dyn FnOnce(&mut Connection) + Send>;

/// One worker owns the connection. A write resolves only after SQLite commits.
#[derive(Clone)]
pub struct SqliteStore {
    jobs: mpsc::Sender<DbJob>,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReaderError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(storage_error)?;
        }
        Self::from_connection(Connection::open(path).map_err(storage_error)?)
    }
    pub fn in_memory() -> Result<Self, ReaderError> {
        Self::from_connection(Connection::open_in_memory().map_err(storage_error)?)
    }
    fn from_connection(mut connection: Connection) -> Result<Self, ReaderError> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(storage_error)?;
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage_error)?;
        if version > 1 {
            return Err(ReaderError::Storage(
                "Database was created by a newer Ember version".into(),
            ));
        }
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
            BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS ember_records (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL) WITHOUT ROWID;
            PRAGMA user_version=1; COMMIT;").map_err(storage_error)?;
        let (jobs, receiver) = mpsc::channel::<DbJob>();
        std::thread::Builder::new()
            .name("ember-database".into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    job(&mut connection);
                }
            })
            .map_err(storage_error)?;
        Ok(Self { jobs })
    }
    fn execute<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<T, ReaderError> + Send + 'static,
    ) -> BoxFuture<'static, Result<T, ReaderError>> {
        let (sender, receiver) = oneshot::channel();
        let queued = self.jobs.send(Box::new(move |connection| {
            let _ = sender.send(work(connection));
        }));
        Box::pin(async move {
            queued.map_err(|_| ReaderError::Storage("Database worker stopped".into()))?;
            receiver
                .await
                .map_err(|_| ReaderError::Storage("Database operation was interrupted".into()))?
        })
    }
}

impl Store for SqliteStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<String>, ReaderError>> {
        let key = key.to_owned();
        self.execute(move |db| {
            db.query_row(
                "SELECT value FROM ember_records WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)
        })
    }
    fn put<'a>(&'a self, key: &'a str, value: &'a str) -> BoxFuture<'a, Result<(), ReaderError>> {
        let key = key.to_owned();
        let value = value.to_owned();
        self.execute(move |db| {
            db.execute("INSERT INTO ember_records(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key,value]).map_err(storage_error)?; Ok(())
        })
    }
    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), ReaderError>> {
        let key = key.to_owned();
        self.execute(move |db| {
            db.execute("DELETE FROM ember_records WHERE key=?1", [key])
                .map_err(storage_error)?;
            Ok(())
        })
    }
    fn scan_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> BoxFuture<'a, Result<Vec<(String, String)>, ReaderError>> {
        let prefix = prefix.to_owned();
        self.execute(move |db| {
            // SQLite LIKE is ASCII case-insensitive by default. Record prefixes
            // are exact and case-sensitive, including usernames and literal %/_ .
            let mut statement = db.prepare("SELECT key,value FROM ember_records WHERE key>=?1 AND substr(key,1,length(?1))=?1 COLLATE BINARY ORDER BY key")
                .map_err(storage_error)?;
            let rows = statement.query_map([prefix], |row| Ok((row.get(0)?, row.get(1)?))).map_err(storage_error)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
        })
    }
}

fn storage_error(error: impl std::fmt::Display) -> ReaderError {
    ReaderError::Storage(error.to_string())
}
