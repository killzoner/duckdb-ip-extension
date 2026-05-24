//! Pure-Rust IP address parsing logic.
//!
//! No FFI, no `unsafe`, no dependencies beyond `std`. Exercised directly by
//! `cargo test --lib` (and `cargo miri test --lib --all-features` for UB
//! checks — Miri can't traverse the FFI shims).
//!
//! The FFI callbacks bridging these helpers to DuckDB vectors live in
//! `scalar`.

pub(crate) mod scalar;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

fn parse_ip(addr: &str) -> Option<IpAddr> {
    if addr.is_empty() {
        return None;
    }
    IpAddr::from_str(addr).ok()
}

/// Parses an IPv6 address string into its 16-byte packed representation.
///
/// Returns `None` for IPv4 input, empty strings, or any unparseable value.
/// Accepts the full IPv6 string grammar (canonical, `::` shorthand, IPv4-mapped).
///
/// Calls `Ipv6Addr::from_str` directly rather than going through `IpAddr` —
/// std's IPv6-only parser doesn't accept bare `127.0.0.1` etc., so non-v6
/// inputs fail-fast at the parser without needing a post-match family check.
pub(crate) fn ipv6_to_bytes(addr: &str) -> Option<[u8; 16]> {
    if addr.is_empty() {
        return None;
    }
    Ipv6Addr::from_str(addr).ok().map(|v6| v6.octets())
}

/// Parses an IPv4 address string into its 4-byte packed representation.
///
/// Returns `None` for IPv6 input, empty strings, or any unparseable value.
pub(crate) fn ipv4_to_bytes(addr: &str) -> Option<[u8; 4]> {
    if addr.is_empty() {
        return None;
    }
    Ipv4Addr::from_str(addr).ok().map(|v4| v4.octets())
}

/// Parses any IP address string into its packed representation: 4 bytes for
/// IPv4, 16 bytes for IPv6. Returns `None` for unparseable or empty input.
pub(crate) fn ip_to_bytes(addr: &str) -> Option<Vec<u8>> {
    match parse_ip(addr)? {
        IpAddr::V6(v6) => Some(v6.octets().to_vec()),
        IpAddr::V4(v4) => Some(v4.octets().to_vec()),
    }
}

/// Returns the address family of a parseable IP string: `4` for IPv4, `6`
/// for IPv6. Returns `None` for unparseable or empty input.
///
/// Pairs with [`ip_to_bytes`] when callers need to disambiguate the BLOB
/// length they got back.
pub(crate) fn ip_family(addr: &str) -> Option<u8> {
    match parse_ip(addr)? {
        IpAddr::V6(_) => Some(6),
        IpAddr::V4(_) => Some(4),
    }
}

/// Returns `true` if the 16-byte big-endian IPv6 representation is a
/// globally-routable address.
///
/// "Globally routable" excludes the unspecified address, loopback, multicast,
/// link-local unicast (`fe80::/10`), unique local (`fc00::/7`), IETF
/// documentation prefix (`2001:db8::/32`), and IPv4-mapped IPv6 addresses
/// (`::ffff:0:0/96`).
pub(crate) fn is_global_ipv6_octets(octets: [u8; 16]) -> bool {
    let v6 = Ipv6Addr::from(octets);

    if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() {
        return false;
    }

    // Link-local unicast: fe80::/10
    if octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80 {
        return false;
    }
    // Unique local: fc00::/7
    if (octets[0] & 0xfe) == 0xfc {
        return false;
    }
    // Documentation: 2001:db8::/32
    if octets[0] == 0x20 && octets[1] == 0x01 && octets[2] == 0x0d && octets[3] == 0xb8 {
        return false;
    }
    // IPv4-mapped: ::ffff:0:0/96
    if octets[..10] == [0; 10] && octets[10..12] == [0xff, 0xff] {
        return false;
    }

    true
}

/// Returns `Some(true)` if the input is a globally-routable IPv6 address.
///
/// See [`is_global_ipv6_octets`] for the exclusion criteria.
///
/// Returns `None` for IPv4 input or anything unparseable.
pub(crate) fn is_global_ipv6(addr: &str) -> Option<bool> {
    if addr.is_empty() {
        return None;
    }
    let v6 = Ipv6Addr::from_str(addr).ok()?;
    Some(is_global_ipv6_octets(v6.octets()))
}

/// Returns `true` if the 4-byte IPv4 representation is a globally-routable
/// unicast address.
///
/// Excludes: unspecified (`0.0.0.0/8`), private (RFC 1918:
/// `10/8`, `172.16/12`, `192.168/16`), CGNAT (`100.64.0.0/10`),
/// loopback (`127/8`), link-local (`169.254/16`), IETF protocol assignments
/// (`192.0.0.0/24`), documentation (`192.0.2/24`, `198.51.100/24`,
/// `203.0.113/24`), benchmarking (`198.18.0.0/15`), multicast
/// (`224.0.0.0/4`), reserved / former Class E + broadcast (`240.0.0.0/4`).
pub(crate) fn is_global_ipv4_octets(octets: [u8; 4]) -> bool {
    let v4 = Ipv4Addr::from(octets);

    if v4.is_unspecified()
        || v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_documentation()
        || v4.is_multicast()
    {
        return false;
    }
    // 0.0.0.0/8 ("this network" — broader than is_unspecified, which only matches 0.0.0.0)
    if octets[0] == 0 {
        return false;
    }
    // 100.64.0.0/10 (CGNAT)
    if octets[0] == 100 && (octets[1] & 0xc0) == 0x40 {
        return false;
    }
    // 192.0.0.0/24 (IETF protocol assignments)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return false;
    }
    // 198.18.0.0/15 (benchmarking)
    if octets[0] == 198 && (octets[1] & 0xfe) == 0x12 {
        return false;
    }
    // 240.0.0.0/4 (reserved / former Class E; covers 255.255.255.255 broadcast too)
    if (octets[0] & 0xf0) == 0xf0 {
        return false;
    }

    true
}

/// Returns `Some(true)` if the input is a globally-routable unicast IPv4
/// address. See [`is_global_ipv4_octets`] for the exclusion criteria.
///
/// Returns `None` for IPv6 input or anything unparseable.
pub(crate) fn is_global_ipv4(addr: &str) -> Option<bool> {
    if addr.is_empty() {
        return None;
    }
    let v4 = Ipv4Addr::from_str(addr).ok()?;
    Some(is_global_ipv4_octets(v4.octets()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ipv6_to_bytes ────────────────────────────────────────────────────────

    #[test]
    fn ipv6_parses_canonical_form() {
        let bytes = ipv6_to_bytes("2001:db8:0:0:0:0:0:1").unwrap();
        assert_eq!(bytes[0], 0x20);
        assert_eq!(bytes[1], 0x01);
        assert_eq!(bytes[2], 0x0d);
        assert_eq!(bytes[3], 0xb8);
        assert_eq!(bytes[15], 0x01);
    }

    #[test]
    fn ipv6_parses_shorthand_form() {
        assert_eq!(
            ipv6_to_bytes("2001:db8::1"),
            ipv6_to_bytes("2001:db8:0:0:0:0:0:1")
        );
    }

    #[test]
    fn ipv6_parses_ipv4_mapped() {
        let bytes = ipv6_to_bytes("::ffff:192.0.2.1").unwrap();
        assert_eq!(&bytes[10..12], &[0xff, 0xff]);
        assert_eq!(&bytes[12..16], &[192, 0, 2, 1]);
    }

    #[test]
    fn ipv6_parses_loopback() {
        let bytes = ipv6_to_bytes("::1").unwrap();
        let mut expected = [0u8; 16];
        expected[15] = 1;
        assert_eq!(bytes, expected);
    }

    #[test]
    fn ipv6_rejects_ipv4() {
        assert_eq!(ipv6_to_bytes("192.168.1.1"), None);
    }

    #[test]
    fn ipv6_rejects_empty() {
        assert_eq!(ipv6_to_bytes(""), None);
    }

    #[test]
    fn ipv6_rejects_garbage() {
        assert_eq!(ipv6_to_bytes("not-an-ip"), None);
        assert_eq!(ipv6_to_bytes("2001:db8:::1"), None);
        assert_eq!(ipv6_to_bytes(":::"), None);
    }

    // ── ipv4_to_bytes ────────────────────────────────────────────────────────

    #[test]
    fn ipv4_parses_basic() {
        assert_eq!(ipv4_to_bytes("192.168.1.1"), Some([192, 168, 1, 1]));
        assert_eq!(ipv4_to_bytes("0.0.0.0"), Some([0, 0, 0, 0]));
        assert_eq!(ipv4_to_bytes("255.255.255.255"), Some([255, 255, 255, 255]));
    }

    #[test]
    fn ipv4_rejects_ipv6() {
        assert_eq!(ipv4_to_bytes("::1"), None);
        assert_eq!(ipv4_to_bytes("2001:db8::1"), None);
    }

    #[test]
    fn ipv4_rejects_empty() {
        assert_eq!(ipv4_to_bytes(""), None);
    }

    #[test]
    fn ipv4_rejects_garbage() {
        assert_eq!(ipv4_to_bytes("256.0.0.0"), None);
        assert_eq!(ipv4_to_bytes("not-an-ip"), None);
        assert_eq!(ipv4_to_bytes("192.168.1"), None);
    }

    // ── is_global_ipv6 ───────────────────────────────────────────────────────

    #[test]
    fn global_ipv6_accepts_public_unicast() {
        assert_eq!(is_global_ipv6("2a00:1450:4007:80f::200e"), Some(true));
        assert_eq!(is_global_ipv6("2600::1"), Some(true));
    }

    #[test]
    fn global_ipv6_rejects_loopback() {
        assert_eq!(is_global_ipv6("::1"), Some(false));
    }

    #[test]
    fn global_ipv6_rejects_unspecified() {
        assert_eq!(is_global_ipv6("::"), Some(false));
    }

    #[test]
    fn global_ipv6_rejects_link_local() {
        assert_eq!(is_global_ipv6("fe80::1"), Some(false));
        assert_eq!(is_global_ipv6("febf::1"), Some(false));
    }

    #[test]
    fn global_ipv6_rejects_unique_local() {
        assert_eq!(is_global_ipv6("fc00::1"), Some(false));
        assert_eq!(is_global_ipv6("fd00::1"), Some(false));
    }

    #[test]
    fn global_ipv6_rejects_documentation() {
        assert_eq!(is_global_ipv6("2001:db8::1"), Some(false));
        assert_eq!(is_global_ipv6("2001:db8:abcd::1"), Some(false));
    }

    #[test]
    fn global_ipv6_rejects_ipv4_mapped() {
        assert_eq!(is_global_ipv6("::ffff:8.8.8.8"), Some(false));
    }

    #[test]
    fn global_ipv6_rejects_multicast() {
        assert_eq!(is_global_ipv6("ff02::1"), Some(false));
    }

    #[test]
    fn global_ipv6_returns_none_for_ipv4() {
        assert_eq!(is_global_ipv6("192.168.1.1"), None);
        assert_eq!(is_global_ipv6("8.8.8.8"), None);
    }

    #[test]
    fn global_ipv6_returns_none_for_garbage() {
        assert_eq!(is_global_ipv6(""), None);
        assert_eq!(is_global_ipv6("not-an-ip"), None);
    }

    // ── is_global_ipv4 ──────────────────────────────────────────────────────

    #[test]
    fn global_ipv4_accepts_public_unicast() {
        assert_eq!(is_global_ipv4("8.8.8.8"), Some(true));
        assert_eq!(is_global_ipv4("1.1.1.1"), Some(true));
        assert_eq!(is_global_ipv4("93.184.216.34"), Some(true)); // example.com
    }

    #[test]
    fn global_ipv4_rejects_this_network() {
        // 0.0.0.0/8
        assert_eq!(is_global_ipv4("0.0.0.0"), Some(false));
        assert_eq!(is_global_ipv4("0.1.2.3"), Some(false));
    }

    #[test]
    fn global_ipv4_rejects_private_rfc1918() {
        assert_eq!(is_global_ipv4("10.0.0.1"), Some(false));
        assert_eq!(is_global_ipv4("172.16.0.1"), Some(false));
        assert_eq!(is_global_ipv4("172.31.255.254"), Some(false));
        assert_eq!(is_global_ipv4("192.168.1.1"), Some(false));
    }

    #[test]
    fn global_ipv4_rejects_cgnat() {
        // 100.64.0.0/10
        assert_eq!(is_global_ipv4("100.64.0.0"), Some(false));
        assert_eq!(is_global_ipv4("100.127.255.255"), Some(false));
        // Just outside the CGNAT range is global.
        assert_eq!(is_global_ipv4("100.128.0.0"), Some(true));
    }

    #[test]
    fn global_ipv4_rejects_loopback_and_link_local() {
        assert_eq!(is_global_ipv4("127.0.0.1"), Some(false));
        assert_eq!(is_global_ipv4("169.254.1.1"), Some(false));
    }

    #[test]
    fn global_ipv4_rejects_protocol_and_documentation() {
        // 192.0.0.0/24 (IETF protocol assignments)
        assert_eq!(is_global_ipv4("192.0.0.1"), Some(false));
        // Documentation
        assert_eq!(is_global_ipv4("192.0.2.1"), Some(false));
        assert_eq!(is_global_ipv4("198.51.100.1"), Some(false));
        assert_eq!(is_global_ipv4("203.0.113.1"), Some(false));
    }

    #[test]
    fn global_ipv4_rejects_benchmarking() {
        // 198.18.0.0/15
        assert_eq!(is_global_ipv4("198.18.0.0"), Some(false));
        assert_eq!(is_global_ipv4("198.19.255.255"), Some(false));
    }

    #[test]
    fn global_ipv4_rejects_multicast_reserved_broadcast() {
        assert_eq!(is_global_ipv4("224.0.0.1"), Some(false));
        assert_eq!(is_global_ipv4("240.0.0.1"), Some(false));
        assert_eq!(is_global_ipv4("255.255.255.255"), Some(false));
    }

    #[test]
    fn global_ipv4_returns_none_for_ipv6() {
        assert_eq!(is_global_ipv4("::1"), None);
        assert_eq!(is_global_ipv4("2001:db8::1"), None);
    }

    #[test]
    fn global_ipv4_returns_none_for_garbage() {
        assert_eq!(is_global_ipv4(""), None);
        assert_eq!(is_global_ipv4("not-an-ip"), None);
        assert_eq!(is_global_ipv4("256.0.0.0"), None);
    }

    // ── ip_to_bytes / ip_family (family-agnostic helpers) ───────────────────

    #[test]
    fn ip_to_bytes_dispatches_on_family() {
        assert_eq!(
            ip_to_bytes("::1").unwrap(),
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
        );
        assert_eq!(ip_to_bytes("192.168.1.1").unwrap(), vec![192, 168, 1, 1]);
        assert_eq!(ip_to_bytes("2001:db8::1").unwrap().len(), 16);
    }

    #[test]
    fn ip_to_bytes_returns_none_on_garbage() {
        assert_eq!(ip_to_bytes(""), None);
        assert_eq!(ip_to_bytes("not-an-ip"), None);
        assert_eq!(ip_to_bytes("256.0.0.0"), None);
    }

    #[test]
    fn ip_family_dispatches() {
        assert_eq!(ip_family("::1"), Some(6));
        assert_eq!(ip_family("2001:db8::1"), Some(6));
        assert_eq!(ip_family("::ffff:192.0.2.1"), Some(6));
        assert_eq!(ip_family("192.168.1.1"), Some(4));
        assert_eq!(ip_family("0.0.0.0"), Some(4));
        assert_eq!(ip_family(""), None);
        assert_eq!(ip_family("not-an-ip"), None);
    }

    #[test]
    fn ip_to_bytes_and_family_agree() {
        for input in ["::1", "2001:db8::1", "192.168.1.1", "0.0.0.0"] {
            let bytes = ip_to_bytes(input).unwrap();
            let family = ip_family(input).unwrap();
            match family {
                4 => assert_eq!(bytes.len(), 4, "ipv4 should be 4 bytes for {input:?}"),
                6 => assert_eq!(bytes.len(), 16, "ipv6 should be 16 bytes for {input:?}"),
                _ => panic!("unexpected family {family} for {input:?}"),
            }
        }
    }

    // ── Known-good byte vectors ─────────────────────────────────────────────
    //
    // Hand-curated regression vectors. Each entry pins the exact byte output
    // for an input string, locking the parser's behaviour against accidental
    // changes (e.g. a future stdlib `IpAddr::from_str` change that breaks one
    // form of shorthand).

    #[rustfmt::skip]
    const IPV6_VECTORS: &[(&str, Option<[u8; 16]>)] = &[
        ("::",                                         Some([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
        ("::1",                                        Some([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])),
        ("::ffff:0.0.0.0",                             Some([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0, 0, 0, 0])),
        ("::ffff:192.0.2.1",                           Some([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 192, 0, 2, 1])),
        ("::ffff:255.255.255.255",                     Some([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])),
        ("2001:db8::1",                                Some([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])),
        ("2001:0db8:0000:0000:0000:0000:0000:0001",    Some([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])),
        ("fe80::1",                                    Some([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])),
        ("fc00::1",                                    Some([0xfc, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])),
        ("ff02::1",                                    Some([0xff, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])),
        ("2a00:1450:4007:80f::200e",                   Some([0x2a, 0, 0x14, 0x50, 0x40, 0x07, 0x08, 0x0f, 0, 0, 0, 0, 0, 0, 0x20, 0x0e])),
        ("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",    Some([0xff; 16])),
        ("192.168.1.1",                                None),
        ("8.8.8.8",                                    None),
        ("not-an-ip",                                  None),
        ("",                                           None),
        ("2001:db8:::1",                               None),
        (":::",                                        None),
        ("fe80::g",                                    None),
    ];

    #[rustfmt::skip]
    const IPV4_VECTORS: &[(&str, Option<[u8; 4]>)] = &[
        ("0.0.0.0",         Some([0, 0, 0, 0])),
        ("1.2.3.4",         Some([1, 2, 3, 4])),
        ("10.0.0.1",        Some([10, 0, 0, 1])),
        ("127.0.0.1",       Some([127, 0, 0, 1])),
        ("192.168.1.1",     Some([192, 168, 1, 1])),
        ("255.255.255.255", Some([255, 255, 255, 255])),
        ("::1",             None),
        ("2001:db8::1",     None),
        ("256.0.0.0",       None),
        ("192.168.1",       None),
        ("not-an-ip",       None),
        ("",                None),
    ];

    #[test]
    fn ipv6_known_good_vectors() {
        for (input, expected) in IPV6_VECTORS {
            assert_eq!(ipv6_to_bytes(input), *expected, "ipv6 {input:?}");
        }
    }

    #[test]
    fn ipv4_known_good_vectors() {
        for (input, expected) in IPV4_VECTORS {
            assert_eq!(ipv4_to_bytes(input), *expected, "ipv4 {input:?}");
        }
    }

    // ── proptest round-trips ─────────────────────────────────────────────────
    //
    // Skipped under Miri: proptest's runner uses threading, filesystem access
    // for regression persistence, and `getrandom` — none of which Miri can
    // execute without disabling isolation. Deterministic tests above still run.
    #[cfg(not(miri))]
    proptest::proptest! {
        #[test]
        fn ipv6_roundtrip(octets in proptest::array::uniform16(0u8..=255)) {
            let s = Ipv6Addr::from(octets).to_string();
            proptest::prop_assert_eq!(ipv6_to_bytes(&s), Some(octets));
        }

        #[test]
        fn ipv4_roundtrip(octets in proptest::array::uniform4(0u8..=255)) {
            let s = std::net::Ipv4Addr::from(octets).to_string();
            proptest::prop_assert_eq!(ipv4_to_bytes(&s), Some(octets));
        }

        #[test]
        fn ipv6_and_ipv4_dont_cross(octets in proptest::array::uniform4(0u8..=255)) {
            let v4 = std::net::Ipv4Addr::from(octets).to_string();
            proptest::prop_assert_eq!(ipv6_to_bytes(&v4), None);
            let v6 = Ipv6Addr::from([0u8; 16]).to_string();
            proptest::prop_assert_eq!(ipv4_to_bytes(&v6), None);
        }

        #[test]
        fn ip_to_bytes_matches_per_family_ipv6(octets in proptest::array::uniform16(0u8..=255)) {
            let s = Ipv6Addr::from(octets).to_string();
            proptest::prop_assert_eq!(ip_to_bytes(&s), Some(octets.to_vec()));
            proptest::prop_assert_eq!(ip_family(&s), Some(6));
        }

        #[test]
        fn ip_to_bytes_matches_per_family_ipv4(octets in proptest::array::uniform4(0u8..=255)) {
            let s = std::net::Ipv4Addr::from(octets).to_string();
            proptest::prop_assert_eq!(ip_to_bytes(&s), Some(octets.to_vec()));
            proptest::prop_assert_eq!(ip_family(&s), Some(4));
        }
    }
}
