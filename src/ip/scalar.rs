//! FFI scalar callbacks for the VARCHAR-input IP functions.
//!
//! Each shim is invoked by DuckDB with a vectorized chunk; it reads one row
//! at a time, delegates to the pure-Rust logic in [`super`], and writes the
//! result (or NULL) to the output vector.

use libduckdb_sys::{duckdb_data_chunk, duckdb_function_info, duckdb_vector};
use quack_rs::vector::{VectorReader, VectorWriter};

use super::{ip_family, ip_to_bytes, ipv4_to_bytes, ipv6_to_bytes, is_global_ipv4, is_global_ipv6};

/// Scalar callback for `ipv6_to_binary(VARCHAR) -> BLOB`.
///
/// # Safety
///
/// Invoked by DuckDB. Caller guarantees `input`/`output` are valid for the
/// duration of the call, column 0 of `input` holds VARCHAR, and `output`
/// holds BLOB sized to match `reader.row_count()`.
pub(crate) unsafe extern "C" fn ipv6_to_binary_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let reader = unsafe { VectorReader::new(input, 0) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..reader.row_count() {
        if !unsafe { reader.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let s = unsafe { reader.read_str(row) };
        match ipv6_to_bytes(s) {
            Some(bytes) => unsafe { writer.write_blob(row, &bytes) },
            None => unsafe { writer.set_null(row) },
        }
    }
}

/// Scalar callback for `ipv4_to_binary(VARCHAR) -> BLOB`.
///
/// # Safety
///
/// Same contract as `ipv6_to_binary_scalar`.
pub(crate) unsafe extern "C" fn ipv4_to_binary_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let reader = unsafe { VectorReader::new(input, 0) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..reader.row_count() {
        if !unsafe { reader.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let s = unsafe { reader.read_str(row) };
        match ipv4_to_bytes(s) {
            Some(bytes) => unsafe { writer.write_blob(row, &bytes) },
            None => unsafe { writer.set_null(row) },
        }
    }
}

/// Scalar callback for `is_global_ipv6(VARCHAR) -> BOOLEAN`.
///
/// # Safety
///
/// Same contract as `ipv6_to_binary_scalar`, but `output` holds BOOLEAN.
pub(crate) unsafe extern "C" fn is_global_ipv6_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let reader = unsafe { VectorReader::new(input, 0) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..reader.row_count() {
        if !unsafe { reader.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let s = unsafe { reader.read_str(row) };
        match is_global_ipv6(s) {
            Some(b) => unsafe { writer.write_bool(row, b) },
            None => unsafe { writer.set_null(row) },
        }
    }
}

/// Scalar callback for `is_global_ipv4(VARCHAR) -> BOOLEAN`.
///
/// # Safety
///
/// Same contract as `ipv6_to_binary_scalar`, but `output` holds BOOLEAN.
pub(crate) unsafe extern "C" fn is_global_ipv4_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let reader = unsafe { VectorReader::new(input, 0) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..reader.row_count() {
        if !unsafe { reader.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let s = unsafe { reader.read_str(row) };
        match is_global_ipv4(s) {
            Some(b) => unsafe { writer.write_bool(row, b) },
            None => unsafe { writer.set_null(row) },
        }
    }
}

/// Scalar callback for `ip_to_binary(VARCHAR) -> BLOB`.
///
/// # Safety
///
/// Same contract as `ipv6_to_binary_scalar`.
pub(crate) unsafe extern "C" fn ip_to_binary_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let reader = unsafe { VectorReader::new(input, 0) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..reader.row_count() {
        if !unsafe { reader.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let s = unsafe { reader.read_str(row) };
        match ip_to_bytes(s) {
            Some(bytes) => unsafe { writer.write_blob(row, &bytes) },
            None => unsafe { writer.set_null(row) },
        }
    }
}

/// Scalar callback for `ip_family(VARCHAR) -> UTINYINT`.
///
/// # Safety
///
/// Same contract as `ipv6_to_binary_scalar`, but `output` holds UTINYINT.
pub(crate) unsafe extern "C" fn ip_family_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let reader = unsafe { VectorReader::new(input, 0) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..reader.row_count() {
        if !unsafe { reader.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let s = unsafe { reader.read_str(row) };
        match ip_family(s) {
            Some(family) => unsafe { writer.write_u8(row, family) },
            None => unsafe { writer.set_null(row) },
        }
    }
}
