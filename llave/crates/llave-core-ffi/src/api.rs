use flutter_rust_bridge::frb;

// ---------------------------------------------------------------------------
// FFI-safe types (no lifetimes, simple owned data)
// ---------------------------------------------------------------------------

/// Session information returned after activation.
#[frb]
pub struct FfiSession {
    pub device_id: String,
    pub nif: String,
    pub created_at: String,
    pub has_firebase_token: bool,
}

/// Result of a PIN request.
#[frb]
pub struct FfiPinResult {
    pub pin: String,
    pub time_to_live_seconds: String,
    pub nif: String,
}

/// Status of the current session.
#[frb]
pub struct FfiStatus {
    pub active: bool,
    pub nif: Option<String>,
    pub device_id: Option<String>,
    pub created_at: Option<String>,
    pub has_firebase_token: bool,
}

/// Generic API result carrying JSON data.
#[frb]
pub struct FfiApiResult {
    pub ok: bool,
    pub data: String, // JSON string
    pub error: Option<String>,
}

/// NIF check result.
#[frb]
pub struct FfiNifCheckResult {
    pub nif: String,
    pub status: String,
    pub response_json: String,
}

// ---------------------------------------------------------------------------
// Storage initialisation — must be called before any other function.
// ---------------------------------------------------------------------------

/// Initialise the Rust core storage backend.
///
/// On **Linux desktop** the Rust core uses [`KeyringStorage`] which persists
/// the session in the OS secret store (GNOME Keyring / KDE Wallet / etc.)
/// directly — Flutter does not need to handle persistence.
///
/// On **mobile** (Android/iOS) the core uses [`MemoryStorage`]; Flutter is
/// responsible for persisting via `flutter_secure_storage` and passing the
/// saved JSON here on startup.
#[frb]
pub fn init_core(session_json: Option<String>) -> Result<bool, String> {
    if cfg!(target_os = "linux") {
        // Desktop Linux: Rust owns persistence via the OS keyring.
        llave_core::init_storage(Box::new(llave_core::KeyringStorage::new()));
    } else {
        // Mobile: in-memory storage, Flutter handles persistence.
        llave_core::init_storage(Box::new(llave_core::MemoryStorage::new()));
        if let Some(json) = session_json {
            llave_core::Session::set_session_data(&json).map_err(|e| e.to_string())?;
        }
    }
    Ok(true)
}

/// Export the current session as a JSON string.
///
/// Flutter should call this after operations that create or mutate the session
/// (e.g. `activate_device`) and persist the returned JSON in
/// `flutter_secure_storage`.  Returns `null` if there is no active session.
#[frb]
pub fn export_session() -> Option<String> {
    llave_core::Session::get_session_data().ok()
}

// ---------------------------------------------------------------------------
// Bridge functions
// ---------------------------------------------------------------------------

/// Activate this device with Llave.
#[frb]
pub async fn activate_device(
    nif: String,
    password: Option<String>,
) -> Result<FfiSession, String> {
    let nif = llave_core::config::validate_nif(&nif).map_err(|e| e.to_string())?;
    let device_id = uuid::Uuid::new_v4().to_string();
    let device_password = password.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;
    let session =
        llave_core::auth::activate_device(&client, &nif, &device_id, &device_password)
            .await
            .map_err(|e| e.to_string())?;

    // Save config
    if let Ok(mut cfg) = llave_core::Config::load() {
        cfg.nif = Some(nif.clone());
        cfg.device_id = Some(device_id.clone());
        let _ = cfg.save();
    }

    Ok(FfiSession {
        device_id: session.device_id,
        nif: session.nif,
        created_at: session.created_at,
        has_firebase_token: session.firebase_token.is_some(),
    })
}

/// Request a Llave PIN.
#[frb]
pub async fn request_pin() -> Result<FfiPinResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;
    let (pin, ttl) = llave_core::auth::request_pin(&client, &session)
        .await
        .map_err(|e| e.to_string())?;

    Ok(FfiPinResult {
        pin,
        time_to_live_seconds: ttl,
        nif: session.nif,
    })
}

/// Get current session status.
#[frb]
pub fn get_status() -> FfiStatus {
    match llave_core::Session::load() {
        Ok(session) => FfiStatus {
            active: true,
            nif: Some(session.nif),
            device_id: Some(session.device_id),
            created_at: Some(session.created_at),
            has_firebase_token: session.firebase_token.is_some(),
        },
        Err(_) => FfiStatus {
            active: false,
            nif: None,
            device_id: None,
            created_at: None,
            has_firebase_token: false,
        },
    }
}

/// Check if a NIF is registered in Llave.
#[frb]
pub async fn check_nif(nif: Option<String>) -> Result<FfiNifCheckResult, String> {
    let nif = if let Some(n) = nif {
        llave_core::config::validate_nif(&n).map_err(|e| e.to_string())?
    } else {
        let cfg = llave_core::Config::load().map_err(|e| e.to_string())?;
        cfg.nif.ok_or("No NIF configured")?
    };

    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;
    let device_id = uuid::Uuid::new_v4().to_string();

    let _starting = client
        .starting(&device_id, &nif, "")
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .is_nif_activated(&device_id, &nif)
        .await
        .map_err(|e| e.to_string())?;

    Ok(FfiNifCheckResult {
        nif,
        status: resp.status,
        response_json: serde_json::to_string(&resp.respuesta).unwrap_or_default(),
    })
}

/// Get account data.
#[frb]
pub async fn get_my_data() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client
        .check_my_data(&session.device_id, &session.nif)
        .await
    {
        Ok(resp) => Ok(FfiApiResult {
            ok: true,
            data: serde_json::to_string(&resp.respuesta).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Get operations history.
#[frb]
pub async fn get_history() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client
        .operations_history(
            &session.device_id,
            &session.device_password,
            &session.nif,
        )
        .await
    {
        Ok(resp) => Ok(FfiApiResult {
            ok: true,
            data: serde_json::to_string(&resp.respuesta).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Poll for pending authentication requests (single poll).
#[frb]
pub async fn get_pending_requests() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    match llave_core::auth::poll_pending_requests(&client, &session).await {
        Ok(data) => Ok(FfiApiResult {
            ok: true,
            data: serde_json::to_string(&data).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Confirm a pending Llave Móvil authentication request.
#[frb]
pub async fn confirm_request(token: String, idp_code: String) -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    match llave_core::auth::confirm_authentication(&client, &session, &token, &idp_code).await {
        Ok(data) => Ok(FfiApiResult {
            ok: true,
            data: serde_json::to_string(&data).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Reject a pending Llave Móvil authentication request.
#[frb]
pub async fn reject_request(token: String, idp_code: String) -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    match llave_core::auth::reject_authentication(&client, &session, &token, &idp_code).await {
        Ok(data) => Ok(FfiApiResult {
            ok: true,
            data: serde_json::to_string(&data).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// QR code authentication.
#[frb]
pub async fn qr_authenticate(value: String) -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client
        .qr_authenticate(&session.device_id, &session.nif, &value)
        .await
    {
        Ok(resp) => Ok(FfiApiResult {
            ok: true,
            data: serde_json::to_string(&resp.respuesta).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Deactivate this device.
#[frb]
pub async fn deactivate() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client
        .deactivate_authentication(&session.device_id, &session.nif)
        .await
    {
        Ok(resp) => {
            let _ = llave_core::Session::delete();
            Ok(FfiApiResult {
                ok: true,
                data: serde_json::to_string(&serde_json::json!({
                    "status": resp.status,
                    "message": "Device deactivated"
                }))
                .unwrap_or_default(),
                error: None,
            })
        }
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Log out and clear saved credentials.
#[frb]
pub fn logout() -> Result<bool, String> {
    llave_core::Session::delete().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Validate a NIF format without making any API calls.
#[frb]
pub fn validate_nif(nif: String) -> Result<String, String> {
    llave_core::config::validate_nif(&nif).map_err(|e| e.to_string())
}

/// Authenticate using DNI/NIE weak authentication (no certificate needed).
///
/// Requires the NIF, the expiry date of the physical DNI card (DD/MM/YYYY),
/// and the support number printed on the card.
#[frb]
pub async fn dni_authenticate(
    nif: String,
    fecha: String,
    soporte: String,
) -> Result<FfiApiResult, String> {
    let nif = llave_core::config::validate_nif(&nif).map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    match llave_core::auth::authenticate_dni(&client, &nif, &fecha, &soporte).await {
        Ok(html) => Ok(FfiApiResult {
            ok: true,
            data: html,
            error: None,
        }),
        Err(e) => Ok(FfiApiResult {
            ok: false,
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}
