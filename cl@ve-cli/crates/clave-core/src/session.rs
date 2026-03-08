use crate::error::Result;
use crate::storage::storage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub device_id: String,
    pub nif: String,
    pub device_password: String,
    pub firebase_token: Option<String>,
    pub created_at: String,
}

impl Session {
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(self)?;
        storage()?.save(&json)
    }

    pub fn load() -> Result<Self> {
        let json = storage()?.load()?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(session)
    }

    pub fn delete() -> Result<()> {
        storage()?.delete()
    }

    /// Hydrate session from a JSON string (used by FFI layer).
    ///
    /// Flutter loads the JSON from its platform secure storage and passes it
    /// here so the Rust core can use the session for subsequent API calls.
    pub fn set_session_data(json: &str) -> Result<()> {
        // Validate that the JSON is a valid session before storing
        let _: Session = serde_json::from_str(json)?;
        storage()?.save(json)
    }

    /// Export session as JSON (used by FFI layer).
    ///
    /// Flutter calls this after operations that mutate the session (e.g.
    /// activation) to persist the result in platform secure storage.
    pub fn get_session_data() -> Result<String> {
        storage()?.load()
    }
}
