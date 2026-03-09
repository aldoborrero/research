use crate::error::{LlaveError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://www2.agenciatributaria.gob.es";
const BASE_URL_WWW12: &str = "https://www12.agenciatributaria.gob.es";

const APP_VERSION: &str = "6.2.5";
const OS_NAME: &str = "Linux";
const OS_VERSION: &str = "CLI";
const DEVICE_MODEL: &str = "llave-cli";

/// Standard response envelope from all Llave API endpoints.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub visible: Option<String>,
    pub crashlytics: Option<String>,
    pub codigo_error: Option<String>,
    pub mensaje: Option<String>,
    pub respuesta: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn into_result(self) -> Result<T> {
        if self.status == "OK" {
            self.respuesta
                .ok_or_else(|| LlaveError::Api {
                    status: self.status,
                    code: self.codigo_error.unwrap_or_default(),
                    message: "Empty response payload".into(),
                })
        } else {
            let code = self.codigo_error.unwrap_or_default();
            let message = self.mensaje.unwrap_or_else(|| "Unknown error".into());
            tracing::error!(status = %self.status, code = %code, msg = %message, "api error");
            Err(LlaveError::Api {
                status: self.status,
                code,
                message,
            })
        }
    }
}

// --- Response types ---

#[derive(Debug, Deserialize, Serialize)]
pub struct StartingResponse {
    #[serde(rename = "listaBlancaUrls")]
    pub lista_blanca_urls: Option<Vec<String>>,
    #[serde(rename = "listaNegraUrls")]
    pub lista_negra_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IsNifActivatedResponse {
    #[serde(rename = "activado")]
    pub activated: Option<String>,
    #[serde(rename = "nivelAcceso")]
    pub access_level: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestPinResponse {
    pub pin: Option<String>,
    #[serde(rename = "timeToLive")]
    pub time_to_live: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthenticateResponse {
    pub pending_requests: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CheckMyDataResponse {
    pub nombre: Option<String>,
    pub nif: Option<String>,
    pub telefono: Option<String>,
    #[serde(rename = "nivelAcceso")]
    pub nivel_acceso: Option<String>,
    #[serde(rename = "fechaCaducidad")]
    pub fecha_caducidad: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActivateResponse {
    pub device_id: Option<String>,
    pub user_password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OperationsHistoryResponse {
    pub operaciones: Option<Vec<serde_json::Value>>,
}

/// The Llave API client.
pub struct LlaveClient {
    client: Client,
    trace_id: String,
}

impl LlaveClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .cookie_store(true)
            .user_agent(format!("llave-cli/{APP_VERSION}"))
            .timeout(std::time::Duration::from_secs(190))
            .build()?;

        let trace_id = uuid::Uuid::new_v4().to_string();
        tracing::debug!(trace_id = %trace_id, "client created");

        Ok(Self { client, trace_id })
    }

    async fn post_form<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        url: &str,
        form: &[(&str, &str)],
    ) -> Result<ApiResponse<T>> {
        tracing::debug!(endpoint = endpoint, "aeat request");
        let resp = self
            .client
            .post(url)
            .header("TrazasApp", &self.trace_id)
            .form(form)
            .send()
            .await?;
        let status = resp.status();
        tracing::debug!(endpoint = endpoint, http_status = %status, "aeat response");
        Ok(resp.json().await?)
    }

    /// Initialize a session with the Llave backend.
    pub async fn starting(
        &self,
        device_id: &str,
        nif: &str,
        token_push: &str,
    ) -> Result<ApiResponse<StartingResponse>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveStartingSv");
        self.post_form("starting", &url, &[
                ("device_id", device_id),
                ("NIF", nif),
                ("sistema_operativo", OS_NAME),
                ("token_push", token_push),
                ("version_os", OS_VERSION),
                ("version_app", APP_VERSION),
                ("modelo", DEVICE_MODEL),
            ]).await
    }

    /// Check if a NIF is activated in Llave.
    pub async fn is_nif_activated(
        &self,
        device_id: &str,
        nif: &str,
    ) -> Result<ApiResponse<IsNifActivatedResponse>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveIsNifActivatedSv");
        self.post_form("is_nif_activated", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
        ]).await
    }

    /// Request a Llave PIN.
    pub async fn request_pin(
        &self,
        device_id: &str,
        device_password: &str,
        nif: &str,
    ) -> Result<ApiResponse<RequestPinResponse>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveRequestPinSv");
        self.post_form("request_pin", &url, &[
            ("device_id", device_id),
            ("user_password", device_password),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
        ]).await
    }

    /// Confirm a Llave Móvil authentication request.
    pub async fn authenticate(
        &self,
        device_id: &str,
        device_password: &str,
        nif: &str,
        token_clave_movil: &str,
        codigo_idp: &str,
    ) -> Result<ApiResponse<AuthenticateResponse>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveAuthenticateSv");
        self.post_form("authenticate", &url, &[
            ("device_id", device_id),
            ("user_password", device_password),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
            ("tokenClaveMovil", token_clave_movil),
            ("codigoIdP", codigo_idp),
        ]).await
    }

    /// Cancel a pending Llave Móvil authentication.
    pub async fn cancel_authenticate(
        &self,
        device_id: &str,
        nif: &str,
        token_clave_movil: &str,
        codigo_idp: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveCancelAuthenticateSv");
        self.post_form("cancel_authenticate", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
            ("tokenClaveMovil", token_clave_movil),
            ("codigoIdP", codigo_idp),
        ]).await
    }

    /// Check user account data.
    pub async fn check_my_data(
        &self,
        device_id: &str,
        nif: &str,
    ) -> Result<ApiResponse<CheckMyDataResponse>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveCheckMyDataSv");
        self.post_form("check_my_data", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
        ]).await
    }

    /// Activate device authentication.
    pub async fn activate_authentication(
        &self,
        device_password: &str,
        token_push: &str,
    ) -> Result<ApiResponse<ActivateResponse>> {
        let url = format!("{BASE_URL_WWW12}/wlpl/MOVI-P24H/LlaveActivateAuthenticationSv");
        self.post_form("activate_authentication", &url, &[
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
            ("token_push", token_push),
            ("user_password", device_password),
            ("modelo", DEVICE_MODEL),
        ]).await
    }

    /// Deactivate device authentication.
    pub async fn deactivate_authentication(
        &self,
        device_id: &str,
        nif: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveDesactivateAuthSv");
        self.post_form("deactivate_authentication", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
        ]).await
    }

    /// Authenticate with DNI/NIE + date of birth (weak auth).
    pub async fn authenticate_dni_nie(
        &self,
        nif: &str,
        fecha: &str,
        soporte: &str,
    ) -> Result<String> {
        let url = format!(
            "{BASE_URL}/wlpl/BUCV-JDIT/AutenticaDniNieContrasteh?ref=%2Fwlpl%2FMOVI-AEAT%2FAccesoW12Sv"
        );
        tracing::debug!(endpoint = "authenticate_dni_nie", "aeat request");
        let resp = self
            .client
            .post(&url)
            .header("TrazasApp", &self.trace_id)
            .form(&[
                ("NIF", nif),
                ("FECHA", fecha),
                ("SOPORTE", soporte),
                ("botonAutenticacionDebil", "Acceder"),
                ("APP", "CLAVE"),
                ("modo", ""),
                ("AZUL", ""),
                ("FECHANIE", ""),
            ])
            .send()
            .await?;

        let status = resp.status();
        tracing::debug!(endpoint = "authenticate_dni_nie", http_status = %status, "aeat response");
        Ok(resp.text().await?)
    }

    /// Get pending operations for polling.
    pub async fn request_all_operations(
        &self,
        device_id: &str,
        nif: &str,
        timestamp: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveRequestAllOperationsSv");
        self.post_form("request_all_operations", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("timestamp", timestamp),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
        ]).await
    }

    /// Query operations history.
    pub async fn operations_history(
        &self,
        device_id: &str,
        device_password: &str,
        nif: &str,
    ) -> Result<ApiResponse<OperationsHistoryResponse>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveOperationsHistorySv");
        self.post_form("operations_history", &url, &[
            ("device_id", device_id),
            ("user_password", device_password),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
            ("aut_fir", ""),
            ("resultado", ""),
            ("organismo", ""),
            ("fecha_desde", ""),
            ("fecha_hasta", ""),
            ("tipo", ""),
            ("orden", ""),
        ]).await
    }

    /// Get Llave Móvil activation page.
    pub async fn get_llave_movil(&self) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL_WWW12}/wlpl/MOVI-P24H/ObtenerClaveMovil");
        tracing::debug!(endpoint = "get_llave_movil", "aeat request");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        tracing::debug!(endpoint = "get_llave_movil", http_status = %status, "aeat response");
        Ok(resp.json().await?)
    }

    /// Validate Llave Móvil token.
    pub async fn validate_llave_movil(
        &self,
        token: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL_WWW12}/wlpl/MOVI-P24H/ValidarClaveMovil");
        self.post_form("validate_llave_movil", &url, &[
            ("tokenClaveMovil", token),
        ]).await
    }

    /// Request SMS verification code for device activation.
    pub async fn request_sms_code(&self) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL_WWW12}/wlpl/MOVI-P24H/ObtenerClaveMovilSMS");
        self.post_form("request_sms_code", &url, &[]).await
    }

    /// Validate SMS verification code.
    pub async fn validate_sms_code(
        &self,
        timestamp: &str,
        token: &str,
        pin: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL_WWW12}/wlpl/MOVI-P24H/ValidarClaveMovilSMS");
        self.post_form("validate_sms_code", &url, &[
            ("timeStampAltaSms", timestamp),
            ("tokenClaveMovilSms", token),
            ("pinAcceso", pin),
        ]).await
    }

    /// Register a push notification token with the server.
    pub async fn set_firebase_token(
        &self,
        device_id: &str,
        nif: &str,
        token_push: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/LlaveSetFirebaseTokenSv");
        self.post_form("set_firebase_token", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
            ("token_push", token_push),
        ]).await
    }

    /// Check current request state (pending authentication requests).
    pub async fn request_state(
        &self,
        device_id: &str,
        device_password: &str,
        nif: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL_WWW12}/wlpl/MOVI-P24H/LlaveRequestStateSv");
        self.post_form("request_state", &url, &[
            ("device_id", device_id),
            ("user_password", device_password),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
        ]).await
    }

    /// Get pending market petitions.
    pub async fn get_pending_petitions(
        &self,
        device_id: &str,
        nif: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/ObtenerPeticionesMarketsSv");
        self.post_form("get_pending_petitions", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
        ]).await
    }

    /// QR code authentication.
    pub async fn qr_authenticate(
        &self,
        device_id: &str,
        nif: &str,
        qr_value: &str,
    ) -> Result<ApiResponse<serde_json::Value>> {
        let url = format!("{BASE_URL}/wlpl/MOVI-P24H/ClaveMovilQrSv");
        self.post_form("qr_authenticate", &url, &[
            ("device_id", device_id),
            ("NIF", nif),
            ("sistema_operativo", OS_NAME),
            ("version_os", OS_VERSION),
            ("version_app", APP_VERSION),
            ("valor_qr", qr_value),
        ]).await
    }
}
