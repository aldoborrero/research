import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Persists session JSON in the platform's native secure storage.
///
/// - **Android**: AES-256 via Android Keystore (`EncryptedSharedPreferences`)
/// - **iOS**: Keychain Services
///
/// The Rust core keeps the session in-memory ([`MemoryStorage`]); this class
/// is the durable backing store that Flutter owns.
class SecureSessionStore {
  static const _key = 'llave_session';
  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );

  /// Read the saved session JSON, or `null` if none exists.
  static Future<String?> read() => _storage.read(key: _key);

  /// Write session JSON to secure storage.
  static Future<void> write(String json) =>
      _storage.write(key: _key, value: json);

  /// Delete the saved session.
  static Future<void> delete() => _storage.delete(key: _key);
}
