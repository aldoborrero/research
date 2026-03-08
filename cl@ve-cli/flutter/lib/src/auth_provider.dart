import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'clave_bridge.dart';
import 'secure_session_store.dart';

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
  final ClaveSession session;
  const AuthAuthenticated(this.session);
}

class AuthError extends AuthState {
  final String message;
  const AuthError(this.message);
}

/// Manages authentication lifecycle.
class AuthNotifier extends StateNotifier<AuthState> {
  final ClaveBridge _bridge;

  AuthNotifier(this._bridge) : super(const AuthLoading());

  /// Initialise the Rust core and restore a persisted session.
  ///
  /// Must be called once before any other bridge interaction.
  Future<void> init() async {
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
      state = AuthAuthenticated(ClaveSession(
        deviceId: status.deviceId!,
        nif: status.nif!,
        createdAt: status.createdAt!,
        hasFirebaseToken: status.hasFirebaseToken,
      ));
    } else {
      state = const AuthUnauthenticated();
    }
  }

  /// Activate device and transition to authenticated state.
  Future<void> activate(String nif, String? password) async {
    state = const AuthLoading();
    try {
      final session = await _bridge.activateDevice(nif, password);
      // Persist session to platform secure storage
      final json = _bridge.exportSession();
      if (json != null) {
        await SecureSessionStore.write(json);
      }
      state = AuthAuthenticated(session);
    } catch (e) {
      state = AuthError(e.toString());
    }
  }

  /// Log out and clear session.
  Future<void> logout() async {
    _bridge.logout();
    await SecureSessionStore.delete();
    state = const AuthUnauthenticated();
  }

  /// Deactivate device remotely, then log out.
  Future<void> deactivate() async {
    try {
      await _bridge.deactivate();
    } catch (_) {
      // Even if remote deactivation fails, clear local session.
    }
    await SecureSessionStore.delete();
    state = const AuthUnauthenticated();
  }
}

/// Auth state provider.
final authProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  final bridge = ref.watch(claveBridgeProvider);
  return AuthNotifier(bridge);
});

/// Convenience: whether the user is authenticated.
final isAuthenticatedProvider = Provider<bool>((ref) {
  return ref.watch(authProvider) is AuthAuthenticated;
});

/// Convenience: current session (null if not authenticated).
final currentSessionProvider = Provider<ClaveSession?>((ref) {
  final auth = ref.watch(authProvider);
  return auth is AuthAuthenticated ? auth.session : null;
});
