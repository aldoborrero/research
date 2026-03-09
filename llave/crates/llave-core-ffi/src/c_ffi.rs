//! Manual C FFI layer for Flutter dart:ffi, bypassing flutter_rust_bridge codegen.
//!
//! Every function returns a `*mut c_char` (JSON string) that the caller **must**
//! free with [`llave_free_string`].  The JSON envelope is:
//!   `{"ok": true, "data": <value>}` on success
//!   `{"ok": false, "error": "message"}` on failure

use std::ffi::{c_char, CStr, CString};
use std::sync::OnceLock;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn rt() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime"))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

unsafe fn to_opt(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

unsafe fn to_str(ptr: *const c_char) -> String {
    to_opt(ptr).unwrap_or_default()
}

fn c(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

fn ok_json(data: serde_json::Value) -> *mut c_char {
    c(serde_json::json!({"ok": true, "data": data}).to_string())
}

fn err_json(msg: &str) -> *mut c_char {
    c(serde_json::json!({"ok": false, "error": msg}).to_string())
}

// ---------------------------------------------------------------------------
// Free
// ---------------------------------------------------------------------------

/// Free a string returned by any `llave_*` function.
#[no_mangle]
pub extern "C" fn llave_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

// ---------------------------------------------------------------------------
// init / session management
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn llave_init_core(session_json: *const c_char) -> *mut c_char {
    match crate::api::init_core(to_opt(session_json)) {
        Ok(_) => ok_json(serde_json::json!(true)),
        Err(e) => err_json(&e),
    }
}

#[no_mangle]
pub extern "C" fn llave_export_session() -> *mut c_char {
    match crate::api::export_session() {
        Some(json) => ok_json(serde_json::Value::String(json)),
        None => ok_json(serde_json::Value::Null),
    }
}

#[no_mangle]
pub extern "C" fn llave_get_status() -> *mut c_char {
    let s = crate::api::get_status();
    ok_json(serde_json::json!({
        "active": s.active,
        "nif": s.nif,
        "deviceId": s.device_id,
        "createdAt": s.created_at,
        "hasFirebaseToken": s.has_firebase_token,
    }))
}

#[no_mangle]
pub extern "C" fn llave_logout() -> *mut c_char {
    match crate::api::logout() {
        Ok(_) => ok_json(serde_json::json!(true)),
        Err(e) => err_json(&e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn llave_validate_nif(nif: *const c_char) -> *mut c_char {
    match crate::api::validate_nif(to_str(nif)) {
        Ok(v) => ok_json(serde_json::Value::String(v)),
        Err(e) => err_json(&e),
    }
}

// ---------------------------------------------------------------------------
// Async operations (block_on)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn llave_activate_device(
    nif: *const c_char,
    password: *const c_char,
) -> *mut c_char {
    let nif = to_str(nif);
    let password = to_opt(password);
    match rt().block_on(crate::api::activate_device(nif, password)) {
        Ok(s) => ok_json(serde_json::json!({
            "deviceId": s.device_id,
            "nif": s.nif,
            "createdAt": s.created_at,
            "hasFirebaseToken": s.has_firebase_token,
        })),
        Err(e) => err_json(&e),
    }
}

#[no_mangle]
pub extern "C" fn llave_request_pin() -> *mut c_char {
    match rt().block_on(crate::api::request_pin()) {
        Ok(r) => ok_json(serde_json::json!({
            "pin": r.pin,
            "timeToLiveSeconds": r.time_to_live_seconds,
            "nif": r.nif,
        })),
        Err(e) => err_json(&e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn llave_check_nif(nif: *const c_char) -> *mut c_char {
    let nif = to_opt(nif);
    match rt().block_on(crate::api::check_nif(nif)) {
        Ok(r) => ok_json(serde_json::json!({
            "nif": r.nif,
            "status": r.status,
            "responseJson": r.response_json,
        })),
        Err(e) => err_json(&e),
    }
}

fn api_result_json(r: Result<crate::api::FfiApiResult, String>) -> *mut c_char {
    match r {
        Ok(r) => ok_json(serde_json::json!({
            "ok": r.ok,
            "data": r.data,
            "error": r.error,
        })),
        Err(e) => err_json(&e),
    }
}

#[no_mangle]
pub extern "C" fn llave_get_my_data() -> *mut c_char {
    api_result_json(rt().block_on(crate::api::get_my_data()))
}

#[no_mangle]
pub extern "C" fn llave_get_history() -> *mut c_char {
    api_result_json(rt().block_on(crate::api::get_history()))
}

#[no_mangle]
pub extern "C" fn llave_get_pending_requests() -> *mut c_char {
    api_result_json(rt().block_on(crate::api::get_pending_requests()))
}

#[no_mangle]
pub unsafe extern "C" fn llave_confirm_request(
    token: *const c_char,
    idp_code: *const c_char,
) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::confirm_request(to_str(token), to_str(idp_code))))
}

#[no_mangle]
pub unsafe extern "C" fn llave_reject_request(
    token: *const c_char,
    idp_code: *const c_char,
) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::reject_request(to_str(token), to_str(idp_code))))
}

#[no_mangle]
pub unsafe extern "C" fn llave_qr_authenticate(value: *const c_char) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::qr_authenticate(to_str(value))))
}

#[no_mangle]
pub extern "C" fn llave_deactivate() -> *mut c_char {
    api_result_json(rt().block_on(crate::api::deactivate()))
}

#[no_mangle]
pub unsafe extern "C" fn llave_dni_authenticate(
    nif: *const c_char,
    fecha: *const c_char,
    soporte: *const c_char,
) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::dni_authenticate(
        to_str(nif),
        to_str(fecha),
        to_str(soporte),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn llave_set_firebase_token(token_push: *const c_char) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::set_firebase_token(to_str(token_push))))
}

#[no_mangle]
pub extern "C" fn llave_request_sms_code() -> *mut c_char {
    api_result_json(rt().block_on(crate::api::request_sms_code()))
}

#[no_mangle]
pub unsafe extern "C" fn llave_validate_sms_code(
    timestamp: *const c_char,
    token: *const c_char,
    pin: *const c_char,
) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::validate_sms_code(
        to_str(timestamp),
        to_str(token),
        to_str(pin),
    )))
}

#[no_mangle]
pub extern "C" fn llave_get_llave_movil() -> *mut c_char {
    api_result_json(rt().block_on(crate::api::get_llave_movil()))
}

#[no_mangle]
pub unsafe extern "C" fn llave_validate_llave_movil(token: *const c_char) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::validate_llave_movil(to_str(token))))
}

#[no_mangle]
pub extern "C" fn llave_get_pending_petitions() -> *mut c_char {
    api_result_json(rt().block_on(crate::api::get_pending_petitions()))
}

#[no_mangle]
pub unsafe extern "C" fn llave_listen_for_requests(
    interval_secs: u64,
    max_attempts: u32,
) -> *mut c_char {
    api_result_json(rt().block_on(crate::api::listen_for_requests(
        interval_secs,
        max_attempts,
    )))
}
