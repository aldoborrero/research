import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

import 'llave_bridge.dart';
import 'secure_session_store.dart';

final _log = Logger('AuthNotifier');

/// Authentication state for the app.
sealed class AuthState {
  const AuthState();
}

class AuthLoading extends AuthState {
  const AuthLoading();
}

class AuthUnauthenticated extends AuthState {
  const AuthUnauthenticated();
}

/// Session exists on disk but is encrypted — user must enter PIN.
class AuthLocked extends AuthState {
  const AuthLocked();
}

/// Session needs a PIN to be set (right after activation).
class AuthNeedsPin extends AuthState {
  const AuthNeedsPin();
}

class AuthAuthenticated extends AuthState {
  final LlaveSession session;
  const AuthAuthenticated(this.session);
}

class AuthError extends AuthState {
  final String message;
  const AuthError(this.message);
}

/// Manages authentication lifecycle.
class AuthNotifier extends StateNotifier<AuthState> {
  final LlaveBridge _bridge;

  AuthNotifier(this._bridge) : super(const AuthLoading());

  /// Initialise the Rust core and check for a persisted session.
  ///
  /// If an encrypted blob exists, transitions to [AuthLocked] so the
  /// user can enter their PIN. Otherwise, [AuthUnauthenticated].
  Future<void> init() async {
    _log.info('initialising core');
    // Init Rust core without session data — we'll load it after PIN entry.
    await _bridge.initCore(null);

    final sealed = await SecureSessionStore.read();
    if (sealed != null && sealed.isNotEmpty) {
      _log.info('encrypted session found — waiting for PIN');
      state = const AuthLocked();
    } else {
      _log.info('no session');
      state = const AuthUnauthenticated();
    }
  }

  /// Unlock an encrypted session with the user's PIN.
  ///
  /// Reads the sealed blob from secure storage, decrypts it in Rust,
  /// and transitions to [AuthAuthenticated] on success.
  Future<void> unlock(String pin) async {
    state = const AuthLoading();
    try {
      final sealed = await SecureSessionStore.read();
      if (sealed == null || sealed.isEmpty) {
        state = const AuthUnauthenticated();
        return;
      }
      _bridge.decryptAndLoadSession(sealed, pin);
      checkSession();
    } catch (e) {
      _log.warning('unlock failed: $e');
      // Return to locked so the user can retry.
      state = const AuthLocked();
      rethrow;
    }
  }

  /// Encrypt the current session with a PIN and persist the sealed blob.
  ///
  /// Called after activation when the user sets their PIN for the first time.
  Future<void> setPin(String pin) async {
    try {
      final sealed = _bridge.encryptSession(pin);
      await SecureSessionStore.write(sealed);
      _log.info('session encrypted and persisted');
      checkSession();
    } catch (e) {
      _log.severe('setPin failed: $e');
      state = AuthError(e.toString());
    }
  }

  /// Change the PIN protecting the session.
  Future<void> changePin(String oldPin, String newPin) async {
    final sealed = await SecureSessionStore.read();
    if (sealed == null || sealed.isEmpty) {
      throw StateError('No encrypted session to re-key');
    }
    final newSealed = _bridge.changeSessionPin(sealed, oldPin, newPin);
    await SecureSessionStore.write(newSealed);
    _log.info('PIN changed');
  }

  /// Check if a session already exists (called on startup).
  void checkSession() {
    final status = _bridge.getStatus();
    if (status.active &&
        status.nif != null &&
        status.deviceId != null &&
        status.createdAt != null) {
      _log.info('session active');
      state = AuthAuthenticated(LlaveSession(
        deviceId: status.deviceId!,
        nif: status.nif!,
        createdAt: status.createdAt!,
        hasFirebaseToken: status.hasFirebaseToken,
      ));
    } else {
      _log.info('no session');
      state = const AuthUnauthenticated();
    }
  }

  /// Activate device and transition to [AuthNeedsPin] so the user sets a PIN.
  Future<void> activate(String nif, String? password) async {
    _log.info('activating device');
    state = const AuthLoading();
    try {
      await _bridge.activateDevice(nif, password);
      _log.info('device activated — prompting for PIN setup');
      state = const AuthNeedsPin();
    } catch (e) {
      _log.severe('activation failed: $e');
      state = AuthError(e.toString());
    }
  }

  /// Phase 1: DNI/NIE auth → registration check → request SMS code.
  Future<LlaveApiResult> dniAuthenticate(
      String nif, String fecha, String soporte) async {
    _log.info('authenticating via DNI/NIE (phase 1: request SMS)');
    state = const AuthLoading();
    try {
      final result = await _bridge.dniAuthenticate(nif, fecha, soporte);
      if (result.ok) {
        _log.info('DNI/NIE auth succeeded — SMS sent');
        state = const AuthUnauthenticated();
      } else {
        _log.warning('DNI/NIE auth failed: ${result.error}');
        state = AuthError(result.error ?? 'DNI/NIE authentication failed');
      }
      return result;
    } catch (e) {
      _log.severe('DNI/NIE authentication error: $e');
      state = AuthError(e.toString());
      rethrow;
    }
  }

  /// Phase 2: Validate SMS code + activate device → prompt for PIN setup.
  Future<LlaveApiResult> dniCompleteActivation(
      String nif,
      String cookiesJson,
      String timestampAltaSms,
      String tokenClaveMovilSms,
      String smsPin) async {
    _log.info('completing DNI/NIE activation (phase 2: validate SMS + activate)');
    state = const AuthLoading();
    try {
      final result = await _bridge.dniCompleteActivation(
          nif, cookiesJson, timestampAltaSms, tokenClaveMovilSms, smsPin);
      if (result.ok) {
        _log.info('DNI/NIE + SMS activation succeeded — prompting for PIN setup');
        state = const AuthNeedsPin();
      } else {
        _log.warning('DNI/NIE activation failed: ${result.error}');
        state = AuthError(result.error ?? 'Activation failed');
      }
      return result;
    } catch (e) {
      _log.severe('DNI/NIE activation error: $e');
      state = AuthError(e.toString());
      rethrow;
    }
  }

  /// Log out and clear session.
  Future<void> logout() async {
    _log.info('logging out');
    _bridge.logout();
    await SecureSessionStore.delete();
    state = const AuthUnauthenticated();
  }

  /// Deactivate device remotely, then log out.
  Future<void> deactivate() async {
    _log.info('deactivating device');
    try {
      await _bridge.deactivate();
    } catch (e) {
      _log.warning('remote deactivation failed: $e');
    }
    await SecureSessionStore.delete();
    state = const AuthUnauthenticated();
  }
}

/// Auth state provider.
final authProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  final bridge = ref.watch(llaveBridgeProvider);
  return AuthNotifier(bridge);
});

/// Convenience: whether the user is authenticated.
final isAuthenticatedProvider = Provider<bool>((ref) {
  return ref.watch(authProvider) is AuthAuthenticated;
});

/// Convenience: current session (null if not authenticated).
final currentSessionProvider = Provider<LlaveSession?>((ref) {
  final auth = ref.watch(authProvider);
  return auth is AuthAuthenticated ? auth.session : null;
});
