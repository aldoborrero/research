use crate::error::{ClaveError, Result};
use std::sync::OnceLock;

/// Platform-agnostic secure credential storage.
///
/// Each platform provides its own implementation:
/// - **Desktop (CLI)**: [`KeyringStorage`] — delegates to the OS secret store
///   (GNOME Keyring, macOS Keychain, Windows Credential Manager) via the
///   `keyring` crate.
/// - **Mobile (FFI)**: [`MemoryStorage`] — holds credentials in-memory while
///   the app is running. Flutter hydrates from `flutter_secure_storage` on
///   startup and persists back after mutations.
pub trait SecureStorage: Send + Sync {
    fn save(&self, data: &str) -> Result<()>;
    fn load(&self) -> Result<String>;
    fn delete(&self) -> Result<()>;
}

static STORAGE: OnceLock<Box<dyn SecureStorage>> = OnceLock::new();

/// Register the storage backend. Must be called once at startup.
pub fn init_storage(backend: Box<dyn SecureStorage>) {
    let _ = STORAGE.set(backend);
}

/// Get the registered storage backend.
pub(crate) fn storage() -> Result<&'static dyn SecureStorage> {
    STORAGE
        .get()
        .map(|b| b.as_ref())
        .ok_or_else(|| ClaveError::Config("Storage backend not initialized".into()))
}

// ---------------------------------------------------------------------------
// Desktop: OS keyring
// ---------------------------------------------------------------------------

/// Stores credentials in the OS secret store via the `keyring` crate.
pub struct KeyringStorage {
    service: String,
    /// A fixed account identifier (the NIF is unknown at init time, so we use a
    /// well-known key and store the full session JSON as the "password").
    account: String,
}

impl KeyringStorage {
    pub fn new() -> Self {
        Self {
            service: "clave-cli".into(),
            account: "session".into(),
        }
    }
}

impl SecureStorage for KeyringStorage {
    fn save(&self, data: &str) -> Result<()> {
        let entry = keyring::Entry::new(&self.service, &self.account)
            .map_err(|e| ClaveError::Keyring(e.to_string()))?;
        entry
            .set_password(data)
            .map_err(|e| ClaveError::Keyring(e.to_string()))?;
        Ok(())
    }

    fn load(&self) -> Result<String> {
        let entry = keyring::Entry::new(&self.service, &self.account)
            .map_err(|e| ClaveError::Keyring(e.to_string()))?;
        entry
            .get_password()
            .map_err(|e| ClaveError::Keyring(e.to_string()))
    }

    fn delete(&self) -> Result<()> {
        let entry = keyring::Entry::new(&self.service, &self.account)
            .map_err(|e| ClaveError::Keyring(e.to_string()))?;
        let _ = entry.delete_credential();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Mobile / FFI: in-memory store
// ---------------------------------------------------------------------------

/// In-memory credential store for mobile platforms.
///
/// Flutter is responsible for persisting to / loading from the platform's
/// native secure storage (`flutter_secure_storage`). On app startup, Flutter
/// calls [`crate::session::Session::set_session_data`] to hydrate this store;
/// after mutations (activate, logout) the FFI layer returns session data for
/// Flutter to persist.
pub struct MemoryStorage {
    data: std::sync::RwLock<Option<String>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: std::sync::RwLock::new(None),
        }
    }
}

impl SecureStorage for MemoryStorage {
    fn save(&self, data: &str) -> Result<()> {
        let mut guard = self.data.write().map_err(|e| {
            ClaveError::Config(format!("storage lock poisoned: {e}"))
        })?;
        *guard = Some(data.to_owned());
        Ok(())
    }

    fn load(&self) -> Result<String> {
        let guard = self.data.read().map_err(|e| {
            ClaveError::Config(format!("storage lock poisoned: {e}"))
        })?;
        guard.clone().ok_or(ClaveError::NoSession)
    }

    fn delete(&self) -> Result<()> {
        let mut guard = self.data.write().map_err(|e| {
            ClaveError::Config(format!("storage lock poisoned: {e}"))
        })?;
        *guard = None;
        Ok(())
    }
}
