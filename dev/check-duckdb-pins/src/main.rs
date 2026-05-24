//! Verifies that `LIB_DUCKDB_VERSION` (declared in the project Makefile) is
//! reflected in `.gitmodules` (submodule branch) and `Cargo.lock` (resolved
//! `libduckdb-sys` crate version).
//!
//! Invoked from the repo root, e.g. via `make check-duckdb-pins`.

use std::process::{Command, ExitCode};

const SUBMODULE_KEY: &str = "submodule.extension-ci-tools.branch";

fn main() -> ExitCode {
    let Some(duckdb) = std::env::args().nth(1) else {
        eprintln!("usage: check-duckdb-pins <duckdb-version>  (e.g. 1.5.3)");
        return ExitCode::FAILURE;
    };
    let Some(expected_sys) = expected_sys_minor(&duckdb) else {
        eprintln!("expected version like 1.5.3, got {duckdb:?}");
        return ExitCode::FAILURE;
    };

    let lock = std::fs::read_to_string("Cargo.lock").expect("read Cargo.lock");

    let mut ok = true;
    ok &= check_submodule_branch(&duckdb);
    ok &= check_libduckdb_sys(&lock, &expected_sys);

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Maps a DuckDB `1.5.X` version to the corresponding `libduckdb-sys` minor
/// version string (e.g. `1.5.3` → `1.10503`). Returns `None` for malformed
/// input. The crate's numbering scheme changed at the 1.5.0 release; revisit
/// when DuckDB moves past 1.5.
fn expected_sys_minor(duckdb: &str) -> Option<String> {
    let parts: Vec<u32> = duckdb.split('.').filter_map(|p| p.parse().ok()).collect();
    let [_, minor, patch] = parts[..] else {
        return None;
    };
    Some(format!("1.10{minor}{patch:02}"))
}

fn check_submodule_branch(duckdb: &str) -> bool {
    let want = format!("v{duckdb}");
    let out = Command::new("git")
        .args(["config", "-f", ".gitmodules", SUBMODULE_KEY])
        .output()
        .expect("invoke git config");
    let got = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if got != want {
        eprintln!("FAIL: .gitmodules {SUBMODULE_KEY} = {got:?}, expected {want:?}");
        eprintln!("  fix: git config -f .gitmodules {SUBMODULE_KEY} {want}");
        return false;
    }
    true
}

fn check_libduckdb_sys(lock: &str, expected_minor: &str) -> bool {
    let got = libduckdb_sys_version(lock).expect("libduckdb-sys missing from Cargo.lock");
    if !got.starts_with(&format!("{expected_minor}.")) {
        eprintln!("FAIL: Cargo.lock libduckdb-sys = {got:?}, expected {expected_minor}.x");
        return false;
    }
    true
}

/// Extracts the resolved `libduckdb-sys` version from a Cargo.lock TOML text.
fn libduckdb_sys_version(lock: &str) -> Option<String> {
    let lock: toml::Value = toml::from_str(lock).ok()?;
    let pkgs = lock.get("package")?.as_array()?;
    pkgs.iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("libduckdb-sys"))
        .and_then(|p| p.get("version").and_then(|v| v.as_str()))
        .map(|s| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_duckdb_to_libduckdb_sys_minor() {
        assert_eq!(expected_sys_minor("1.5.3").as_deref(), Some("1.10503"));
        assert_eq!(expected_sys_minor("1.5.10").as_deref(), Some("1.10510"));
    }

    #[test]
    fn rejects_malformed_version() {
        assert_eq!(expected_sys_minor("v1.5.3"), None);
        assert_eq!(expected_sys_minor("1.5"), None);
        assert_eq!(expected_sys_minor("not-a-version"), None);
    }

    const LOCK_FIXTURE: &str = r#"
[[package]]
name = "duckdb-ip-extension"
version = "0.1.0"

[[package]]
name = "libduckdb-sys"
version = "1.10503.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;

    #[test]
    fn finds_libduckdb_sys_version_in_lock() {
        assert_eq!(
            libduckdb_sys_version(LOCK_FIXTURE).as_deref(),
            Some("1.10503.1"),
        );
    }

    #[test]
    fn returns_none_when_libduckdb_sys_absent() {
        let lock = r#"
[[package]]
name = "only-other-crate"
version = "0.0.0"
"#;
        assert_eq!(libduckdb_sys_version(lock), None);
    }

    #[test]
    fn check_libduckdb_sys_accepts_matching_minor() {
        assert!(check_libduckdb_sys(LOCK_FIXTURE, "1.10503"));
    }

    #[test]
    fn check_libduckdb_sys_rejects_mismatched_minor() {
        // Cargo.lock has 1.10503.1, expected 1.10504 — must fail.
        assert!(!check_libduckdb_sys(LOCK_FIXTURE, "1.10504"));
    }
}
