use crate::error::{LlaveError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Global proxy URL. Set via [`set_proxy`] before creating any [`LlaveClient`].
/// Checked by `LlaveClient::new()` — if set, all requests go through this proxy.
/// Supports HTTP, HTTPS, and SOCKS5 URLs (e.g. `socks5://127.0.0.1:1080`).
static PROXY_URL: Mutex<Option<String>> = Mutex::new(None);

/// Set the global proxy URL used by all subsequent `LlaveClient` instances.
pub fn set_proxy(url: Option<String>) {
    *PROXY_URL.lock().unwrap() = url;
}

/// Get the currently configured proxy URL.
pub fn get_proxy() -> Option<String> {
    PROXY_URL.lock().unwrap().clone()
}

/// XDG-compliant configuration directory
pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| LlaveError::Config("Cannot determine config directory".into()))?
        .join("llave-cli");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// XDG-compliant data directory
pub fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .ok_or_else(|| LlaveError::Config("Cannot determine data directory".into()))?
        .join("llave-cli");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub nif: Option<String>,
    pub device_id: Option<String>,
    pub language: Option<String>,
    /// HTTP/HTTPS/SOCKS5 proxy URL (e.g. `socks5://127.0.0.1:1080`).
    pub proxy: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_dir()?.join("config.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_dir()?.join("config.json");
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

/// Validate Spanish NIF/NIE format
pub fn validate_nif(nif: &str) -> Result<String> {
    let nif = nif.trim().to_uppercase();
    if nif.len() < 8 || nif.len() > 9 {
        return Err(LlaveError::InvalidNif(nif));
    }

    let first = nif.chars().next().unwrap();
    let valid = match first {
        '0'..='9' => true,       // DNI
        'X' | 'Y' | 'Z' => true, // NIE
        _ => false,
    };

    if !valid {
        return Err(LlaveError::InvalidNif(nif));
    }

    Ok(nif)
}
