/// Flutter ↔ Rust bridge bindings.
///
/// After running codegen (`flutter_rust_bridge_codegen generate`), uncomment
/// the exports below to expose the generated Dart FFI functions.
///
/// To generate:
///   cd flutter
///   flutter_rust_bridge_codegen generate

library;

export 'rust/frb_generated.dart';
export 'rust/api/api.dart';
