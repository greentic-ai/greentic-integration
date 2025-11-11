use std::{collections::HashMap, fs, sync::Arc};

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpsert {
    pub key: String,
    pub tenant: String,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub flow_id: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub context: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRecord {
    pub key: String,
    pub tenant: String,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub flow_id: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub context: Value,
    #[serde(default)]
    pub updated_at_epoch_ms: u64,
}

#[derive(Debug, Default, Clone)]
pub struct SessionFilter {
    pub tenant: Option<String>,
    pub team: Option<String>,
    pub user: Option<String>,
}

impl SessionFilter {
    pub fn new(tenant: Option<String>, team: Option<String>, user: Option<String>) -> Self {
        Self { tenant, team, user }
    }

    pub fn matches(&self, record: &SessionRecord) -> bool {
        self.tenant
            .as_ref()
            .is_none_or(|tenant| record.tenant == *tenant)
            && self
                .team
                .as_ref()
                .is_none_or(|team| record.team.as_deref() == Some(team.as_str()))
            && self
                .user
                .as_ref()
                .is_none_or(|user| record.user.as_deref() == Some(user.as_str()))
    }
}

pub trait SessionStore: Send + Sync {
    fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionRecord>>;
    fn purge(&self, filter: &SessionFilter) -> Result<usize>;
    fn upsert(&self, record: SessionUpsert) -> Result<SessionRecord>;
    fn find(&self, filter: &SessionFilter) -> Result<Option<SessionRecord>>;
    fn remove(&self, key: &str) -> Result<()>;
}

#[derive(Default)]
pub struct InMemorySessionStore {
    inner: Mutex<HashMap<String, SessionRecord>>,
}

impl InMemorySessionStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(HashMap::new()),
        })
    }
}

impl SessionStore for InMemorySessionStore {
    fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionRecord>> {
        let guard = self.inner.lock();
        Ok(guard
            .values()
            .filter(|record| filter.matches(record))
            .cloned()
            .collect())
    }

    fn purge(&self, filter: &SessionFilter) -> Result<usize> {
        let mut guard = self.inner.lock();
        let before = guard.len();
        guard.retain(|_, record| !filter.matches(record));
        Ok(before - guard.len())
    }

    fn upsert(&self, payload: SessionUpsert) -> Result<SessionRecord> {
        let mut guard = self.inner.lock();
        let record = SessionRecord {
            key: payload.key,
            tenant: payload.tenant,
            team: payload.team,
            user: payload.user,
            flow_id: payload.flow_id,
            node_id: payload.node_id,
            context: payload.context,
            updated_at_epoch_ms: current_timestamp_ms(),
        };
        guard.insert(record.key.clone(), record.clone());
        Ok(record)
    }

    fn find(&self, filter: &SessionFilter) -> Result<Option<SessionRecord>> {
        let guard = self.inner.lock();
        Ok(guard
            .values()
            .find(|record| filter.matches(record))
            .cloned())
    }

    fn remove(&self, key: &str) -> Result<()> {
        self.inner.lock().remove(key);
        Ok(())
    }
}

pub struct FileSessionStore {
    path: Utf8PathBuf,
    inner: Mutex<HashMap<String, SessionRecord>>,
}

impl FileSessionStore {
    pub fn new(path: Utf8PathBuf) -> Result<Arc<Self>> {
        let data = Self::load_from_disk(&path).unwrap_or_default();
        Ok(Arc::new(Self {
            path,
            inner: Mutex::new(data),
        }))
    }

    fn load_from_disk(path: &Utf8PathBuf) -> Result<HashMap<String, SessionRecord>> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, "[]")?;
            return Ok(HashMap::new());
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read session store {path}"))?;
        if raw.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let rows: Vec<SessionRecord> =
            serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {path}"))?;
        Ok(rows.into_iter().map(|row| (row.key.clone(), row)).collect())
    }

    fn persist(&self, guard: &HashMap<String, SessionRecord>) -> Result<()> {
        let rows: Vec<_> = guard.values().cloned().collect();
        let json = serde_json::to_string_pretty(&rows)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, json)
            .with_context(|| format!("failed to write session store {}", self.path))?;
        Ok(())
    }
}

impl SessionStore for FileSessionStore {
    fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionRecord>> {
        let guard = self.inner.lock();
        Ok(guard
            .values()
            .filter(|record| filter.matches(record))
            .cloned()
            .collect())
    }

    fn purge(&self, filter: &SessionFilter) -> Result<usize> {
        let mut guard = self.inner.lock();
        let before = guard.len();
        guard.retain(|_, record| !filter.matches(record));
        let removed = before - guard.len();
        if removed > 0 {
            self.persist(&guard)?;
        }
        Ok(removed)
    }

    fn upsert(&self, payload: SessionUpsert) -> Result<SessionRecord> {
        let mut guard = self.inner.lock();
        let record = SessionRecord {
            key: payload.key,
            tenant: payload.tenant,
            team: payload.team,
            user: payload.user,
            flow_id: payload.flow_id,
            node_id: payload.node_id,
            context: payload.context,
            updated_at_epoch_ms: current_timestamp_ms(),
        };
        guard.insert(record.key.clone(), record.clone());
        self.persist(&guard)?;
        Ok(record)
    }

    fn find(&self, filter: &SessionFilter) -> Result<Option<SessionRecord>> {
        Ok(self
            .inner
            .lock()
            .values()
            .find(|record| filter.matches(record))
            .cloned())
    }

    fn remove(&self, key: &str) -> Result<()> {
        let mut guard = self.inner.lock();
        guard.remove(key);
        self.persist(&guard)?;
        Ok(())
    }
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn in_memory_find_and_remove() {
        let store = InMemorySessionStore::new();
        let record = SessionUpsert {
            key: "sess-123".into(),
            tenant: "acme".into(),
            team: Some("ops".into()),
            user: Some("user-1".into()),
            flow_id: Some("flow-a".into()),
            node_id: Some("node-1".into()),
            context: json!({"hello": "world"}),
        };
        store.upsert(record).unwrap();

        let filter = SessionFilter::new(
            Some("acme".into()),
            Some("ops".into()),
            Some("user-1".into()),
        );
        let found = store.find(&filter).unwrap().expect("session present");
        assert_eq!(found.key, "sess-123");
        assert_eq!(found.flow_id.as_deref(), Some("flow-a"));

        store.remove("sess-123").unwrap();
        assert!(store.find(&filter).unwrap().is_none());
    }

    #[test]
    fn file_store_persists_sessions() {
        let temp = tempdir().unwrap();
        let path =
            Utf8PathBuf::from_path_buf(temp.path().join("sessions.json")).expect("utf8 path");
        let store = FileSessionStore::new(path).unwrap();

        let record = SessionUpsert {
            key: "sess-999".into(),
            tenant: "tenant-x".into(),
            team: None,
            user: Some("user-z".into()),
            flow_id: Some("flow-z".into()),
            node_id: None,
            context: json!({"x": 1}),
        };
        store.upsert(record).unwrap();

        let filter = SessionFilter::new(Some("tenant-x".into()), None, Some("user-z".into()));
        let results = store.list(&filter).unwrap();
        assert_eq!(results.len(), 1);

        store.remove("sess-999").unwrap();
        assert!(store.list(&filter).unwrap().is_empty());
    }
}
