//! Validation of TrueType glyphs, and their rewriting without instructions.

use super::ReadBe;
use crate::FontDecodeError;

const MALFORMED: FontDecodeError = FontDecodeError::Malformed("glyf");

const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;

const ON_X_SHORT: u8 = 0x02;
const ON_Y_SHORT: u8 = 0x04;
const REPEAT: u8 = 0x08;
const X_SAME_OR_POSITIVE: u8 = 0x10;
const Y_SAME_OR_POSITIVE: u8 = 0x20;

/// The glyphs that composites refer to, kept compactly while the table is walked.
#[derive(Default)]
pub(super) struct ComponentGraph {
    /// A composite glyph and the start of its components, in glyph order.
    composites: Vec<(u16, u32)>,
    components: Vec<u16>,
}

impl ComponentGraph {
    fn children(&self, glyph: u16) -> &[u16] {
        let Ok(index) = self.composites.binary_search_by_key(&glyph, |entry| entry.0) else {
            return &[];
        };
        let start = self.composites[index].1 as usize;
        let end = self.composites.get(index + 1).map_or(self.components.len(), |next| next.1 as usize);
        &self.components[start..end]
    }

    /// Refuses a cycle of components, and nesting deeper than `max_depth`. Iterative, so
    /// that a hostile font cannot exhaust the stack. The nesting of each glyph is remembered,
    /// so a chain is measured in full whatever order its glyphs are visited in.
    pub(super) fn check(&self, glyph_count: usize, max_depth: usize) -> Result<(), FontDecodeError> {
        const UNVISITED: u16 = 0;
        const ACTIVE: u16 = u16::MAX;
        const TOO_DEEP: FontDecodeError = FontDecodeError::LimitExceeded("composite glyph depth");
        if self.composites.is_empty() {
            return Ok(());
        }
        // For a finished glyph: its nesting plus one (a glyph without components has nesting 0).
        let mut state = vec![UNVISITED; glyph_count];
        let mut stack: Vec<(u16, usize, usize)> = Vec::with_capacity(max_depth + 1);
        for &(root, _) in &self.composites {
            if state[root as usize] != UNVISITED {
                continue;
            }
            state[root as usize] = ACTIVE;
            stack.push((root, 0, 0));
            while let Some(top) = stack.last_mut() {
                let (glyph, next, deepest) = *top;
                let Some(&child) = self.children(glyph).get(next) else {
                    let nesting = if self.children(glyph).is_empty() { 0 } else { deepest + 1 };
                    if nesting > max_depth {
                        return Err(TOO_DEEP);
                    }
                    state[glyph as usize] = nesting as u16 + 1;
                    stack.pop();
                    if let Some(parent) = stack.last_mut() {
                        parent.2 = parent.2.max(nesting);
                    }
                    continue;
                };
                top.1 += 1;
                match state[child as usize] {
                    ACTIVE => return Err(FontDecodeError::Malformed("a composite glyph contains itself")),
                    UNVISITED => {
                        if stack.len() > max_depth {
                            return Err(TOO_DEEP);
                        }
                        state[child as usize] = ACTIVE;
                        stack.push((child, 0, 0));
                    }
                    finished => top.2 = deepest.max(finished as usize - 1),
                }
            }
        }
        Ok(())
    }
}

/// Where the instructions of a validated glyph are.
pub(super) enum GlyphLayout {
    /// No outline.
    Empty,
    /// A simple glyph: the length field at `length_offset`, then its instructions, then
    /// flags and coordinates up to `end`.
    Simple { length_offset: usize, instructions: usize, end: usize },
    /// A composite: its component records end at `records_end`.
    Composite { records_end: usize },
}

/// Validates one glyph and records its components.
pub(super) fn validate_glyph(
    glyph: &[u8],
    id: u16,
    glyph_count: usize,
    graph: &mut ComponentGraph,
) -> Result<GlyphLayout, FontDecodeError> {
    if glyph.is_empty() {
        return Ok(GlyphLayout::Empty);
    }
    let contours = glyph.i16_at(0).ok_or(MALFORMED)?;
    if glyph.len() < 10 {
        return Err(MALFORMED);
    }
    if contours >= 0 {
        return validate_simple(glyph, contours as usize);
    }
    if contours != -1 {
        return Err(MALFORMED);
    }
    let start = graph.components.len() as u32;
    let mut offset = 10usize;
    let mut has_instructions = false;
    loop {
        let flags = glyph.u16_at(offset).ok_or(MALFORMED)?;
        let component = glyph.u16_at(offset + 2).ok_or(MALFORMED)?;
        if component as usize >= glyph_count {
            return Err(MALFORMED);
        }
        graph.components.push(component);
        offset += 4 + if flags & ARG_1_AND_2_ARE_WORDS != 0 { 4 } else { 2 };
        offset += if flags & WE_HAVE_A_SCALE != 0 {
            2
        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            4
        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
            8
        } else {
            0
        };
        if offset > glyph.len() {
            return Err(MALFORMED);
        }
        has_instructions |= flags & WE_HAVE_INSTRUCTIONS != 0;
        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }
    graph.composites.push((id, start));
    let records_end = offset;
    if has_instructions {
        let length = glyph.u16_at(offset).ok_or(MALFORMED)? as usize;
        if offset + 2 + length > glyph.len() {
            return Err(MALFORMED);
        }
    }
    Ok(GlyphLayout::Composite { records_end })
}

fn validate_simple(glyph: &[u8], contours: usize) -> Result<GlyphLayout, FontDecodeError> {
    let mut offset = 10usize;
    let mut points = 0usize;
    for index in 0..contours {
        let end = glyph.u16_at(offset).ok_or(MALFORMED)? as usize;
        if index > 0 && end < points {
            return Err(MALFORMED);
        }
        points = end + 1;
        offset += 2;
    }
    let length_offset = offset;
    let instructions = glyph.u16_at(offset).ok_or(MALFORMED)? as usize;
    offset += 2 + instructions;
    if offset > glyph.len() {
        return Err(MALFORMED);
    }
    let (mut x_bytes, mut y_bytes, mut seen) = (0usize, 0usize, 0usize);
    while seen < points {
        let flag = glyph.u8_at(offset).ok_or(MALFORMED)?;
        offset += 1;
        let mut repeat = 1usize;
        if flag & REPEAT != 0 {
            repeat += glyph.u8_at(offset).ok_or(MALFORMED)? as usize;
            offset += 1;
        }
        if seen + repeat > points {
            return Err(MALFORMED);
        }
        seen += repeat;
        if flag & ON_X_SHORT != 0 {
            x_bytes += repeat;
        } else if flag & X_SAME_OR_POSITIVE == 0 {
            x_bytes += 2 * repeat;
        }
        if flag & ON_Y_SHORT != 0 {
            y_bytes += repeat;
        } else if flag & Y_SAME_OR_POSITIVE == 0 {
            y_bytes += 2 * repeat;
        }
    }
    let end = offset + x_bytes + y_bytes;
    if end > glyph.len() {
        return Err(MALFORMED);
    }
    Ok(GlyphLayout::Simple { length_offset, instructions, end })
}

/// Appends a validated glyph to `output`, without its instructions.
pub(super) fn write_without_instructions(glyph: &[u8], layout: &GlyphLayout, output: &mut Vec<u8>) {
    match *layout {
        GlyphLayout::Empty => {}
        GlyphLayout::Simple { length_offset, instructions, end } => {
            output.extend_from_slice(&glyph[..length_offset]);
            output.extend_from_slice(&[0, 0]);
            output.extend_from_slice(&glyph[length_offset + 2 + instructions..end]);
        }
        GlyphLayout::Composite { records_end } => {
            let start = output.len();
            output.extend_from_slice(&glyph[..records_end]);
            let record = &mut output[start..];
            let mut offset = 10usize;
            loop {
                let flags = u16::from_be_bytes([record[offset], record[offset + 1]]);
                let cleared = flags & !WE_HAVE_INSTRUCTIONS;
                record[offset..offset + 2].copy_from_slice(&cleared.to_be_bytes());
                offset += 4 + if flags & ARG_1_AND_2_ARE_WORDS != 0 { 4 } else { 2 };
                offset += if flags & WE_HAVE_A_SCALE != 0 {
                    2
                } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                    4
                } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                    8
                } else {
                    0
                };
                if flags & MORE_COMPONENTS == 0 {
                    break;
                }
            }
        }
    }
}
