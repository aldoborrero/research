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
