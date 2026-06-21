use std::cmp::Ordering;
use std::f32;
use crate::utils;

pub(crate) fn piecewise_linear_lookup(index: f32, mapping: &[f32]) -> f32 {
    let lower_value = mapping[f32::floor(index) as usize];
    let upper_value = mapping[f32::ceil(index) as usize];
    utils::lerp(lower_value, upper_value, f32::fract(index))
}

pub(crate) fn piecewise_linear_find_index(query_value: f32, mapping: &[f32]) -> f32 {
    let upper_index = match mapping
        .binary_search_by(|value| value.partial_cmp(&query_value).unwrap_or(Ordering::Less))
    {
        Ok(index) => return index as f32,
        Err(upper_index) => upper_index,
    };
    if upper_index == 0 || upper_index >= mapping.len() {
        return upper_index as f32;
    }
    let lower_index = upper_index - 1;
    let (upper_value, lower_value) = (mapping[upper_index], mapping[lower_index]);
    let t = (query_value - lower_value) / (upper_value - lower_value);
    lower_index as f32 + t
}

