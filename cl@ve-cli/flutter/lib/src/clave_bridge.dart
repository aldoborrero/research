import 'package:flutter_riverpod/flutter_riverpod.dart';

// ---------------------------------------------------------------------------
// Dart-side models mirroring the FFI types from clave-core-ffi.
// These will be replaced by flutter_rust_bridge generated types after codegen.
// ---------------------------------------------------------------------------

class ClaveSession {
  final String deviceId;
  final String nif;
  final String createdAt;
  final bool hasFirebaseToken;

  const ClaveSession({
    required this.deviceId,
    required this.nif,
    required this.createdAt,
    this.hasFirebaseToken = false,
  });
}

class ClaveStatus {
  final bool active;
  final String? nif;
  final String? deviceId;
  final String? createdAt;
  final bool hasFirebaseToken;

  const ClaveStatus({
    required this.active,
    this.nif,
    this.deviceId,
    this.createdAt,
    this.hasFirebaseToken = false,
  });

  const ClaveStatus.inactive()
      : active = false,
        nif = null,
        deviceId = null,
        createdAt = null,
        hasFirebaseToken = false;
}

class ClavePinResult {
  final String pin;
  final String timeToLiveSeconds;
  final String nif;

  const ClavePinResult({
    required this.pin,
    required this.timeToLiveSeconds,
    required this.nif,
  });
}

class ClaveApiResult {
  final bool ok;
  final String data;
  final String? error;

  const ClaveApiResult({required this.ok, required this.data, this.error});
}

class ClaveNifCheckResult {
  final String nif;
  final String status;
  final String responseJson;

  const ClaveNifCheckResult({
    required this.nif,
    required this.status,
    required this.responseJson,
  });
}

// ---------------------------------------------------------------------------
// Bridge service interface.
//
// MockClaveBridge is used until flutter_rust_bridge codegen is run.
// Then swap to RealClaveBridge which calls the generated FFI functions.
// ---------------------------------------------------------------------------

abstract class ClaveBridge {
  /// Initialise the Rust core. [sessionJson] is the previously-saved session
  /// from secure storage, or `null` on first launch.
  Future<void> initCore(String? sessionJson);

  /// Export the current session as JSON for secure storage persistence.
  String? exportSession();

  Future<ClaveSession> activateDevice(String nif, String? password);
  Future<ClavePinResult> requestPin();
  ClaveStatus getStatus();
  Future<ClaveNifCheckResult> checkNif(String? nif);
  Future<ClaveApiResult> getMyData();
  Future<ClaveApiResult> getHistory();
  Future<ClaveApiResult> getPendingRequests();
  Future<ClaveApiResult> confirmRequest(String token, String idpCode);
  Future<ClaveApiResult> rejectRequest(String token, String idpCode);
  Future<ClaveApiResult> qrAuthenticate(String value);
  Future<ClaveApiResult> deactivate();
  bool logout();
  String validateNif(String nif);
}

/// Mock implementation that simulates the Rust bridge for UI development.
class MockClaveBridge implements ClaveBridge {
  ClaveSession? _session;

  @override
  Future<void> initCore(String? sessionJson) async {
    // In mock mode, no Rust core to initialise.
  }

  @override
  String? exportSession() {
    // Mock: no real session data to export.
    return null;
  }

  @override
  Future<ClaveSession> activateDevice(String nif, String? password) async {
    // Simulate network delay
    await Future<void>.delayed(const Duration(seconds: 1));

    final validated = validateNif(nif);

    _session = ClaveSession(
      deviceId: 'mock-device-${DateTime.now().millisecondsSinceEpoch}',
      nif: validated,
      createdAt: DateTime.now().toIso8601String(),
    );
    return _session!;
  }

  @override
  Future<ClavePinResult> requestPin() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 800));
    final pin = (100000 + (DateTime.now().microsecond % 900000)).toString();
    return ClavePinResult(
      pin: pin,
      timeToLiveSeconds: '180',
      nif: _session!.nif,
    );
  }

  @override
  ClaveStatus getStatus() {
    if (_session == null) return const ClaveStatus.inactive();
    return ClaveStatus(
      active: true,
      nif: _session!.nif,
      deviceId: _session!.deviceId,
      createdAt: _session!.createdAt,
      hasFirebaseToken: _session!.hasFirebaseToken,
    );
  }

  @override
  Future<ClaveNifCheckResult> checkNif(String? nif) async {
    await Future<void>.delayed(const Duration(milliseconds: 500));
    final n = nif ?? _session?.nif ?? 'unknown';
    return ClaveNifCheckResult(
      nif: n,
      status: 'OK',
      responseJson: '{"activated": true}',
    );
  }

  @override
  Future<ClaveApiResult> getMyData() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 600));
    return ClaveApiResult(
      ok: true,
      data: '{"nombre":"Mock User","telefono":"***1234","nivelAcceso":"AVANZADO"}',
    );
  }

  @override
  Future<ClaveApiResult> getHistory() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 600));
    return const ClaveApiResult(ok: true, data: '{"operations":[]}');
  }

  @override
  Future<ClaveApiResult> getPendingRequests() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 400));
    return const ClaveApiResult(ok: true, data: '{"pending":[]}');
  }

  @override
  Future<ClaveApiResult> confirmRequest(String token, String idpCode) async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    return const ClaveApiResult(ok: true, data: '{"confirmed":true}');
  }

  @override
  Future<ClaveApiResult> rejectRequest(String token, String idpCode) async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    return const ClaveApiResult(ok: true, data: '{"rejected":true}');
  }

  @override
  Future<ClaveApiResult> qrAuthenticate(String value) async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    return const ClaveApiResult(ok: true, data: '{"authenticated":true}');
  }

  @override
  Future<ClaveApiResult> deactivate() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    _session = null;
    return const ClaveApiResult(
      ok: true,
      data: '{"status":"OK","message":"Device deactivated"}',
    );
  }

  @override
  bool logout() {
    _session = null;
    return true;
  }

  @override
  String validateNif(String nif) {
    final trimmed = nif.trim().toUpperCase();
    if (trimmed.length < 8 || trimmed.length > 9) {
      throw ArgumentError('NIF must be 8-9 characters');
    }
    final first = trimmed[0];
    if (!RegExp(r'[0-9XYZ]').hasMatch(first)) {
      throw ArgumentError('NIF must start with 0-9, X, Y, or Z');
    }
    return trimmed;
  }

  void _requireSession() {
    if (_session == null) {
      throw StateError('No active session. Activate device first.');
    }
  }
}

// ---------------------------------------------------------------------------
// Riverpod providers
// ---------------------------------------------------------------------------

/// Single bridge instance shared across the app.
final claveBridgeProvider = Provider<ClaveBridge>((ref) {
  // TODO: Replace with RealClaveBridge after flutter_rust_bridge codegen:
  // return RealClaveBridge();
  return MockClaveBridge();
});
