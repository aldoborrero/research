use crate::api::ClaveClient;
use crate::error::Result;
use crate::session::Session;

/// High-level authentication operations that combine multiple API calls.

/// Full activation flow: starting → check NIF → activate.
pub async fn activate_device(
    client: &ClaveClient,
    nif: &str,
    device_id: &str,
    device_password: &str,
) -> Result<Session> {
    // Step 1: Initialize session
    tracing::info!("Initializing session with Cl@ve backend...");
    let starting = client.starting(device_id, nif, "").await?;
    if starting.status != "OK" {
        return Err(crate::error::ClaveError::Api {
            status: starting.status,
            code: starting.codigo_error.unwrap_or_default(),
            message: starting.mensaje.unwrap_or_else(|| "Starting failed".into()),
        });
    }
    tracing::info!("Session initialized successfully");

    // Step 2: Check if NIF is activated
    tracing::info!("Checking NIF activation status...");
    let activated = client.is_nif_activated(device_id, nif).await?;
    tracing::info!("NIF check status: {}", activated.status);

    // Step 3: Activate authentication
    tracing::info!("Activating device authentication...");
    let _activate = client.activate_authentication(device_password, "").await?;

    let session = Session {
        device_id: device_id.to_string(),
        nif: nif.to_string(),
        device_password: device_password.to_string(),
        firebase_token: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    session.save()?;
    tracing::info!("Device activated and session saved");

    Ok(session)
}

/// Request a Cl@ve PIN using saved session credentials.
pub async fn request_pin(client: &ClaveClient, session: &Session) -> Result<(String, String)> {
    // Initialize session first
    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await?;

    // Request PIN
    let resp = client
        .request_pin(&session.device_id, &session.device_password, &session.nif)
        .await?;

    let pin_data = resp.into_result()?;
    let pin = pin_data.pin.unwrap_or_default();
    let ttl = pin_data.time_to_live.unwrap_or_default();

    Ok((pin, ttl))
}

/// DNI/NIE weak authentication flow.
pub async fn authenticate_dni(
    client: &ClaveClient,
    nif: &str,
    fecha: &str,
    soporte: &str,
) -> Result<String> {
    client.authenticate_dni_nie(nif, fecha, soporte).await
}

/// Poll for pending authentication requests (replaces push notifications).
///
/// Since the CLI cannot receive Firebase push notifications, we poll
/// `ClaveRequestAllOperationsSv` and `ClaveRequestStateSv` to discover
/// pending Cl@ve Móvil authentication requests.
pub async fn poll_pending_requests(
    client: &ClaveClient,
    session: &Session,
) -> Result<serde_json::Value> {
    // Initialize session
    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await?;

    // Check request state first (faster, targeted check)
    let state = client
        .request_state(
            &session.device_id,
            &session.device_password,
            &session.nif,
        )
        .await?;

    if state.status == "OK" {
        if let Some(ref resp) = state.respuesta {
            if !resp.is_null() {
                return Ok(serde_json::json!({
                    "source": "request_state",
                    "status": state.status,
                    "data": resp,
                }));
            }
        }
    }

    // Fall back to all operations endpoint
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let operations = client
        .request_all_operations(&session.device_id, &session.nif, &timestamp)
        .await?;

    Ok(serde_json::json!({
        "source": "all_operations",
        "status": operations.status,
        "data": operations.respuesta,
    }))
}

/// Confirm a pending Cl@ve Móvil authentication request.
pub async fn confirm_authentication(
    client: &ClaveClient,
    session: &Session,
    token_clave_movil: &str,
    codigo_idp: &str,
) -> Result<serde_json::Value> {
    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await?;

    let resp = client
        .authenticate(
            &session.device_id,
            &session.device_password,
            &session.nif,
            token_clave_movil,
            codigo_idp,
        )
        .await?;

    Ok(serde_json::json!({
        "status": resp.status,
        "pending_requests": resp.respuesta.as_ref().and_then(|r| r.pending_requests.as_deref()),
    }))
}

/// Reject a pending Cl@ve Móvil authentication request.
pub async fn reject_authentication(
    client: &ClaveClient,
    session: &Session,
    token_clave_movil: &str,
    codigo_idp: &str,
) -> Result<serde_json::Value> {
    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await?;

    let resp = client
        .cancel_authenticate(
            &session.device_id,
            &session.nif,
            token_clave_movil,
            codigo_idp,
        )
        .await?;

    Ok(serde_json::json!({
        "status": resp.status,
        "response": resp.respuesta,
    }))
}

/// Continuously poll for pending requests, returning when one is found.
/// Polls every `interval_secs` seconds up to `max_attempts` times.
pub async fn listen_for_requests(
    client: &ClaveClient,
    session: &Session,
    interval_secs: u64,
    max_attempts: u32,
) -> Result<serde_json::Value> {
    for attempt in 1..=max_attempts {
        tracing::debug!("Polling attempt {attempt}/{max_attempts}...");

        let result = poll_pending_requests(client, session).await?;

        // Check if there's actual data (not null/empty)
        if let Some(data) = result.get("data") {
            if !data.is_null() {
                let has_content = match data {
                    serde_json::Value::Object(m) => !m.is_empty(),
                    serde_json::Value::Array(a) => !a.is_empty(),
                    serde_json::Value::String(s) => !s.is_empty(),
                    _ => true,
                };
                if has_content {
                    return Ok(result);
                }
            }
        }

        if attempt < max_attempts {
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    }

    Ok(serde_json::json!({
        "status": "timeout",
        "message": format!("No pending requests found after {max_attempts} attempts"),
    }))
}
