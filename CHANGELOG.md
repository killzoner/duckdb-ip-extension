# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-10-28
### Changed
- Name the release files `duckdb_ip_extension.<platform>.duckdb_extension`, so that DuckDB loads them as is

### Fixed
- Reject a DuckDB version with a non-numeric part in the version pin check

## [0.2.0] - 2026-10-26
### Changed
- Build against DuckDB 2.0.0 (`libduckdb-sys` 1.20000.0) and quack-rs 0.19

## [0.1.0] - 2026-06-01
### Added
- Initial version
