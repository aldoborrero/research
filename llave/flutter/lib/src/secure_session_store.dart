import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:logging/logging.dart';

final _log = Logger('SecureSessionStore');

/// Persists the PIN-encrypted session blob durably.
///
/// - **Android**: `EncryptedSharedPreferences` via Android Keystore
/// - **iOS**: Keychain Services
/// - **Linux**: file-based (`~/.local/share/llave/session.enc`, mode 0600)
///   because `libsecret` requires a running keyring daemon which is often
///   unavailable in headless / container / Wayland-only setups.
///
/// The blob is already encrypted with Argon2id + AES-256-GCM before it reaches
/// this layer, so the file-based Linux fallback does not weaken security — the
/// PIN is the protection, not the storage backend.
class SecureSessionStore {
  static const _key = 'llave_session';

  // Android / iOS — use platform keychain.
  static const _platformStorage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );

  /// Read the saved session, or `null` if none exists.
  static Future<String?> read() async {
    try {
      if (Platform.isLinux) return await _linuxRead();
      final value = await _platformStorage.read(key: _key);
      _log.fine('read: ${value != null ? '${value.length} bytes' : 'null'}');
      return value;
    } catch (e, st) {
      _log.severe('read failed', e, st);
      return null;
    }
  }

  /// Write the PIN-encrypted blob. Throws on failure.
  static Future<void> write(String value) async {
    _log.fine('write: ${value.length} bytes');
    if (Platform.isLinux) {
      await _linuxWrite(value);
    } else {
      await _platformStorage.write(key: _key, value: value);
    }
    _log.info('write succeeded');
  }

  /// Delete the saved session.
  static Future<void> delete() async {
    try {
      if (Platform.isLinux) {
        await _linuxDelete();
      } else {
        await _platformStorage.delete(key: _key);
      }
      _log.info('delete succeeded');
    } catch (e, st) {
      _log.severe('delete failed', e, st);
    }
  }

  // -------------------------------------------------------------------------
  // Linux file-based storage (~/.local/share/llave/session.enc)
  // -------------------------------------------------------------------------

  static File get _linuxFile {
    final home = Platform.environment['HOME'] ?? '/tmp';
    return File('$home/.local/share/llave/session.enc');
  }

  static Future<String?> _linuxRead() async {
    final file = _linuxFile;
    if (!await file.exists()) {
      _log.fine('linux read: file does not exist');
      return null;
    }
    final value = await file.readAsString();
    _log.fine('linux read: ${value.length} bytes from ${file.path}');
    return value;
  }

  static Future<void> _linuxWrite(String value) async {
    final file = _linuxFile;
    await file.parent.create(recursive: true);
    await file.writeAsString(value, flush: true);
    // Restrict to owner-only (0600).
    await Process.run('chmod', ['600', file.path]);
    _log.fine('linux write: ${value.length} bytes → ${file.path}');
  }

  static Future<void> _linuxDelete() async {
    final file = _linuxFile;
    if (await file.exists()) {
      await file.delete();
      _log.fine('linux delete: ${file.path}');
    }
  }
}
