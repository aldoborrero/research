import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Persists session JSON in the platform's native secure storage.
///
/// - **Android**: AES-256 via Android Keystore (`EncryptedSharedPreferences`)
/// - **iOS**: Keychain Services
/// - **Linux**: libsecret (GNOME Keyring / KDE Wallet)
///
/// The FFI layer always uses in-memory storage ([`MemoryStorage`]); this
/// class is the durable backing store that Flutter owns on all platforms.
class SecureSessionStore {
  static const _key = 'llave_session';
  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );

  /// Whether Flutter should handle session persistence.
  ///
  /// The FFI layer always uses MemoryStorage, so Flutter must persist on
  /// every platform. (Only the standalone CLI binary uses KeyringStorage.)
  static bool get _rustOwnsPersistence => false;

  /// Read the saved session JSON, or `null` if none exists.
  static Future<String?> read() async {
    if (_rustOwnsPersistence) return null;
    try {
      return await _storage.read(key: _key);
    } catch (_) {
      return null;
    }
  }

  /// Write session JSON to secure storage.
  static Future<void> write(String json) async {
    if (_rustOwnsPersistence) return;
    try {
      await _storage.write(key: _key, value: json);
    } catch (_) {}
  }

  /// Delete the saved session.
  static Future<void> delete() async {
    if (_rustOwnsPersistence) return;
    try {
      await _storage.delete(key: _key);
    } catch (_) {}
  }
}
