//! FFI scalar callbacks for the INET-input functions.

use libduckdb_sys::{
    duckdb_data_chunk, duckdb_data_chunk_get_size, duckdb_data_chunk_get_vector,
    duckdb_function_info, duckdb_vector,
};
use quack_rs::vector::{StructReader, VectorReader, VectorWriter};

use super::{is_global_bytes, to_binary_bytes};

/// Reads the `(ip_type, address)` pair from row `row` of an INET-typed STRUCT.
///
/// duckdb-inet stores the address with the high bit flipped (XOR `i128::MIN`)
/// so the HUGEINT signed ordering matches the natural unsigned ordering of
/// IPv6. We undo the XOR here and return the canonical big-endian bytes.
///
/// # Safety
///
/// `struct_reader` must be valid for `row` and the struct must have at least 2
/// fields with the first two being UTinyInt + HugeInt.
unsafe fn read_inet_row(struct_reader: &StructReader, row: usize) -> (u8, [u8; 16]) {
    let family = unsafe { struct_reader.read_u8(row, 0) };
    let addr = unsafe { struct_reader.read_i128(row, 1) };
    (family, (addr ^ i128::MIN).to_be_bytes())
}

/// Scalar callback for `inet_to_binary(INET) -> BLOB`.
///
/// # Safety
///
/// Invoked by DuckDB. Caller guarantees input column 0 holds an INET-typed
/// STRUCT vector with exactly 3 fields, output holds BLOB sized to the chunk
/// row count.
pub(crate) unsafe extern "C" fn to_binary_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let row_count = unsafe { duckdb_data_chunk_get_size(input) } as usize;
    let struct_vec = unsafe { duckdb_data_chunk_get_vector(input, 0) };
    let outer = unsafe { VectorReader::new(input, 0) };
    let struct_reader = unsafe { StructReader::new(struct_vec, 3, row_count) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..row_count {
        if !unsafe { outer.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let (family, address_be) = unsafe { read_inet_row(&struct_reader, row) };
        match to_binary_bytes(family, address_be) {
            Some(bytes) => unsafe { writer.write_blob(row, &bytes) },
            None => unsafe { writer.set_null(row) },
        }
    }
}

/// Scalar callback for `is_global_inet(INET) -> BOOLEAN`.
///
/// # Safety
///
/// Same contract as `to_binary_scalar`, but `output` holds BOOLEAN.
pub(crate) unsafe extern "C" fn is_global_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let row_count = unsafe { duckdb_data_chunk_get_size(input) } as usize;
    let struct_vec = unsafe { duckdb_data_chunk_get_vector(input, 0) };
    let outer = unsafe { VectorReader::new(input, 0) };
    let struct_reader = unsafe { StructReader::new(struct_vec, 3, row_count) };
    let mut writer = unsafe { VectorWriter::new(output) };
    for row in 0..row_count {
        if !unsafe { outer.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let (family, address_be) = unsafe { read_inet_row(&struct_reader, row) };
        match is_global_bytes(family, address_be) {
            Some(b) => unsafe { writer.write_bool(row, b) },
            None => unsafe { writer.set_null(row) },
        }
    }
}
