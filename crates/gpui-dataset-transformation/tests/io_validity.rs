//! Masked-field round trips through every field-bearing mesh format.
//!
//! Convention (see `validity.rs`): masked entries export as NaN plus a
//! `<name>__valid` companion; readers restore the mask and drop the
//! companion. A finite placeholder under a mask must never come back as
//! a valid measurement, and a bare NaN from a foreign file must read as
//! missing data instead of failing validation.

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    read_vtk, read_vtu, read_vtu_bytes, read_xdmf, write_vtk, write_vtu, write_vtu_appended,
    write_vtu_compressed, write_xdmf,
};
use gpui_mesh::{Association, DataSet, Field};

fn masked_dataset() -> DataSet {
    let mut pressure = Field::scalar("pressure", vec![1.0, 99.0, 3.0, 4.0]);
    pressure.valid = Some(vec![true, false, true, true].into());
    let mut velocity = Field::vector("velocity", vec![[1.0; 3], [2.0; 3], [9.0; 3], [4.0; 3]]);
    velocity.valid = Some(vec![true, true, false, true].into());
    let mut cell_id = Field::scalar("cell_id", vec![7.0, 42.0]).on_cells();
    cell_id.valid = Some(vec![false, true].into());
    DataSet::surface(
        "masked",
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[[0, 1, 2], [0, 2, 3]],
    )
    .with_field(pressure)
    .with_field(velocity)
    .with_field(cell_id)
}

fn assert_masks_restored(parsed: &DataSet) {
    assert_eq!(parsed.validate(), Ok(()));
    let pressure = parsed.field(Association::Point, "pressure").unwrap();
    assert_eq!(
        pressure.valid.as_deref(),
        Some([true, false, true, true].as_slice())
    );
    assert!(
        pressure.as_scalar().unwrap()[1].is_nan(),
        "placeholder 99 must not survive"
    );
    assert_eq!(pressure.as_scalar().unwrap()[0], 1.0);
    assert!(!pressure.is_valid(1) && pressure.is_valid(0));
    let velocity = parsed.field(Association::Point, "velocity").unwrap();
    assert_eq!(
        velocity.valid.as_deref(),
        Some([true, true, false, true].as_slice())
    );
    assert!(velocity.as_vector().unwrap()[2].iter().all(|c| c.is_nan()));
    let cell_id = parsed.field(Association::Cell, "cell_id").unwrap();
    assert_eq!(cell_id.valid.as_deref(), Some([false, true].as_slice()));
    for field in parsed.point_data.iter().chain(&parsed.cell_data) {
        assert!(
            !field.name.ends_with("__valid"),
            "companion {} must be folded into the mask",
            field.name
        );
    }
}

#[test]
fn legacy_vtk_round_trips_masks() {
    let dataset = masked_dataset();
    let text = write_vtk(&dataset);
    assert!(
        text.contains("pressure__valid"),
        "companion must be written:\n{text}"
    );
    assert!(
        !text.contains("\n99"),
        "masked placeholder must not be exported:\n{text}"
    );
    assert_masks_restored(&read_vtk(&text).unwrap());
}

#[test]
fn vtu_variants_round_trip_masks() {
    let dataset = masked_dataset();
    assert_masks_restored(&read_vtu(&write_vtu(&dataset)).unwrap());
    assert_masks_restored(&read_vtu(&write_vtu_compressed(&dataset)).unwrap());
    assert_masks_restored(&read_vtu_bytes(&write_vtu_appended(&dataset)).unwrap());
}

#[test]
fn xdmf_round_trips_masks() {
    let dataset = masked_dataset();
    let bundle = write_xdmf(&dataset);
    assert!(bundle.xml.contains("pressure__valid"));
    let parsed = read_xdmf(&bundle.xml, |name| {
        bundle
            .binaries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, bytes)| bytes.clone())
    })
    .unwrap();
    assert_masks_restored(&parsed);
}

#[test]
fn bare_nan_reads_as_masked_missing_data() {
    // A foreign/legacy file with a bare NaN and no companion: previously
    // this failed validation on re-read.
    let text = "\
# vtk DataFile Version 3.0
bare
ASCII
DATASET UNSTRUCTURED_GRID
POINTS 3 double
0 0 0
1 0 0
0 1 0
CELLS 1 4
3 0 1 2
CELL_TYPES 1
5
POINT_DATA 3
SCALARS s double 1
LOOKUP_TABLE default
1
NaN
3
";
    let parsed = read_vtk(text).unwrap();
    let field = parsed.field(Association::Point, "s").unwrap();
    assert_eq!(field.valid.as_deref(), Some([true, false, true].as_slice()));
    assert_eq!(parsed.validate(), Ok(()));
}

#[test]
fn unmasked_fields_gain_no_companion_or_mask() {
    let dataset = DataSet::surface(
        "plain",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
    )
    .with_field(Field::scalar("s", vec![1.0, 2.0, 3.0]));
    let text = write_vtk(&dataset);
    assert!(!text.contains("__valid"));
    let parsed = read_vtk(&text).unwrap();
    assert_eq!(parsed.field(Association::Point, "s").unwrap().valid, None);
}
