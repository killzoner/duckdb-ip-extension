# Benchmark — getting packed IP bytes out of DuckDB

Compares three apples-to-apples paths for converting a **VARCHAR-sourced** IP address into its packed binary form (4 bytes for IPv4, 16 bytes for IPv6), plus a separate reference number for the isolated `inet_to_binary(INET)` cost when the column is already stored as INET upstream.

## What's compared

The **main comparison** is three paths that each consume a VARCHAR column and do the full conversion inside the timing loop:

| Path | SQL |
|---|---|
| direct | `ipv{4,6}_to_binary(s)` |
| via INET (FFI-fast route, this crate) | `inet_to_binary(s::INET)` |
| via INET + `host()` roundtrip (status quo with `duckdb-inet` alone) | `ipv{4,6}_to_binary(host(s::INET))` |

The **reference group** times `inet_to_binary(INET)` on a pre-materialised INET column. It's not a competitor to the three above — it's the conversion-only cost, useful if a pipeline upstream already produces INET data and the VARCHAR → INET cast was paid elsewhere.

## Results

### IPv4 — from VARCHAR (main comparison)

| Path | Time | Throughput | Relative |
|---|---|---|---|
| `ipv4_to_binary(VARCHAR)` | 3.13 ms | 31.9 M elem/s | **1.00×** |
| `inet_to_binary(VARCHAR::INET)` | 8.74 ms | 11.4 M elem/s | 2.79× slower |
| `ipv4_to_binary(host(VARCHAR::INET))` | 17.24 ms | 5.80 M elem/s | 5.51× slower |

### IPv4 — from INET (reference, conversion-only)

| Path | Time | Throughput |
|---|---|---|
| `inet_to_binary(INET)` | 1.97 ms | 50.8 M elem/s |

### IPv6 — from VARCHAR (main comparison)

| Path | Time | Throughput | Relative |
|---|---|---|---|
| `inet_to_binary(VARCHAR::INET)` | 5.75 ms | 17.4 M elem/s | **1.00×** |
| `ipv6_to_binary(VARCHAR)` | 6.34 ms | 15.8 M elem/s | 1.10× slower |
| `ipv6_to_binary(host(VARCHAR::INET))` | 25.14 ms | 3.98 M elem/s | 4.37× slower |

### IPv6 — from INET (reference, conversion-only)

| Path | Time | Throughput |
|---|---|---|
| `inet_to_binary(INET)` | 2.98 ms | 33.6 M elem/s |

## Reproducing

```bash
# 1. Build the extension.
make build-extension

# 2. Run the bench (downloads libduckdb on first invocation).
DUCKDB_DOWNLOAD_LIB=1 cargo bench --features bundled-test --bench ip_bench
```
