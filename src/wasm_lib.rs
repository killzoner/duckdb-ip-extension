// Staticlib entry for the wasm build (via `[[example]]` with `crate-type = ["staticlib"]`).
// Shares all logic with `src/lib.rs` via `include!("extension.rs")` — see that file
// for why the split exists.
include!("extension.rs");
