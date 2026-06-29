use super::color_rgba::ColorRgba;
use super::default::default_line_width;
use super::line_segment_spec::LineSegmentSpec;
use super::point3::Point3;
use super::validate::validate_id;
use super::validate::validate_positive_f32;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineStripSpec {
    pub id: String,
    pub points: Vec<Point3>,
    #[serde(default)]
    pub color: ColorRgba,
    #[serde(default = "default_line_width")]
    pub width: f32,
}

impl LineStripSpec {
    pub fn validate(&self) -> Result<(), Scene3DError> {
        validate_id(&self.id, "line_strip.id")?;
        if self.points.len() < 2 {
            return Err(Scene3DError::EmptyData {
                field: "line_strip.points",
            });
        }
        for point in &self.points {
            point.validate("line_strip.points")?;
        }
        self.color.validate("line_strip.color")?;
        validate_positive_f32(self.width, "line_strip.width")
    }

    pub fn to_segments(&self) -> impl Iterator<Item = LineSegmentSpec> + '_ {
        self.points.windows(2).map(move |pair| LineSegmentSpec {
            from: pair[0],
            to: pair[1],
            color: self.color,
            width: self.width,
        })
    }

    pub(crate) fn hash_into(&self, h: &mut impl Hasher) {
        self.id.hash(h);
        for point in &self.points {
            point.hash_into(h);
        }
        self.color.hash_into(h);
        self.width.to_bits().hash(h);
    }
}
