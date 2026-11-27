//! Structural validation of the tables a font is kept or refused by.
//!
//! Each function answers whether a table is structurally sound. Layout, CFF and color tables
//! get their headers and offsets checked here; the deep validation of their content stays with
//! the shaper and rasterizer that read them (`docs/porting/web-fonts.md`, section 7).

use super::ReadBe;

/// The fields of `head` that the rest of the font is read with.
#[derive(Clone, Copy, Debug)]
pub(super) struct Head {
    pub long_loca: bool,
}

pub(super) fn parse_head(table: &[u8]) -> Option<Head> {
    if table.len() < 54 || table.u32_at(12)? != 0x5F0F_3CF5 {
        return None;
    }
    let units_per_em = table.u16_at(18)?;
    if !(16..=16384).contains(&units_per_em) {
        return None;
    }
    let format = table.i16_at(50)?;
    if !(0..=1).contains(&format) || table.i16_at(52)? != 0 {
        return None;
    }
    Some(Head { long_loca: format == 1 })
}

/// The number of glyphs, from `maxp`.
pub(super) fn parse_maxp(table: &[u8]) -> Option<u16> {
    let (version, glyphs) = (table.u32_at(0)?, table.u16_at(4)?);
    let sized = (version == 0x0000_5000 && table.len() >= 6) || (version == 0x0001_0000 && table.len() >= 32);
    (sized && glyphs > 0).then_some(glyphs)
}

/// The number of metrics in `hhea` or `vhea`.
pub(super) fn parse_metrics_header(table: &[u8], glyphs: u16) -> Option<u16> {
    if table.len() < 36 || table.u16_at(0)? != 1 && table.u32_at(0)? != 0x0001_1000 {
        return None;
    }
    if table.i16_at(32)? != 0 {
        return None;
    }
    let metrics = table.u16_at(34)?;
    (metrics >= 1 && metrics <= glyphs).then_some(metrics)
}

/// The exact length of `hmtx` or `vmtx`.
pub(super) fn metrics_length(metrics: u16, glyphs: u16) -> usize {
    metrics as usize * 4 + (glyphs as usize - metrics as usize) * 2
}

fn valid_cmap_subtable(table: &[u8], offset: usize) -> Option<bool> {
    let format = table.u16_at(offset)?;
    let rest = table.get(offset..)?;
    Some(match format {
        0 => rest.u16_at(2)? as usize >= 262 && rest.len() >= 262,
        4 => valid_cmap_format4(rest)?,
        6 => {
            let (length, count) = (rest.u16_at(2)? as usize, rest.u16_at(8)? as usize);
            length <= rest.len() && 10 + count * 2 <= length
        }
        10 => {
            let (length, count) = (rest.u32_at(4)? as usize, rest.u32_at(16)? as usize);
            length <= rest.len() && count.checked_mul(2)?.checked_add(20)? <= length
        }
        12 | 13 => valid_cmap_groups(rest)?,
        14 => {
            let (length, count) = (rest.u32_at(2)? as usize, rest.u32_at(6)? as usize);
            length <= rest.len() && count.checked_mul(11)?.checked_add(10)? <= length
        }
        _ => false,
    })
}

fn valid_cmap_format4(table: &[u8]) -> Option<bool> {
    let length = table.u16_at(2)? as usize;
    let doubled = table.u16_at(6)? as usize;
    if length > table.len() || doubled < 2 || doubled % 2 != 0 || 16 + doubled * 4 > length {
        return Some(false);
    }
    let segments = doubled / 2;
    let (ends, starts) = (14usize, 16 + doubled);
    let range_offsets = starts + doubled * 2;
    let mut previous = None;
    for index in 0..segments {
        let end = table.u16_at(ends + index * 2)?;
        let start = table.u16_at(starts + index * 2)?;
        if start > end || previous.is_some_and(|last| start <= last) {
            return Some(false);
        }
        previous = Some(end);
        let position = range_offsets + index * 2;
        let range_offset = table.u16_at(position)? as usize;
        // The closing segment of U+FFFF often has an offset that points nowhere, and no
        // character is read through it.
        if range_offset != 0 && !(start == 0xFFFF && end == 0xFFFF) {
            let last = position + range_offset + (end - start) as usize * 2;
            if range_offset % 2 != 0 || last + 2 > length {
                return Some(false);
            }
        }
    }
    Some(true)
}

fn valid_cmap_groups(table: &[u8]) -> Option<bool> {
    let (length, count) = (table.u32_at(4)? as usize, table.u32_at(12)? as usize);
    if length > table.len() || count.checked_mul(12)?.checked_add(16)? > length {
        return Some(false);
    }
    let mut previous = None;
    for index in 0..count {
        let entry = 16 + index * 12;
        let (start, end) = (table.u32_at(entry)?, table.u32_at(entry + 4)?);
        if start > end || end > 0x10FFFF || previous.is_some_and(|last| start <= last) {
            return Some(false);
        }
        previous = Some(end);
    }
    Some(true)
}

/// Validates `cmap`. Returns the records to keep; a font needs at least one.
pub(super) fn valid_cmap_records(table: &[u8]) -> Option<Vec<usize>> {
    if table.u16_at(0)? != 0 {
        return None;
    }
    let count = table.u16_at(2)? as usize;
    if 4 + count * 8 > table.len() {
        return None;
    }
    let keep: Vec<usize> = (0..count)
        .filter(|index| {
            table
                .u32_at(4 + index * 8 + 4)
                .and_then(|offset| valid_cmap_subtable(table, offset as usize))
                .unwrap_or(false)
        })
        .collect();
    (!keep.is_empty()).then_some(keep)
}

/// `cmap` with only the kept records. The subtables stay where they are.
pub(super) fn rewrite_cmap(table: &[u8], keep: &[usize]) -> Vec<u8> {
    let mut output = table.to_vec();
    output[2..4].copy_from_slice(&(keep.len() as u16).to_be_bytes());
    for (target, source) in keep.iter().enumerate() {
        let from = 4 + source * 8;
        output.copy_within(from..from + 8, 4 + target * 8);
    }
    output
}

pub(super) fn valid_name(table: &[u8]) -> bool {
    let check = || -> Option<bool> {
        let format = table.u16_at(0)?;
        let (count, strings) = (table.u16_at(2)? as usize, table.u16_at(4)? as usize);
        if format > 1 || 6 + count * 12 > table.len() || strings > table.len() {
            return Some(false);
        }
        for index in 0..count {
            let record = 6 + index * 12;
            let (length, offset) = (table.u16_at(record + 8)? as usize, table.u16_at(record + 10)? as usize);
            if strings + offset + length > table.len() {
                return Some(false);
            }
        }
        Some(true)
    };
    check().unwrap_or(false)
}

pub(super) fn valid_os2(table: &[u8]) -> bool {
    let required = match table.u16_at(0) {
        Some(0) => 68,
        Some(1) => 86,
        Some(2..=4) => 96,
        Some(5) => 100,
        _ => return false,
    };
    table.len() >= required
}

pub(super) fn valid_post(table: &[u8]) -> bool {
    table.len() >= 32 && matches!(table.u32_at(0), Some(0x0001_0000 | 0x0002_0000 | 0x0002_5000 | 0x0003_0000))
}

/// `GSUB`, `GPOS` and `GDEF`: the version and that the offsets of the header are in the table.
pub(super) fn valid_layout_header(tag: &[u8; 4], table: &[u8]) -> bool {
    let check = || -> Option<bool> {
        if table.u16_at(0)? != 1 {
            return Some(false);
        }
        let minor = table.u16_at(2)?;
        let offsets: &[usize] = match (tag, minor) {
            (b"GDEF", 0) => &[4, 6, 8, 10],
            (b"GDEF", 2) => &[4, 6, 8, 10, 12],
            (b"GDEF", 3) => &[4, 6, 8, 10, 12],
            (b"GSUB" | b"GPOS", 0 | 1) => &[4, 6, 8],
            _ => return Some(false),
        };
        for &field in offsets {
            if table.u16_at(field)? as usize >= table.len().max(1) {
                return Some(false);
            }
        }
        Some(true)
    };
    check().unwrap_or(false)
}

/// The number of axes of a valid `fvar`.
pub(super) fn parse_fvar(table: &[u8]) -> Option<u16> {
    if table.u16_at(0)? != 1 {
        return None;
    }
    let axes_offset = table.u16_at(4)? as usize;
    let axes = table.u16_at(8)? as usize;
    let axis_size = table.u16_at(10)? as usize;
    let instances = table.u16_at(12)? as usize;
    let instance_size = table.u16_at(14)? as usize;
    if axes == 0 || axis_size != 20 || instance_size < 4 + axes * 4 {
        return None;
    }
    if axes_offset + axes * 20 + instances * instance_size > table.len() {
        return None;
    }
    for index in 0..axes {
        let axis = axes_offset + index * 20;
        let (min, default, max) =
            (table.u32_at(axis + 4)? as i32, table.u32_at(axis + 8)? as i32, table.u32_at(axis + 12)? as i32);
        if min > default || default > max {
            return None;
        }
    }
    Some(axes as u16)
}

pub(super) fn valid_gvar(table: &[u8], axes: u16, glyphs: u16) -> bool {
    let check = || -> Option<bool> {
        if table.u16_at(0)? != 1 || table.u16_at(4)? != axes || table.u16_at(12)? != glyphs {
            return Some(false);
        }
        let long = table.u16_at(14)? & 1 != 0;
        let data = table.u32_at(16)? as usize;
        let entry = if long { 4 } else { 2 };
        let count = glyphs as usize + 1;
        if 20 + count * entry > table.len() || data > table.len() {
            return Some(false);
        }
        let mut previous = 0usize;
        for index in 0..count {
            let value =
                if long { table.u32_at(20 + index * 4)? as usize } else { table.u16_at(20 + index * 2)? as usize * 2 };
            if value < previous || data + value > table.len() {
                return Some(false);
            }
            previous = value;
        }
        Some(true)
    };
    check().unwrap_or(false)
}

pub(super) fn valid_colr(table: &[u8]) -> bool {
    let check = || -> Option<bool> {
        let version = table.u16_at(0)?;
        if version > 1 {
            return Some(false);
        }
        let (bases, base_offset) = (table.u16_at(2)? as usize, table.u32_at(4)? as usize);
        let (layers_offset, layers) = (table.u32_at(8)? as usize, table.u16_at(12)? as usize);
        let fits = |offset: usize, size: usize| offset.checked_add(size).is_some_and(|end| end <= table.len());
        if (bases > 0 && !fits(base_offset, bases * 6)) || (layers > 0 && !fits(layers_offset, layers * 4)) {
            return Some(false);
        }
        if version == 1 {
            if table.len() < 34 {
                return Some(false);
            }
            for field in [14usize, 18, 22, 26, 30] {
                if table.u32_at(field)? as usize > table.len() {
                    return Some(false);
                }
            }
        }
        Some(true)
    };
    check().unwrap_or(false)
}

pub(super) fn valid_cpal(table: &[u8]) -> bool {
    let check = || -> Option<bool> {
        if table.u16_at(0)? > 1 {
            return Some(false);
        }
        let entries = table.u16_at(2)? as usize;
        let palettes = table.u16_at(4)? as usize;
        let colors = table.u16_at(6)? as usize;
        let records = table.u32_at(8)? as usize;
        if palettes == 0 || 12 + palettes * 2 > table.len() || records.checked_add(colors * 4)? > table.len() {
            return Some(false);
        }
        for index in 0..palettes {
            if table.u16_at(12 + index * 2)? as usize + entries > colors {
                return Some(false);
            }
        }
        Some(true)
    };
    check().unwrap_or(false)
}

pub(super) fn valid_sbix(table: &[u8], glyphs: u16) -> bool {
    let check = || -> Option<bool> {
        if table.u16_at(0)? < 1 {
            return Some(false);
        }
        let strikes = table.u32_at(4)? as usize;
        if strikes.checked_mul(4)?.checked_add(8)? > table.len() {
            return Some(false);
        }
        let offsets = (glyphs as usize + 1) * 4;
        for index in 0..strikes {
            let strike = table.u32_at(8 + index * 4)? as usize;
            if strike.checked_add(4 + offsets)? > table.len() {
                return Some(false);
            }
        }
        Some(true)
    };
    check().unwrap_or(false)
}

/// The header of `CFF ` and that its first four INDEX structures are inside the table.
pub(super) fn valid_cff(table: &[u8]) -> bool {
    fn skip_index(table: &[u8], offset: usize) -> Option<usize> {
        let count = table.u16_at(offset)? as usize;
        if count == 0 {
            return Some(offset + 2);
        }
        let size = table.u8_at(offset + 2)? as usize;
        if !(1..=4).contains(&size) {
            return None;
        }
        let array = offset + 3;
        let last = array + count * size;
        let bytes = table.get(last..last + size)?;
        let end = bytes.iter().fold(0usize, |value, byte| (value << 8) | *byte as usize);
        if end == 0 {
            return None;
        }
        let data = (last + size).checked_add(end - 1)?;
        (data <= table.len()).then_some(data)
    }
    let check = || -> Option<bool> {
        if table.u8_at(0)? != 1 {
            return Some(false);
        }
        let mut offset = table.u8_at(2)? as usize;
        for _ in 0..4 {
            offset = skip_index(table, offset)?;
        }
        Some(true)
    };
    check().unwrap_or(false)
}

pub(super) fn valid_cff2(table: &[u8]) -> bool {
    let check = || -> Option<bool> {
        if table.u8_at(0)? != 2 {
            return Some(false);
        }
        let header = table.u8_at(2)? as usize;
        let top = table.u16_at(3)? as usize;
        Some(header >= 5 && header.checked_add(top)? <= table.len())
    };
    check().unwrap_or(false)
}

pub(super) fn valid_svg(table: &[u8]) -> bool {
    table.u16_at(0) == Some(0) && table.u32_at(2).is_some_and(|offset| offset as usize + 2 <= table.len())
}
