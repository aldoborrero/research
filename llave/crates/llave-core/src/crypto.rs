use crate::error::{LlaveError, Result};
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
