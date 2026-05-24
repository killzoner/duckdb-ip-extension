//! `duckdb-inet` interop: pure-logic helpers that consume an INET value's
//! family byte + 16-byte big-endian address buffer.
//!
//! `INET` is `STRUCT(ip_type UTINYINT, address HUGEINT, mask USMALLINT)` with
//! alias `"INET"`. `ip_type` is `1` for IPv4, `2` for IPv6. `address` is the
//! 16-byte big-endian representation; for IPv4 the four octets sit in the low
//! bytes (last 4 of the BE buffer).
//!
//! The entire module is gated behind the `inet-interop` Cargo feature so
//! users who don't load `duckdb-inet` at runtime can omit it cleanly.

pub(crate) mod scalar;

use crate::ip;
use quack_rs::connection::{Connection, Registrar};
use quack_rs::error::ExtensionError;
use quack_rs::scalar::ScalarFunctionBuilder;
use quack_rs::types::{LogicalType, TypeId};

/// Registers the INET-input scalar functions on `con`.
///
/// # Safety
///
/// `con` must reference a valid DuckDB connection for the duration of the call.
pub(crate) unsafe fn register(con: &Connection) -> Result<(), ExtensionError> {
    unsafe {
        con.register_scalar(
            ScalarFunctionBuilder::new("inet_to_binary")
                .param_logical(build_inet_logical_type())
                .returns(TypeId::Blob)
                .function(scalar::to_binary_scalar),
        )?;
        con.register_scalar(
            ScalarFunctionBuilder::new("is_global_inet")
                .param_logical(build_inet_logical_type())
                .returns(TypeId::Boolean)
                .function(scalar::is_global_scalar),
        )?;
    }
    Ok(())
}

fn build_inet_logical_type() -> LogicalType {
    let inet_type = LogicalType::struct_type(&[
        ("ip_type", TypeId::UTinyInt),
        ("address", TypeId::HugeInt),
        ("mask", TypeId::USmallInt),
    ]);
    // SAFETY: `inet_type` was just constructed; we own the only handle, and
    // `set_alias` only stores an internal alias string.
    unsafe { inet_type.set_alias("INET") };
    inet_type
}

/// Extracts the packed byte representation from an INET value.
///
/// `family` is the `ip_type` field: 1 = IPv4, 2 = IPv6, 0 = invalid. Returns
/// `None` for unknown family values.
pub(crate) fn to_binary_bytes(family: u8, address_be: [u8; 16]) -> Option<Vec<u8>> {
    match family {
        1 => Some(address_be[12..16].to_vec()),
        2 => Some(address_be.to_vec()),
        _ => None,
    }
}

/// Returns `Some(true)` if the INET represents a globally-routable address.
///
/// Dispatches on `family`: `1` (IPv4) takes the last 4 bytes of the
/// big-endian buffer and runs `ip::is_global_ipv4_octets`; `2` (IPv6) takes
/// all 16 and runs `ip::is_global_ipv6_octets`. Returns `None` for any
/// other family value.
pub(crate) fn is_global_bytes(family: u8, address_be: [u8; 16]) -> Option<bool> {
    match family {
        1 => {
            let mut octets = [0u8; 4];
            octets.copy_from_slice(&address_be[12..16]);
            Some(ip::is_global_ipv4_octets(octets))
        }
        2 => Some(ip::is_global_ipv6_octets(address_be)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_binary_ipv4_takes_low_4_bytes() {
        let mut address_be = [0u8; 16];
        address_be[12..16].copy_from_slice(&[192, 168, 1, 1]);
        assert_eq!(to_binary_bytes(1, address_be), Some(vec![192, 168, 1, 1]));
    }

    #[test]
    fn to_binary_ipv6_takes_all_16_bytes() {
        let address_be = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        assert_eq!(to_binary_bytes(2, address_be), Some(address_be.to_vec()));
    }

    #[test]
    fn to_binary_returns_none_for_unknown_family() {
        assert_eq!(to_binary_bytes(0, [0u8; 16]), None);
        assert_eq!(to_binary_bytes(3, [0u8; 16]), None);
    }

    #[test]
    fn is_global_ipv4_dispatches_to_octets() {
        let mut address_be = [0u8; 16];
        address_be[12..16].copy_from_slice(&[8, 8, 8, 8]);
        assert_eq!(is_global_bytes(1, address_be), Some(true));

        address_be[12..16].copy_from_slice(&[192, 168, 1, 1]);
        assert_eq!(is_global_bytes(1, address_be), Some(false));
    }

    #[test]
    fn is_global_ipv6_dispatches_to_octets() {
        let public = [
            0x2a, 0x00, 0x14, 0x50, 0x40, 0x07, 0x08, 0x0f, 0, 0, 0, 0, 0, 0, 0x20, 0x0e,
        ];
        assert_eq!(is_global_bytes(2, public), Some(true));

        let loopback = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        assert_eq!(is_global_bytes(2, loopback), Some(false));
    }

    #[test]
    fn is_global_returns_none_for_unknown_family() {
        assert_eq!(is_global_bytes(0, [0u8; 16]), None);
        assert_eq!(is_global_bytes(3, [0u8; 16]), None);
    }
}
