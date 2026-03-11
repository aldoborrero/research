use flutter_rust_bridge::frb;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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
/// The FFI layer always uses [`MemoryStorage`]; Flutter is responsible for
/// persisting via `flutter_secure_storage` and passing the saved JSON here
/// on startup.  (The CLI uses [`KeyringStorage`] directly — see
/// `llave-cli/src/main.rs`.)
#[frb]
pub fn init_core(session_json: Option<String>) -> Result<bool, String> {
    // Initialise logfmt tracing for Rust core (only once; ignore if already set).
    let _ = tracing_subscriber::Registry::default()
        .with(tracing_logfmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llave_core=info,llave_core_ffi=info,warn".parse().unwrap()),
        )
        .try_init();

    // FFI is always called from Flutter, which manages persistence via
    // flutter_secure_storage.  Use MemoryStorage on all platforms — the
    // KeyringStorage backend is reserved for the standalone CLI.
    tracing::info!(backend = "memory", "init storage");
    llave_core::init_storage(Box::new(llave_core::MemoryStorage::new()));
    if let Some(json) = session_json {
        llave_core::Session::set_session_data(&json).map_err(|e| e.to_string())?;
        tracing::debug!("session hydrated from flutter");
    }
    Ok(true)
}

/// Set the proxy URL for all subsequent API calls.
///
/// Supports HTTP, HTTPS, and SOCKS5 URLs (e.g. `socks5://127.0.0.1:1080`).
/// Pass `None` to disable the proxy.
#[frb]
pub fn set_proxy(url: Option<String>) {
    tracing::info!(proxy = url.as_deref().unwrap_or("none"), "set proxy");
    llave_core::set_proxy(url);
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
    let device_password = password.unwrap_or_else(|| llave_core::crypto::generate_device_password());

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
        .clave_starting(&device_id, &nif, "")
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .clave_is_nif_activated(&device_id, &nif)
        .await
        .map_err(|e| e.to_string())?;

    Ok(FfiNifCheckResult {
        nif,
        status: resp.status,
        response_json: serde_json::to_string(&resp.respuesta).unwrap_or_default(),
    })
}

/// Get account data.
///
/// Mirrors the Android app flow: ClaveIsNifActivatedSv → ClaveCheckMyDataSv.
/// The response contains `email`, `numTelefono`, and `nivelRegistro`.
#[frb]
pub async fn get_my_data() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    // Step 1: ClaveIsNifActivatedSv (the Android app calls this before CheckMyData).
    let _activated = client
        .clave_is_nif_activated(&session.device_id, &session.nif)
        .await
        .map_err(|e| e.to_string())?;

    // Step 2: ClaveCheckMyDataSv (returns email, numTelefono, nivelRegistro).
    match client
        .clave_check_my_data(&session.device_id, &session.nif)
        .await
    {
        Ok(resp) => {
            // Build a combined response including NIF (not returned by server
            // but known from the session) so the UI can display it.
            let mut data = serde_json::Map::new();
            data.insert("nif".into(), serde_json::json!(session.nif));
            if let Some(ref inner) = resp.respuesta {
                if let Some(ref v) = inner.email {
                    data.insert("email".into(), serde_json::json!(v));
                }
                if let Some(ref v) = inner.num_telefono {
                    data.insert("numTelefono".into(), serde_json::json!(v));
                }
                if let Some(ref v) = inner.nivel_registro {
                    data.insert("nivelRegistro".into(), serde_json::json!(v));
                }
            }
            Ok(FfiApiResult {
                ok: true,
                data: serde_json::to_string(&data).unwrap_or_default(),
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

/// Get operations history.
#[frb]
pub async fn get_history() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .clave_starting(&session.device_id, &session.nif, "")
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
        .clave_starting(&session.device_id, &session.nif, "")
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
        .clave_starting(&session.device_id, &session.nif, "")
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

// ---------------------------------------------------------------------------
// PIN-based session encryption
// ---------------------------------------------------------------------------

/// Encrypt the current in-memory session with a PIN.
///
/// Returns a base64-encoded sealed blob. Flutter should store this in
/// `flutter_secure_storage` instead of raw JSON.
#[frb]
pub fn encrypt_session(pin: String) -> Result<String, String> {
    let json = llave_core::Session::get_session_data().map_err(|e| e.to_string())?;
    llave_core::crypto::seal_session(&json, &pin).map_err(|e| e.to_string())
}

/// Decrypt a sealed session blob and load it into the Rust core.
///
/// On success the session is available for all subsequent API calls.
/// Returns an error if the PIN is wrong (AES-GCM auth fails).
#[frb]
pub fn decrypt_and_load_session(sealed: String, pin: String) -> Result<bool, String> {
    let json =
        llave_core::crypto::open_session(&sealed, &pin).map_err(|e| e.to_string())?;
    llave_core::Session::set_session_data(&json).map_err(|e| e.to_string())?;
    tracing::debug!("session decrypted and loaded");
    Ok(true)
}

/// Re-encrypt the current session with a new PIN.
///
/// Requires the old PIN to decrypt first (validates the caller knows it),
/// then re-encrypts with the new PIN and returns the new sealed blob.
#[frb]
pub fn change_session_pin(
    sealed: String,
    old_pin: String,
    new_pin: String,
) -> Result<String, String> {
    let json =
        llave_core::crypto::open_session(&sealed, &old_pin).map_err(|e| e.to_string())?;
    llave_core::crypto::seal_session(&json, &new_pin).map_err(|e| e.to_string())
}

/// Phase 1: DNI/NIE auth → registration check → request SMS code.
///
/// Returns the registration state, masked phone number, SMS tokens,
/// and serialised cookies for phase 2.
///
/// After this call, the user must enter the SMS PIN they receive,
/// then call [`dni_complete_activation`] with the PIN and cookies.
#[frb]
pub async fn dni_authenticate(
    nif: String,
    fecha: String,
    soporte: String,
) -> Result<FfiApiResult, String> {
    let nif = llave_core::config::validate_nif(&nif).map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    match llave_core::auth::dni_request_sms(&client, &nif, &fecha, &soporte).await {
        Ok(phase1) => {
            tracing::info!(
                movil = %phase1.sms.movil,
                "DNI/NIE auth succeeded — SMS sent"
            );
            Ok(FfiApiResult {
                ok: true,
                data: serde_json::to_string(&serde_json::json!({
                    "registrado": phase1.state.registrado,
                    "nivel_registro": phase1.state.nivel_registro,
                    "telefono": phase1.state.telefono,
                    "movil": phase1.sms.movil,
                    "hora_peticion": phase1.sms.hora_peticion,
                    "timestamp_alta_sms": phase1.sms.timestamp_alta_sms,
                    "token_clave_movil_sms": phase1.sms.token_clave_movil_sms,
                    "cookies_json": phase1.cookies_json,
                }))
                .unwrap_or_default(),
                error: None,
            })
        }
        Err(e) => {
            tracing::warn!(err = %e, "DNI/NIE auth + SMS request failed");
            Ok(FfiApiResult {
                ok: false,
                data: String::new(),
                error: Some(e.to_string()),
            })
        }
    }
}

/// Phase 2: Validate SMS code + activate device.
///
/// Takes the SMS tokens and cookies from [`dni_authenticate`] (phase 1),
/// validates the user-entered SMS PIN, then activates the device on www6.
#[frb]
pub async fn dni_complete_activation(
    nif: String,
    cookies_json: String,
    timestamp_alta_sms: String,
    token_clave_movil_sms: String,
    sms_pin: String,
) -> Result<FfiApiResult, String> {
    let nif = llave_core::config::validate_nif(&nif).map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;
    let device_password = llave_core::crypto::generate_device_password();

    match llave_core::auth::dni_validate_and_activate(
        &client,
        &cookies_json,
        &nif,
        &timestamp_alta_sms,
        &token_clave_movil_sms,
        &sms_pin,
        &device_password,
    )
    .await
    {
        Ok(session) => {
            if let Ok(mut cfg) = llave_core::Config::load() {
                cfg.nif = Some(nif.clone());
                cfg.device_id = Some(session.device_id.clone());
                let _ = cfg.save();
            }
            tracing::info!(nif = %session.nif, "DNI/NIE + SMS activation succeeded");
            Ok(FfiApiResult {
                ok: true,
                data: serde_json::to_string(&serde_json::json!({
                    "device_id": session.device_id,
                    "nif": session.nif,
                }))
                .unwrap_or_default(),
                error: None,
            })
        }
        Err(e) => {
            tracing::warn!(err = %e, "DNI/NIE SMS validation + activation failed");
            Ok(FfiApiResult {
                ok: false,
                data: String::new(),
                error: Some(e.to_string()),
            })
        }
    }
}

/// Register a Firebase push notification token.
#[frb]
pub async fn set_firebase_token(token_push: String) -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .clave_starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client
        .set_firebase_token(&session.device_id, &session.nif, &token_push)
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

/// Request an SMS verification code.
#[frb]
pub async fn request_sms_code() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .clave_starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client.request_sms_code().await {
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

/// Validate an SMS verification code.
#[frb]
pub async fn validate_sms_code(
    timestamp: String,
    token: String,
    pin: String,
) -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .clave_starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client.validate_sms_code(&timestamp, &token, &pin).await {
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

/// Get Llave Móvil token.
#[frb]
pub async fn get_llave_movil() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .clave_starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client.get_llave_movil().await {
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

/// Validate a Llave Móvil token.
#[frb]
pub async fn validate_llave_movil(token: String) -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .clave_starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client.validate_llave_movil(&token).await {
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

/// Get pending market petitions.
#[frb]
pub async fn get_pending_petitions() -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    let _starting = client
        .clave_starting(&session.device_id, &session.nif, "")
        .await
        .map_err(|e| e.to_string())?;

    match client
        .get_pending_petitions(&session.device_id, &session.nif)
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

/// Listen for pending authentication requests (polling loop).
#[frb]
pub async fn listen_for_requests(
    interval_secs: u64,
    max_attempts: u32,
) -> Result<FfiApiResult, String> {
    let session = llave_core::Session::load().map_err(|e| e.to_string())?;
    let client = llave_core::LlaveClient::new().map_err(|e| e.to_string())?;

    match llave_core::auth::listen_for_requests(&client, &session, interval_secs, max_attempts)
        .await
    {
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
