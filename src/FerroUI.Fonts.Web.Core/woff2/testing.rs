//! A WOFF2 encoder for tests: it applies the transforms of the format and leaves compression to
//! the caller, so fixtures are built at test time from fonts of the repository.

use super::KNOWN_TAGS;
use crate::sfnt::{ReadBe, Tag};

/// The tables of a font, by tag.
pub fn font_tables(font: &[u8]) -> Vec<(Tag, Vec<u8>)> {
    let count = font.u16_at(4).unwrap() as usize;
    let mut tables: Vec<(Tag, Vec<u8>)> = (0..count)
        .map(|index| {
            let record = 12 + index * 16;
            let tag: Tag = font[record..record + 4].try_into().unwrap();
            let (offset, length) =
                (font.u32_at(record + 8).unwrap() as usize, font.u32_at(record + 12).unwrap() as usize);
            (tag, font[offset..offset + length].to_vec())
        })
        .collect();
    tables.sort_by_key(|(tag, _)| *tag);
    tables
}

/// One table of a font.
pub fn font_table(font: &[u8], tag: &Tag) -> Option<Vec<u8>> {
    font_tables(font).into_iter().find(|(name, _)| name == tag).map(|(_, table)| table)
}

/// A glyph, independent of how its bytes are packed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outline {
    Empty,
    Simple { bbox: [i16; 4], ends: Vec<u16>, points: Vec<(i16, i16, bool)>, instructions: Vec<u8>, overlap: bool },
    Composite { bbox: [i16; 4], records: Vec<u8>, instructions: Vec<u8> },
}

/// Reads the glyphs of a TrueType font.
pub fn outlines(font: &[u8]) -> Vec<Outline> {
    let head = font_table(font, b"head").unwrap();
    let maxp = font_table(font, b"maxp").unwrap();
    let loca = font_table(font, b"loca").unwrap();
    let glyf = font_table(font, b"glyf").unwrap();
    let long = head.i16_at(50).unwrap() == 1;
    let count = maxp.u16_at(4).unwrap() as usize;
    let entry = |index: usize| {
        if long {
            loca.u32_at(index * 4).unwrap() as usize
        } else {
            loca.u16_at(index * 2).unwrap() as usize * 2
        }
    };
    (0..count).map(|index| parse_outline(&glyf[entry(index)..entry(index + 1)])).collect()
}

fn parse_outline(glyph: &[u8]) -> Outline {
    if glyph.is_empty() {
        return Outline::Empty;
    }
    let contours = glyph.i16_at(0).unwrap();
    let bbox = [glyph.i16_at(2).unwrap(), glyph.i16_at(4).unwrap(), glyph.i16_at(6).unwrap(), glyph.i16_at(8).unwrap()];
    if contours < 0 {
        let mut offset = 10;
        let mut has_instructions = false;
        loop {
            let flags = glyph.u16_at(offset).unwrap();
            has_instructions |= flags & 0x100 != 0;
            offset += 4 + if flags & 1 != 0 { 4 } else { 2 };
            offset += if flags & 8 != 0 {
                2
            } else if flags & 0x40 != 0 {
                4
            } else if flags & 0x80 != 0 {
                8
            } else {
                0
            };
            if flags & 0x20 == 0 {
                break;
            }
        }
        let records = glyph[10..offset].to_vec();
        let instructions = if has_instructions {
            let length = glyph.u16_at(offset).unwrap() as usize;
            glyph[offset + 2..offset + 2 + length].to_vec()
        } else {
            Vec::new()
        };
        return Outline::Composite { bbox, records, instructions };
    }
    let contours = contours as usize;
    let ends: Vec<u16> = (0..contours).map(|index| glyph.u16_at(10 + index * 2).unwrap()).collect();
    let total = ends.last().map_or(0, |end| *end as usize + 1);
    let mut offset = 10 + contours * 2;
    let length = glyph.u16_at(offset).unwrap() as usize;
    let instructions = glyph[offset + 2..offset + 2 + length].to_vec();
    offset += 2 + length;
    let mut flags = Vec::with_capacity(total);
    while flags.len() < total {
        let flag = glyph[offset];
        offset += 1;
        flags.push(flag);
        if flag & 8 != 0 {
            for _ in 0..glyph[offset] {
                flags.push(flag);
            }
            offset += 1;
        }
    }
    let overlap = flags.first().is_some_and(|flag| flag & 0x40 != 0);
    let mut read_axis = |short: u8, same: u8| -> Vec<i16> {
        let mut value = 0i32;
        flags
            .iter()
            .map(|flag| {
                if flag & short != 0 {
                    let delta = glyph[offset] as i32;
                    offset += 1;
                    value += if flag & same != 0 { delta } else { -delta };
                } else if flag & same == 0 {
                    value += glyph.i16_at(offset).unwrap() as i32;
                    offset += 2;
                }
                value as i16
            })
            .collect()
    };
    let xs = read_axis(0x02, 0x10);
    let ys = read_axis(0x04, 0x20);
    let points = (0..total).map(|index| (xs[index], ys[index], flags[index] & 1 != 0)).collect();
    Outline::Simple { bbox, ends, points, instructions, overlap }
}

fn write_base128(output: &mut Vec<u8>, value: u32) {
    let mut started = false;
    for shift in [28, 21, 14, 7] {
        let bits = (value >> shift) & 0x7F;
        if bits != 0 || started {
            output.push(bits as u8 | 0x80);
            started = true;
        }
    }
    output.push((value & 0x7F) as u8);
}

/// Writes a `255UInt16`.
pub fn write_u255(output: &mut Vec<u8>, value: u16) {
    match value {
        0..=252 => output.push(value as u8),
        253..=505 => output.extend_from_slice(&[255, (value - 253) as u8]),
        506..=761 => output.extend_from_slice(&[254, (value - 506) as u8]),
        _ => {
            output.push(253);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn write_triplet(flags: &mut Vec<u8>, data: &mut Vec<u8>, dx: i32, dy: i32, on_curve: bool) {
    let (abs_x, abs_y) = (dx.unsigned_abs(), dy.unsigned_abs());
    let on = if on_curve { 0u32 } else { 128 };
    let x_sign = u32::from(dx >= 0);
    let y_sign = u32::from(dy >= 0);
    let signs = x_sign + 2 * y_sign;
    if dx == 0 && abs_y < 1280 {
        flags.push((on + ((abs_y & 0xF00) >> 7) + y_sign) as u8);
        data.push(abs_y as u8);
    } else if dy == 0 && abs_x < 1280 {
        flags.push((on + 10 + ((abs_x & 0xF00) >> 7) + x_sign) as u8);
        data.push(abs_x as u8);
    } else if abs_x < 65 && abs_y < 65 && abs_x > 0 && abs_y > 0 {
        flags.push((on + 20 + ((abs_x - 1) & 0x30) + (((abs_y - 1) & 0x30) >> 2) + signs) as u8);
        data.push((((abs_x - 1) & 0x0F) << 4 | ((abs_y - 1) & 0x0F)) as u8);
    } else if abs_x < 769 && abs_y < 769 && abs_x > 0 && abs_y > 0 {
        flags.push((on + 84 + 12 * (((abs_x - 1) & 0x300) >> 8) + (((abs_y - 1) & 0x300) >> 6) + signs) as u8);
        data.extend_from_slice(&[(abs_x - 1) as u8, (abs_y - 1) as u8]);
    } else if abs_x < 4096 && abs_y < 4096 {
        flags.push((on + 120 + signs) as u8);
        data.extend_from_slice(&[(abs_x >> 4) as u8, ((abs_x << 4) | (abs_y >> 8)) as u8, abs_y as u8]);
    } else {
        flags.push((on + 124 + signs) as u8);
        data.extend_from_slice(&[(abs_x >> 8) as u8, abs_x as u8, (abs_y >> 8) as u8, abs_y as u8]);
    }
}

/// The transformed `glyf` of a font.
pub fn transform_glyf(font: &[u8]) -> Vec<u8> {
    let glyphs = outlines(font);
    let head = font_table(font, b"head").unwrap();
    let index_format = head.u16_at(50).unwrap();
    let count = glyphs.len();
    let bitmap_length = ((count + 31) >> 5) << 2;
    let mut bbox_bitmap = vec![0u8; bitmap_length];
    let mut overlap_bitmap = vec![0u8; count.div_ceil(8)];
    let (mut n_contours, mut n_points, mut flags, mut data) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut composites, mut bboxes, mut instructions) = (Vec::new(), Vec::new(), Vec::new());
    let mut any_overlap = false;
    for (index, glyph) in glyphs.iter().enumerate() {
        match glyph {
            Outline::Empty => n_contours.extend_from_slice(&0i16.to_be_bytes()),
            Outline::Composite { bbox, records, instructions: code } => {
                n_contours.extend_from_slice(&(-1i16).to_be_bytes());
                bbox_bitmap[index >> 3] |= 0x80 >> (index & 7);
                for value in bbox {
                    bboxes.extend_from_slice(&value.to_be_bytes());
                }
                composites.extend_from_slice(records);
                let mut offset = 0;
                let mut has_instructions = false;
                loop {
                    let record_flags = records.u16_at(offset).unwrap();
                    has_instructions |= record_flags & 0x100 != 0;
                    offset += 4 + if record_flags & 1 != 0 { 4 } else { 2 };
                    offset += if record_flags & 8 != 0 {
                        2
                    } else if record_flags & 0x40 != 0 {
                        4
                    } else if record_flags & 0x80 != 0 {
                        8
                    } else {
                        0
                    };
                    if record_flags & 0x20 == 0 {
                        break;
                    }
                }
                if has_instructions {
                    write_u255(&mut data, code.len() as u16);
                    instructions.extend_from_slice(code);
                }
            }
            Outline::Simple { bbox, ends, points, instructions: code, overlap } => {
                n_contours.extend_from_slice(&(ends.len() as i16).to_be_bytes());
                let mut previous = 0i32;
                for end in ends {
                    write_u255(&mut n_points, (*end as i32 + 1 - previous) as u16);
                    previous = *end as i32 + 1;
                }
                let (mut x, mut y) = (0i32, 0i32);
                for &(px, py, on_curve) in points {
                    write_triplet(&mut flags, &mut data, px as i32 - x, py as i32 - y, on_curve);
                    x = px as i32;
                    y = py as i32;
                }
                write_u255(&mut data, code.len() as u16);
                instructions.extend_from_slice(code);
                let computed = [
                    points.iter().map(|point| point.0).min().unwrap_or(0),
                    points.iter().map(|point| point.1).min().unwrap_or(0),
                    points.iter().map(|point| point.0).max().unwrap_or(0),
                    points.iter().map(|point| point.1).max().unwrap_or(0),
                ];
                if computed != *bbox {
                    bbox_bitmap[index >> 3] |= 0x80 >> (index & 7);
                    for value in bbox {
                        bboxes.extend_from_slice(&value.to_be_bytes());
                    }
                }
                if *overlap {
                    any_overlap = true;
                    overlap_bitmap[index >> 3] |= 0x80 >> (index & 7);
                }
            }
        }
    }
    let mut bbox_stream = bbox_bitmap;
    bbox_stream.extend_from_slice(&bboxes);
    let mut output = Vec::new();
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(&u16::from(any_overlap).to_be_bytes());
    output.extend_from_slice(&(count as u16).to_be_bytes());
    output.extend_from_slice(&index_format.to_be_bytes());
    for stream in [&n_contours, &n_points, &flags, &data, &composites, &bbox_stream, &instructions] {
        output.extend_from_slice(&(stream.len() as u32).to_be_bytes());
    }
    for stream in [&n_contours, &n_points, &flags, &data, &composites, &bbox_stream, &instructions] {
        output.extend_from_slice(stream);
    }
    if any_overlap {
        output.extend_from_slice(&overlap_bitmap);
    }
    output
}

/// The transformed `hmtx`, when each left side bearing equals the `xMin` of its glyph.
pub fn transform_hmtx(font: &[u8]) -> Option<Vec<u8>> {
    let hhea = font_table(font, b"hhea")?;
    let hmtx = font_table(font, b"hmtx")?;
    let glyphs = outlines(font);
    let metrics = hhea.u16_at(34)? as usize;
    let x_min = |glyph: &Outline| match glyph {
        Outline::Empty => 0,
        Outline::Simple { bbox, .. } | Outline::Composite { bbox, .. } => bbox[0],
    };
    for (index, glyph) in glyphs.iter().enumerate() {
        let bearing = if index < metrics {
            hmtx.i16_at(index * 4 + 2)?
        } else {
            hmtx.i16_at(metrics * 4 + (index - metrics) * 2)?
        };
        if bearing != x_min(glyph) {
            return None;
        }
    }
    let mut output = vec![0x03];
    for index in 0..metrics {
        output.extend_from_slice(&hmtx[index * 4..index * 4 + 2]);
    }
    Some(output)
}

/// A Brotli stream of uncompressed meta-blocks (RFC 7932, section 9.2), which any conforming
/// decoder reads.
pub fn stored_brotli(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() + data.len() / 4096 + 8);
    let mut first = true;
    for chunk in data.chunks(0x1_0000) {
        // WBITS (one 0 bit, for 16) on the first block, then ISLAST = 0, MNIBBLES = 0 (four),
        // MLEN - 1 in 16 bits and ISUNCOMPRESSED = 1, padded with zeros to a byte.
        let shift = u32::from(first);
        let length = (chunk.len() - 1) as u32;
        let bits = (length << (3 + shift)) | (1 << (19 + shift));
        output.extend_from_slice(&bits.to_le_bytes()[..3]);
        output.extend_from_slice(chunk);
        first = false;
    }
    if first {
        // An empty stream: WBITS, ISLAST = 1, ISLASTEMPTY = 1.
        output.push(0b110);
    } else {
        output.push(0b11);
    }
    output
}

/// How [`encode_woff2`] writes a font.
#[derive(Clone, Copy, Debug)]
pub struct EncodeOptions {
    pub transform_glyf: bool,
    pub transform_hmtx: bool,
}

/// Writes a WOFF2 file, compressing the table stream with `compress`.
pub fn encode_woff2(font: &[u8], options: EncodeOptions, compress: &dyn Fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let tables = font_tables(font);
    let transformed_glyf = options.transform_glyf && tables.iter().any(|(tag, _)| tag == b"glyf");
    let hmtx = if transformed_glyf && options.transform_hmtx { transform_hmtx(font) } else { None };
    let mut order: Vec<&(Tag, Vec<u8>)> = tables.iter().filter(|(tag, _)| tag != b"loca").collect();
    if let Some(glyf) = order.iter().position(|(tag, _)| tag == b"glyf") {
        let loca = tables.iter().find(|(tag, _)| tag == b"loca").unwrap();
        order.insert(glyf + 1, loca);
    }
    let (mut directory, mut stream) = (Vec::new(), Vec::new());
    for (tag, table) in order.iter().map(|entry| (&entry.0, &entry.1)) {
        let known = KNOWN_TAGS.iter().position(|name| name == tag);
        let (version, transformed): (u8, Option<Vec<u8>>) = match tag {
            b"glyf" if transformed_glyf => (0, Some(transform_glyf(font))),
            b"loca" if transformed_glyf => (0, Some(Vec::new())),
            b"glyf" | b"loca" => (3, None),
            b"hmtx" if hmtx.is_some() => (1, hmtx.clone()),
            _ => (0, None),
        };
        directory.push(known.unwrap_or(63) as u8 | (version << 6));
        if known.is_none() {
            directory.extend_from_slice(&tag[..]);
        }
        write_base128(&mut directory, table.len() as u32);
        match &transformed {
            Some(bytes) => {
                write_base128(&mut directory, bytes.len() as u32);
                stream.extend_from_slice(bytes);
            }
            None => stream.extend_from_slice(table),
        }
    }
    let compressed = compress(&stream);
    // The file ends on a four-byte boundary, as decoders of browsers require.
    let total = (48 + directory.len() + compressed.len() + 3) & !3;
    let mut file = Vec::with_capacity(total);
    file.extend_from_slice(b"wOF2");
    file.extend_from_slice(&font[..4]);
    file.extend_from_slice(&(total as u32).to_be_bytes());
    file.extend_from_slice(&(tables.len() as u16).to_be_bytes());
    file.extend_from_slice(&0u16.to_be_bytes());
    file.extend_from_slice(&(font.len() as u32).to_be_bytes());
    file.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    file.extend_from_slice(&[0; 24]);
    file.extend_from_slice(&directory);
    file.extend_from_slice(&compressed);
    file.resize(total, 0);
    file
}
