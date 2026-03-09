/// Placeholder for flutter_rust_bridge generated bindings.
///
/// After running codegen, this file will be replaced with the actual generated
/// Dart bindings that call into llave-core-ffi via FFI.
///
/// To generate:
///   cd flutter
///   flutter_rust_bridge_codegen generate
///
/// The generated code will provide typed Dart functions like:
///   - activateDevice(nif, password) -> FfiSession
///   - requestPin() -> FfiPinResult
///   - getStatus() -> FfiStatus
///   - getPendingRequests() -> FfiApiResult
///   - confirmRequest(token, idpCode) -> FfiApiResult
///   - rejectRequest(token, idpCode) -> FfiApiResult
///   - getHistory() -> FfiApiResult
///   - getMyData() -> FfiApiResult
///   - qrAuthenticate(value) -> FfiApiResult
///   - deactivate() -> FfiApiResult
///   - logout() -> bool
///   - validateNif(nif) -> String

library;

// TODO: Export generated bridge after running flutter_rust_bridge_codegen:
// export 'rust/frb_generated.dart';
// export 'rust/api.dart';
