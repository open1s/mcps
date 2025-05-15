use std::{collections::HashMap,sync::Arc};

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SessionItem {
    pub items: HashMap<String, Value>,
    pub expires_at: DateTime<Utc>,
}

impl SessionItem {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            expires_at: Utc::now(),
        }
    }

    pub fn set_item(&mut self, key: String, value: Value) {
        self.items.insert(key, value);
    }

    pub fn get_item(&self, key: &str) -> Option<&Value> {
        self.items.get(key)
    }
}


#[derive(Clone)]
pub struct SessionStore {
    pub store: Arc<DashMap<String, SessionItem>>,
}

impl SessionStore {
    pub fn new() -> Self {
        let store = Arc::new(DashMap::new());
        let session_store = Self {
            store: store.clone(),
        };

        let cleanup_interval = std::time::Duration::from_secs(60);
        std::thread::spawn(move || loop {
            std::thread::sleep(cleanup_interval);
            
            let now = Utc::now();
            let keys_to_remove: Vec<String> = store
                .iter()
                .filter_map(|entry| {
                    if entry.value().expires_at <= now {
                        Some(entry.key().clone())
                    } else {
                        None
                    }
                })
                .collect();

            for key in keys_to_remove {
                store.remove(&key);
            }
        });
        session_store
    }

    pub fn create_session(&self,id: String, expires_in_secs: i64) {
        let expires_at = Utc::now() + Duration::seconds(expires_in_secs);
        self.store.insert(id.clone(), SessionItem {expires_at: expires_at, items: HashMap::new()});
    }

    pub fn set_session_value(&self, session_id: &str, key: String, value: Value) {
        if let Some(mut entry) = self.store.get_mut(session_id) {
            entry.set_item(key, value);
        }
    }

    pub fn get_session(&self, session_id: String) -> Option<SessionItem> {
        self.store.get(&session_id).and_then(|entry| {
            if entry.expires_at > Utc::now() {
                Some(entry.clone())
            } else {
                self.store.remove(&session_id); // remove expired
                None
            }
        })
    }

    pub fn invalidate_session(&self, session_id: &str) {
        self.store.remove(session_id);
    }
}


//create global session store
pub static SESSION_STORE: Lazy<SessionStore> = Lazy::new(|| {
    let store = SessionStore::new(); 
    store
});