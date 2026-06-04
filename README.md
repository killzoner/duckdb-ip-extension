[![continuous-integration](https://github.com/killzoner/duckdb-ip-extension/actions/workflows/continuous-integration.yml/badge.svg)](https://github.com/killzoner/duckdb-ip-extension/actions/workflows/continuous-integration.yml)

# duckdb-ip-extension

> **DuckDB IP parsing extension**

## About

SQL functions that convert IPv4 / IPv6 addresses to their packed binary form (4 / 16 bytes), starting from either `VARCHAR` or the `INET` type from [`duckdb-inet`](https://github.com/duckdb/duckdb-inet), with helpers for address-family detection and global-routability checks (see the [Functions](#functions) table).

`duckdb-inet` only exposes addresses as strings (via `host()`), so today the only way to get the bytes is to stringify then re-parse. This extension reads the bytes directly out of the INET struct — half the storage of strings, no per-row parsing.

Built in pure Rust on [`quack-rs`](https://github.com/tomtom215/quack-rs) and DuckDB's stable C Extension API.

## Usage

```bash
git submodule update --init
make build-extension
```

VARCHAR-input functions:

```bash
duckdb -unsigned -c "
  LOAD '$PWD/duckdb_ip_extension.duckdb_extension';
  SELECT ipv4_to_binary('192.168.1.1'), ipv6_to_binary('2001:db8::1');
"
```

INET-input functions (load `duckdb-inet` first so the `INET` type is registered):

```bash
duckdb -unsigned -c "
  INSTALL inet; LOAD inet;
  LOAD '$PWD/duckdb_ip_extension.duckdb_extension';
  SELECT inet_to_binary('::1'::INET), is_global_inet('192.168.1.1'::INET);
"
```

## Functions

| SQL function | Signature | Returns |
|---|---|---|
| `ipv4_to_binary(addr)` | `VARCHAR → BLOB` | 4-byte big-endian IPv4 |
| `ipv6_to_binary(addr)` | `VARCHAR → BLOB` | 16-byte big-endian IPv6 |
| `ip_to_binary(addr)` | `VARCHAR → BLOB` | 4 or 16 bytes depending on family (pair with `ip_family`) |
| `ip_family(addr)` | `VARCHAR → UTINYINT` | `4` for IPv4, `6` for IPv6 |
| `is_global_ipv4(addr)` | `VARCHAR → BOOLEAN` | true if globally-routable unicast IPv4 |
| `is_global_ipv6(addr)` | `VARCHAR → BOOLEAN` | true if globally-routable IPv6 |

All functions return `NULL` for unparseable input or a family mismatch (e.g. IPv6 passed to `ipv4_to_binary`). The exact reserved ranges excluded by the `is_global_*` checks are documented in the doc comments in [`src/ip/mod.rs`](./src/ip/mod.rs).

### INET interop

The `inet-interop` Cargo feature (enabled by default) additionally registers:

| SQL function | Signature | Returns |
|---|---|---|
| `inet_to_binary(addr)` | `INET → BLOB` | 4 or 16 bytes depending on the INET's family |
| `is_global_inet(addr)` | `INET → BOOLEAN` | `is_global_ipv4` / `is_global_ipv6` semantics by family |

Both return `NULL` for an invalid INET family. Details in [`src/inet/mod.rs`](./src/inet/mod.rs).

Requires `duckdb-inet` to be loaded **before** this extension at runtime. Opt out with `cargo build --no-default-features`.

## Building from source

Artifacts are attached to GitHub Releases for `linux_amd64` and `linux_arm64`. To build locally:

```bash
git submodule update --init
make build-extension  # Produces duckdb_ip_extension.duckdb_extension
```

This compiles with `cargo build --release` and appends the required DuckDB metadata footer.
**Overrides:** `PROFILE=dev`, `EXTENSION_VERSION`, `DUCKDB_PLATFORM`, `OUTPUT_FILE`. See `make help`.

### Running tests

```bash
cargo test      # Unit + property tests
cargo bench     # Throughput benchmarks
cargo +nightly miri test --lib  # UB checks
```

**Integration Tests:** The `integration_sql` suite requires a linkable `libduckdb`. Both options below use pre-built binaries and do not require compiling DuckDB from C++ source:

```bash
# Option A: Automatic download (cached in target/)
DUCKDB_DOWNLOAD_LIB=1 cargo test --features bundled-test

# Option B: Manual setup
make install_lib_duckdb    # Unpacks into ./local_lib/
DUCKDB_LIB_DIR=$PWD/local_lib cargo test --features bundled-test
```

Rebuild the artifact via `make build-extension` if it's stale relative to `src/`.

## See also

- [`BENCHMARK.md`](./BENCHMARK.md) — benchmark methodology and results.
- [`TODO.md`](./TODO.md)

## Development

Parts of this project were developed with the assistance of AI tools.

## Related projects

- [`duckdb/duckdb-inet`](https://github.com/duckdb/duckdb-inet) — core `INET` type and arithmetic; no binary extraction.
- [`tomtom215/quack-rs`](https://github.com/tomtom215/quack-rs) — the Rust SDK this extension is built on.
