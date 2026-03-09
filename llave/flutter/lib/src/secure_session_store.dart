import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Persists session JSON in the platform's native secure storage.
///
/// - **Android**: AES-256 via Android Keystore (`EncryptedSharedPreferences`)
/// - **iOS**: Keychain Services
/// - **Linux**: No-op — the Rust core uses its own [`KeyringStorage`] which
///   persists directly to the OS secret store (GNOME Keyring / KDE Wallet).
///
/// The Rust core keeps the session in-memory on mobile ([`MemoryStorage`]);
/// this class is the durable backing store that Flutter owns on those
/// platforms only.
class SecureSessionStore {
  static const _key = 'llave_session';
  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );

  /// Whether Flutter should handle session persistence.
  ///
  /// On Linux desktop, Rust's KeyringStorage handles persistence directly,
  /// so Flutter storage operations are skipped.
  static bool get _rustOwnsPersistence => Platform.isLinux;

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
