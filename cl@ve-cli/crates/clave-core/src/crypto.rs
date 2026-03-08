use crate::config::data_dir;
use crate::error::{ClaveError, Result};
use base64::Engine;

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
        .map_err(|e| ClaveError::Crypto(format!("PBKDF2 failed: {e}")))?;

    // AES-128-CBC encryption with PKCS7 padding
    use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
    type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

    let encryptor = Aes128CbcEnc::new_from_slices(&key, &iv)
        .map_err(|e| ClaveError::Crypto(format!("AES init failed: {e}")))?;

    let plaintext_bytes = plaintext.as_bytes();
    // Allocate buffer with padding space
    let mut buf = vec![0u8; plaintext_bytes.len() + 16];
    buf[..plaintext_bytes.len()].copy_from_slice(plaintext_bytes);

    let ciphertext = encryptor
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plaintext_bytes.len())
        .map_err(|e| ClaveError::Crypto(format!("AES encrypt failed: {e}")))?;

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

/// Encrypt data using AES-256-GCM with a locally-managed key.
///
/// Returns a JSON blob containing the base64-encoded nonce and ciphertext.
/// The encryption key is generated on first use and stored in the data directory
/// with restrictive file permissions (`0600`), mirroring how Android's
/// `EncryptedSharedPreferences` protects data at rest via a master key stored
/// in the Android Keystore.
pub fn encrypt_local(plaintext: &[u8]) -> Result<Vec<u8>> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    use rand::RngCore;

    let key_bytes = load_or_create_local_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| ClaveError::Crypto(format!("AES-GCM key init failed: {e}")))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| ClaveError::Crypto(format!("AES-GCM encrypt failed: {e}")))?;

    let b64 = base64::engine::general_purpose::STANDARD;
    let envelope = serde_json::json!({
        "v": 1,
        "nonce": b64.encode(nonce_bytes),
        "data": b64.encode(&ciphertext),
    });
    serde_json::to_vec(&envelope).map_err(Into::into)
}

/// Decrypt data that was encrypted with [`encrypt_local`].
pub fn decrypt_local(encrypted: &[u8]) -> Result<Vec<u8>> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};

    let envelope: serde_json::Value = serde_json::from_slice(encrypted)?;
    let b64 = base64::engine::general_purpose::STANDARD;

    let nonce_b64 = envelope["nonce"]
        .as_str()
        .ok_or_else(|| ClaveError::Crypto("missing nonce in encrypted envelope".into()))?;
    let data_b64 = envelope["data"]
        .as_str()
        .ok_or_else(|| ClaveError::Crypto("missing data in encrypted envelope".into()))?;

    let nonce_bytes = b64
        .decode(nonce_b64)
        .map_err(|e| ClaveError::Crypto(format!("invalid nonce base64: {e}")))?;
    let ciphertext = b64
        .decode(data_b64)
        .map_err(|e| ClaveError::Crypto(format!("invalid data base64: {e}")))?;

    let key_bytes = load_or_create_local_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| ClaveError::Crypto(format!("AES-GCM key init failed: {e}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| ClaveError::Crypto(format!("AES-GCM decrypt failed: {e}")))
}

/// Load (or generate on first use) a 256-bit AES key stored at
/// `$XDG_DATA_HOME/clave-cli/.key`.
///
/// The file is created with mode `0600` so only the owning user can read it.
fn load_or_create_local_key() -> Result<[u8; 32]> {
    let key_path = data_dir()?.join(".key");

    if key_path.exists() {
        let raw = std::fs::read(&key_path)?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(raw.as_slice())
            .map_err(|e| ClaveError::Crypto(format!("corrupt key file: {e}")))?;
        let key: [u8; 32] = decoded
            .try_into()
            .map_err(|_| ClaveError::Crypto("key file has wrong length".into()))?;
        Ok(key)
    } else {
        use rand::RngCore;

        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);

        // Write with restrictive permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create_new(true).mode(0o600);
            use std::io::Write;
            let mut f = opts
                .open(&key_path)
                .map_err(|e| ClaveError::Crypto(format!("failed to create key file: {e}")))?;
            f.write_all(encoded.as_bytes())?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&key_path, encoded.as_bytes())?;
        }

        Ok(key)
    }
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| ClaveError::Crypto(format!("Invalid hex: {e}")))
        })
        .collect()
}
