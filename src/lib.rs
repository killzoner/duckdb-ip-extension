//! DuckDB extension exposing IPv4/IPv6 address parsing into the raw binary
//! representation (4 bytes for IPv4, 16 bytes for IPv6).
//!
//! The pure-Rust parsing logic lives in `ip`; the matching FFI callbacks
//! live in `ip::scalar`. INET-input variants (feature-gated on `inet-interop`)
//! live in `inet` / `inet::scalar`.

mod ip;

#[cfg(feature = "inet-interop")]
mod inet;

use quack_rs::connection::{Connection, Registrar};
use quack_rs::error::ExtensionError;
use quack_rs::scalar::ScalarFunctionBuilder;
use quack_rs::types::TypeId;

/// Registers all functions exposed by this extension.
///
/// # Safety
///
/// `con` must reference a valid DuckDB connection for the duration of the call.
unsafe fn register_all(con: &Connection) -> Result<(), ExtensionError> {
    unsafe {
        con.register_scalar(
            ScalarFunctionBuilder::new("ipv6_to_binary")
                .param(TypeId::Varchar)
                .returns(TypeId::Blob)
                .function(ip::scalar::ipv6_to_binary_scalar),
        )?;
        con.register_scalar(
            ScalarFunctionBuilder::new("ipv4_to_binary")
                .param(TypeId::Varchar)
                .returns(TypeId::Blob)
                .function(ip::scalar::ipv4_to_binary_scalar),
        )?;
        con.register_scalar(
            ScalarFunctionBuilder::new("is_global_ipv6")
                .param(TypeId::Varchar)
                .returns(TypeId::Boolean)
                .function(ip::scalar::is_global_ipv6_scalar),
        )?;
        con.register_scalar(
            ScalarFunctionBuilder::new("is_global_ipv4")
                .param(TypeId::Varchar)
                .returns(TypeId::Boolean)
                .function(ip::scalar::is_global_ipv4_scalar),
        )?;
        con.register_scalar(
            ScalarFunctionBuilder::new("ip_to_binary")
                .param(TypeId::Varchar)
                .returns(TypeId::Blob)
                .function(ip::scalar::ip_to_binary_scalar),
        )?;
        con.register_scalar(
            ScalarFunctionBuilder::new("ip_family")
                .param(TypeId::Varchar)
                .returns(TypeId::UTinyInt)
                .function(ip::scalar::ip_family_scalar),
        )?;

        #[cfg(feature = "inet-interop")]
        inet::register(con)?;
    }
    Ok(())
}

quack_rs::entry_point_v2!(duckdb_ip_extension_init_c_api, |con| register_all(con));
