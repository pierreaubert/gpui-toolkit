#![no_main]

//! Legacy VTK reader fuzz target: arbitrary text must never panic;
//! successful parses must validate (including restored validity masks).

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let bounded = &data[..data.len().min(256 * 1024)];
    let Ok(text) = std::str::from_utf8(bounded) else {
        return;
    };
    if let Ok(dataset) = gpui_dataset_transformation::read_vtk(text) {
        assert!(dataset.validate().is_ok());
    }
});
