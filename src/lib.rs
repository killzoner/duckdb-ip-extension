//! DuckDB extension exposing IPv4/IPv6 address parsing into the raw binary
//! representation (4 bytes for IPv4, 16 bytes for IPv6).
//!
//! The pure-Rust parsing logic lives in `ip`; the matching FFI callbacks
//! live in `ip::scalar`. INET-input variants (feature-gated on `inet-interop`)
//! live in `inet` / `inet::scalar`.

include!("extension.rs");
