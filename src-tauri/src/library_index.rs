use rusqlite::{params, Connection};
use std::{collections::HashMap, fs, path::Path};

use crate::library_model::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    pub(crate) size: u64,
    pub(crate) modified_ms: u64,
    pub(crate) lrc_size: u64,
    pub(crate) lrc_modified_ms: u64,
}

#[derive(Clone)]
pub(crate) struct CachedTrack {
    pub(crate) fingerprint: Fingerprint,
    pub(crate) file_key: Option<String>,
    pub(crate) node: Node,
}

fn open(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS tracks (
                 path TEXT PRIMARY KEY NOT NULL,
                 size INTEGER NOT NULL,
                 modified_ms INTEGER NOT NULL,
                 lrc_size INTEGER NOT NULL,
                 lrc_modified_ms INTEGER NOT NULL,
                 file_key TEXT,
                 node_json TEXT NOT NULL
             );",
        )
        .map_err(|error| error.to_string())?;
    let has_file_key = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('tracks') WHERE name = 'file_key')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| error.to_string())?;
    if !has_file_key {
        connection
            .execute("ALTER TABLE tracks ADD COLUMN file_key TEXT", [])
            .map_err(|error| error.to_string())?;
    }
    Ok(connection)
}

pub(crate) fn load(path: &Path) -> Result<HashMap<String, CachedTrack>, String> {
    let connection = open(path)?;
    let mut statement = connection
        .prepare(
            "SELECT path, size, modified_ms, lrc_size, lrc_modified_ms, file_key, node_json
             FROM tracks",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let size: i64 = row.get(1)?;
            let modified_ms: i64 = row.get(2)?;
            let lrc_size: i64 = row.get(3)?;
            let lrc_modified_ms: i64 = row.get(4)?;
            let file_key: Option<String> = row.get(5)?;
            let node_json: String = row.get(6)?;
            Ok((
                path,
                size,
                modified_ms,
                lrc_size,
                lrc_modified_ms,
                file_key,
                node_json,
            ))
        })
        .map_err(|error| error.to_string())?;

    let mut cache = HashMap::new();
    for row in rows {
        let Ok((path, size, modified_ms, lrc_size, lrc_modified_ms, file_key, node_json)) = row
        else {
            continue;
        };
        let Ok(node) = serde_json::from_str::<Node>(&node_json) else {
            continue;
        };
        if size < 0 || modified_ms < 0 || lrc_size < 0 || lrc_modified_ms < 0 {
            continue;
        }
        cache.insert(
            path,
            CachedTrack {
                fingerprint: Fingerprint {
                    size: size as u64,
                    modified_ms: modified_ms as u64,
                    lrc_size: lrc_size as u64,
                    lrc_modified_ms: lrc_modified_ms as u64,
                },
                file_key,
                node,
            },
        );
    }
    Ok(cache)
}

pub(crate) fn persist(
    path: &Path,
    updates: Vec<(String, CachedTrack)>,
    stale: Vec<String>,
) -> Result<(), String> {
    let mut connection = open(path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    {
        let mut upsert = transaction
            .prepare_cached(
                "INSERT INTO tracks(
                    path, size, modified_ms, lrc_size, lrc_modified_ms, file_key, node_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(path) DO UPDATE SET
                    size=excluded.size,
                    modified_ms=excluded.modified_ms,
                    lrc_size=excluded.lrc_size,
                    lrc_modified_ms=excluded.lrc_modified_ms,
                    file_key=excluded.file_key,
                    node_json=excluded.node_json",
            )
            .map_err(|error| error.to_string())?;
        for (path, entry) in updates {
            let node_json =
                serde_json::to_string(&entry.node).map_err(|error| error.to_string())?;
            upsert
                .execute(params![
                    path,
                    entry.fingerprint.size as i64,
                    entry.fingerprint.modified_ms as i64,
                    entry.fingerprint.lrc_size as i64,
                    entry.fingerprint.lrc_modified_ms as i64,
                    entry.file_key,
                    node_json,
                ])
                .map_err(|error| error.to_string())?;
        }
    }
    {
        let mut delete = transaction
            .prepare_cached("DELETE FROM tracks WHERE path = ?1")
            .map_err(|error| error.to_string())?;
        for path in stale {
            delete.execute([path]).map_err(|error| error.to_string())?;
        }
    }
    transaction.commit().map_err(|error| error.to_string())
}
