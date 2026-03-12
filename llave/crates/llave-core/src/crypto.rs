use crate::error::{LlaveError, Result};
use base64::Engine;
use sha2::{Sha512, Digest};

// ---------------------------------------------------------------------------
// PIN-based session encryption (Argon2id + AES-256-GCM)
// ---------------------------------------------------------------------------

/// Current envelope version byte.
const SEALED_VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12; // AES-GCM standard

/// Encrypt session JSON with a user-chosen PIN.
///
/// Format: `base64( version(1) || salt(16) || nonce(12) || ciphertext+tag )`
///
/// The key is derived via Argon2id so offline brute-force is expensive.
pub fn seal_session(session_json: &str, pin: &str) -> Result<String> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    use rand::RngCore;

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key = derive_key(pin, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| LlaveError::Crypto(format!("AES-GCM init: {e}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, session_json.as_bytes())
        .map_err(|e| LlaveError::Crypto(format!("AES-GCM encrypt: {e}")))?;

    // Assemble envelope
    let mut envelope = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
    envelope.push(SEALED_VERSION);
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);

    Ok(base64::engine::general_purpose::STANDARD.encode(&envelope))
}

/// Decrypt a sealed session blob using the user's PIN.
///
/// Returns the original session JSON, or an error if the PIN is wrong
/// (AES-GCM authentication will fail).
pub fn open_session(sealed_b64: &str, pin: &str) -> Result<String> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};

    let envelope = base64::engine::general_purpose::STANDARD
        .decode(sealed_b64)
        .map_err(|e| LlaveError::Crypto(format!("base64 decode: {e}")))?;

    let min_len = 1 + SALT_LEN + NONCE_LEN + 16; // 16 = GCM tag
    if envelope.len() < min_len {
        return Err(LlaveError::Crypto("sealed data too short".into()));
    }

    let version = envelope[0];
    if version != SEALED_VERSION {
        return Err(LlaveError::Crypto(format!(
            "unsupported sealed version: {version}"
        )));
    }

    let salt = &envelope[1..1 + SALT_LEN];
    let nonce_bytes = &envelope[1 + SALT_LEN..1 + SALT_LEN + NONCE_LEN];
    let ciphertext = &envelope[1 + SALT_LEN + NONCE_LEN..];

    let key = derive_key(pin, salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| LlaveError::Crypto(format!("AES-GCM init: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| LlaveError::Crypto("wrong PIN or corrupted data".into()))?;

    String::from_utf8(plaintext)
        .map_err(|e| LlaveError::Crypto(format!("invalid UTF-8 after decrypt: {e}")))
}

/// Derive a 256-bit key from a PIN and salt using Argon2id.
fn derive_key(pin: &str, salt: &[u8]) -> Result<[u8; 32]> {
    use argon2::Argon2;

    let mut key = [0u8; 32];
    // Default Argon2id params: 19 MiB memory, 2 iterations, 1 lane.
    // Good balance between mobile performance and brute-force resistance.
    Argon2::default()
        .hash_password_into(pin.as_bytes(), salt, &mut key)
        .map_err(|e| LlaveError::Crypto(format!("Argon2id: {e}")))?;
    Ok(key)
}

/// Encrypt data for browser handoff using server-provided AES-CBC parameters.
///
/// Mirrors the Android app's `KeyChainManager.encryptTextForBrowser()`:
/// - Key derivation: PBKDF2-HMAC-SHA1, 1000 iterations, 128-bit key
/// - Encryption: AES-128-CBC with PKCS5 padding
/// - Output: Base64 → URL-encoded
pub fn encrypt_for_browser(
    passphrase: &str,
    iv_hex: &str,
    salt_hex: &str,
    plaintext: &str,
) -> Result<String> {
    use hmac::Hmac;
    use sha1::Sha1;

    // Decode hex salt and IV
    let salt = hex_to_bytes(salt_hex)?;
    let iv = hex_to_bytes(iv_hex)?;

    // PBKDF2 key derivation
    let mut key = [0u8; 16]; // 128 bits
    pbkdf2::pbkdf2::<Hmac<Sha1>>(passphrase.as_bytes(), &salt, 1000, &mut key)
        .map_err(|e| LlaveError::Crypto(format!("PBKDF2 failed: {e}")))?;

    // AES-128-CBC encryption with PKCS7 padding
    use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
    type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

    let encryptor = Aes128CbcEnc::new_from_slices(&key, &iv)
        .map_err(|e| LlaveError::Crypto(format!("AES init failed: {e}")))?;

    let plaintext_bytes = plaintext.as_bytes();
    // Allocate buffer with padding space
    let mut buf = vec![0u8; plaintext_bytes.len() + 16];
    buf[..plaintext_bytes.len()].copy_from_slice(plaintext_bytes);

    let ciphertext = encryptor
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plaintext_bytes.len())
        .map_err(|e| LlaveError::Crypto(format!("AES encrypt failed: {e}")))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(ciphertext);
    Ok(urlencoding::encode(&b64).into_owned())
}

/// Build the plaintext payload for browser encryption
pub fn build_browser_payload(
    device_id: &str,
    device_password: &str,
    target_url: &str,
    language: &str,
    nif: Option<&str>,
    contraste: Option<(&str, &str)>, // (dato_contraste, tipo_autenticacion)
    pin_data: Option<(&str, &str)>,  // (codigo, pin)
) -> String {
    let lang_suffix = if language == "en" {
        format!("{language}_GB")
    } else {
        format!("{language}_ES")
    };

    let mut payload = format!(
        "$id_dispositivo:{device_id}$password_dispositivo:{device_password}$url_destino:{target_url}$idioma:{lang_suffix}"
    );

    if let Some(nif) = nif {
        if !nif.is_empty() {
            payload.push_str(&format!("$NIF:{nif}"));
        }
    }

    if let Some((dato, tipo)) = contraste {
        if !dato.is_empty() {
            payload.push_str(&format!("$datoContraste:{dato}$tipoAutenticacion:{tipo}"));
        }
    }

    if let Some((codigo, pin)) = pin_data {
        if !codigo.is_empty() {
            payload.push_str(&format!("$codigo:{codigo}$pin:{pin}"));
        }
    }

    payload
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| LlaveError::Crypto(format!("Invalid hex: {e}")))
        })
        .collect()
}

/// Generate a device password matching the Android app's format.
///
/// The Android app generates 256 random alphanumeric characters, then
/// SHA-512 hashes them, producing a 128-char uppercase hex string.
pub fn generate_device_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let random_string: String = (0..256)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect();
    let hash = Sha512::digest(random_string.as_bytes());
    hash.iter()
        .map(|b| format!("{:02X}", b))
        .collect()
}
