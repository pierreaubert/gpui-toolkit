#![no_main]

//! VTU reader fuzz target: arbitrary bytes must never panic, hang
//! unboundedly, or allocate past the configured read budgets. Valid
//! parses must also validate as datasets.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Bound one case so the fuzzer explores structure, not payload size.
    let bounded = &data[..data.len().min(256 * 1024)];
    if let Ok(dataset) = gpui_dataset_transformation::read_vtu_bytes(bounded) {
        assert!(dataset.validate().is_ok());
    }
    // Tight budgets exercise the typed BudgetExceeded paths.
    let limits = gpui_dataset_transformation::VtuReadLimits {
        max_input_bytes: 4096,
        max_array_bytes: 1024,
        max_decoded_bytes: 8192,
        max_arrays: 16,
        max_blocks: 64,
    };
    let _ = gpui_dataset_transformation::read_vtu_bytes_with_limits(bounded, limits);
});
