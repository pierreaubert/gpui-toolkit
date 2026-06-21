/// Simplified bidi metadata helper, ported from chenglou/pretext which was
/// forked from pdf.js via Sebastian's text-layout. Classifies characters into
/// bidi types, computes embedding levels, and maps them onto prepared segments.
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
enum BidiType {
    L,
    R,
    AL,
    AN,
    EN,
    ES,
    ET,
    CS,
    ON,
    BN,
    B,
    S,
    WS,
    NSM,
}

use BidiType::*;

#[rustfmt::skip]
static BASE_TYPES: [BidiType; 256] = [
    BN, BN, BN, BN, BN, BN, BN, BN, BN, S,  B,  S,  WS,
    B,  BN, BN, BN, BN, BN, BN, BN, BN, BN, BN, BN, BN,
    BN, BN, B,  B,  B,  S,  WS, ON, ON, ET, ET, ET, ON,
    ON, ON, ON, ON, ON, CS, ON, CS, ON, EN, EN, EN,
    EN, EN, EN, EN, EN, EN, EN, ON, ON, ON, ON, ON,
    ON, ON, L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,
    L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  ON, ON,
    ON, ON, ON, ON, L,  L,  L,  L,  L,  L,  L,  L,  L,  L,
    L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,
    L,  ON, ON, ON, ON, BN, BN, BN, BN, BN, BN, B,  BN,
    BN, BN, BN, BN, BN, BN, BN, BN, BN, BN, BN, BN,
    BN, BN, BN, BN, BN, BN, BN, BN, BN, BN, BN, BN,
    BN, CS, ON, ET, ET, ET, ET, ON, ON, ON, ON, L,  ON,
    ON, ON, ON, ON, ET, ET, EN, EN, ON, L,  ON, ON, ON,
    EN, L,  ON, ON, ON, ON, ON, L,  L,  L,  L,  L,  L,  L,
    L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,
    L,  ON, L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,
    L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,  L,
    L,  L,  L,  ON, L,  L,  L,  L,  L,  L,  L,  L,
];

#[rustfmt::skip]
static ARABIC_TYPES: [BidiType; 256] = [
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    CS, AL, ON, ON, NSM,NSM,NSM,NSM,NSM,NSM,AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, NSM,NSM,NSM,NSM,NSM,NSM,NSM,
    NSM,NSM,NSM,NSM,NSM,NSM,NSM,AL, AL, AL, AL,
    AL, AL, AL, AN, AN, AN, AN, AN, AN, AN, AN, AN,
    AN, ET, AN, AN, AL, AL, AL, NSM,AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, NSM,NSM,NSM,NSM,NSM,NSM,NSM,NSM,NSM,NSM,
    NSM,NSM,NSM,NSM,NSM,NSM,NSM,NSM,NSM,ON, NSM,
    NSM,NSM,NSM,AL, AL, AL, AL, AL, AL, AL, AL, AL,
    AL, AL, AL, AL, AL, AL, AL, AL, AL,
];

thread_local! {
    /// Reusable scratch buffer for per-character bidi types.
    static BIDI_TYPES_SCRATCH: RefCell<Vec<BidiType>> = const { RefCell::new(Vec::new()) };
    /// Reusable scratch buffer for per-character bidi embedding levels.
    static BIDI_LEVELS_SCRATCH: RefCell<Vec<i8>> = const { RefCell::new(Vec::new()) };
    /// Reusable scratch buffer for byte offsets of each character start.
    static BIDI_CHAR_STARTS_SCRATCH: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    /// Single-entry cache for bidi levels keyed by normalized text.
    static BIDI_LEVELS_CACHE: RefCell<Option<(String, Vec<i8>)>> = const { RefCell::new(None) };
}

fn classify_char(code: u32) -> BidiType {
    if code <= 0x00FF {
        BASE_TYPES[code as usize]
    } else if (0x0590..=0x05F4).contains(&code) || (0xFB1D..=0xFB4F).contains(&code) {
        R
    } else if (0x0600..=0x06FF).contains(&code) {
        ARABIC_TYPES[(code & 0xFF) as usize]
    } else if (0x0700..=0x08FF).contains(&code)
        || (0xFB50..=0xFDFF).contains(&code)
        || (0xFE70..=0xFEFC).contains(&code)
        || (0x1EE00..=0x1EEFF).contains(&code)
    {
        AL
    } else if code == 0x200E || code == 0x202A || code == 0x202D {
        L
    } else if code == 0x200F || code == 0x202B || code == 0x202E {
        R
    } else if code == 0x202C || (0x2066..=0x2069).contains(&code) {
        BN
    } else {
        L
    }
}

/// Compute per-character bidi embedding levels.
///
/// This is a simplified implementation that uses a ratio heuristic to
/// determine the paragraph embedding level instead of the Unicode-standard
/// first-strong-character rule. If fewer than ~30% of characters are
/// strongly directional (R, AL, or AN), the paragraph is treated as LTR
/// (level 0); otherwise it is treated as RTL (level 1).
///
/// Returns `None` if the text contains no strongly directional characters,
/// indicating uniform LTR.
fn compute_bidi_levels(s: &str) -> Option<Vec<i8>> {
    BIDI_TYPES_SCRATCH.with(|types_scratch| {
        let mut types = types_scratch.borrow_mut();
        types.clear();
        let mut num_bidi = 0u32;

        for ch in s.chars() {
            let t = classify_char(ch as u32);
            if t == R || t == AL || t == AN {
                num_bidi += 1;
            }
            types.push(t);
        }

        let len = types.len();
        if len == 0 || num_bidi == 0 {
            return None;
        }

        let start_level: i8 = if (len as f64 / num_bidi as f64) < 0.3 {
            0
        } else {
            1
        };

        BIDI_LEVELS_SCRATCH.with(|levels_scratch| {
            let mut levels = levels_scratch.borrow_mut();
            levels.clear();
            levels.resize(len, start_level);

            let e: BidiType = if start_level & 1 != 0 { R } else { L };
            let sor = e;

            // W1-W7
            let mut last_type = sor;
            for t in types.iter_mut() {
                if *t == NSM {
                    *t = last_type;
                } else {
                    last_type = *t;
                }
            }
            last_type = sor;
            for t in types.iter_mut() {
                match *t {
                    EN if last_type == AL => {
                        *t = AN;
                    }
                    R | L | AL => {
                        last_type = *t;
                    }
                    _ => {}
                }
            }
            for t in types.iter_mut() {
                if *t == AL {
                    *t = R;
                }
            }
            for i in 1..len.saturating_sub(1) {
                if types[i] == ES && types[i - 1] == EN && types[i + 1] == EN {
                    types[i] = EN;
                }
                if types[i] == CS
                    && (types[i - 1] == EN || types[i - 1] == AN)
                    && types[i + 1] == types[i - 1]
                {
                    types[i] = types[i - 1];
                }
            }
            for i in 0..len {
                if types[i] != EN {
                    continue;
                }
                let mut j = i as isize - 1;
                while j >= 0 && types[j as usize] == ET {
                    types[j as usize] = EN;
                    j -= 1;
                }
                let mut j = i + 1;
                while j < len && types[j] == ET {
                    types[j] = EN;
                    j += 1;
                }
            }
            for t in types.iter_mut() {
                match *t {
                    WS | ES | ET | CS => *t = ON,
                    _ => {}
                }
            }
            last_type = sor;
            for t in types.iter_mut() {
                match *t {
                    EN if last_type == L => {
                        *t = L;
                    }
                    R | L => {
                        last_type = *t;
                    }
                    _ => {}
                }
            }

            // N1-N2
            let mut i = 0;
            while i < len {
                if types[i] != ON {
                    i += 1;
                    continue;
                }
                let mut end = i + 1;
                while end < len && types[end] == ON {
                    end += 1;
                }
                let before = if i > 0 { types[i - 1] } else { sor };
                let after = if end < len { types[end] } else { sor };
                let b_dir = if before != L { R } else { L };
                let a_dir = if after != L { R } else { L };
                if b_dir == a_dir {
                    for t in types[i..end].iter_mut() {
                        *t = b_dir;
                    }
                }
                i = end;
            }
            for t in types.iter_mut() {
                if *t == ON {
                    *t = e;
                }
            }

            // I1-I2
            for i in 0..len {
                let t = types[i];
                if levels[i] & 1 == 0 {
                    if t == R {
                        levels[i] += 1;
                    } else if t == AN || t == EN {
                        levels[i] += 2;
                    }
                } else if t == L || t == AN || t == EN {
                    levels[i] += 1;
                }
            }

            Some(levels.clone())
        })
    })
}

/// Compute per-segment bidi levels from the full text and segment start offsets.
///
/// Returns `None` if the text contains no strongly directional characters,
/// in which case all segments are implicitly LTR (level 0).
pub fn compute_segment_levels(normalized: &str, seg_starts: &[usize]) -> Option<Vec<i8>> {
    let bidi_levels = BIDI_LEVELS_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((ref cached_text, ref levels)) = *cache
            && cached_text == normalized
        {
            return Some(levels.clone());
        }
        let levels = compute_bidi_levels(normalized)?;
        *cache = Some((normalized.to_string(), levels.clone()));
        Some(levels)
    })?;

    // seg_starts are byte offsets; bidi_levels are indexed by char position.
    // Use a compact list of char-start byte offsets and binary search instead of
    // a dense byte→char map.
    BIDI_CHAR_STARTS_SCRATCH.with(|starts_scratch| {
        let mut char_starts = starts_scratch.borrow_mut();
        char_starts.clear();
        char_starts.extend(normalized.char_indices().map(|(i, _)| i));

        let seg_levels: Vec<i8> = seg_starts
            .iter()
            .map(|&start| {
                let char_idx = byte_offset_to_char_index(start, &char_starts);
                bidi_levels.get(char_idx).copied().unwrap_or(0)
            })
            .collect();

        Some(seg_levels)
    })
}

fn byte_offset_to_char_index(byte_offset: usize, char_starts: &[usize]) -> usize {
    if char_starts.is_empty() || byte_offset == 0 {
        return 0;
    }
    match char_starts.binary_search(&byte_offset) {
        Ok(idx) => idx,
        Err(0) => 0,
        Err(idx) => idx - 1,
    }
}

#[cfg(test)]
fn bidi_types_scratch_capacity() -> usize {
    BIDI_TYPES_SCRATCH.with(|s| s.borrow().capacity())
}

#[cfg(test)]
mod tests;
