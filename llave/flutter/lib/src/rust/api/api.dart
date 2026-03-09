// Native C FFI bindings to libllave_core_ffi.so.
//
// Replaces the flutter_rust_bridge stubs with direct dart:ffi calls
// to the `extern "C"` functions exported by the Rust FFI crate.

import 'dart:convert';
import 'dart:ffi' as ffi;
import 'dart:isolate';

import 'package:ffi/ffi.dart';

// ---------------------------------------------------------------------------
// FFI types (unchanged from stubs)
// ---------------------------------------------------------------------------

class FfiSession {
  final String deviceId;
  final String nif;
  final String createdAt;
  final bool hasFirebaseToken;
  const FfiSession({
    required this.deviceId,
    required this.nif,
    required this.createdAt,
    required this.hasFirebaseToken,
  });
}

class FfiPinResult {
  final String pin;
  final String timeToLiveSeconds;
  final String nif;
  const FfiPinResult({
    required this.pin,
    required this.timeToLiveSeconds,
    required this.nif,
  });
}

class FfiStatus {
  final bool active;
  final String? nif;
  final String? deviceId;
  final String? createdAt;
  final bool hasFirebaseToken;
  const FfiStatus({
    required this.active,
    this.nif,
    this.deviceId,
    this.createdAt,
    required this.hasFirebaseToken,
  });
}

class FfiApiResult {
  final bool ok;
  final String data;
  final String? error;
  const FfiApiResult({required this.ok, required this.data, this.error});
}

class FfiNifCheckResult {
  final String nif;
  final String status;
  final String responseJson;
  const FfiNifCheckResult({
    required this.nif,
    required this.status,
    required this.responseJson,
  });
}

// ---------------------------------------------------------------------------
// Native library handle
// ---------------------------------------------------------------------------

/// Must be set before any FFI call. Typically set from main().
late String nativeLibPath;

// C function typedefs
typedef _FreeStringC = ffi.Void Function(ffi.Pointer<Utf8>);
typedef _FreeStringDart = void Function(ffi.Pointer<Utf8>);

typedef _Fn0C = ffi.Pointer<Utf8> Function();
typedef _Fn0Dart = ffi.Pointer<Utf8> Function();

typedef _Fn1C = ffi.Pointer<Utf8> Function(ffi.Pointer<Utf8>);
typedef _Fn1Dart = ffi.Pointer<Utf8> Function(ffi.Pointer<Utf8>);

typedef _Fn2C = ffi.Pointer<Utf8> Function(ffi.Pointer<Utf8>, ffi.Pointer<Utf8>);
typedef _Fn2Dart = ffi.Pointer<Utf8> Function(ffi.Pointer<Utf8>, ffi.Pointer<Utf8>);

typedef _Fn3C = ffi.Pointer<Utf8> Function(
    ffi.Pointer<Utf8>, ffi.Pointer<Utf8>, ffi.Pointer<Utf8>);
typedef _Fn3Dart = ffi.Pointer<Utf8> Function(
    ffi.Pointer<Utf8>, ffi.Pointer<Utf8>, ffi.Pointer<Utf8>);

// ---------------------------------------------------------------------------
// Low-level helpers (usable from any isolate)
// ---------------------------------------------------------------------------

ffi.Pointer<Utf8> _toNative(String? s) {
  if (s == null) return ffi.nullptr.cast();
  return s.toNativeUtf8();
}

void _freeNative(ffi.Pointer<Utf8> p) {
  if (p.address != 0) calloc.free(p);
}

Map<String, dynamic> _parseResult(ffi.DynamicLibrary lib, ffi.Pointer<Utf8> ptr) {
  final json = ptr.toDartString();
  final free = lib.lookupFunction<_FreeStringC, _FreeStringDart>('llave_free_string');
  free(ptr);
  return jsonDecode(json) as Map<String, dynamic>;
}

Map<String, dynamic> _checkOk(Map<String, dynamic> result) {
  if (result['ok'] != true) {
    throw Exception(result['error'] ?? 'Unknown native error');
  }
  return result;
}

/// Call a 0-arg native function.
Map<String, dynamic> _call0(String libPath, String symbol) {
  final lib = ffi.DynamicLibrary.open(libPath);
  final fn = lib.lookupFunction<_Fn0C, _Fn0Dart>(symbol);
  return _parseResult(lib, fn());
}

/// Call a 1-arg native function.
Map<String, dynamic> _call1(String libPath, String symbol, String? a1) {
  final lib = ffi.DynamicLibrary.open(libPath);
  final fn = lib.lookupFunction<_Fn1C, _Fn1Dart>(symbol);
  final p1 = _toNative(a1);
  final result = _parseResult(lib, fn(p1));
  _freeNative(p1);
  return result;
}

/// Call a 2-arg native function.
Map<String, dynamic> _call2(String libPath, String symbol, String? a1, String? a2) {
  final lib = ffi.DynamicLibrary.open(libPath);
  final fn = lib.lookupFunction<_Fn2C, _Fn2Dart>(symbol);
  final p1 = _toNative(a1);
  final p2 = _toNative(a2);
  final result = _parseResult(lib, fn(p1, p2));
  _freeNative(p1);
  _freeNative(p2);
  return result;
}

/// Call a 3-arg native function.
Map<String, dynamic> _call3(
    String libPath, String symbol, String? a1, String? a2, String? a3) {
  final lib = ffi.DynamicLibrary.open(libPath);
  final fn = lib.lookupFunction<_Fn3C, _Fn3Dart>(symbol);
  final p1 = _toNative(a1);
  final p2 = _toNative(a2);
  final p3 = _toNative(a3);
  final result = _parseResult(lib, fn(p1, p2, p3));
  _freeNative(p1);
  _freeNative(p2);
  _freeNative(p3);
  return result;
}

// ---------------------------------------------------------------------------
// Sync API (called on main isolate)
// ---------------------------------------------------------------------------

Future<bool> initCore({String? sessionJson}) async {
  final r = _checkOk(_call1(nativeLibPath, 'llave_init_core', sessionJson));
  return r['data'] as bool;
}

String? exportSession() {
  final r = _checkOk(_call0(nativeLibPath, 'llave_export_session'));
  return r['data'] as String?;
}

FfiStatus getStatus() {
  final r = _checkOk(_call0(nativeLibPath, 'llave_get_status'));
  final d = r['data'] as Map<String, dynamic>;
  return FfiStatus(
    active: d['active'] as bool,
    nif: d['nif'] as String?,
    deviceId: d['deviceId'] as String?,
    createdAt: d['createdAt'] as String?,
    hasFirebaseToken: d['hasFirebaseToken'] as bool,
  );
}

bool logout() {
  _checkOk(_call0(nativeLibPath, 'llave_logout'));
  return true;
}

String validateNif({required String nif}) {
  final r = _checkOk(_call1(nativeLibPath, 'llave_validate_nif', nif));
  return r['data'] as String;
}

// ---------------------------------------------------------------------------
// Async API (run in isolate to avoid blocking UI)
// ---------------------------------------------------------------------------

Future<FfiSession> activateDevice({required String nif, String? password}) async {
  final lp = nativeLibPath;
  final r = await Isolate.run(() => _checkOk(_call2(lp, 'llave_activate_device', nif, password)));
  final d = r['data'] as Map<String, dynamic>;
  return FfiSession(
    deviceId: d['deviceId'] as String,
    nif: d['nif'] as String,
    createdAt: d['createdAt'] as String,
    hasFirebaseToken: d['hasFirebaseToken'] as bool,
  );
}

Future<FfiPinResult> requestPin() async {
  final lp = nativeLibPath;
  final r = await Isolate.run(() => _checkOk(_call0(lp, 'llave_request_pin')));
  final d = r['data'] as Map<String, dynamic>;
  return FfiPinResult(
    pin: d['pin'] as String,
    timeToLiveSeconds: d['timeToLiveSeconds'] as String,
    nif: d['nif'] as String,
  );
}

Future<FfiNifCheckResult> checkNif({String? nif}) async {
  final lp = nativeLibPath;
  final r = await Isolate.run(() => _checkOk(_call1(lp, 'llave_check_nif', nif)));
  final d = r['data'] as Map<String, dynamic>;
  return FfiNifCheckResult(
    nif: d['nif'] as String,
    status: d['status'] as String,
    responseJson: d['responseJson'] as String,
  );
}

FfiApiResult _toApiResult(Map<String, dynamic> r) {
  final d = r['data'] as Map<String, dynamic>;
  return FfiApiResult(
    ok: d['ok'] as bool,
    data: d['data'] as String? ?? '',
    error: d['error'] as String?,
  );
}

Future<FfiApiResult> getMyData() async {
  final lp = nativeLibPath;
  return Isolate.run(() => _toApiResult(_checkOk(_call0(lp, 'llave_get_my_data'))));
}

Future<FfiApiResult> getHistory() async {
  final lp = nativeLibPath;
  return Isolate.run(() => _toApiResult(_checkOk(_call0(lp, 'llave_get_history'))));
}

Future<FfiApiResult> getPendingRequests() async {
  final lp = nativeLibPath;
  return Isolate.run(() => _toApiResult(_checkOk(_call0(lp, 'llave_get_pending_requests'))));
}

Future<FfiApiResult> confirmRequest({required String token, required String idpCode}) async {
  final lp = nativeLibPath;
  return Isolate.run(
      () => _toApiResult(_checkOk(_call2(lp, 'llave_confirm_request', token, idpCode))));
}

Future<FfiApiResult> rejectRequest({required String token, required String idpCode}) async {
  final lp = nativeLibPath;
  return Isolate.run(
      () => _toApiResult(_checkOk(_call2(lp, 'llave_reject_request', token, idpCode))));
}

Future<FfiApiResult> qrAuthenticate({required String value}) async {
  final lp = nativeLibPath;
  return Isolate.run(
      () => _toApiResult(_checkOk(_call1(lp, 'llave_qr_authenticate', value))));
}

Future<FfiApiResult> deactivate() async {
  final lp = nativeLibPath;
  return Isolate.run(() => _toApiResult(_checkOk(_call0(lp, 'llave_deactivate'))));
}

Future<FfiApiResult> dniAuthenticate(
    {required String nif, required String fecha, required String soporte}) async {
  final lp = nativeLibPath;
  return Isolate.run(
      () => _toApiResult(_checkOk(_call3(lp, 'llave_dni_authenticate', nif, fecha, soporte))));
}
