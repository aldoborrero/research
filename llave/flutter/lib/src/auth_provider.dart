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

  /// Initialise the Rust core and restore a persisted session.
  ///
  /// Must be called once before any other bridge interaction.
  Future<void> init() async {
    _log.info('initialising core');
    final savedJson = await SecureSessionStore.read();
    await _bridge.initCore(savedJson);
    checkSession();
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

  /// Activate device and transition to authenticated state.
  Future<void> activate(String nif, String? password) async {
    _log.info('activating device');
    state = const AuthLoading();
    try {
      final session = await _bridge.activateDevice(nif, password);
      final json = _bridge.exportSession();
      if (json != null) {
        await SecureSessionStore.write(json);
      }
      _log.info('device activated');
      state = AuthAuthenticated(session);
    } catch (e) {
      _log.severe('activation failed: $e');
      state = AuthError(e.toString());
    }
  }

  /// Authenticate via DNI/NIE and transition to authenticated state.
  Future<LlaveApiResult> dniAuthenticate(
      String nif, String fecha, String soporte) async {
    _log.info('authenticating via DNI/NIE');
    state = const AuthLoading();
    try {
      final result = await _bridge.dniAuthenticate(nif, fecha, soporte);
      if (result.ok) {
        // Re-check session status — the backend should have established one.
        checkSession();
        // If the backend didn't establish a full session (e.g. DNI-only flow),
        // persist whatever state exists.
        final json = _bridge.exportSession();
        if (json != null) {
          await SecureSessionStore.write(json);
        }
        _log.info('DNI/NIE authentication successful');
      } else {
        _log.warning('DNI/NIE authentication failed: ${result.error}');
        state = AuthError(result.error ?? 'DNI/NIE authentication failed');
      }
      return result;
    } catch (e) {
      _log.severe('DNI/NIE authentication error: $e');
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
