# Llave AEAT - Reverse Engineering Report

**APK**: `es.aeat.pin24h` v6.2.5 (build 288)
**Decompiled with**: jadx 1.5.1, apktool 2.7.0
**Date**: 2026-03-08

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [API Endpoints](#api-endpoints)
3. [Authentication Flows](#authentication-flows)
4. [Request/Response Formats](#requestresponse-formats)
5. [Cryptographic Primitives](#cryptographic-primitives)
6. [Certificate Handling](#certificate-handling)
7. [TLS Configuration](#tls-configuration)
8. [Device Registration & Activation](#device-registration--activation)
9. [Credential Storage](#credential-storage)
10. [Hardcoded Values & Constants](#hardcoded-values--constants)

---

## Architecture Overview

The app follows Clean Architecture (presentation → domain → data) written in Kotlin:

```
es.aeat.pin24h/
├── data/
│   ├── webservices/     # Retrofit API client
│   │   ├── LlaveApi.java           # Retrofit interface (all API endpoints)
│   │   ├── RetrofitClient.java     # Retrofit builder + OkHttp setup
│   │   ├── SecuredHttpClient.java  # TLS configuration
│   │   ├── RedirectInterceptor.java
│   │   ├── AcceptLanguageInterceptor.java
│   │   ├── JSONInterceptor.java
│   │   └── UserAgentInterceptor.java
│   ├── repository/
│   │   └── Repository.java         # Main data source (maps use cases to API calls)
│   ├── manager/
│   │   ├── KeyChainManager.java    # Encrypted credential storage
│   │   └── CookiesManagerSingleton.java
│   └── model/
│       └── CookiesInApp.java
├── domain/
│   ├── model/
│   │   ├── request/    # All request DTOs
│   │   └── response/   # All response DTOs
│   ├── usecases/       # Business logic
│   └── interfaces/     # Repository contracts
├── presentation/       # Activities, Fragments, ViewModels
└── common/
    └── utils/
        ├── UrlUtils.java           # URL routing logic
        ├── DeviceUtils.java        # Device ID generation
        └── StringUtils.java        # Encoding helpers
```

Additionally, DNIe (electronic national ID) support comes from:
```
es.gob.fnmt.dniedroid/
└── net/http/clave/
    └── ClaveAuthentication.java  # SAML-based auth via pasarela.clave.gob.es
```

## API Endpoints

### Base URL
```
https://www2.agenciatributaria.gob.es/
```

All API calls use **Retrofit** with **form-urlencoded POST** (most endpoints) or **JSON body POST**.

### Core Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `wlpl/MOVI-P24H/LlaveStartingSv` | POST (form) | App initialization, returns URL whitelists/blacklists and menu config |
| `wlpl/MOVI-P24H/LlaveIsNifActivatedSv` | POST (form) | Check if a NIF is activated in Llave |
| `wlpl/MOVI-P24H/LlaveRequestPinSv` | POST (form) | Request a Llave PIN (returns pin + TTL) |
| `wlpl/MOVI-P24H/LlaveAuthenticateSv` | POST (form) | Authenticate (Llave Móvil confirm) |
| `wlpl/MOVI-P24H/LlaveCancelAuthenticateSv` | POST (form) | Cancel an authentication request |
| `wlpl/MOVI-P24H/LlaveActivateAuthenticationSv` | POST (form) | Activate device authentication (hosted at www6 or www1) |
| `wlpl/MOVI-P24H/LlaveDesactivateAuthSv` | POST (form) | Deactivate device authentication |
| `wlpl/MOVI-P24H/LlaveCheckMyDataSv` | POST (form) | Check user account data |
| `wlpl/MOVI-P24H/LlaveRequestAllOperationsSv` | POST (form) | Request all pending operations |
| `wlpl/MOVI-P24H/LlaveMigrationSv` | POST (form) | Migrate from old Llave PIN to new version |
| `wlpl/MOVI-P24H/LlaveSetFirebaseTokenSv` | POST (form) | Register push notification token |
| `wlpl/MOVI-P24H/ClaveMovilQrSv` | POST (form) | QR code authentication |
| `wlpl/MOVI-P24H/ObtenerPeticionesMarketsSv` | POST | Get pending market petitions |
| `wlpl/MOVI-P24H/LlaveRequestStateSv` | POST (form) | Get current request state (www12 or www1) |

### Llave Móvil Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `wlpl/MOVI-P24H/ObtenerClaveMovil` (www12) | GET | Get Llave Móvil activation page |
| `wlpl/MOVI-P24H/ValidarClaveMovil` (www12) | POST (form) | Validate Llave Móvil token |
| `wlpl/MOVI-P24H/ObtenerClaveMovilSMS` (www12) | POST | Get Llave Móvil SMS code |
| `wlpl/MOVI-P24H/ValidarClaveMovilSMS` (www12) | POST (form) | Validate SMS code |

### DNI/NIE Authentication

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `wlpl/BUCV-JDIT/AutenticaDniNieContrasteh?ref=%2Fwlpl%2FMOVI-AEAT%2FAccesoW12Sv` | POST (form) | Authenticate with DNI/NIE + date of birth |

### Certificate Registration

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `wlpl/MOVI-JDIT/C/IniciarAltaConCertificadoSv` | POST (JSON) | Initiate certificate-based registration |
| `wlpl/MOVI-JDIT/C/ValidarAltaConCertificadoSv` (www12) | POST (JSON) | Validate certificate registration |

### Operations History

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `wlpl/MOVI-P24H/LlaveOperationsHistorySv` | POST (form) | Query operations history with filters |

### Supporting URLs (browser-based)

| URL | Purpose |
|-----|---------|
| `wlpl/MOVI-JDIT/Acceso?gestion=REGISTRO_CERTIFICADO&nif=` | Register via certificate |
| `wlpl/MOVI-JDIT/Acceso?gestion=REGISTRO_CARTA` | Register via letter |
| `wlpl/MOVI-JDIT/Acceso?gestion=MODIFICA_TELEFONO&nif=` | Change phone number |
| `wlpl/MOVI-JDIT/Acceso?gestion=OBTENER_SUPERIOR&nif=` | Get advanced registration |
| `wlpl/MOVI-JDIT/Acceso?info=AYUDA_CERT_ANDROID` | Help with Android certificates |
| `wlpl/MOVI-JDIT/Acceso?ext=OFICINAS_CLAVE` | Llave offices |
| `wlpl/MOVI-JDIT/Acceso?ext=REGISTRO_PRESENCIAL` | In-person registration |
| `wlpl/MOVI-JDIT/SoporteAppSv?APP=CLAVE` | App support |

### Host Variants

The app uses multiple server hosts:
- **www2.agenciatributaria.gob.es** — Primary API base URL
- **www12.agenciatributaria.gob.es** — Llave Móvil, state checks, SMS validation
- **www1.agenciatributaria.gob.es** — Browser-based operations, alternate for www12
- **www6.agenciatributaria.gob.es** — Device activation (alternate for www1)
- **sede.agenciatributaria.gob.es** — AEAT sede (website)
- **pasarela.clave.gob.es** — SAML identity gateway (DNIe/certificate auth)

## Authentication Flows

### Flow 1: Llave PIN Request (Main Flow)

```
1. LlaveStartingSv  →  Initialize session (device_id, NIF, OS info)
                       Response: { status: "OK", respuesta: { listaBlancaUrls, menu } }

2. LlaveIsNifActivatedSv  →  Check if NIF is registered
                              Response: device activation status

3. LlaveRequestPinSv  →  Request a temporary PIN
                          Fields: device_id, user_password, NIF, OS info
                          Response: { status: "OK", respuesta: { pin, timeToLive } }
```

The PIN is a server-generated one-time code with a TTL (time-to-live).

### Flow 2: Llave Móvil Authentication

```
1. LlaveStartingSv  →  Initialize
2. (Push notification received with authentication request)
3. LlaveAuthenticateSv  →  Confirm authentication
   Fields: device_id, user_password, NIF, tokenClaveMovil, codigoIdP
   Response: { status: "OK", respuesta: { pending_requests } }
```

Or to cancel:
```
3. LlaveCancelAuthenticateSv  →  Reject authentication
```

### Flow 3: DNI/NIE + Date of Birth (Weak Authentication)

```
1. AutenticaDniNieContrasteh  →  POST form with:
   - NIF: document number
   - FECHA: date of validity (DD-MM-YYYY format)
   - SOPORTE: support number from the ID card
   - botonAutenticacionDebil: "Acceder" (submit button)
   - APP: "CLAVE"
   - modo: authentication mode
   - AZUL: additional field for NIE
   - FECHANIE: NIE-specific date
   - TrazasApp header: trace/logging info

   Response: Raw HTML body (not JSON) → parsed for redirect
```

### Flow 4: Device Activation via SMS

```
1. ObtenerClaveMovilSMS  →  Request SMS verification code
2. ValidarClaveMovilSMS  →  Validate SMS code
   Fields: timeStampAltaSms, tokenClaveMovilSms, pinAcceso
```

### Flow 5: Device Activation via Token

```
1. ObtenerClaveMovil  →  Get activation page
2. ValidarClaveMovil  →  Validate token
   Fields: tokenClaveMovil
```

### Flow 6: Certificate-Based Registration

```
1. IniciarAltaConCertificadoSv  →  POST JSON body
2. ValidarAltaConCertificadoSv  →  POST JSON body
```

### Flow 7: SAML Authentication via pasarela.clave.gob.es (DNIe)

The `ClaveAuthentication` class in the DNIe library handles SAML-based auth:

```
1. Start with HTML from pasarela.clave.gob.es/Proxy2/ServiceProvider
2. Parse HTML forms iteratively:
   Step 1: Find form matching initial attributes → POST
   Step 2: Find form "idpRedirect" → POST (sets value "AFIRMA")
   Step 3: Find form "redirectForm" → POST
   Step 4: Find form "redirectForm" → POST
   Step 5: Find form "redirectForm" → POST

Alternative (non-Proxy2 path):
   Step 1: Find initial form → POST
   Step 2: Find form "nationalRedirect" → POST
   Step 3: Find form id="AuthenticateCitizen" → POST
   Step 4: Find form id="prepareResponseAfirma" → POST
   Step 5: Find form "redirectForm" → extract redirectTo value
```

Each step:
- Parses the returned HTML
- Finds the target `<form>` element
- Extracts all `<input>` fields as key-value pairs
- Extracts the action URL
- POSTs the form data
- Uses the response HTML for the next step

## Request/Response Formats

### Common Request Fields (form-urlencoded)

| Field | Description |
|-------|-------------|
| `device_id` | Unique device identifier (generated/stored locally) |
| `user_password` | Device-specific password (generated during activation) |
| `NIF` | Spanish tax identification number (DNI/NIE) |
| `sistema_operativo` | OS type (e.g., "Android") |
| `version_os` | OS version string |
| `version_app` | App version string |
| `modelo` | Device model name |
| `token_push` | Firebase Cloud Messaging token |
| `TrazasApp` | Header: trace/logging identifier |

### Standard Response Envelope

All JSON responses follow this pattern:

```json
{
  "status": "OK" | "KO",
  "visible": "S" | "N",
  "crashlytics": "S" | "N",
  "codigo_error": "ERROR_CODE",
  "mensaje": "Human-readable error message",
  "respuesta": { ... }
}
```

- `status`: "OK" for success, "KO" for failure
- `visible`: Whether to show error to user ("S"=yes, "N"=no)
- `crashlytics`: Whether to report to crashlytics
- `codigo_error`: Machine-readable error code
- `mensaje`: Human-readable message
- `respuesta`: The actual response payload (type varies per endpoint)

### Specific Response Payloads

**LlaveRequestPin → respuesta:**
```json
{
  "pin": "123456",
  "timeToLive": "180"
}
```

**LlaveAuthenticate → respuesta:**
```json
{
  "pending_requests": "2"
}
```

**LlaveStarting → respuesta:**
```json
{
  "listaBlancaUrls": ["url1", "url2"],
  "listaNegraUrls": ["url3"],
  "menu": { ... }
}
```

**LlaveCheckMyData → respuesta:**
```json
{
  "nombre": "...",
  "nif": "...",
  "telefono": "...",
  "nivelAcceso": "...",
  "fechaCaducidad": "..."
}
```

## Cryptographic Primitives

### 1. Browser Data Encryption (AES-CBC)

Found in `KeyChainManager.encryptTextForBrowser()`:

```
Algorithm: AES/CBC/PKCS5PADDING
Key derivation: PBKDF2WithHmacSHA1
  - Passphrase: serverPassphrase (received from server)
  - Salt: serverSalt (hex string, received from server)
  - Iterations: 1000
  - Key length: 128 bits
IV: serverIv (hex string, received from server)
Output: Base64 → URI-encoded
```

The encrypted payload format:
```
$id_dispositivo:<device_id>
$password_dispositivo:<device_password>
$url_destino:<target_url>
$idioma:<lang>_ES
[$NIF:<nif>]
[$datoContraste:<contrast_data>$tipoAutenticacion:<auth_type>]
[$codigo:<code>$pin:<pin>]
```

### 2. Local Credential Encryption (AES-GCM)

Found in `KeyChainManager.getValueLlavePin()`:

```
Algorithm: AES/GCM/NoPadding
Key: AndroidKeyStore "LlavePinAlias" (AES, 256-bit)
IV: Randomly generated, stored in preferences (Base64-encoded)
GCM Tag Length: 128 bits
```

### 3. Encrypted Shared Preferences

Uses Android's `EncryptedSharedPreferences` for storing:
- `nif_usuario` — User's NIF
- `id_dispositivo` — Device ID
- `user_password` — Device password
- `id_firebase` — Firebase token
- `token` — Session token
- `map_cookies` — Serialized cookie jar

### 4. SAML Form-Based Flow

The DNIe path through `pasarela.clave.gob.es` uses standard SAML redirects:
- No custom crypto beyond TLS mutual auth with the DNIe certificate
- The eID card provides the `SSLSocketFactory` for client certificate authentication
- Forms contain `SAMLResponse` fields for assertion relay

## Certificate Handling

### Client Certificates (DNIe via NFC)

The app supports reading the Spanish DNIe (electronic national ID) via NFC:

```java
// CertificadoNfcData provides SSLSocketFactory for mutual TLS
certificadoNfcData.getDnieSSLSocketFactory()
```

This is used in `RetrofitClient.createHttpClientWithCertificate()` to create an OkHttp client that presents the eID certificate during TLS handshake.

### Software Certificates

The app can also use software certificates stored in Android KeyChain:
- Stored in `EncryptedSharedPreferences` under `"user_certificates"`
- Can be imported from `.p12`/`.pfx` files
- Used for certificate-based registration flows

### Certificate Registration Endpoints

```
IniciarAltaConCertificadoSv → Start registration with certificate
ValidarAltaConCertificadoSv → Complete registration with certificate
```

These use JSON request bodies (not form-urlencoded).

## TLS Configuration

Found in `SecuredHttpClient`:

```java
// Supported TLS versions
TLS 1.2, TLS 1.3

// Cipher suites
TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
TLS_AES_128_GCM_SHA256
TLS_AES_256_GCM_SHA384
TLS_CHACHA20_POLY1305_SHA256

// Timeouts
Connect: 190 seconds
Read: 190 seconds
Write: 190 seconds

// Certificate pinning: NONE
// The app does NOT implement certificate pinning!
// In debug mode (IGNORE_SECURE_HTTP=true), it trusts all certificates
```

### Cookie Management

Cookies are managed via `JavaNetCookieJar` through `CookiesManagerSingleton`. Before each API call, the Repository calls `setCookiesInJar()` to ensure proper cookie state.

## Device Registration & Activation

### Device Activation Flow

```
1. App generates a unique device_id
2. User enters NIF
3. LlaveStartingSv → registers device, gets session
4. LlaveActivateAuthenticationSv → activates push notifications
   Fields: sistema_operativo, version_os, version_app, token_push,
           user_password, modelo
5. Server generates user_password and returns activation confirmation
6. Device stores: device_id, user_password, nif in EncryptedSharedPreferences
```

### QR Code Authentication

```
ClaveMovilQrSv → POST with valor_qr (scanned QR value)
```

## Credential Storage

### Android Keystore

- Key alias: `"LlavePinAlias"`
- Algorithm: AES
- Block mode: GCM
- Padding: NoPadding
- `setRandomizedEncryptionRequired(false)` — allows reuse of IV

### SharedPreferences Files

| Preference File | Purpose |
|-----------------|---------|
| `LlavePin` | Legacy encrypted credential storage |
| `ClaveAuthentication` | Authentication passwords (migrated) |
| `LlaveCertificados` | Software certificate storage |
| EncryptedSharedPreferences (default) | Primary secure storage |

### Stored Credentials

| Key | Description |
|-----|-------------|
| `nif_usuario` | User's NIF/NIE |
| `id_dispositivo` | Unique device ID |
| `user_password` | Device-specific password (for API auth) |
| `id_firebase` | FCM push token |
| `token` | Session token |
| `password_dispositivo` | Device password (legacy) |
| `dato_contraste_usuario` | Contrast data for verification |
| `map_cookies` | Serialized cookies (JSON HashMap) |
| `user_certificates` | Imported software certificates |

## Hardcoded Values & Constants

### URLs

```
Base URL:                    https://www2.agenciatributaria.gob.es/
Llave Gateway:               https://pasarela.clave.gob.es/
Llave Gateway SP:            https://pasarela.clave.gob.es/Proxy2/ServiceProvider
AEAT Sede:                   https://sede.agenciatributaria.gob.es/
```

### HTTP Headers

```
TrazasApp: <trace_id>        (custom header for server-side tracing)
Referer: https://pasarela.clave.gob.es/  (for SAML flows)
Accept-Language: <lang>_ES or <lang>_GB
Content-Type: application/json (for JSON endpoints)
User-Agent: <custom_user_agent>  (from DeviceUtils)
```

### App Constants

```
APP field value: "CLAVE"
botonAutenticacionDebil: "Acceder"
SAML IdP hint: "AFIRMA"
```

### Build Config Flags

```
IGNORE_SECURE_HTTP: Boolean (debug vs release TLS behavior)
HTTP_LOG_LEVEL: HttpLoggingInterceptor.Level
```

---

## Summary of Key Findings

1. **No TOTP/HOTP**: The app does NOT generate OTPs locally. PINs are server-generated via `LlaveRequestPinSv` and returned with a TTL.

2. **No Certificate Pinning**: The app relies on system trust store only. In debug mode, it even trusts all certificates.

3. **Device-bound auth**: Authentication is tied to `device_id` + `user_password` (generated during activation). The `user_password` acts as a device secret.

4. **Server-side encryption params**: The AES encryption key material (`serverPassphrase`, `serverIv`, `serverSalt`) comes from the server, making it a server-controlled scheme.

5. **SAML flow for DNIe**: Certificate-based auth goes through `pasarela.clave.gob.es` using standard SAML form redirects, not a REST API.

6. **Multiple auth levels**:
   - **Weak**: DNI/NIE + date of birth (AutenticaDniNieContrasteh)
   - **Medium**: Llave PIN (server-generated, device-bound)
   - **Strong**: Llave Móvil (push notification + confirm)
   - **Strongest**: DNIe certificate (mutual TLS via SAML)

7. **Cookies are critical**: The app carefully manages cookies across requests using `setCookiesInJar()` before each API call, suggesting server-side session tracking.
