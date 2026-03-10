import 'package:flutter_riverpod/flutter_riverpod.dart';

// Generated FFI bindings — available after running flutter_rust_bridge_codegen.
import 'rust/api/api.dart' as ffi;

// ---------------------------------------------------------------------------
// Dart-side models mirroring the FFI types from llave-core-ffi.
// These will be replaced by flutter_rust_bridge generated types after codegen.
// ---------------------------------------------------------------------------

class LlaveSession {
  final String deviceId;
  final String nif;
  final String createdAt;
  final bool hasFirebaseToken;

  const LlaveSession({
    required this.deviceId,
    required this.nif,
    required this.createdAt,
    this.hasFirebaseToken = false,
  });
}

class LlaveStatus {
  final bool active;
  final String? nif;
  final String? deviceId;
  final String? createdAt;
  final bool hasFirebaseToken;

  const LlaveStatus({
    required this.active,
    this.nif,
    this.deviceId,
    this.createdAt,
    this.hasFirebaseToken = false,
  });

  const LlaveStatus.inactive()
      : active = false,
        nif = null,
        deviceId = null,
        createdAt = null,
        hasFirebaseToken = false;
}

class LlavePinResult {
  final String pin;
  final String timeToLiveSeconds;
  final String nif;

  const LlavePinResult({
    required this.pin,
    required this.timeToLiveSeconds,
    required this.nif,
  });
}

class LlaveApiResult {
  final bool ok;
  final String data;
  final String? error;

  const LlaveApiResult({required this.ok, required this.data, this.error});
}

class LlaveNifCheckResult {
  final String nif;
  final String status;
  final String responseJson;

  const LlaveNifCheckResult({
    required this.nif,
    required this.status,
    required this.responseJson,
  });
}

// ---------------------------------------------------------------------------
// Bridge service interface.
//
// MockLlaveBridge is used until flutter_rust_bridge codegen is run.
// Then swap to RealLlaveBridge which calls the generated FFI functions.
// ---------------------------------------------------------------------------

abstract class LlaveBridge {
  /// Initialise the Rust core. [sessionJson] is the previously-saved session
  /// from secure storage, or `null` on first launch.
  Future<void> initCore(String? sessionJson);

  /// Export the current session as JSON for secure storage persistence.
  String? exportSession();

  Future<LlaveSession> activateDevice(String nif, String? password);
  Future<LlavePinResult> requestPin();
  LlaveStatus getStatus();
  Future<LlaveNifCheckResult> checkNif(String? nif);
  Future<LlaveApiResult> getMyData();
  Future<LlaveApiResult> getHistory();
  Future<LlaveApiResult> getPendingRequests();
  Future<LlaveApiResult> confirmRequest(String token, String idpCode);
  Future<LlaveApiResult> rejectRequest(String token, String idpCode);
  Future<LlaveApiResult> qrAuthenticate(String value);
  Future<LlaveApiResult> deactivate();
  Future<LlaveApiResult> dniAuthenticate(String nif, String fecha, String soporte);
  Future<LlaveApiResult> dniCompleteActivation(String nif, String cookiesJson,
      String timestampAltaSms, String tokenClaveMovilSms, String smsPin);
  bool logout();
  String validateNif(String nif);
}

/// Mock implementation that simulates the Rust bridge for UI development.
class MockLlaveBridge implements LlaveBridge {
  LlaveSession? _session;

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
  Future<LlaveSession> activateDevice(String nif, String? password) async {
    // Simulate network delay
    await Future<void>.delayed(const Duration(seconds: 1));

    final validated = validateNif(nif);

    _session = LlaveSession(
      deviceId: 'mock-device-${DateTime.now().millisecondsSinceEpoch}',
      nif: validated,
      createdAt: DateTime.now().toIso8601String(),
    );
    return _session!;
  }

  @override
  Future<LlavePinResult> requestPin() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 800));
    final pin = (100000 + (DateTime.now().microsecond % 900000)).toString();
    return LlavePinResult(
      pin: pin,
      timeToLiveSeconds: '180',
      nif: _session!.nif,
    );
  }

  @override
  LlaveStatus getStatus() {
    if (_session == null) return const LlaveStatus.inactive();
    return LlaveStatus(
      active: true,
      nif: _session!.nif,
      deviceId: _session!.deviceId,
      createdAt: _session!.createdAt,
      hasFirebaseToken: _session!.hasFirebaseToken,
    );
  }

  @override
  Future<LlaveNifCheckResult> checkNif(String? nif) async {
    await Future<void>.delayed(const Duration(milliseconds: 500));
    final n = nif ?? _session?.nif ?? 'unknown';
    return LlaveNifCheckResult(
      nif: n,
      status: 'OK',
      responseJson: '{"activated": true}',
    );
  }

  @override
  Future<LlaveApiResult> getMyData() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 600));
    return LlaveApiResult(
      ok: true,
      data: '{"nombre":"Mock User","telefono":"***1234","nivelAcceso":"AVANZADO"}',
    );
  }

  @override
  Future<LlaveApiResult> getHistory() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 600));
    return const LlaveApiResult(ok: true, data: '{"operations":[]}');
  }

  @override
  Future<LlaveApiResult> getPendingRequests() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 400));
    return const LlaveApiResult(ok: true, data: '{"pending":[]}');
  }

  @override
  Future<LlaveApiResult> confirmRequest(String token, String idpCode) async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    return const LlaveApiResult(ok: true, data: '{"confirmed":true}');
  }

  @override
  Future<LlaveApiResult> rejectRequest(String token, String idpCode) async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    return const LlaveApiResult(ok: true, data: '{"rejected":true}');
  }

  @override
  Future<LlaveApiResult> qrAuthenticate(String value) async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    return const LlaveApiResult(ok: true, data: '{"authenticated":true}');
  }

  @override
  Future<LlaveApiResult> deactivate() async {
    _requireSession();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    _session = null;
    return const LlaveApiResult(
      ok: true,
      data: '{"status":"OK","message":"Device deactivated"}',
    );
  }

  @override
  Future<LlaveApiResult> dniAuthenticate(String nif, String fecha, String soporte) async {
    await Future<void>.delayed(const Duration(milliseconds: 800));
    return LlaveApiResult(
      ok: true,
      data: '{"movil":"***1234","hora_peticion":"12:00","timestamp_alta_sms":"mock_ts","token_clave_movil_sms":"mock_token","cookies_json":"[]","registrado":"S","nivel_registro":"A","telefono":"S"}',
    );
  }

  @override
  Future<LlaveApiResult> dniCompleteActivation(String nif, String cookiesJson,
      String timestampAltaSms, String tokenClaveMovilSms, String smsPin) async {
    await Future<void>.delayed(const Duration(milliseconds: 800));
    final validated = validateNif(nif);
    _session = LlaveSession(
      deviceId: 'mock-dni-${DateTime.now().millisecondsSinceEpoch}',
      nif: validated,
      createdAt: DateTime.now().toIso8601String(),
    );
    return LlaveApiResult(
      ok: true,
      data: '{"device_id":"${_session!.deviceId}","nif":"$validated"}',
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
// Real bridge calling into Rust via flutter_rust_bridge FFI.
// ---------------------------------------------------------------------------

class RealLlaveBridge implements LlaveBridge {
  @override
  Future<void> initCore(String? sessionJson) async {
    await ffi.initCore(sessionJson: sessionJson);
  }

  @override
  String? exportSession() => ffi.exportSession();

  @override
  Future<LlaveSession> activateDevice(String nif, String? password) async {
    final s = await ffi.activateDevice(nif: nif, password: password);
    return LlaveSession(
      deviceId: s.deviceId,
      nif: s.nif,
      createdAt: s.createdAt,
      hasFirebaseToken: s.hasFirebaseToken,
    );
  }

  @override
  Future<LlavePinResult> requestPin() async {
    final r = await ffi.requestPin();
    return LlavePinResult(
      pin: r.pin,
      timeToLiveSeconds: r.timeToLiveSeconds,
      nif: r.nif,
    );
  }

  @override
  LlaveStatus getStatus() {
    final s = ffi.getStatus();
    return LlaveStatus(
      active: s.active,
      nif: s.nif,
      deviceId: s.deviceId,
      createdAt: s.createdAt,
      hasFirebaseToken: s.hasFirebaseToken,
    );
  }

  @override
  Future<LlaveNifCheckResult> checkNif(String? nif) async {
    final r = await ffi.checkNif(nif: nif);
    return LlaveNifCheckResult(
      nif: r.nif,
      status: r.status,
      responseJson: r.responseJson,
    );
  }

  @override
  Future<LlaveApiResult> getMyData() async {
    final r = await ffi.getMyData();
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> getHistory() async {
    final r = await ffi.getHistory();
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> getPendingRequests() async {
    final r = await ffi.getPendingRequests();
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> confirmRequest(String token, String idpCode) async {
    final r = await ffi.confirmRequest(token: token, idpCode: idpCode);
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> rejectRequest(String token, String idpCode) async {
    final r = await ffi.rejectRequest(token: token, idpCode: idpCode);
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> qrAuthenticate(String value) async {
    final r = await ffi.qrAuthenticate(value: value);
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> deactivate() async {
    final r = await ffi.deactivate();
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> dniAuthenticate(String nif, String fecha, String soporte) async {
    final r = await ffi.dniAuthenticate(nif: nif, fecha: fecha, soporte: soporte);
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  Future<LlaveApiResult> dniCompleteActivation(String nif, String cookiesJson,
      String timestampAltaSms, String tokenClaveMovilSms, String smsPin) async {
    final r = await ffi.dniCompleteActivation(
        nif: nif,
        cookiesJson: cookiesJson,
        timestampAltaSms: timestampAltaSms,
        tokenClaveMovilSms: tokenClaveMovilSms,
        smsPin: smsPin);
    return LlaveApiResult(ok: r.ok, data: r.data, error: r.error);
  }

  @override
  bool logout() => ffi.logout();

  @override
  String validateNif(String nif) => ffi.validateNif(nif: nif);
}

// ---------------------------------------------------------------------------
// Riverpod providers
// ---------------------------------------------------------------------------

enum BridgeType { mock, real }

/// Which bridge implementation to use. Toggle from DevScreen.
final bridgeTypeProvider = StateProvider<BridgeType>((ref) => BridgeType.real);

/// Single bridge instance shared across the app, rebuilt when [bridgeTypeProvider] changes.
final llaveBridgeProvider = Provider<LlaveBridge>((ref) {
  return switch (ref.watch(bridgeTypeProvider)) {
    BridgeType.mock => MockLlaveBridge(),
    BridgeType.real => RealLlaveBridge(),
  };
});
