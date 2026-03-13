use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::errors::{self, Error};

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedState {
    exhausted_keys: HashMap<String, DateTime<Utc>>,
}

fn key_hash(key: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3]
    )
}

fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mcp-youtube")
        .join("exhausted_keys.json")
}

fn last_midnight_pacific() -> DateTime<Utc> {
    let now_pacific = Utc::now().with_timezone(&chrono_tz::US::Pacific);
    now_pacific
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight")
        .and_local_timezone(chrono_tz::US::Pacific)
        .single()
        .expect("unambiguous midnight")
        .with_timezone(&Utc)
}

pub struct KeyPool {
    keys: Vec<SecretString>,
    exhausted: RwLock<HashMap<String, DateTime<Utc>>>,
    invalid: RwLock<HashSet<usize>>,
    cache_path: PathBuf,
}

impl KeyPool {
    #[must_use]
    pub fn new(keys: Vec<SecretString>) -> Self {
        let path = cache_path();
        let exhausted = Self::load_persisted(&path);
        Self {
            keys,
            exhausted: RwLock::new(exhausted),
            invalid: RwLock::new(HashSet::new()),
            cache_path: path,
        }
    }

    fn load_persisted(path: &Path) -> HashMap<String, DateTime<Utc>> {
        let Ok(data) = std::fs::read_to_string(path) else {
            return HashMap::new();
        };
        let Ok(state) = serde_json::from_str::<PersistedState>(&data) else {
            return HashMap::new();
        };
        state.exhausted_keys
    }

    fn persist(&self, exhausted: &HashMap<String, DateTime<Utc>>) {
        let state = PersistedState {
            exhausted_keys: exhausted.clone(),
        };
        if let Some(parent) = self.cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.cache_path, json);
        }
    }

    /// # Errors
    /// Returns an error if all keys are exhausted or the API call fails.
    pub async fn execute_with_key<F, Fut, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn(String) -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        // Auto-reset expired exhaustions
        let midnight = last_midnight_pacific();
        {
            let mut exhausted = self.exhausted.write().await;
            exhausted.retain(|_, ts| *ts >= midnight);
        }

        let mut tried = HashSet::new();

        loop {
            let selected = self.select_key(&tried).await;
            let Some((idx, key_str)) = selected else {
                return Err(Error::AllKeysExhausted);
            };

            tried.insert(idx);

            match f(key_str.clone()).await {
                Ok(val) => return Ok(val),
                Err(e) if errors::is_quota_exceeded(&e) => {
                    let hash = key_hash(&key_str);
                    tracing::warn!("API key #{idx} quota exceeded, rotating to next key");
                    let mut exhausted = self.exhausted.write().await;
                    exhausted.insert(hash, Utc::now());
                    self.persist(&exhausted);
                    drop(exhausted);
                    // continue to try next key
                }
                Err(e) if errors::is_key_invalid(&e) => {
                    tracing::warn!("API key #{idx} is invalid — skipping for this session");
                    let mut invalid = self.invalid.write().await;
                    invalid.insert(idx);
                    // continue to try next key
                }
                Err(e) if errors::is_rate_limited(&e) => {
                    return Err(e);
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn select_key(&self, tried: &HashSet<usize>) -> Option<(usize, String)> {
        let exhausted = self.exhausted.read().await;
        let invalid = self.invalid.read().await;

        // Collect indices to skip (tried + invalid)
        let skip: HashSet<usize> = tried
            .iter()
            .copied()
            .chain(invalid.iter().copied())
            .collect();
        drop(invalid);

        // First: find a non-exhausted, non-invalid, non-tried key
        for (idx, key) in self.keys.iter().enumerate() {
            if skip.contains(&idx) {
                continue;
            }
            let hash = key_hash(key.expose_secret());
            if exhausted.contains_key(&hash) {
                continue;
            }
            return Some((idx, key.expose_secret().to_string()));
        }

        // Fallback: try the oldest-exhausted key that hasn't been tried
        let mut oldest: Option<(usize, &str, &DateTime<Utc>)> = None;
        for (idx, key) in self.keys.iter().enumerate() {
            if skip.contains(&idx) {
                continue;
            }
            let hash = key_hash(key.expose_secret());
            if let Some(ts) = exhausted.get(&hash)
                && (oldest.is_none() || ts < oldest.unwrap().2)
            {
                oldest = Some((idx, key.expose_secret(), ts));
            }
        }

        oldest.map(|(idx, key, _)| (idx, key.to_string()))
    }
}
