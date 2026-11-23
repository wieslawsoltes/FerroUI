//! The reconstruction of `glyf` and `loca` from the transformed `glyf` of WOFF2.

use super::Reader;
use crate::sfnt::ReadBe;
use crate::FontDecodeError;
use std::ops::Range;

const BAD: FontDecodeError = FontDecodeError::Malformed("WOFF2 glyf transform");

const ON_CURVE: u8 = 0x01;
const X_SHORT: u8 = 0x02;
const Y_SHORT: u8 = 0x04;
const REPEAT: u8 = 0x08;
const X_SAME: u8 = 0x10;
const Y_SAME: u8 = 0x20;
const OVERLAP_SIMPLE: u8 = 0x40;

const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;

/// One of the substreams: a position that cannot leave its range.
#[derive(Clone, Debug)]
struct Cursor {
    position: usize,
    end: usize,
}

impl Cursor {
    fn take<'s>(&mut self, stream: &'s [u8], count: usize) -> Result<&'s [u8], FontDecodeError> {
        let next = self.position.checked_add(count).filter(|next| *next <= self.end).ok_or(BAD)?;
        let bytes = stream.get(self.position..next).ok_or(BAD)?;
        self.position = next;
        Ok(bytes)
    }

    fn u16(&mut self, stream: &[u8]) -> Result<u16, FontDecodeError> {
        let bytes = self.take(stream, 2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u255(&mut self, stream: &[u8]) -> Result<u16, FontDecodeError> {
        let limit = stream.get(..self.end).ok_or(BAD)?;
        let mut reader = Reader::new(limit, self.position);
        let value = reader.u255().map_err(|_| BAD)?;
        self.position = reader.position;
        Ok(value)
    }
}

/// Decodes one point of the triplet encoding.
#[inline]
fn decode_triplet(flag: u8, data: &[u8]) -> (i32, i32) {
    let with_sign = |flag: u8, magnitude: i32| if flag & 1 != 0 { magnitude } else { -magnitude };
    let flag_value = flag as i32;
    match flag {
        0..=9 => (0, with_sign(flag, ((flag_value & 14) << 7) + data[0] as i32)),
        10..=19 => (with_sign(flag, (((flag_value - 10) & 14) << 7) + data[0] as i32), 0),
        20..=83 => {
            let (high, low) = (flag_value - 20, data[0] as i32);
            (
                with_sign(flag, 1 + (high & 0x30) + (low >> 4)),
                with_sign(flag >> 1, 1 + ((high & 0x0C) << 2) + (low & 0x0F)),
            )
        }
        84..=119 => {
            let high = flag_value - 84;
            (
                with_sign(flag, 1 + ((high / 12) << 8) + data[0] as i32),
                with_sign(flag >> 1, 1 + (((high % 12) >> 2) << 8) + data[1] as i32),
            )
        }
        120..=123 => {
            let middle = data[1] as i32;
            (
                with_sign(flag, ((data[0] as i32) << 4) + (middle >> 4)),
                with_sign(flag >> 1, ((middle & 0x0F) << 8) + data[2] as i32),
            )
        }
        _ => (
            with_sign(flag, ((data[0] as i32) << 8) + data[1] as i32),
            with_sign(flag >> 1, ((data[2] as i32) << 8) + data[3] as i32),
        ),
    }
}

#[cfg(test)]
pub(super) fn decode_triplet_for_test(flag: u8, data: &[u8]) -> (i32, i32) {
    decode_triplet(flag, data)
}

fn triplet_bytes(flag: u8) -> usize {
    match flag {
        0..=83 => 1,
        84..=119 => 2,
        120..=123 => 3,
        _ => 4,
    }
}

#[derive(Clone, Copy)]
struct Point {
    x: i16,
    y: i16,
    on_curve: bool,
}

/// Rebuilds the glyphs one at a time.
pub(super) struct GlyphRebuilder {
    glyph_count: usize,
    long_loca: bool,
    n_contours: Cursor,
    n_points: Cursor,
    flags: Cursor,
    glyphs: Cursor,
    composites: Cursor,
    bbox_bitmap: Range<usize>,
    bboxes: Cursor,
    instructions: Cursor,
    overlap: Option<Range<usize>>,
    next: usize,
    output: Vec<u8>,
    loca: Vec<u32>,
    x_mins: Vec<i16>,
    points: Vec<Point>,
    max_output: usize,
}

impl GlyphRebuilder {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        stream: &[u8],
        offset: usize,
        length: usize,
        loca_length: usize,
        maxp_glyphs: u16,
        head_loca_format: i16,
        original_length: usize,
        max_output: usize,
    ) -> Result<Self, FontDecodeError> {
        let end = offset.checked_add(length).filter(|end| *end <= stream.len()).ok_or(BAD)?;
        let table = &stream[offset..end];
        let mut reader = Reader::new(table, 0);
        if reader.u16().map_err(|_| BAD)? != 0 {
            return Err(BAD);
        }
        let options = reader.u16().map_err(|_| BAD)?;
        let glyph_count = reader.u16().map_err(|_| BAD)?;
        let index_format = reader.u16().map_err(|_| BAD)?;
        if glyph_count != maxp_glyphs || glyph_count == 0 || index_format > 1 {
            return Err(BAD);
        }
        if index_format as i16 != head_loca_format {
            return Err(BAD);
        }
        let long_loca = index_format == 1;
        let expected_loca = (glyph_count as usize + 1) * if long_loca { 4 } else { 2 };
        if loca_length != expected_loca {
            return Err(FontDecodeError::Malformed("WOFF2 loca length"));
        }
        let mut sizes = [0usize; 7];
        for size in &mut sizes {
            *size = reader.u32().map_err(|_| BAD)? as usize;
        }
        let mut position = offset + reader.position;
        let mut next_range = |size: usize| -> Result<Range<usize>, FontDecodeError> {
            let start = position;
            position = position.checked_add(size).filter(|next| *next <= end).ok_or(BAD)?;
            Ok(start..position)
        };
        let cursor = |range: Range<usize>| Cursor { position: range.start, end: range.end };
        let n_contours = cursor(next_range(sizes[0])?);
        let n_points = cursor(next_range(sizes[1])?);
        let flags = cursor(next_range(sizes[2])?);
        let glyphs = cursor(next_range(sizes[3])?);
        let composites = cursor(next_range(sizes[4])?);
        let bbox_range = next_range(sizes[5])?;
        let instructions = cursor(next_range(sizes[6])?);
        let overlap = if options & 1 != 0 { Some(next_range((glyph_count as usize).div_ceil(8))?) } else { None };
        let bitmap_length = ((glyph_count as usize + 31) >> 5) << 2;
        if bbox_range.len() < bitmap_length {
            return Err(BAD);
        }
        let bbox_bitmap = bbox_range.start..bbox_range.start + bitmap_length;
        let bboxes = Cursor { position: bbox_bitmap.end, end: bbox_range.end };
        Ok(Self {
            glyph_count: glyph_count as usize,
            long_loca,
            n_contours,
            n_points,
            flags,
            glyphs,
            composites,
            bbox_bitmap,
            bboxes,
            instructions,
            overlap,
            next: 0,
            output: Vec::with_capacity(original_length.min(max_output)),
            loca: Vec::with_capacity(glyph_count as usize + 1),
            x_mins: Vec::with_capacity(glyph_count as usize),
            points: Vec::new(),
            max_output,
        })
    }

    fn has_explicit_bbox(&self, stream: &[u8], glyph: usize) -> Result<bool, FontDecodeError> {
        let byte = stream.u8_at(self.bbox_bitmap.start + (glyph >> 3)).ok_or(BAD)?;
        Ok(byte & (0x80 >> (glyph & 7)) != 0)
    }

    fn has_overlap(&self, stream: &[u8], glyph: usize) -> Result<bool, FontDecodeError> {
        match &self.overlap {
            Some(range) => {
                let byte = stream.u8_at(range.start + (glyph >> 3)).ok_or(BAD)?;
                Ok(byte & (0x80 >> (glyph & 7)) != 0)
            }
            None => Ok(false),
        }
    }

    /// Rebuilds glyphs until the budget is spent. Returns whether all of them are done.
    pub(super) fn step(&mut self, stream: &[u8], budget: &mut usize) -> Result<bool, FontDecodeError> {
        while self.next < self.glyph_count {
            if *budget == 0 {
                return Ok(false);
            }
            let before = self.output.len();
            self.loca.push(u32::try_from(before).map_err(|_| BAD)?);
            let glyph = self.next;
            let contours = self.n_contours.u16(stream)? as i16;
            let x_min = match contours {
                0 => {
                    if self.has_explicit_bbox(stream, glyph)? {
                        return Err(BAD);
                    }
                    0
                }
                -1 => self.rebuild_composite(stream, glyph)?,
                count if count > 0 => self.rebuild_simple(stream, glyph, count as usize)?,
                _ => return Err(BAD),
            };
            self.x_mins.push(x_min);
            // `loca` of the short format holds offsets divided by two.
            let alignment = if self.long_loca { 4 } else { 2 };
            let padded = self.output.len().checked_add(alignment - 1).ok_or(BAD)? & !(alignment - 1);
            if padded > self.max_output {
                return Err(FontDecodeError::LimitExceeded("decoded size"));
            }
            self.output.resize(padded, 0);
            *budget = budget.saturating_sub((padded - before).max(16));
            self.next += 1;
        }
        Ok(true)
    }

    fn write_bbox(&mut self, stream: &[u8]) -> Result<i16, FontDecodeError> {
        let bbox = self.bboxes.take(stream, 8)?;
        self.output.extend_from_slice(bbox);
        Ok(i16::from_be_bytes([bbox[0], bbox[1]]))
    }

    fn rebuild_composite(&mut self, stream: &[u8], glyph: usize) -> Result<i16, FontDecodeError> {
        if !self.has_explicit_bbox(stream, glyph)? {
            return Err(BAD);
        }
        self.output.extend_from_slice(&(-1i16).to_be_bytes());
        let x_min = self.write_bbox(stream)?;
        let mut has_instructions = false;
        loop {
            let flags = self.composites.u16(stream)?;
            has_instructions |= flags & WE_HAVE_INSTRUCTIONS != 0;
            let mut size = 2 + if flags & ARG_1_AND_2_ARE_WORDS != 0 { 4 } else { 2 };
            size += if flags & WE_HAVE_A_SCALE != 0 {
                2
            } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                4
            } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                8
            } else {
                0
            };
            self.output.extend_from_slice(&flags.to_be_bytes());
            let record = self.composites.take(stream, size)?;
            self.output.extend_from_slice(record);
            if flags & MORE_COMPONENTS == 0 {
                break;
            }
        }
        if has_instructions {
            let length = self.glyphs.u255(stream)? as usize;
            let instructions = self.instructions.take(stream, length)?;
            self.output.extend_from_slice(&(length as u16).to_be_bytes());
            self.output.extend_from_slice(instructions);
        }
        self.check_size()?;
        Ok(x_min)
    }

    fn check_size(&self) -> Result<(), FontDecodeError> {
        if self.output.len() > self.max_output {
            return Err(FontDecodeError::LimitExceeded("decoded size"));
        }
        Ok(())
    }

    fn rebuild_simple(&mut self, stream: &[u8], glyph: usize, contours: usize) -> Result<i16, FontDecodeError> {
        let start = self.output.len();
        self.output.extend_from_slice(&(contours as i16).to_be_bytes());
        // Room for the bounding box, written once the points are known.
        self.output.extend_from_slice(&[0; 8]);
        let mut total = 0usize;
        for _ in 0..contours {
            let count = self.n_points.u255(stream)? as usize;
            if count == 0 {
                return Err(BAD);
            }
            total += count;
            if total > 0x1_0000 {
                return Err(BAD);
            }
            self.output.extend_from_slice(&((total - 1) as u16).to_be_bytes());
        }
        let flags = self.flags.take(stream, total)?;
        self.points.clear();
        let (mut x, mut y) = (0i32, 0i32);
        for &flag in flags {
            let on_curve = flag & 0x80 == 0;
            let index = flag & 0x7F;
            let data = self.glyphs.take(stream, triplet_bytes(index))?;
            let (dx, dy) = decode_triplet(index, data);
            x += dx;
            y += dy;
            let (Ok(px), Ok(py)) = (i16::try_from(x), i16::try_from(y)) else {
                return Err(BAD);
            };
            self.points.push(Point { x: px, y: py, on_curve });
        }
        let length = self.glyphs.u255(stream)? as usize;
        let instructions = self.instructions.take(stream, length)?;
        self.output.extend_from_slice(&(length as u16).to_be_bytes());
        self.output.extend_from_slice(instructions);
        let overlap = self.has_overlap(stream, glyph)?;
        write_points(&self.points, overlap, &mut self.output)?;
        self.check_size()?;

        let x_min = if self.has_explicit_bbox(stream, glyph)? {
            let bbox = self.bboxes.take(stream, 8)?;
            self.output[start + 2..start + 10].copy_from_slice(bbox);
            i16::from_be_bytes([bbox[0], bbox[1]])
        } else {
            let (mut x_min, mut y_min, mut x_max, mut y_max) = (i16::MAX, i16::MAX, i16::MIN, i16::MIN);
            for point in &self.points {
                x_min = x_min.min(point.x);
                y_min = y_min.min(point.y);
                x_max = x_max.max(point.x);
                y_max = y_max.max(point.y);
            }
            for (index, value) in [x_min, y_min, x_max, y_max].into_iter().enumerate() {
                self.output[start + 2 + index * 2..start + 4 + index * 2].copy_from_slice(&value.to_be_bytes());
            }
            x_min
        };
        Ok(x_min)
    }

    /// The rebuilt `glyf`, `loca`, and the `xMin` of each glyph.
    pub(super) fn finish(mut self) -> Result<(Vec<u8>, Vec<u8>, Vec<i16>), FontDecodeError> {
        self.loca.push(u32::try_from(self.output.len()).map_err(|_| BAD)?);
        let mut loca = Vec::with_capacity(self.loca.len() * if self.long_loca { 4 } else { 2 });
        for offset in &self.loca {
            if self.long_loca {
                loca.extend_from_slice(&offset.to_be_bytes());
            } else {
                let half = u16::try_from(offset / 2).map_err(|_| FontDecodeError::Malformed("WOFF2 loca overflow"))?;
                loca.extend_from_slice(&half.to_be_bytes());
            }
        }
        Ok((self.output, loca, self.x_mins))
    }
}

/// Writes flags and coordinates in the compact form of TrueType.
fn write_points(points: &[Point], overlap: bool, output: &mut Vec<u8>) -> Result<(), FontDecodeError> {
    let (mut x_bytes, mut y_bytes) = (Vec::with_capacity(points.len()), Vec::with_capacity(points.len()));
    let (mut last_x, mut last_y) = (0i32, 0i32);
    let mut last_flag: Option<u8> = None;
    let mut repeat = 0u8;
    for (index, point) in points.iter().enumerate() {
        let mut flag = if point.on_curve { ON_CURVE } else { 0 };
        if index == 0 && overlap {
            flag |= OVERLAP_SIMPLE;
        }
        let (dx, dy) = (point.x as i32 - last_x, point.y as i32 - last_y);
        if !(-0x8000..=0x7FFF).contains(&dx) || !(-0x8000..=0x7FFF).contains(&dy) {
            return Err(BAD);
        }
        if dx == 0 {
            flag |= X_SAME;
        } else if (-255..=255).contains(&dx) {
            flag |= X_SHORT | if dx > 0 { X_SAME } else { 0 };
            x_bytes.push(dx.unsigned_abs() as u8);
        } else {
            x_bytes.extend_from_slice(&(dx as i16).to_be_bytes());
        }
        if dy == 0 {
            flag |= Y_SAME;
        } else if (-255..=255).contains(&dy) {
            flag |= Y_SHORT | if dy > 0 { Y_SAME } else { 0 };
            y_bytes.push(dy.unsigned_abs() as u8);
        } else {
            y_bytes.extend_from_slice(&(dy as i16).to_be_bytes());
        }
        if last_flag == Some(flag) && repeat != 255 {
            if let Some(previous) = output.last_mut() {
                if repeat == 0 {
                    *previous |= REPEAT;
                }
            }
            repeat += 1;
        } else {
            if repeat != 0 {
                output.push(repeat);
                repeat = 0;
            }
            output.push(flag);
        }
        last_flag = Some(flag);
        last_x = point.x as i32;
        last_y = point.y as i32;
    }
    if repeat != 0 {
        output.push(repeat);
    }
    output.extend_from_slice(&x_bytes);
    output.extend_from_slice(&y_bytes);
    Ok(())
}
