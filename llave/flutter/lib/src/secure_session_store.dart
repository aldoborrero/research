import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Persists session JSON in the platform's native secure storage.
///
/// - **Android**: AES-256 via Android Keystore (`EncryptedSharedPreferences`)
/// - **iOS**: Keychain Services
/// - **Linux**: libsecret (GNOME Keyring) or file-based fallback
///
/// The Rust core keeps the session in-memory ([`MemoryStorage`]); this class
/// is the durable backing store that Flutter owns.
///
/// All operations are wrapped in try/catch so that keyring failures (e.g.
/// missing GNOME Keyring on Linux) don't crash the app — the session simply
/// won't persist across restarts.
class SecureSessionStore {
  static const _key = 'llave_session';
  static final _storage = FlutterSecureStorage(
    aOptions: const AndroidOptions(encryptedSharedPreferences: true),
    lOptions: _linuxOptions(),
  );

  static LinuxOptions _linuxOptions() {
    // Use the default label so the entry is identifiable in the keyring.
    return const LinuxOptions();
  }

  /// Read the saved session JSON, or `null` if none exists.
  static Future<String?> read() async {
    try {
      return await _storage.read(key: _key);
    } catch (e) {
      // Keyring unavailable — fall back to file storage.
      return _fileFallback.read();
    }
  }

  /// Write session JSON to secure storage.
  static Future<void> write(String json) async {
    try {
      await _storage.write(key: _key, value: json);
    } catch (e) {
      // Keyring unavailable — fall back to file storage.
      await _fileFallback.write(json);
    }
  }

  /// Delete the saved session.
  static Future<void> delete() async {
    try {
      await _storage.delete(key: _key);
    } catch (_) {}
    // Also clean up any file fallback.
    await _fileFallback.delete();
  }

  static final _fileFallback = _FileSessionStore();
}

/// Simple file-based fallback for environments without a system keyring.
///
/// Stores the session JSON in `$XDG_DATA_HOME/llave/session.json` (or
/// `~/.local/share/llave/session.json`).  Not encrypted — but better than
/// crashing.
class _FileSessionStore {
  File get _file {
    final dataHome = Platform.environment['XDG_DATA_HOME'] ??
        '${Platform.environment['HOME']}/.local/share';
    return File('$dataHome/llave/session.json');
  }

  Future<String?> read() async {
    try {
      final f = _file;
      if (await f.exists()) return await f.readAsString();
    } catch (_) {}
    return null;
  }

  Future<void> write(String json) async {
    try {
      final f = _file;
      await f.parent.create(recursive: true);
      await f.writeAsString(json);
    } catch (_) {}
  }

  Future<void> delete() async {
    try {
      final f = _file;
      if (await f.exists()) await f.delete();
    } catch (_) {}
  }
}
