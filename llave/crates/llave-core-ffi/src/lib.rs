//! FFI bridge layer for exposing llave-core to Flutter via flutter_rust_bridge.
//!
//! This crate provides a flat, async-friendly API surface that the bridge
//! codegen can consume. All types crossing the FFI boundary are simple
//! structs/enums with owned data (no lifetimes, no generics).

#![allow(unexpected_cfgs)]

pub mod api;
