# PIN-Based Session Encryption Plan

## Goal
Protect the stored session (including `device_password`) with a user-chosen PIN/password. The app requires this PIN on every launch to decrypt the session — without it, the stored data is useless.

## Architecture Decision

**Encrypt in Rust, not Flutter.** The Rust core already has the crypto crates (`aes`, `pbkdf2`, `hmac`) and this keeps the sensitive `device_password` encrypted even in memory until the PIN is provided. Flutter only ever sees an opaque encrypted blob.

## Changes

### 1. Rust: Add `seal_session` / `open_session` to `llave-core/src/crypto.rs`

- **KDF**: Argon2id (add `argon2` crate) — much stronger than the existing PBKDF2-SHA1 used for browser handoff. Fallback: PBKDF2-HMAC-SHA256 with 600k iterations if we want to avoid a new dependency.
- **Encryption**: AES-256-GCM (add `aes-gcm` crate) — authenticated encryption, prevents tampering.
- **Format**: `version(1B) || salt(16B) || nonce(12B) || ciphertext || tag(16B)` → base64-encoded string.
- `seal_session(session_json: &str, pin: &str) -> Result<String>` — returns encrypted blob.
- `open_session(encrypted: &str, pin: &str) -> Result<String>` — returns decrypted JSON, or error on wrong PIN.

### 2. Rust: New Cargo dependencies in `llave-core/Cargo.toml`

```toml
argon2 = "0.5"
aes-gcm = "0.10"
rand = "0.8"  # (if not already present, for salt/nonce generation)
```

### 3. Rust FFI: Expose via `llave-core-ffi/src/api.rs`

- `encrypt_session(pin: String) -> Result<String>` — loads current session from MemoryStorage, encrypts with PIN, returns blob.
- `decrypt_and_load_session(encrypted: String, pin: String) -> Result<bool>` — decrypts blob, hydrates MemoryStorage. Returns error on wrong PIN.
- `change_pin(old_pin: String, new_pin: String) -> Result<String>` — re-encrypts session with new PIN.

### 4. Flutter Bridge: Add methods to `llave_bridge.dart`

```dart
Future<String> encryptSession(String pin);
Future<bool> decryptAndLoadSession(String encrypted, String pin);
Future<String> changePin(String oldPin, String newPin);
```

### 5. Flutter: Modify `SecureSessionStore`

- `write()` now stores the **encrypted blob** (not raw JSON).
- `read()` returns the **encrypted blob** (not raw JSON).
- No decryption happens here — it's just opaque storage.

### 6. Flutter: Modify `AuthNotifier` / startup flow

- **`init()`**: Read encrypted blob from store. If blob exists → set state to a new `AuthLocked` state (session exists but needs PIN). If no blob → `AuthUnauthenticated`.
- **`unlock(pin)`**: New method. Calls `decryptAndLoadSession(blob, pin)`. On success → `AuthAuthenticated`. On failure → `AuthError("Wrong PIN")`.
- **After activation**: Prompt user to set a PIN. Call `encryptSession(pin)` and store the blob.

### 7. Flutter: New `AuthLocked` state

```dart
class AuthLocked extends AuthState {
  const AuthLocked();
}
```

This state means "we have an encrypted session on disk but need the PIN to unlock it."

### 8. Flutter: New `UnlockScreen`

- Simple PIN/password entry screen.
- Shown when `AuthState` is `AuthLocked`.
- On submit → calls `AuthNotifier.unlock(pin)`.
- Wrong PIN → shake animation + error message.
- Option to "Forgot PIN" → wipes session and goes to activation.
- Optional: biometric unlock (store the PIN in biometric-protected Keychain entry using `local_auth` + `flutter_secure_storage` with biometric access control).

### 9. Flutter: PIN setup during activation

- After successful activation (`activate()` or `dniCompleteActivation()`), show a "Set your PIN" screen.
- PIN confirmation (enter twice).
- Minimum 4 digits, or allow alphanumeric password.
- Call `encryptSession(pin)` → `SecureSessionStore.write(blob)`.

### 10. Flutter: Routing changes in `main.dart`

- Add `/unlock` route → `UnlockScreen`.
- Add `/set-pin` route → `SetPinScreen`.
- Redirect logic: `AuthLocked` → `/unlock`.
- After activation → navigate to `/set-pin` before going to home.

### 11. Flutter: Settings — Change PIN

- Add "Change PIN" option in `SettingsScreen`.
- Requires entering old PIN, then new PIN twice.
- Calls `changePin(oldPin, newPin)` → re-encrypts and re-stores.

## File Changes Summary

| File | Action |
|------|--------|
| `crates/llave-core/Cargo.toml` | Add `argon2`, `aes-gcm`, `rand` |
| `crates/llave-core/src/crypto.rs` | Add `seal_session()`, `open_session()` |
| `crates/llave-core-ffi/src/api.rs` | Add `encrypt_session()`, `decrypt_and_load_session()`, `change_pin()` |
| `flutter/lib/src/llave_bridge.dart` | Add bridge methods + mock impl |
| `flutter/lib/src/auth_provider.dart` | Add `AuthLocked` state, `unlock()`, modify `init()` |
| `flutter/lib/src/secure_session_store.dart` | No changes needed (already stores opaque strings) |
| `flutter/lib/screens/unlock_screen.dart` | **New** — PIN entry to unlock |
| `flutter/lib/screens/set_pin_screen.dart` | **New** — PIN setup after activation |
| `flutter/lib/screens/settings_screen.dart` | Add "Change PIN" option |
| `flutter/lib/main.dart` | Add routes, update redirect logic |
| `flutter/pubspec.yaml` | Optionally add `local_auth` for biometrics |

## Security Properties

- **At rest**: Session encrypted with AES-256-GCM, key derived from user PIN via Argon2id.
- **In memory**: Decrypted only after correct PIN, held in Rust MemoryStorage.
- **Wrong PIN**: Authenticated encryption fails → no partial data leaked.
- **No PIN stored**: The PIN itself is never stored anywhere — it's only used to derive the key.
- **Brute force**: Argon2id with tuned parameters makes offline brute-force expensive.
