use crate::config::data_dir;
use crate::error::{ClaveError, Result};
use serde::{Deserialize, Serialize};

const KEYRING_SERVICE: &str = "clave-cli";

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
        // Try keyring first, fall back to file
        match save_to_keyring(self) {
            Ok(()) => Ok(()),
            Err(_) => save_to_file(self),
        }
    }

    pub fn load() -> Result<Self> {
        // Try keyring first, fall back to file
        match load_from_keyring() {
            Ok(session) => Ok(session),
            Err(_) => load_from_file(),
        }
    }

    pub fn delete() -> Result<()> {
        let _ = delete_from_keyring();
        let path = data_dir()?.join("session.json");
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

fn save_to_keyring(session: &Session) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &session.nif)
        .map_err(|e| ClaveError::Keyring(e.to_string()))?;
    let json = serde_json::to_string(session)?;
    entry
        .set_password(&json)
        .map_err(|e| ClaveError::Keyring(e.to_string()))?;
    Ok(())
}

fn load_from_keyring() -> Result<Session> {
    // We need to know the NIF to look up the entry, check file first for NIF
    let path = data_dir()?.join("session_nif.txt");
    let nif = std::fs::read_to_string(&path).map_err(|_| ClaveError::NoSession)?;
    let entry = keyring::Entry::new(KEYRING_SERVICE, nif.trim())
        .map_err(|e| ClaveError::Keyring(e.to_string()))?;
    let json = entry
        .get_password()
        .map_err(|e| ClaveError::Keyring(e.to_string()))?;
    let session: Session = serde_json::from_str(&json)?;
    Ok(session)
}

fn delete_from_keyring() -> Result<()> {
    let path = data_dir()?.join("session_nif.txt");
    if let Ok(nif) = std::fs::read_to_string(&path) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, nif.trim()) {
            let _ = entry.delete_credential();
        }
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

fn save_to_file(session: &Session) -> Result<()> {
    let dir = data_dir()?;
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(dir.join("session.json"), json)?;
    // Also save NIF for keyring lookup
    std::fs::write(dir.join("session_nif.txt"), &session.nif)?;
    Ok(())
}

fn load_from_file() -> Result<Session> {
    let path = data_dir()?.join("session.json");
    let data = std::fs::read_to_string(&path).map_err(|_| ClaveError::NoSession)?;
    let session: Session = serde_json::from_str(&data)?;
    Ok(session)
}
