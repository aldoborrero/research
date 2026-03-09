// Stub file — replaced by flutter_rust_bridge_codegen generate.
//
// These stubs let the app compile before codegen has been run.
// Every function throws [UnimplementedError] so that switching to
// RealLlaveBridge without running codegen first gives a clear message.

const _msg = 'Run flutter_rust_bridge_codegen generate first';

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

Future<bool> initCore({String? sessionJson}) => throw UnimplementedError(_msg);
String? exportSession() => throw UnimplementedError(_msg);
Future<FfiSession> activateDevice({required String nif, String? password}) =>
    throw UnimplementedError(_msg);
Future<FfiPinResult> requestPin() => throw UnimplementedError(_msg);
FfiStatus getStatus() => throw UnimplementedError(_msg);
Future<FfiNifCheckResult> checkNif({String? nif}) =>
    throw UnimplementedError(_msg);
Future<FfiApiResult> getMyData() => throw UnimplementedError(_msg);
Future<FfiApiResult> getHistory() => throw UnimplementedError(_msg);
Future<FfiApiResult> getPendingRequests() => throw UnimplementedError(_msg);
Future<FfiApiResult> confirmRequest(
        {required String token, required String idpCode}) =>
    throw UnimplementedError(_msg);
Future<FfiApiResult> rejectRequest(
        {required String token, required String idpCode}) =>
    throw UnimplementedError(_msg);
Future<FfiApiResult> qrAuthenticate({required String value}) =>
    throw UnimplementedError(_msg);
Future<FfiApiResult> deactivate() => throw UnimplementedError(_msg);
bool logout() => throw UnimplementedError(_msg);
String validateNif({required String nif}) => throw UnimplementedError(_msg);
