import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:logging/logging.dart';

final _log = Logger('SecureSessionStore');

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

  /// Read the saved session JSON, or `null` if none exists.
  static Future<String?> read() async {
    try {
      final value = await _storage.read(key: _key);
      _log.fine('read: ${value != null ? '${value.length} bytes' : 'null'}');
      return value;
    } catch (e, st) {
      _log.severe('read failed', e, st);
      return null;
    }
  }

  /// Write session JSON to secure storage.
  ///
  /// Throws on failure so callers know the write did not succeed.
  static Future<void> write(String value) async {
    _log.fine('write: ${value.length} bytes');
    await _storage.write(key: _key, value: value);
    _log.info('write succeeded');
  }

  /// Delete the saved session.
  static Future<void> delete() async {
    try {
      await _storage.delete(key: _key);
      _log.info('delete succeeded');
    } catch (e, st) {
      _log.severe('delete failed', e, st);
    }
  }
}
