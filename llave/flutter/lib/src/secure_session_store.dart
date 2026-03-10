import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:logging/logging.dart';

final _log = Logger('SecureSessionStore');

/// Persists the PIN-encrypted session blob durably.
///
/// - **Android**: `EncryptedSharedPreferences` via Android Keystore
/// - **iOS**: Keychain Services
/// - **Linux**: libsecret (GNOME Keyring / KDE Wallet), with automatic
///   fallback to `~/.local/share/llave/session.enc` (mode 0600) when the
///   keyring daemon is unavailable.
///
/// The blob is already encrypted with Argon2id + AES-256-GCM before it reaches
/// this layer, so the file fallback does not weaken security — the PIN is the
/// protection, not the storage backend.
class SecureSessionStore {
  static const _key = 'llave_session';

  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );

  /// Whether we've detected that libsecret is broken and switched to file.
  static bool _linuxFallback = false;

  /// Read the saved session, or `null` if none exists.
  static Future<String?> read() async {
    // Try libsecret first on Linux, fall back to file if it fails.
    if (Platform.isLinux && _linuxFallback) {
      return _fileRead();
    }

    try {
      final value = await _storage.read(key: _key);
      _log.fine('read (keychain): ${value != null ? '${value.length} bytes' : 'null'}');
      return value;
    } catch (e, st) {
      if (Platform.isLinux) {
        _log.warning('libsecret read failed, falling back to file: $e');
        _linuxFallback = true;
        return _fileRead();
      }
      _log.severe('read failed', e, st);
      return null;
    }
  }

  /// Write the PIN-encrypted blob. Throws on failure.
  static Future<void> write(String value) async {
    _log.fine('write: ${value.length} bytes');

    if (Platform.isLinux && _linuxFallback) {
      await _fileWrite(value);
      _log.info('write succeeded (file fallback)');
      return;
    }

    try {
      await _storage.write(key: _key, value: value);
      _log.info('write succeeded (keychain)');
    } catch (e) {
      if (Platform.isLinux) {
        _log.warning('libsecret write failed, falling back to file: $e');
        _linuxFallback = true;
        await _fileWrite(value);
        _log.info('write succeeded (file fallback)');
        return;
      }
      rethrow;
    }
  }

  /// Delete the saved session.
  static Future<void> delete() async {
    // Delete from both backends on Linux to avoid stale data.
    if (Platform.isLinux) {
      await _fileDelete();
      try {
        await _storage.delete(key: _key);
      } catch (_) {
        // libsecret may be unavailable — that's fine.
      }
      _log.info('delete succeeded');
      return;
    }

    try {
      await _storage.delete(key: _key);
      _log.info('delete succeeded');
    } catch (e, st) {
      _log.severe('delete failed', e, st);
    }
  }

  // -------------------------------------------------------------------------
  // Linux file fallback (~/.local/share/llave/session.enc)
  // -------------------------------------------------------------------------

  static File get _file {
    final home = Platform.environment['HOME'] ?? '/tmp';
    return File('$home/.local/share/llave/session.enc');
  }

  static Future<String?> _fileRead() async {
    final file = _file;
    if (!await file.exists()) {
      _log.fine('file read: does not exist');
      return null;
    }
    final value = await file.readAsString();
    _log.fine('file read: ${value.length} bytes from ${file.path}');
    return value;
  }

  static Future<void> _fileWrite(String value) async {
    final file = _file;
    final dir = file.parent;
    await dir.create(recursive: true);
    // Set directory to 0700 so only owner can list/traverse.
    await Process.run('chmod', ['700', dir.path]);

    // Write to a temp file first, set 0600, then rename.
    // This avoids a window where the file exists with default permissions.
    final tmp = File('${file.path}.tmp');
    await tmp.writeAsString(value, flush: true);
    final chmodResult = await Process.run('chmod', ['600', tmp.path]);
    if (chmodResult.exitCode != 0) {
      await tmp.delete();
      throw StateError('chmod 600 failed: ${chmodResult.stderr}');
    }
    await tmp.rename(file.path);
    _log.fine('file write: ${value.length} bytes → ${file.path}');
  }

  static Future<void> _fileDelete() async {
    final file = _file;
    if (await file.exists()) {
      await file.delete();
      _log.fine('file delete: ${file.path}');
    }
  }
}
