use super::types::ConformanceFinding;
use std::borrow::Cow;

pub(super) fn finite_positive(
    findings: &mut Vec<ConformanceFinding>,
    id: &'static str,
    value: f32,
    message: &'static str,
) {
    if !value.is_finite() || value <= 0.0 {
        findings.push(ConformanceFinding {
            id,
            message: Cow::Borrowed(message),
        });
    }
}

pub(super) fn finite_non_negative(
    findings: &mut Vec<ConformanceFinding>,
    id: &'static str,
    value: f32,
    message: &'static str,
) {
    if !value.is_finite() || value < 0.0 {
        findings.push(ConformanceFinding {
            id,
            message: Cow::Borrowed(message),
        });
    }
}
