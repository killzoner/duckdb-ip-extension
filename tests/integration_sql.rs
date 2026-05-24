//! End-to-end SQL tests for the IP extension.
//!
//! Loads the pre-built `duckdb_ip_extension.duckdb_extension` artifact from the
//! repo root into a fresh `quack_rs::testing::InMemoryDb` and asserts that each
//! exported scalar function returns the expected result for a representative
//! set of inputs.
//!
//! Run with:
//!     cargo test --features bundled-test --test integration_sql
//!
//! Requires the extension binary to be present at `<repo>/duckdb_ip_extension.duckdb_extension`.
//! If it's stale relative to `src/`, rebuild it manually first (e.g. `make build-extension`
//! or `cargo build --release` plus the footer-append step).

#![cfg(feature = "bundled-test")]

use quack_rs::testing::InMemoryDb;

const EXTENSION_FILENAME: &str = "duckdb_ip_extension.duckdb_extension";

/// Returns the absolute path to the pre-built extension artifact, panicking
/// with a clear message if it's missing.
fn extension_path() -> String {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), EXTENSION_FILENAME);
    assert!(
        std::path::Path::new(&path).exists(),
        r"extension artifact not found at {path}. Build it first (cargo build --release + footer-append, or `make build-extension` if available)."
    );
    path
}

/// Opens a fresh `InMemoryDb` with `allow_unsigned_extensions`, relaxes
/// metadata mismatch, and `LOAD`s the extension. Each test calls this so they
/// run in isolation (no shared state between tests).
fn open_with_extension_loaded() -> InMemoryDb {
    let db = InMemoryDb::open_unsigned().expect("open in-memory db (unsigned)");
    db.execute_batch(&format!("LOAD '{}'", extension_path()))
        .expect("LOAD ip extension");
    db
}

// ── ipv4_to_binary ──────────────────────────────────────────────────────────

#[test]
fn ipv4_to_binary_canonical() {
    let db = open_with_extension_loaded();
    let bytes: Vec<u8> = db
        .query_one("SELECT ipv4_to_binary('127.0.0.1')")
        .expect("query");
    assert_eq!(bytes, vec![127, 0, 0, 1]);
}

#[test]
fn ipv4_to_binary_rejects_ipv6() {
    let db = open_with_extension_loaded();
    let bytes: Option<Vec<u8>> = db.query_one("SELECT ipv4_to_binary('::1')").expect("query");
    assert_eq!(bytes, None);
}

#[test]
fn ipv4_to_binary_rejects_garbage() {
    let db = open_with_extension_loaded();
    let bytes: Option<Vec<u8>> = db
        .query_one("SELECT ipv4_to_binary('not an ip')")
        .expect("query");
    assert_eq!(bytes, None);
}

// ── ipv6_to_binary ──────────────────────────────────────────────────────────

#[test]
fn ipv6_to_binary_loopback() {
    let db = open_with_extension_loaded();
    let bytes: Vec<u8> = db.query_one("SELECT ipv6_to_binary('::1')").expect("query");
    let mut expected = vec![0u8; 16];
    expected[15] = 1;
    assert_eq!(bytes, expected);
}

#[test]
fn ipv6_to_binary_rejects_ipv4() {
    let db = open_with_extension_loaded();
    let bytes: Option<Vec<u8>> = db
        .query_one("SELECT ipv6_to_binary('127.0.0.1')")
        .expect("query");
    assert_eq!(bytes, None);
}

// ── ip_to_binary (family-agnostic) ─────────────────────────────────────────

#[test]
fn ip_to_binary_dispatches_ipv4() {
    let db = open_with_extension_loaded();
    let bytes: Vec<u8> = db
        .query_one("SELECT ip_to_binary('192.0.2.1')")
        .expect("query");
    assert_eq!(bytes, vec![192, 0, 2, 1]);
}

#[test]
fn ip_to_binary_dispatches_ipv6() {
    let db = open_with_extension_loaded();
    let bytes: Vec<u8> = db.query_one("SELECT ip_to_binary('::1')").expect("query");
    assert_eq!(bytes.len(), 16);
    assert_eq!(bytes[15], 1);
}

#[test]
fn ip_to_binary_rejects_garbage() {
    let db = open_with_extension_loaded();
    let bytes: Option<Vec<u8>> = db
        .query_one("SELECT ip_to_binary('not an ip')")
        .expect("query");
    assert_eq!(bytes, None);
}

// ── ip_family ───────────────────────────────────────────────────────────────

#[test]
fn ip_family_ipv4() {
    let db = open_with_extension_loaded();
    let family: u8 = db
        .query_one("SELECT ip_family('127.0.0.1')")
        .expect("query");
    assert_eq!(family, 4);
}

#[test]
fn ip_family_ipv6() {
    let db = open_with_extension_loaded();
    let family: u8 = db.query_one("SELECT ip_family('::1')").expect("query");
    assert_eq!(family, 6);
}

#[test]
fn ip_family_rejects_garbage() {
    let db = open_with_extension_loaded();
    let family: Option<u8> = db
        .query_one("SELECT ip_family('not an ip')")
        .expect("query");
    assert_eq!(family, None);
}

// ── is_global_ipv6 ──────────────────────────────────────────────────────────

#[test]
fn is_global_ipv6_loopback_is_not_global() {
    let db = open_with_extension_loaded();
    let global: bool = db.query_one("SELECT is_global_ipv6('::1')").expect("query");
    assert!(!global, "::1 (loopback) must not be global");
}

#[test]
fn is_global_ipv6_documentation_range_is_not_global() {
    let db = open_with_extension_loaded();
    let global: bool = db
        .query_one("SELECT is_global_ipv6('2001:db8::1')")
        .expect("query");
    assert!(!global, "2001:db8::/32 (documentation) must not be global");
}

#[test]
fn is_global_ipv6_public_address_is_global() {
    let db = open_with_extension_loaded();
    let global: bool = db
        .query_one("SELECT is_global_ipv6('2606:4700:4700::1111')")
        .expect("query");
    assert!(global, "Cloudflare DNS must be global");
}

#[test]
fn is_global_ipv6_rejects_ipv4() {
    let db = open_with_extension_loaded();
    let global: Option<bool> = db
        .query_one("SELECT is_global_ipv6('127.0.0.1')")
        .expect("query");
    assert_eq!(global, None);
}

// ── is_global_ipv4 ──────────────────────────────────────────────────────────

#[test]
fn is_global_ipv4_public_is_global() {
    let db = open_with_extension_loaded();
    let global: bool = db
        .query_one("SELECT is_global_ipv4('8.8.8.8')")
        .expect("query");
    assert!(global);
}

#[test]
fn is_global_ipv4_private_is_not_global() {
    let db = open_with_extension_loaded();
    let global: bool = db
        .query_one("SELECT is_global_ipv4('10.0.0.1')")
        .expect("query");
    assert!(!global);
}

#[test]
fn is_global_ipv4_loopback_is_not_global() {
    let db = open_with_extension_loaded();
    let global: bool = db
        .query_one("SELECT is_global_ipv4('127.0.0.1')")
        .expect("query");
    assert!(!global);
}

#[test]
fn is_global_ipv4_rejects_ipv6() {
    let db = open_with_extension_loaded();
    let global: Option<bool> = db.query_one("SELECT is_global_ipv4('::1')").expect("query");
    assert_eq!(global, None);
}

// ── INET-input tests ────────────────────────────────────────────────────────
//
// Gated on the `inet-interop` Cargo feature (which also gates the
// `inet_to_binary` / `is_global_inet` SQL function registrations) since these
// tests use `'…'::INET` literals and need duckdb-inet loaded.

#[cfg(feature = "inet-interop")]
mod inet {
    use super::{InMemoryDb, extension_path};

    fn open_with_inet_and_extension_loaded() -> InMemoryDb {
        let db = InMemoryDb::open_unsigned().expect("open in-memory db (unsigned)");
        db.execute_batch("INSTALL inet; LOAD inet")
            .expect("INSTALL/LOAD duckdb-inet (needs network on first run)");
        db.execute_batch(&format!("LOAD '{}'", extension_path()))
            .expect("LOAD ip extension");
        db
    }

    // inet_to_binary — value assertions, not just IS NOT NULL.

    #[test]
    fn inet_to_binary_ipv4_canonical() {
        let db = open_with_inet_and_extension_loaded();
        let bytes: Vec<u8> = db
            .query_one("SELECT inet_to_binary('192.168.1.1'::INET)")
            .expect("query");
        assert_eq!(bytes, vec![192, 168, 1, 1]);
    }

    #[test]
    fn inet_to_binary_ipv6_loopback() {
        let db = open_with_inet_and_extension_loaded();
        let bytes: Vec<u8> = db
            .query_one("SELECT inet_to_binary('::1'::INET)")
            .expect("query");
        let mut expected = vec![0u8; 16];
        expected[15] = 1;
        assert_eq!(bytes, expected);
    }

    #[test]
    fn inet_to_binary_ipv6_public() {
        let db = open_with_inet_and_extension_loaded();
        let bytes: Vec<u8> = db
            .query_one("SELECT inet_to_binary('2606:4700:4700::1111'::INET)")
            .expect("query");
        assert_eq!(
            bytes,
            vec![
                0x26, 0x06, 0x47, 0x00, 0x47, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0x11, 0x11
            ]
        );
    }

    // is_global_inet

    #[test]
    fn is_global_inet_ipv4_public() {
        let db = open_with_inet_and_extension_loaded();
        let global: bool = db
            .query_one("SELECT is_global_inet('1.1.1.1'::INET)")
            .expect("query");
        assert!(global);
    }

    #[test]
    fn is_global_inet_ipv4_private() {
        let db = open_with_inet_and_extension_loaded();
        let global: bool = db
            .query_one("SELECT is_global_inet('192.168.1.1'::INET)")
            .expect("query");
        assert!(!global);
    }

    #[test]
    fn is_global_inet_ipv6_public() {
        let db = open_with_inet_and_extension_loaded();
        let global: bool = db
            .query_one("SELECT is_global_inet('2606:4700:4700::1111'::INET)")
            .expect("query");
        assert!(global);
    }

    #[test]
    fn is_global_inet_ipv6_loopback() {
        let db = open_with_inet_and_extension_loaded();
        let global: bool = db
            .query_one("SELECT is_global_inet('::1'::INET)")
            .expect("query");
        assert!(!global);
    }
}
