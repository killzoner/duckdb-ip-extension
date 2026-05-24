//! End-to-end CPU comparison of three ways to convert a string IP address
//! into its packed binary form via DuckDB, for both IPv4 and IPv6.
//!
//! All three paths in the **main** group consume a VARCHAR column and do
//! the full conversion inside the timing loop:
//!
//!   1. `_to_binary(VARCHAR)`              — direct parse (baseline).
//!   2. `inet_to_binary(VARCHAR::INET)`    — cast to INET, then FFI read
//!      via this crate's `inet-interop` feature.
//!   3. `_to_binary(host(VARCHAR::INET))`  — status quo with duckdb-inet
//!      alone: cast to INET, stringify with `host()`, then re-parse.
//!
//! A separate **reference** group times the isolated `inet_to_binary(INET)`
//! call on a pre-materialised INET column. That number is not a competitor
//! to the three above — it's the conversion cost in isolation, useful if
//! the column is already stored as INET upstream and the VARCHAR → INET
//! cast was paid elsewhere.
//!
//! Run with:
//!
//! ```sh
//! DUCKDB_DOWNLOAD_LIB=1 cargo bench --features bundled-test --bench ip_bench
//! ```
//!
//! Requires:
//!   - The built `.duckdb_extension` artifact at
//!     `<repo>/duckdb_ip_extension.duckdb_extension`, **built with
//!     `--features inet-interop`** so `inet_to_binary` is registered.
//!   - Network access on first run to `INSTALL` the `inet` community
//!     extension (cached locally afterwards).

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use quack_rs::testing::InMemoryDb;

const N: usize = 100_000;
const EXTENSION_FILENAME: &str = "duckdb_ip_extension.duckdb_extension";

fn extension_path() -> String {
    let p = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), EXTENSION_FILENAME);
    assert!(
        std::path::Path::new(&p).exists(),
        r"extension artifact not found at {p}. Rebuild with `cargo build --release` and append the footer."
    );
    p
}

fn setup_db() -> InMemoryDb {
    let db = InMemoryDb::open_unsigned().expect("open in-memory db (unsigned)");
    db.execute_batch("INSTALL inet; LOAD inet")
        .expect("INSTALL/LOAD duckdb-inet (needs network on first run)");
    db.execute_batch(&format!("LOAD '{}'", extension_path()))
        .expect("LOAD ip extension");

    // Both VARCHAR and pre-cast INET forms are materialised once at setup.
    // The main bench paths only read s_v4 / s_v6 and pay the cast cost in
    // their own timing loop; the reference group reads inet_v4 / inet_v6
    // so its number is the isolated conversion cost.
    db.execute_batch(&format!(
        r"CREATE TABLE bench AS
         WITH src AS (
           SELECT
             '10.' || ((i // 65536) % 256)::INTEGER
                  || '.' || ((i // 256) % 256)::INTEGER
                  || '.' || (i % 256)::INTEGER AS s_v4,
             'fe80::' || lower(to_hex((i % 65536)::INTEGER)) AS s_v6
           FROM generate_series(0, {}) AS t(i)
         )
         SELECT
           s_v4, s_v4::INET AS inet_v4,
           s_v6, s_v6::INET AS inet_v6
         FROM src",
        N - 1
    ))
    .expect("create bench table");
    db
}

fn bench_ipv4_from_varchar(db: &InMemoryDb, c: &mut Criterion) {
    let mut group = c.benchmark_group("ipv4_from_varchar");
    group.throughput(Throughput::Elements(N as u64));

    group.bench_function("ipv4_to_binary(VARCHAR)", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one("SELECT count(*) FROM bench WHERE ipv4_to_binary(s_v4) IS NOT NULL")
                .expect("v4 direct");
        });
    });

    group.bench_function("inet_to_binary(VARCHAR::INET)", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one(
                    "SELECT count(*) FROM bench WHERE inet_to_binary(s_v4::INET) IS NOT NULL",
                )
                .expect("v4 via inet");
        });
    });

    group.bench_function("ipv4_to_binary(host(VARCHAR::INET))", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one(
                    r"SELECT count(*) FROM bench WHERE ipv4_to_binary(host(s_v4::INET)) IS NOT NULL",
                )
                .expect("v4 via inet + host");
        });
    });

    group.finish();
}

fn bench_ipv4_from_inet_reference(db: &InMemoryDb, c: &mut Criterion) {
    let mut group = c.benchmark_group("ipv4_from_inet_reference");
    group.throughput(Throughput::Elements(N as u64));

    group.bench_function("inet_to_binary(INET)", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one("SELECT count(*) FROM bench WHERE inet_to_binary(inet_v4) IS NOT NULL")
                .expect("v4 reference");
        });
    });

    group.finish();
}

fn bench_ipv6_from_varchar(db: &InMemoryDb, c: &mut Criterion) {
    let mut group = c.benchmark_group("ipv6_from_varchar");
    group.throughput(Throughput::Elements(N as u64));

    group.bench_function("ipv6_to_binary(VARCHAR)", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one("SELECT count(*) FROM bench WHERE ipv6_to_binary(s_v6) IS NOT NULL")
                .expect("v6 direct");
        });
    });

    group.bench_function("inet_to_binary(VARCHAR::INET)", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one(
                    "SELECT count(*) FROM bench WHERE inet_to_binary(s_v6::INET) IS NOT NULL",
                )
                .expect("v6 via inet");
        });
    });

    group.bench_function("ipv6_to_binary(host(VARCHAR::INET))", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one(
                    r"SELECT count(*) FROM bench WHERE ipv6_to_binary(host(s_v6::INET)) IS NOT NULL",
                )
                .expect("v6 via inet + host");
        });
    });

    group.finish();
}

fn bench_ipv6_from_inet_reference(db: &InMemoryDb, c: &mut Criterion) {
    let mut group = c.benchmark_group("ipv6_from_inet_reference");
    group.throughput(Throughput::Elements(N as u64));

    group.bench_function("inet_to_binary(INET)", |b| {
        b.iter(|| {
            let _: i64 = db
                .query_one("SELECT count(*) FROM bench WHERE inet_to_binary(inet_v6) IS NOT NULL")
                .expect("v6 reference");
        });
    });

    group.finish();
}

fn bench(c: &mut Criterion) {
    let db = setup_db();
    bench_ipv4_from_varchar(&db, c);
    bench_ipv4_from_inet_reference(&db, c);
    bench_ipv6_from_varchar(&db, c);
    bench_ipv6_from_inet_reference(&db, c);
}

criterion_group!(benches, bench);
criterion_main!(benches);
