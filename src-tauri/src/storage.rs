use crate::model::{Entry, Settings};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const FRESH_SECONDS: i64 = 30 * 24 * 60 * 60;
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
pub struct Storage {
    db: Connection,
    settings_path: PathBuf,
}
pub struct Cached {
    pub entries: Vec<Entry>,
    pub fresh: bool,
}
impl Storage {
    pub fn open(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        Self::with_connection(
            Connection::open(dir.join("dio.sqlite3")).map_err(|e| e.to_string())?,
            dir.join("settings.json"),
        )
    }
    fn with_connection(db: Connection, settings_path: PathBuf) -> Result<Self, String> {
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS cache (word TEXT PRIMARY KEY, data TEXT NOT NULL, fetched INTEGER NOT NULL, used INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS saved (word TEXT PRIMARY KEY, data TEXT NOT NULL, added INTEGER NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self { db, settings_path })
    }
    pub fn settings(&self) -> Result<Settings, String> {
        match fs::read(&self.settings_path) {
            Ok(bytes) => {
                let settings: Settings = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("Cannot read settings: {e}"))?;
                settings.validate()?;
                Ok(settings)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn write_settings(&self, settings: &Settings) -> Result<(), String> {
        let temporary = self.settings_path.with_extension("tmp");
        let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
        {
            use std::io::Write;
            let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
        // Windows rename replaces an existing file atomically through MoveFileExW.
        fs::rename(temporary, &self.settings_path).map_err(|e| e.to_string())
    }
    pub fn get(&self, word: &str, timestamp: i64) -> Result<Option<Cached>, String> {
        let row: Option<(String, i64)> = self
            .db
            .query_row(
                "SELECT data, fetched FROM cache WHERE word=?1",
                [word.to_lowercase()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some((data, fetched)) = row {
            self.db
                .execute(
                    "UPDATE cache SET used=?1 WHERE word=?2",
                    params![timestamp, word.to_lowercase()],
                )
                .map_err(|e| e.to_string())?;
            Ok(Some(Cached {
                entries: serde_json::from_str(&data).map_err(|e| e.to_string())?,
                fresh: timestamp.saturating_sub(fetched) < FRESH_SECONDS,
            }))
        } else {
            Ok(None)
        }
    }
    pub fn put(&mut self, word: &str, entries: &[Entry], timestamp: i64) -> Result<(), String> {
        let tx = self.db.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO cache VALUES (?1,?2,?3,?3)",
            params![
                word.to_lowercase(),
                serde_json::to_string(entries).map_err(|e| e.to_string())?,
                timestamp
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM cache WHERE word IN (SELECT word FROM cache ORDER BY used DESC, word LIMIT -1 OFFSET 1000)", []).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn saved(&self) -> Result<Vec<String>, String> {
        let mut statement = self
            .db
            .prepare("SELECT word FROM saved ORDER BY added DESC, word")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn save(&self, word: &str, entries: &[Entry], save: bool) -> Result<(), String> {
        if save {
            self.db.execute(
                "INSERT OR REPLACE INTO saved VALUES (?1,?2,?3)",
                params![
                    word.to_lowercase(),
                    serde_json::to_string(entries).map_err(|e| e.to_string())?,
                    now()
                ],
            )
        } else {
            self.db
                .execute("DELETE FROM saved WHERE word=?1", [word.to_lowercase()])
        }
        .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn saved_entry(&self, word: &str) -> Result<Option<Vec<Entry>>, String> {
        let data: Option<String> = self
            .db
            .query_row(
                "SELECT data FROM saved WHERE word=?1",
                [word.to_lowercase()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        data.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
            .transpose()
    }
    pub fn clear(&self) -> Result<(), String> {
        self.db
            .execute("DELETE FROM cache", [])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_expiry_and_saved_are_independent() {
        let mut store =
            Storage::with_connection(Connection::open_in_memory().unwrap(), PathBuf::new())
                .unwrap();
        let entries = vec![Entry {
            word: "test".into(),
            ..Default::default()
        }];
        store.put("Test", &entries, 10).unwrap();
        assert!(store.get("test", 11).unwrap().unwrap().fresh);
        assert!(
            !store
                .get("test", 10 + FRESH_SECONDS)
                .unwrap()
                .unwrap()
                .fresh
        );
        store.save("Test", &entries, true).unwrap();
        store.clear().unwrap();
        assert!(store.get("test", 11).unwrap().is_none());
        assert_eq!(store.saved().unwrap(), vec!["test"]);
        assert!(store.saved_entry("test").unwrap().is_some());
    }
    #[test]
    fn cache_evicts_oldest_without_removing_saved_words() {
        let mut store =
            Storage::with_connection(Connection::open_in_memory().unwrap(), PathBuf::new())
                .unwrap();
        let entries = vec![Entry {
            word: "word".into(),
            ..Default::default()
        }];
        store.save("old", &entries, true).unwrap();
        for index in 0..1001 {
            store.put(&format!("word{index}"), &entries, index).unwrap();
        }
        assert!(store.get("word0", 1002).unwrap().is_none());
        assert!(store.get("word1000", 1002).unwrap().is_some());
        assert!(store.saved_entry("old").unwrap().is_some());
    }
}
