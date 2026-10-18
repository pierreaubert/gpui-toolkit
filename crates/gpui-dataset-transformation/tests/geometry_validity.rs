// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    GlyphKind, glyphs, solid_arrows, tensor_glyphs, warp_by_scalar, warp_by_vector,
};
use gpui_mesh::{DataSet, Field, FieldData};

fn surface() -> DataSet {
    DataSet::surface(
        "triangle",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
    )
}

#[test]
fn warps_keep_masked_positions_and_preserve_source_validity() {
    for missing in [99.0, f64::NAN] {
        let mut vector = Field::vector("v", vec![[0.0, 0.0, 1.0], [missing; 3], [0.0, 0.0, 2.0]]);
        vector.valid = Some(vec![true, false, true].into());
        let mut scalar = Field::scalar("s", vec![1.0, missing, 2.0]);
        scalar.valid = vector.valid.clone();
        let dataset = surface().with_field(vector).with_field(scalar);
        for output in [
            warp_by_vector(&dataset, "v", 2.0).unwrap(),
            warp_by_scalar(&dataset, "s", 2.0).unwrap(),
        ] {
            output.validate().unwrap();
            assert_eq!(
                output.positions.as_ref(),
                &[[0.0, 0.0, 2.0], [1.0, 0.0, 0.0], [0.0, 1.0, 4.0]]
            );
            for (output, source) in output.point_data.iter().zip(&dataset.point_data) {
                assert!(std::sync::Arc::ptr_eq(
                    output.valid.as_ref().unwrap(),
                    source.valid.as_ref().unwrap()
                ));
            }
        }
        for output in [
            warp_by_vector(&dataset, "v", 0.0).unwrap(),
            warp_by_scalar(&dataset, "s", 0.0).unwrap(),
        ] {
            assert!(std::sync::Arc::ptr_eq(
                &output.positions,
                &dataset.positions
            ));
        }
        let mut missing = dataset;
        for field in &mut missing.point_data {
            field.valid = Some(vec![false; 3].into());
        }
        for output in [
            warp_by_vector(&missing, "v", 2.0).unwrap(),
            warp_by_scalar(&missing, "s", 2.0).unwrap(),
        ] {
            assert!(std::sync::Arc::ptr_eq(
                &output.positions,
                &missing.positions
            ));
        }
    }
}

#[test]
fn warps_validate_parameters_lengths_and_computed_geometry() {
    let dataset = surface()
        .with_field(Field::vector("v", vec![[0.0, 0.0, 1.0]; 3]))
        .with_field(Field::scalar("s", vec![1.0; 3]));
    for scale in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        warp_by_vector(&dataset, "v", scale).unwrap_err();
        warp_by_scalar(&dataset, "s", scale).unwrap_err();
    }
    let mut malformed = dataset.clone();
    malformed.point_data[0].data = FieldData::Vector(vec![[0.0; 3]].into());
    warp_by_vector(&malformed, "v", 1.0).unwrap_err();
    warp_by_scalar(&malformed, "s", 1.0).unwrap_err();
    let mut overflow = dataset.clone();
    overflow.point_data[0].data = FieldData::Vector(vec![[0.0, 0.0, f64::MAX]; 3].into());
    overflow.point_data[1].data = FieldData::Scalar(vec![f64::MAX; 3].into());
    warp_by_vector(&overflow, "v", 2.0).unwrap_err();
    warp_by_scalar(&overflow, "s", 2.0).unwrap_err();
    // A finite FMA result survives an intermediate product larger than MAX.
    let wide = DataSet::surface("wide", vec![[-f64::MAX, 0.0, 0.0]], &[])
        .with_field(Field::vector("v", vec![[f64::MAX, 0.0, 0.0]]));
    let output = warp_by_vector(&wide, "v", 2.0).unwrap();
    output.validate().unwrap();
    assert_eq!(output.positions[0][0], f64::MAX);
    let mut tiny = dataset;
    tiny.positions = vec![[0.0; 3], [1e-100, 0.0, 0.0], [0.0, 1e-100, 0.0]].into();
    let output = warp_by_scalar(&tiny, "s", 1.0).unwrap();
    output.validate().unwrap();
    assert_eq!(output.positions[0][2], 1.0);
}

#[test]
fn glyphs_omit_masked_samples_retain_units_and_center_lines() {
    for missing in [99.0, f64::NAN] {
        let mut vector = Field::vector("v", vec![[2.0, 0.0, 0.0], [missing; 3], [0.0, 2.0, 0.0]]);
        vector.valid = Some(vec![true, false, true].into());
        vector.unit = Some("m/s".into());
        let mut tensor = Field::tensor(
            "t",
            vec![
                [1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
                [missing; 6],
                [4.0, 1.0, 1.0, 0.0, 0.0, 0.0],
            ],
        );
        tensor.valid = vector.valid.clone();
        tensor.unit = Some("Pa".into());
        let dataset = surface().with_field(vector).with_field(tensor);
        let line = glyphs(&dataset, "v", 1.0, 0, GlyphKind::Line).unwrap();
        line.validate().unwrap();
        assert_eq!(line.cell_count(), 2);
        assert_eq!(&line.positions[..2], &[[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
        let arrow = glyphs(&dataset, "v", 1.0, 1, GlyphKind::Arrow).unwrap();
        arrow.validate().unwrap();
        assert_eq!(arrow.cell_count(), 6);
        let solid = solid_arrows(&dataset, "v", 1.0, 1, 0.05).unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.point_count(), 38);
        assert_eq!(solid.point_data[0].unit.as_deref(), Some("m/s"));
        let tensor = tensor_glyphs(&dataset, "t", 1.0, 1).unwrap();
        tensor.validate().unwrap();
        assert_eq!(tensor.point_count(), 52);
        assert_eq!(tensor.point_data[0].unit.as_deref(), Some("Pa"));
        assert_eq!(
            solid_arrows(&dataset, "v", 1.0, 3, 0.05)
                .unwrap()
                .point_count(),
            19
        );
        let mut missing = dataset;
        for field in &mut missing.point_data {
            field.valid = Some(vec![false; 3].into());
        }
        for empty in [
            glyphs(&missing, "v", 1.0, 1, GlyphKind::Arrow).unwrap(),
            solid_arrows(&missing, "v", 1.0, 1, 0.05).unwrap(),
            tensor_glyphs(&missing, "t", 1.0, 1).unwrap(),
        ] {
            empty.validate().unwrap();
            assert_eq!(empty.point_count(), 0);
            assert_eq!(empty.cell_count(), 0);
        }
    }
}

#[test]
fn glyphs_validate_input_and_keep_representable_extreme_vectors() {
    for value in [1e-200, 1e200] {
        let dataset = DataSet::surface("norm", vec![[0.0; 3]], &[])
            .with_field(Field::vector("v", vec![[value, 0.0, 0.0]]));
        let output = solid_arrows(&dataset, "v", 1.0 / value, 1, 0.05).unwrap();
        output.validate().unwrap();
        assert_eq!(output.point_count(), 19);
        assert_eq!(output.positions[18], [1.0, 0.0, 0.0]);
        let line = glyphs(&dataset, "v", 1.0 / value, 1, GlyphKind::Line).unwrap();
        line.validate().unwrap();
        assert_eq!(line.point_count(), 2);
    }
    let dataset = surface()
        .with_field(Field::vector("v", vec![[1.0; 3]; 3]))
        .with_field(Field::tensor("t", vec![[1.0; 6]; 3]));
    for scale in [f64::NAN, f64::INFINITY] {
        glyphs(&dataset, "v", scale, 1, GlyphKind::Arrow).unwrap_err();
        solid_arrows(&dataset, "v", scale, 1, 0.05).unwrap_err();
        solid_arrows(&dataset, "v", 1.0, 1, scale).unwrap_err();
        tensor_glyphs(&dataset, "t", scale, 1).unwrap_err();
    }
    let mut malformed = dataset;
    malformed.point_data[0].data = FieldData::Vector(vec![[0.0; 3]].into());
    glyphs(&malformed, "v", 1.0, 1, GlyphKind::Arrow).unwrap_err();
    solid_arrows(&malformed, "v", 1.0, 1, 0.05).unwrap_err();
    tensor_glyphs(&malformed, "t", 1.0, 1).unwrap_err();
}

#[test]
fn coupled_tensor_glyphs_have_correct_axes_and_outward_winding_for_both_scale_signs() {
    let dataset = DataSet::surface("coupled", vec![[0.0; 3]], &[])
        .with_field(Field::tensor("t", vec![[2.0, 2.0, 4.0, 1.0, 0.0, 0.0]]));
    for scale in [1.0, -1.0] {
        let output = tensor_glyphs(&dataset, "t", scale, 1).unwrap();
        output.validate().unwrap();
        let extent = output
            .positions
            .iter()
            .map(|p| ((p[0] + p[1]) / 2.0_f64.sqrt()).abs())
            .fold(0.0, f64::max);
        assert!((extent - 3.0_f64.sqrt()).abs() < 1e-12);
        let mut edges = std::collections::BTreeMap::<(u32, u32), (usize, i32)>::new();
        let mut volume = 0.0;
        for cell in 0..output.cell_count() {
            let ids = output.cells.cell(cell).unwrap();
            for [u, v] in [[ids[0], ids[1]], [ids[1], ids[2]], [ids[2], ids[0]]] {
                let edge = edges.entry((u.min(v), u.max(v))).or_default();
                edge.0 += 1;
                edge.1 += if u < v { 1 } else { -1 };
            }
            let [a, b, c] = [
                output.positions[ids[0] as usize],
                output.positions[ids[1] as usize],
                output.positions[ids[2] as usize],
            ];
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            volume += a.iter().zip(cross).map(|(a, b)| a * b).sum::<f64>() / 6.0;
        }
        assert!(volume > 0.0);
        assert!(edges.values().all(|edge| *edge == (2, 0)));
    }
}
