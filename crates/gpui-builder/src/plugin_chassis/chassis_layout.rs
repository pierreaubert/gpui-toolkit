use super::solved_chassis::SolvedChassis;
use super::types::FooterSpec;
use super::types::HeaderSpec;
use super::types::SectionSpec;
use super::types::SolvedSection;
use std::cell::RefCell;

/// Complete chassis layout for one plugin.
#[derive(Debug, Clone)]
pub struct ChassisLayout {
    pub header: HeaderSpec,
    pub sections: Vec<SectionSpec>,
    pub footer: Option<FooterSpec>,
}

// Thread-local scratch buffers for the temporary vectors used by
// [`ChassisLayout::solve`]. The function is not recursive, so a single buffer
// per type is sufficient; buffers are cleared and reused each call.
thread_local! {
    static CHASSIS_VISIBLE_SCRATCH: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    static CHASSIS_WIDTHS_SCRATCH: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static CHASSIS_INDICES_SCRATCH: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

/// Run `f` with cleared, reusable scratch vectors sized for `n` sections.
fn with_chassis_scratch<R>(
    n: usize,
    f: impl FnOnce(&mut Vec<bool>, &mut Vec<f32>, &mut Vec<usize>) -> R,
) -> R {
    CHASSIS_VISIBLE_SCRATCH.with(|visible| {
        CHASSIS_WIDTHS_SCRATCH.with(|widths| {
            CHASSIS_INDICES_SCRATCH.with(|indices| {
                let mut visible = visible.borrow_mut();
                let mut widths = widths.borrow_mut();
                let mut indices = indices.borrow_mut();
                visible.clear();
                visible.resize(n, true);
                widths.clear();
                widths.resize(n, 0.0);
                indices.clear();
                f(&mut visible, &mut widths, &mut indices)
            })
        })
    })
}

impl ChassisLayout {
    /// Build a new chassis with the given header and sections (no footer).
    pub fn new(header: HeaderSpec, sections: Vec<SectionSpec>) -> Self {
        Self {
            header,
            sections,
            footer: None,
        }
    }

    /// Attach a footer strip.
    pub fn with_footer(mut self, footer: FooterSpec) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Look up a section by id.
    pub fn section(&self, id: &str) -> Option<&SectionSpec> {
        self.sections.iter().find(|s| s.id == id)
    }

    /// Solve the section row for the given available width.
    ///
    /// Returns one [`SolvedSection`] per input section, in input order. Each
    /// solved section has a final `width` (≥ 0) and a `visible` flag. When
    /// there is not enough space, sections collapse in ascending `priority`
    /// order — lowest first.
    ///
    /// The algorithm:
    /// 1. Sum `min_width` across all sections.
    /// 2. While the sum exceeds `available_width`, drop the section with
    ///    the lowest `priority` (mark `visible = false`, contributes 0
    ///    width) and repeat.
    /// 3. Distribute remaining space proportionally to `preferred_width -
    ///    min_width` of visible sections (clamped at preferred_width).
    /// 4. Any leftover space flexes the highest-priority visible section.
    pub fn solve(&self, available_width: f32) -> SolvedChassis {
        let n = self.sections.len();

        if n == 0 {
            return SolvedChassis {
                sections: vec![],
                total_width: 0.0,
            };
        }

        with_chassis_scratch(n, |visible, widths, visible_indices| {
            // Helper: normalize priority — NaN is treated as never-collapse (1.0).
            let effective_priority = |p: f32| if p.is_nan() { 1.0 } else { p };

            // Step 1+2: collapse lowest-priority sections until min-sum fits.
            loop {
                let min_sum: f32 = self
                    .sections
                    .iter()
                    .zip(visible.iter())
                    .filter(|(_, v)| **v)
                    .map(|(s, _)| s.min_width)
                    .sum();

                if min_sum <= available_width {
                    break;
                }

                // Find the visible section with the lowest priority. If multiple
                // tie, drop the rightmost (later in input order) — this keeps
                // earlier "primary" sections visible longer.
                let drop_idx = (0..n).filter(|i| visible[*i]).min_by(|a, b| {
                    let pa = effective_priority(self.sections[*a].priority);
                    let pb = effective_priority(self.sections[*b].priority);
                    pa.partial_cmp(&pb)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(b.cmp(a)) // tie-break: later index drops first
                });

                match drop_idx {
                    Some(i) => visible[i] = false,
                    None => break, // nothing left to drop
                }

                // Defensive: priority 1.0 sections should never be dropped. If
                // every visible section has priority >= 1.0 we exit the loop —
                // the chassis simply doesn't fit. We accept clipping rather
                // than dropping a never-collapse section.
                if visible
                    .iter()
                    .zip(self.sections.iter())
                    .filter(|(v, _)| **v)
                    .all(|(_, s)| effective_priority(s.priority) >= 1.0)
                {
                    break;
                }
            }

            // Step 3+4: distribute width across visible sections.
            visible_indices.extend((0..n).filter(|i| visible[*i]));
            let min_sum: f32 = visible_indices
                .iter()
                .map(|&i| self.sections[i].min_width)
                .sum();
            let preferred_extra: f32 = visible_indices
                .iter()
                .map(|&i| (self.sections[i].preferred_width - self.sections[i].min_width).max(0.0))
                .sum();

            let extra_space = (available_width - min_sum).max(0.0);
            let mut clipped = false;

            if preferred_extra <= 0.0 || visible_indices.is_empty() {
                // No section wants more than its min — give everyone min, leftover
                // accumulates on the highest-priority section if there is one.
                for &i in &*visible_indices {
                    widths[i] = self.sections[i].min_width;
                }
            } else {
                let factor = (extra_space / preferred_extra).min(1.0);
                for &i in &*visible_indices {
                    let span =
                        (self.sections[i].preferred_width - self.sections[i].min_width).max(0.0);
                    widths[i] = self.sections[i].min_width + span * factor;
                }

                // Distribute any leftover (when extra_space > preferred_extra) to
                // the highest-priority visible section.
                let allocated: f32 = widths.iter().sum();
                let leftover = available_width - allocated;
                if leftover > 0.0
                    && let Some(&i) = visible_indices.iter().max_by(|a, b| {
                        let pa = effective_priority(self.sections[**a].priority);
                        let pb = effective_priority(self.sections[**b].priority);
                        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
                    })
                {
                    widths[i] += leftover;
                }
            }

            // If min_sum still exceeds available_width, clip proportionally.
            let final_min_sum: f32 = visible_indices.iter().map(|&i| widths[i]).sum();
            if final_min_sum > available_width && !visible_indices.is_empty() {
                clipped = true;
                let scale = available_width / final_min_sum;
                for &i in &*visible_indices {
                    widths[i] *= scale;
                }
            }

            let total_width: f32 = visible_indices.iter().map(|&i| widths[i]).sum();

            let solved_sections: Vec<SolvedSection> = (0..n)
                .map(|i| SolvedSection {
                    id: self.sections[i].id.clone(),
                    width: widths[i],
                    visible: visible[i],
                })
                .collect();

            SolvedChassis {
                sections: solved_sections,
                total_width: if clipped {
                    available_width
                } else {
                    total_width
                },
            }
        })
    }
}
