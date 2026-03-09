use axum::{
    extract::{Path, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use rust_embed::Embed;
use serde::Deserialize;

use crate::AppState;

#[derive(Embed)]
#[folder = "frontend/"]
struct FrontendAssets;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_status))
        .route("/activate", post(activate))
        .route("/pin", post(request_pin))
        .route("/pending", get(get_pending))
        .route("/confirm/{request_id}/{idp_code}", post(confirm_request))
        .route("/reject/{request_id}/{idp_code}", post(reject_request))
        .route("/history", get(get_history))
        .route("/my-data", get(get_my_data))
        .route("/qr-auth", post(qr_auth))
        .route("/deactivate", post(deactivate))
}

async fn get_status(State(_state): State<AppState>) -> impl IntoResponse {
    match llave_core::Session::load() {
        Ok(session) => Json(serde_json::json!({
            "ok": true,
            "data": {
                "active": true,
                "nif": session.nif,
                "device_id": session.device_id,
                "created_at": session.created_at,
                "has_firebase_token": session.firebase_token.is_some(),
            }
        }))
        .into_response(),
        Err(_) => Json(serde_json::json!({
            "ok": true,
            "data": {
                "active": false,
                "message": "No active session"
            }
        }))
        .into_response(),
    }
}

#[derive(Deserialize)]
struct ActivateRequest {
    nif: String,
    password: Option<String>,
}

async fn activate(
    State(state): State<AppState>,
    Json(body): Json<ActivateRequest>,
) -> impl IntoResponse {
    let nif = match llave_core::config::validate_nif(&body.nif) {
        Ok(n) => n,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"ok": false, "error": e.to_string()})),
            )
                .into_response()
        }
    };

    let device_id = uuid::Uuid::new_v4().to_string();
    let device_password = body
        .password
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    match llave_core::auth::activate_device(&state.client, &nif, &device_id, &device_password)
        .await
    {
        Ok(session) => {
            if let Ok(mut cfg) = llave_core::Config::load() {
                cfg.nif = Some(nif.clone());
                cfg.device_id = Some(device_id.clone());
                let _ = cfg.save();
            }
            Json(serde_json::json!({
                "ok": true,
                "data": {
                    "nif": session.nif,
                    "device_id": session.device_id,
                    "created_at": session.created_at,
                }
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn request_pin(State(state): State<AppState>) -> impl IntoResponse {
    match llave_core::auth::request_pin(&state.client, &state.session).await {
        Ok((pin, ttl)) => Json(serde_json::json!({
            "ok": true,
            "data": { "pin": pin, "time_to_live_seconds": ttl }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_pending(State(state): State<AppState>) -> impl IntoResponse {
    match llave_core::auth::poll_pending_requests(&state.client, &state.session).await {
        Ok(data) => Json(serde_json::json!({"ok": true, "data": data})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn confirm_request(
    State(state): State<AppState>,
    Path((request_id, idp_code)): Path<(String, String)>,
) -> impl IntoResponse {
    match llave_core::auth::confirm_authentication(
        &state.client,
        &state.session,
        &request_id,
        &idp_code,
    )
    .await
    {
        Ok(data) => Json(serde_json::json!({"ok": true, "data": data})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn reject_request(
    State(state): State<AppState>,
    Path((request_id, idp_code)): Path<(String, String)>,
) -> impl IntoResponse {
    match llave_core::auth::reject_authentication(
        &state.client,
        &state.session,
        &request_id,
        &idp_code,
    )
    .await
    {
        Ok(data) => Json(serde_json::json!({"ok": true, "data": data})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_history(State(state): State<AppState>) -> impl IntoResponse {
    let _starting = state
        .client
        .starting(&state.session.device_id, &state.session.nif, "")
        .await;

    match state
        .client
        .operations_history(
            &state.session.device_id,
            &state.session.device_password,
            &state.session.nif,
        )
        .await
    {
        Ok(resp) => Json(serde_json::json!({
            "ok": true,
            "data": { "status": resp.status, "operations": resp.respuesta }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_my_data(State(state): State<AppState>) -> impl IntoResponse {
    let _starting = state
        .client
        .starting(&state.session.device_id, &state.session.nif, "")
        .await;

    match state
        .client
        .check_my_data(&state.session.device_id, &state.session.nif)
        .await
    {
        Ok(resp) => Json(serde_json::json!({
            "ok": true,
            "data": { "status": resp.status, "details": resp.respuesta }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct QrRequest {
    value: String,
}

async fn qr_auth(
    State(state): State<AppState>,
    Json(body): Json<QrRequest>,
) -> impl IntoResponse {
    let _starting = state
        .client
        .starting(&state.session.device_id, &state.session.nif, "")
        .await;

    match state
        .client
        .qr_authenticate(&state.session.device_id, &state.session.nif, &body.value)
        .await
    {
        Ok(resp) => Json(serde_json::json!({
            "ok": true,
            "data": { "status": resp.status, "response": resp.respuesta }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn deactivate(State(state): State<AppState>) -> impl IntoResponse {
    let _starting = state
        .client
        .starting(&state.session.device_id, &state.session.nif, "")
        .await;

    match state
        .client
        .deactivate_authentication(&state.session.device_id, &state.session.nif)
        .await
    {
        Ok(resp) => {
            let _ = llave_core::Session::delete();
            Json(serde_json::json!({
                "ok": true,
                "data": { "status": resp.status, "message": "Device deactivated" }
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match FrontendAssets::get(path) {
        Some(content) => {
            let mime = match path {
                p if p.ends_with(".html") => "text/html",
                p if p.ends_with(".js") => "application/javascript",
                p if p.ends_with(".css") => "text/css",
                p if p.ends_with(".json") => "application/json",
                p if p.ends_with(".svg") => "image/svg+xml",
                _ => "application/octet-stream",
            };
            ([(header::CONTENT_TYPE, mime)], content.data).into_response()
        }
        None => {
            // SPA fallback
            match FrontendAssets::get("index.html") {
                Some(content) => {
                    ([(header::CONTENT_TYPE, "text/html")], content.data).into_response()
                }
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}
