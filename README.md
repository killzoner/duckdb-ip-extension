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

| SQL function | Signature | Behavior |
|---|---|---|
| `ipv4_to_binary(addr)` | `VARCHAR → BLOB` | 4-byte big-endian IPv4 representation. Returns `NULL` for IPv6 or unparseable input. |
| `ipv6_to_binary(addr)` | `VARCHAR → BLOB` | 16-byte big-endian IPv6 representation. Returns `NULL` for IPv4 addresses or unparseable input. |
| `ip_to_binary(addr)` | `VARCHAR → BLOB` | Family-agnostic: 4 bytes for IPv4, 16 bytes for IPv6. Returns `NULL` for unparseable input. Pair with `ip_family` to disambiguate. |
| `ip_family(addr)` | `VARCHAR → UTINYINT` | `4` for IPv4, `6` for IPv6. Returns `NULL` for unparseable input. |
| `is_global_ipv4(addr)` | `VARCHAR → BOOLEAN` | True if the input is a globally-routable unicast IPv4 address (excludes `0.0.0.0/8` "this network", RFC 1918 private, `100.64.0.0/10` CGNAT, `127/8` loopback, `169.254/16` link-local, `192.0.0.0/24` IETF protocol assignments, `192.0.2/24` + `198.51.100/24` + `203.0.113/24` documentation, `198.18.0.0/15` benchmarking, `224.0.0.0/4` multicast, `240.0.0.0/4` reserved/Class E, `255.255.255.255` broadcast). Returns `NULL` for non-IPv4 or unparseable input. |
| `is_global_ipv6(addr)` | `VARCHAR → BOOLEAN` | True if the input is a globally-routable IPv6 address (excludes `::` unspecified, `::1` loopback, `ff00::/8` multicast, `fe80::/10` link-local unicast, `fc00::/7` unique local, `2001:db8::/32` documentation, `::ffff:0:0/96` IPv4-mapped). Returns `NULL` for non-IPv6 or unparseable input. |

All functions accept the full IPv6 string grammar (canonical form, `::` shorthand, IPv4-mapped `::ffff:a.b.c.d`).

### INET interop

The `inet-interop` Cargo feature is **enabled by default** and additionally registers:

| SQL function | Signature | Behavior |
|---|---|---|
| `inet_to_binary(addr)` | `INET → BLOB` | 4 bytes for IPv4-family INET, 16 bytes for IPv6-family. Returns `NULL` for invalid family. |
| `is_global_inet(addr)` | `INET → BOOLEAN` | Dispatches on the INET's family: applies `is_global_ipv4` or `is_global_ipv6` semantics. Returns `NULL` for invalid family. |

Requires `duckdb-inet` to be loaded **before** this extension at runtime so the INET type is registered when `inet_to_binary` registers itself. Opt out with `cargo build --no-default-features` if you don't need the INET-input variants.

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
