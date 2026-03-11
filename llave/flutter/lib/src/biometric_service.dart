import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:local_auth/local_auth.dart';
import 'package:logging/logging.dart';
import 'package:shared_preferences/shared_preferences.dart';

final _log = Logger('BiometricService');

/// Manages biometric authentication and secure PIN storage.
///
/// When biometrics are enabled, the user's PIN is stored in platform-secure
/// storage (Android Keystore / iOS Keychain) and can be retrieved after a
/// successful biometric check. The PIN is then used to decrypt the session
/// as usual — biometrics is a convenience layer, not a replacement for the
/// Argon2id + AES-256-GCM encryption.
class BiometricService {
  static const _pinKey = 'llave_biometric_pin';
  static const _enabledPrefKey = 'llave_biometric_enabled';

  static final _auth = LocalAuthentication();

  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    iOptions: IOSOptions(
      accessibility: KeychainAccessibility.unlocked,
    ),
  );

  /// Whether the device hardware supports biometrics.
  static Future<bool> isAvailable() async {
    if (!Platform.isAndroid && !Platform.isIOS) return false;
    try {
      final canCheck = await _auth.canCheckBiometrics;
      final isSupported = await _auth.isDeviceSupported();
      return canCheck && isSupported;
    } on PlatformException catch (e) {
      _log.warning('biometric availability check failed: $e');
      return false;
    }
  }

  /// Whether the user has opted-in to biometric unlock.
  static Future<bool> isEnabled() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getBool(_enabledPrefKey) ?? false;
  }

  /// Enable biometric unlock by storing the PIN securely.
  static Future<void> enable(String pin) async {
    await _storage.write(key: _pinKey, value: pin);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_enabledPrefKey, true);
    _log.info('biometric unlock enabled');
  }

  /// Disable biometric unlock and remove the stored PIN.
  static Future<void> disable() async {
    await _storage.delete(key: _pinKey);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_enabledPrefKey, false);
    _log.info('biometric unlock disabled');
  }

  /// Prompt the user for biometric authentication and return the stored PIN
  /// on success. Returns `null` if authentication fails or is cancelled.
  static Future<String?> authenticate() async {
    try {
      final didAuth = await _auth.authenticate(
        localizedReason: 'Unlock Llave with biometrics',
        options: const AuthenticationOptions(
          stickyAuth: true,
          biometricOnly: true,
        ),
      );
      if (!didAuth) {
        _log.fine('biometric auth cancelled or failed');
        return null;
      }
      final pin = await _storage.read(key: _pinKey);
      if (pin == null || pin.isEmpty) {
        _log.warning('biometric auth succeeded but no PIN stored');
        await disable();
        return null;
      }
      _log.info('biometric auth succeeded');
      return pin;
    } on PlatformException catch (e) {
      _log.warning('biometric auth error: $e');
      return null;
    }
  }

  /// Update the stored PIN (e.g. after a PIN change).
  static Future<void> updatePin(String newPin) async {
    final enabled = await isEnabled();
    if (!enabled) return;
    await _storage.write(key: _pinKey, value: newPin);
    _log.info('biometric PIN updated');
  }
}

/// Provider that exposes whether biometric unlock is both available and enabled.
final biometricStateProvider = FutureProvider<BiometricState>((ref) async {
  final available = await BiometricService.isAvailable();
  final enabled = available && await BiometricService.isEnabled();
  return BiometricState(available: available, enabled: enabled);
});

class BiometricState {
  final bool available;
  final bool enabled;
  const BiometricState({required this.available, required this.enabled});
}
