use crate::api::LlaveClient;
use crate::error::Result;
use crate::session::Session;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Events emitted by the authentication listener.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum AuthEvent {
    /// Listener has started polling.
    ListenerStarted,
    /// A poll cycle found pending request data.
    PendingData {
        source: String,
        data: serde_json::Value,
    },
    /// A poll cycle found no pending requests.
    NoPending,
    /// An error occurred during polling (non-fatal).
    PollError { message: String },
    /// Listener has stopped.
    ListenerStopped,
    /// Polling timed out after max attempts.
    Timeout { message: String },
}

/// Full activation flow: starting -> check NIF -> activate.
pub async fn activate_device(
    client: &LlaveClient,
    nif: &str,
    device_id: &str,
    device_password: &str,
) -> Result<Session> {
    tracing::info!("activating device");
    let starting = client.starting(device_id, nif, "").await?;
    if starting.status != "OK" {
        return Err(crate::error::LlaveError::Api {
            status: starting.status,
            code: starting.codigo_error.unwrap_or_default(),
            message: starting.mensaje.unwrap_or_else(|| "Starting failed".into()),
        });
    }

    let activated = client.is_nif_activated(device_id, nif).await?;
    tracing::debug!(nif_status = %activated.status, "nif check");

    let _activate = client.activate_authentication(device_password, "").await?;

    let session = Session {
        device_id: device_id.to_string(),
        nif: nif.to_string(),
        device_password: device_password.to_string(),
        firebase_token: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    session.save()?;
    tracing::info!("device activated");

    Ok(session)
}

/// Request a Llave PIN using saved session credentials.
pub async fn request_pin(client: &LlaveClient, session: &Session) -> Result<(String, String)> {
    tracing::info!("requesting pin");
    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await?;

    let resp = client
        .request_pin(&session.device_id, &session.device_password, &session.nif)
        .await?;

    let pin_data = resp.into_result()?;
    let pin = pin_data.pin.unwrap_or_default();
    let ttl = pin_data.time_to_live.unwrap_or_default();
    tracing::info!(ttl = %ttl, "pin generated");

    Ok((pin, ttl))
}

/// DNI/NIE weak authentication flow.
///
/// This endpoint returns HTML (not JSON).  The only purpose is to establish
/// session cookies that subsequent endpoints (`ClaveRequestStateSv`, etc.)
/// rely on.
pub async fn authenticate_dni(
    client: &LlaveClient,
    nif: &str,
    fecha: &str,
    soporte: &str,
) -> Result<()> {
    client.authenticate_dni_nie(nif, fecha, soporte).await
}

/// DNI/NIE authentication + device activation (full flow).
///
/// Matches the Android app's sequence:
/// 1. `AutenticaDniNieContrasteh` (DNI/NIE weak auth on www2)
/// 2. `ClaveRequestStateSv` (registration check on www12)
/// 3. `ClaveActivateAuthenticationSv` (device activation on www1)
pub async fn dni_activate_device(
    client: &LlaveClient,
    nif: &str,
    fecha: &str,
    soporte: &str,
    device_password: &str,
) -> Result<(crate::api::ClaveRequestStateResponse, Session)> {
    // Step 1: DNI/NIE auth (establishes session cookies).
    // The endpoint returns HTML, not JSON — success is determined by whether
    // session cookies were set.  Errors surface as HTTP/network failures.
    tracing::info!("authenticating via DNI/NIE");
    client.authenticate_dni_nie(nif, fecha, soporte).await?;
    tracing::info!("DNI/NIE auth complete (session cookies captured)");

    // Step 2: Check registration state (uses session cookies from step 1).
    tracing::info!("checking registration state");
    let state_resp = client.clave_request_state().await?;
    if state_resp.status != "OK" {
        return Err(crate::error::LlaveError::Api {
            status: state_resp.status,
            code: state_resp.codigo_error.unwrap_or_default(),
            message: state_resp.mensaje.unwrap_or_else(|| "Registration state check failed".into()),
        });
    }
    let state = state_resp.respuesta.ok_or_else(|| crate::error::LlaveError::Api {
        status: "OK".into(),
        code: String::new(),
        message: "Empty ClaveRequestState response".into(),
    })?;
    tracing::info!(
        registrado = state.registrado.as_deref().unwrap_or("?"),
        telefono = state.telefono.as_deref().unwrap_or("?"),
        nivel = state.nivel_registro.as_deref().unwrap_or("?"),
        "registration state"
    );

    // Step 3: Activate device (uses session cookies from steps 1+2).
    tracing::info!("activating device");
    let activate_resp = client.activate_authentication(device_password, "").await?;
    if activate_resp.status != "OK" {
        return Err(crate::error::LlaveError::Api {
            status: activate_resp.status,
            code: activate_resp.codigo_error.unwrap_or_default(),
            message: activate_resp.mensaje.unwrap_or_else(|| "Device activation failed".into()),
        });
    }

    let activate_data = activate_resp.respuesta.ok_or_else(|| crate::error::LlaveError::Api {
        status: "OK".into(),
        code: String::new(),
        message: "Empty activation response".into(),
    })?;

    let device_id = activate_data.device_id.unwrap_or_default();
    let session = Session {
        device_id,
        nif: nif.to_string(),
        device_password: device_password.to_string(),
        firebase_token: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    session.save()?;
    tracing::info!("device activated via DNI/NIE flow");

    Ok((state, session))
}

/// Poll for pending authentication requests (single poll).
pub async fn poll_pending_requests(
    client: &LlaveClient,
    session: &Session,
) -> Result<serde_json::Value> {
    let _starting = client
        .starting(&session.device_id, &session.nif, "")
        .await?;

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

/// Confirm a pending Llave Móvil authentication request.
pub async fn confirm_authentication(
    client: &LlaveClient,
    session: &Session,
    token_clave_movil: &str,
    codigo_idp: &str,
) -> Result<serde_json::Value> {
    tracing::info!("confirming auth request");
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

/// Reject a pending Llave Móvil authentication request.
pub async fn reject_authentication(
    client: &LlaveClient,
    session: &Session,
    token_clave_movil: &str,
    codigo_idp: &str,
) -> Result<serde_json::Value> {
    tracing::info!("rejecting auth request");
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

/// Continuously poll for pending requests (blocking loop for CLI).
pub async fn listen_for_requests(
    client: &LlaveClient,
    session: &Session,
    interval_secs: u64,
    max_attempts: u32,
) -> Result<serde_json::Value> {
    for attempt in 1..=max_attempts {
        tracing::debug!(attempt = attempt, max = max_attempts, "polling");

        let result = poll_pending_requests(client, session).await?;

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

/// Run an event-driven listener that sends events to a broadcast channel.
/// Used by the web server for SSE streaming.
pub async fn run_listener(
    client: &LlaveClient,
    session: &Session,
    interval: std::time::Duration,
    shutdown: CancellationToken,
    tx: broadcast::Sender<AuthEvent>,
) {
    tracing::info!("listener started");
    let _ = tx.send(AuthEvent::ListenerStarted);

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("listener stopped");
                let _ = tx.send(AuthEvent::ListenerStopped);
                break;
            }
            _ = tokio::time::sleep(interval) => {
                match poll_pending_requests(client, session).await {
                    Ok(result) => {
                        let has_data = result.get("data")
                            .map(|d| !d.is_null() && match d {
                                serde_json::Value::Object(m) => !m.is_empty(),
                                serde_json::Value::Array(a) => !a.is_empty(),
                                serde_json::Value::String(s) => !s.is_empty(),
                                _ => true,
                            })
                            .unwrap_or(false);

                        if has_data {
                            let _ = tx.send(AuthEvent::PendingData {
                                source: result.get("source")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                data: result.get("data").cloned().unwrap_or(serde_json::Value::Null),
                            });
                        } else {
                            let _ = tx.send(AuthEvent::NoPending);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(err = %e, "poll error");
                        let _ = tx.send(AuthEvent::PollError {
                            message: e.to_string(),
                        });
                    }
                }
            }
        }
    }
}
