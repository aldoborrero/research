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

/// Result of phase 1: DNI auth + registration check + SMS request.
///
/// Contains the registration state, SMS metadata for validation,
/// and serialised cookies to resume the session in phase 2.
pub struct DniSmsPhase1 {
    pub state: crate::api::ClaveRequestStateResponse,
    pub sms: crate::api::ObtenerSmsResponse,
    /// Serialised cookies — pass to [`dni_validate_and_activate`] to resume.
    pub cookies_json: String,
}

/// Phase 1: DNI/NIE auth → registration check → request SMS code.
///
/// Matches the Android app's sequence up to the point where the user
/// must enter the SMS PIN:
/// 1. `AutenticaDniNieContrasteh` (DNI/NIE weak auth on www2)
/// 2. `ClaveRequestStateSv` (registration check on www12)
/// 3. `ObtenerClaveMovilSMS` (trigger SMS on www12)
///
/// Returns [`DniSmsPhase1`] with the SMS metadata and serialised cookies.
/// The caller should display the masked phone number to the user, collect
/// the SMS PIN, then call [`dni_validate_and_activate`] with the PIN.
pub async fn dni_request_sms(
    client: &LlaveClient,
    nif: &str,
    fecha: &str,
    soporte: &str,
) -> Result<DniSmsPhase1> {
    // Step 0: Call starting to establish a device session on the server.
    // The Android app always calls ClaveStartingSv at app launch, which sets
    // server-side context that ObtenerClaveMovilSMS may depend on.
    // Try www12 first (ClaveStartingSv), fall back to www2 (LlaveStartingSv).
    let device_id = uuid::Uuid::new_v4().to_string();
    tracing::info!("calling clave_starting (www12) to establish device session");
    match client.clave_starting(&device_id, nif, "").await {
        Ok(resp) if resp.status == "OK" => {
            tracing::info!("clave_starting (www12) succeeded");
        }
        Ok(resp) => {
            tracing::warn!(
                status = %resp.status,
                code = resp.codigo_error.as_deref().unwrap_or("?"),
                message = resp.mensaje.as_deref().unwrap_or("?"),
                "clave_starting (www12) returned non-OK, trying starting (www2)"
            );
            match client.starting(&device_id, nif, "").await {
                Ok(r) if r.status == "OK" => {
                    tracing::info!("starting (www2) succeeded");
                }
                Ok(r) => {
                    tracing::warn!(
                        status = %r.status,
                        code = r.codigo_error.as_deref().unwrap_or("?"),
                        message = r.mensaje.as_deref().unwrap_or("?"),
                        "starting (www2) also returned non-OK (continuing anyway)"
                    );
                }
                Err(e) => {
                    tracing::warn!(err = %e, "starting (www2) failed (continuing anyway)");
                }
            }
        }
        Err(e) => {
            tracing::warn!(err = %e, "clave_starting (www12) failed, trying starting (www2)");
            match client.starting(&device_id, nif, "").await {
                Ok(r) if r.status == "OK" => {
                    tracing::info!("starting (www2) succeeded");
                }
                Ok(r) => {
                    tracing::warn!(
                        status = %r.status,
                        code = r.codigo_error.as_deref().unwrap_or("?"),
                        "starting (www2) also non-OK (continuing anyway)"
                    );
                }
                Err(e2) => {
                    tracing::warn!(err = %e2, "both starting endpoints failed (continuing anyway)");
                }
            }
        }
    }

    // Step 1: DNI/NIE auth (establishes session cookies).
    tracing::info!("authenticating via DNI/NIE");
    client.authenticate_dni_nie(nif, fecha, soporte).await?;
    {
        let c = client.export_cookies();
        let parsed: Vec<(String, String)> = serde_json::from_str(&c).unwrap_or_default();
        let names: Vec<&str> = parsed.iter().map(|(k, _)| k.as_str()).collect();
        tracing::info!(cookie_names = ?names, "DNI/NIE auth complete — cookie jar");
    }

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
    {
        let c = client.export_cookies();
        let parsed: Vec<(String, String)> = serde_json::from_str(&c).unwrap_or_default();
        let names: Vec<&str> = parsed.iter().map(|(k, _)| k.as_str()).collect();
        tracing::info!(cookie_names = ?names, "after ClaveRequestStateSv — cookie jar");
    }

    // Step 3: Request SMS code (triggers SMS to user's registered phone).
    tracing::info!("requesting SMS verification code");
    let sms_resp = client.request_sms_code().await?;
    if sms_resp.status != "OK" {
        return Err(crate::error::LlaveError::Api {
            status: sms_resp.status,
            code: sms_resp.codigo_error.unwrap_or_default(),
            message: sms_resp.mensaje.unwrap_or_else(|| "SMS request failed".into()),
        });
    }
    let sms = sms_resp.respuesta.ok_or_else(|| crate::error::LlaveError::Api {
        status: "OK".into(),
        code: String::new(),
        message: "Empty ObtenerClaveMovilSMS response".into(),
    })?;
    tracing::info!(
        movil = %sms.movil,
        hora = %sms.hora_peticion,
        "SMS code sent"
    );

    // Export cookies so they can be restored in phase 2.
    let cookies_json = client.export_cookies();

    Ok(DniSmsPhase1 {
        state,
        sms,
        cookies_json,
    })
}

/// Phase 2: Validate SMS code → activate device on www6.
///
/// Resumes the session from phase 1 using the serialised cookies,
/// validates the user-entered SMS PIN, then activates the device.
///
/// 4. `ValidarClaveMovilSMS` (validate SMS code on www12 → sets pin24H cookie)
/// 5. `ClaveActivateAuthenticationSv` (device activation on www6)
pub async fn dni_validate_and_activate(
    client: &LlaveClient,
    cookies_json: &str,
    nif: &str,
    timestamp_alta_sms: &str,
    token_clave_movil_sms: &str,
    sms_pin: &str,
    device_password: &str,
) -> Result<Session> {
    // Restore session cookies from phase 1.
    client.import_cookies(cookies_json);
    tracing::info!("restored session cookies from phase 1");

    // Step 4: Validate SMS code (sets pin24H cookie on www12).
    tracing::info!("validating SMS code");
    let validate_resp = client
        .validate_sms_code(timestamp_alta_sms, token_clave_movil_sms, sms_pin)
        .await?;
    if validate_resp.status != "OK" {
        return Err(crate::error::LlaveError::Api {
            status: validate_resp.status,
            code: validate_resp.codigo_error.unwrap_or_default(),
            message: validate_resp.mensaje.unwrap_or_else(|| "SMS validation failed".into()),
        });
    }
    tracing::info!("SMS code validated (pin24H cookie should be set)");

    // Step 5: Activate device on www6 (now has pin24H cookie).
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
    tracing::info!("device activated via DNI/NIE + SMS flow");

    Ok(session)
}

/// DNI/NIE authentication + device activation (legacy single-call flow).
///
/// **Deprecated**: Use [`dni_request_sms`] + [`dni_validate_and_activate`]
/// instead. This function skips the required SMS verification step and will
/// fail with an HTML response from www6.
pub async fn dni_activate_device(
    client: &LlaveClient,
    nif: &str,
    fecha: &str,
    soporte: &str,
    device_password: &str,
) -> Result<(crate::api::ClaveRequestStateResponse, Session)> {
    // Step 1: DNI/NIE auth (establishes session cookies).
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
    // NOTE: This will fail if pin24H cookie is not set (SMS verification skipped).
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
