//! Compact primitive selections sharing canonical topology and scalar storage.

// Rust guideline compliant 2026-02-21

#[cfg(feature = "gpu")]
use crate::SceneError;
use gpui_mesh::{Association, CellType, DataSet, Field};

#[derive(Default)]
pub(crate) struct CellIds {
    small: Vec<u32>,
    wide: Vec<usize>,
}

impl CellIds {
    fn push(&mut self, cell: usize) {
        if let Ok(cell) = u32::try_from(cell) {
            self.small.push(cell);
        } else {
            self.wide.push(cell);
        }
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.small
            .iter()
            .map(|&i| i as usize)
            .chain(self.wide.iter().copied())
    }
    pub(crate) fn len(&self) -> usize {
        self.small.len() + self.wide.len()
    }
    fn compact(&mut self) {
        self.small.shrink_to_fit();
        self.wide.shrink_to_fit();
    }
    fn byte_len(&self) -> u64 {
        (self.small.len() * size_of::<u32>() + self.wide.len() * size_of::<usize>()) as u64
    }
}

#[derive(Default)]
pub(crate) struct PrimitiveCells {
    pub(crate) lines: CellIds,
    pub(crate) points: CellIds,
}

impl PrimitiveCells {
    pub(crate) fn prepare(dataset: &DataSet, field: Option<&Field>) -> Self {
        let mut cells = Self::default();
        for (cell, &kind) in dataset.cells.types.iter().enumerate() {
            if !matches!(kind, CellType::Line | CellType::Vertex) {
                continue;
            }
            let nodes = dataset.cells.cell(cell).expect("validated primitive cell");
            let valid = field.is_none_or(|field| match field.association {
                Association::Point => nodes.iter().all(|&node| field.is_valid(node as usize)),
                Association::Cell => field.is_valid(cell),
            });
            if valid {
                match kind {
                    CellType::Line => cells.lines.push(cell),
                    CellType::Vertex => cells.points.push(cell),
                    _ => unreachable!(),
                }
            }
        }
        cells.lines.compact();
        cells.points.compact();
        cells
    }
    pub(crate) fn byte_len(&self) -> u64 {
        self.lines.byte_len() + self.points.byte_len()
    }
}

pub(crate) fn scalar(field: Option<&Field>, cell: usize, node: u32) -> f64 {
    field.map_or(0.0, |field| {
        field.as_scalar().expect("validated scalar field")[match field.association {
            Association::Point => node as usize,
            Association::Cell => cell,
        }]
    })
}

#[cfg(feature = "gpu")]
pub(crate) fn gpu_vertex(
    dataset: &DataSet,
    field: Option<&Field>,
    cell: usize,
    node: u32,
    origin: [f64; 3],
) -> Result<[f32; 4], SceneError> {
    let position = dataset.positions[node as usize];
    let mut vertex = [0.0; 4];
    for axis in 0..3 {
        vertex[axis] = gpu_value(
            position[axis] - origin[axis],
            "primitive position cannot be represented by f32",
        )?;
    }
    vertex[3] = gpu_value(
        scalar(field, cell, node),
        "primitive scalar cannot be represented by f32",
    )?;
    Ok(vertex)
}

#[cfg(feature = "gpu")]
fn gpu_value(value: f64, message: &'static str) -> Result<f32, SceneError> {
    let converted = value as f32;
    if !converted.is_finite() || (value != 0.0 && (converted == 0.0 || converted.is_subnormal())) {
        return Err(SceneError::UnsupportedPrecision(message));
    }
    Ok(converted)
}

#[cfg(feature = "gpui-gpu")]
pub(crate) fn gpu_supported(actor: &crate::Actor) -> Result<bool, SceneError> {
    if !actor
        .dataset
        .cells
        .types
        .iter()
        .any(|kind| matches!(kind, CellType::Line | CellType::Vertex))
    {
        return Ok(true);
    }
    actor.dataset.validate()?;
    let field = crate::retained::selected_field(actor)?.map(|(_, field)| field);
    let origin = actor.dataset.bounds().map_or([0.0; 3], |b| b.center());
    for (cell, &kind) in actor.dataset.cells.types.iter().enumerate() {
        if !matches!(kind, CellType::Line | CellType::Vertex) {
            continue;
        }
        let nodes = actor.dataset.cells.cell(cell).expect("validated primitive");
        let valid = field.is_none_or(|field| match field.association {
            Association::Cell => field.is_valid(cell),
            Association::Point => nodes.iter().all(|&node| field.is_valid(node as usize)),
        });
        if !valid {
            continue;
        }
        for &node in nodes {
            match gpu_vertex(&actor.dataset, field, cell, node, origin) {
                Ok(_) => {}
                Err(SceneError::UnsupportedPrecision(_)) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_ids_preserve_cell_order_without_truncation() {
        let mut ids = CellIds::default();
        ids.push(0);
        ids.push(u32::MAX as usize);
        #[cfg(target_pointer_width = "64")]
        ids.push(u32::MAX as usize + 1);
        let got = ids.iter().collect::<Vec<_>>();
        assert_eq!(got[0], 0);
        assert_eq!(got[1], u32::MAX as usize);
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(got[2], u32::MAX as usize + 1);
            assert_eq!(ids.byte_len(), 16);
        }
    }
}
