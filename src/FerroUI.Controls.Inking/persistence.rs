//! Versioned, bounded native ink document encoding. This format preserves all
//! portable stroke telemetry; it is deliberately distinct from Windows ISF.

use super::ink_drawing_attributes::{InkDrawingAttributes, InkDrawingAttributesKind, PenTipShape};
use super::ink_point::{InkError, InkPoint};
use super::ink_stroke::InkStroke;
use ferroui_base::media::Color;
use ferroui_base::{Matrix, Point, Size};
use std::collections::HashSet;
use std::rc::Rc;

pub const INK_MIME_TYPE: &str = "application/x-ferroui-ink";
const MAGIC: &[u8; 8] = b"FERROINK";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 20;
const POINT_SIZE: usize = 40;
const STROKE_FIXED_SIZE: usize = 136;

/// Codec resource limits apply before allocation and cumulatively across
/// records. The defaults permit a 64 MiB payload, 100,000 strokes, 65,536
/// samples per stroke and 1,000,000 samples in total.
///
/// These limits bound serialized input and raw telemetry. Stroke construction
/// and collection admission additionally reserve derived geometry against
/// [`InkStroke::MAX_RENDERED_VERTICES`] and
/// [`super::InkStrokeContainer::MAX_RENDERED_VERTICES`]. Changing codec limits
/// does not raise those limits or the collection's raw sample limits. Geometry
/// rejection returns [`InkError::LimitExceeded`] before the collection changes.
/// Cached raw/smoothed samples, contour vectors and renderer-owned geometry
/// occupy separate allocations; the serialized byte cap is not a total heap
/// memory cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InkPersistenceLimits {
    pub max_bytes: usize,
    pub max_strokes: usize,
    pub max_points_per_stroke: usize,
    pub max_total_points: usize,
}
impl InkPersistenceLimits {
    pub const DEFAULT: Self = Self {
        max_bytes: 64 * 1024 * 1024,
        max_strokes: 100_000,
        max_points_per_stroke: 65_536,
        max_total_points: 1_000_000,
    };
}
impl Default for InkPersistenceLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub(crate) fn encode(
    strokes: &[Rc<InkStroke>],
    limits: InkPersistenceLimits,
) -> Result<Vec<u8>, InkError> {
    if strokes.len() > limits.max_strokes || strokes.len() > u32::MAX as usize {
        return Err(InkError::LimitExceeded);
    }
    let mut size = HEADER_SIZE + 4;
    let mut total_points = 0usize;
    let mut seen = HashSet::with_capacity(strokes.len());
    for s in strokes {
        if !seen.insert(s.id()) {
            return Err(InkError::DuplicateStrokeId(s.id()));
        }
        let n = s.ink_points().len();
        total_points = total_points.checked_add(n).ok_or(InkError::LimitExceeded)?;
        size = size
            .checked_add(4 + STROKE_FIXED_SIZE)
            .and_then(|v| n.checked_mul(POINT_SIZE).and_then(|n| v.checked_add(n)))
            .ok_or(InkError::LimitExceeded)?;
        if n > limits.max_points_per_stroke || total_points > limits.max_total_points {
            return Err(InkError::LimitExceeded);
        }
    }
    if size > limits.max_bytes || size - HEADER_SIZE > u32::MAX as usize {
        return Err(InkError::LimitExceeded);
    }
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(MAGIC);
    write_u32(&mut out, VERSION);
    write_u32(&mut out, (size - HEADER_SIZE) as u32);
    write_u32(&mut out, 0);
    write_u32(&mut out, strokes.len() as u32);
    for s in strokes {
        let a = s.drawing_attributes();
        write_u32(
            &mut out,
            (STROKE_FIXED_SIZE + s.ink_points().len() * POINT_SIZE) as u32,
        );
        write_u32(&mut out, s.id());
        out.push(u8::from(s.selected()));
        out.push(
            u8::from(a.draw_as_highlighter)
                | (u8::from(a.ignore_pressure) << 1)
                | (u8::from(a.fit_to_curve) << 2)
                | (u8::from(a.ignore_tilt) << 3),
        );
        out.push(a.pen_tip as u8);
        out.push(a.kind as u8);
        out.extend_from_slice(&[a.color.a, a.color.r, a.color.g, a.color.b]);
        write_f64(&mut out, a.size.width);
        write_f64(&mut out, a.size.height);
        write_f64(&mut out, a.pencil_opacity);
        write_matrix(&mut out, a.pen_tip_transform);
        write_matrix(&mut out, s.point_transform());
        write_u32(&mut out, s.ink_points().len() as u32);
        for p in s.ink_points() {
            write_f64(&mut out, p.position.x);
            write_f64(&mut out, p.position.y);
            for value in [p.pressure, p.tilt_x, p.tilt_y, p.twist] {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&p.timestamp.to_le_bytes());
        }
    }
    debug_assert_eq!(out.len(), size);
    let checksum = crc32(&out[HEADER_SIZE..]);
    out[16..20].copy_from_slice(&checksum.to_le_bytes());
    Ok(out)
}

pub(crate) fn decode(
    data: &[u8],
    limits: InkPersistenceLimits,
) -> Result<Vec<Rc<InkStroke>>, InkError> {
    if data.len() > limits.max_bytes {
        return Err(InkError::LimitExceeded);
    }
    let mut reader = Reader::new(data);
    if reader.take(8)? != MAGIC {
        return Err(InkError::UnsupportedFormat);
    }
    if reader.u32()? != VERSION {
        return Err(InkError::UnsupportedFormat);
    }
    let length = reader.u32()? as usize;
    let checksum = reader.u32()?;
    if length != reader.remaining() {
        return Err(InkError::InvalidData("payload length mismatch"));
    }
    if checksum != crc32(&data[HEADER_SIZE..]) {
        return Err(InkError::InvalidData("checksum mismatch"));
    }
    let count = reader.u32()? as usize;
    if count > limits.max_strokes {
        return Err(InkError::LimitExceeded);
    }
    if count > reader.remaining() / (4 + STROKE_FIXED_SIZE + POINT_SIZE) {
        return Err(InkError::InvalidData(
            "stroke count exceeds remaining bytes",
        ));
    }
    let mut result = Vec::with_capacity(count);
    let mut seen = HashSet::with_capacity(count);
    let mut total_points = 0usize;
    for _ in 0..count {
        let record_len = reader.u32()? as usize;
        let mut record = Reader::new(reader.take(record_len)?);
        let id = record.u32()?;
        if !seen.insert(id) {
            return Err(InkError::DuplicateStrokeId(id));
        }
        let selected = match record.byte()? {
            0 => false,
            1 => true,
            _ => return Err(InkError::InvalidData("invalid selection flag")),
        };
        let flags = record.byte()?;
        if flags & !15 != 0 {
            return Err(InkError::InvalidData("unknown drawing flags"));
        }
        let pen_tip = match record.byte()? {
            0 => PenTipShape::Circle,
            1 => PenTipShape::Rectangle,
            _ => return Err(InkError::InvalidData("unknown pen tip")),
        };
        let kind = match record.byte()? {
            0 => InkDrawingAttributesKind::Default,
            1 => InkDrawingAttributesKind::Pencil,
            _ => return Err(InkError::InvalidData("unknown drawing kind")),
        };
        let color = record.take(4)?;
        let color = Color::from_argb(color[0], color[1], color[2], color[3]);
        let size = Size::new(record.f64()?, record.f64()?);
        let pencil_opacity = record.f64()?;
        let pen_tip_transform = record.matrix()?;
        let transform = record.matrix()?;
        let count = record.u32()? as usize;
        total_points = total_points
            .checked_add(count)
            .ok_or(InkError::LimitExceeded)?;
        if count > limits.max_points_per_stroke || total_points > limits.max_total_points {
            return Err(InkError::LimitExceeded);
        }
        if count == 0 || count.checked_mul(POINT_SIZE) != Some(record.remaining()) {
            return Err(InkError::InvalidData(
                "point count does not match record length",
            ));
        }
        let mut points = Vec::with_capacity(count);
        for _ in 0..count {
            points.push(InkPoint {
                position: Point::new(record.f64()?, record.f64()?),
                pressure: record.f32()?,
                tilt_x: record.f32()?,
                tilt_y: record.f32()?,
                twist: record.f32()?,
                timestamp: record.u64()?,
            });
        }
        let attributes = InkDrawingAttributes {
            color,
            size,
            pen_tip,
            pen_tip_transform,
            draw_as_highlighter: flags & 1 != 0,
            ignore_pressure: flags & 2 != 0,
            fit_to_curve: flags & 4 != 0,
            ignore_tilt: flags & 8 != 0,
            kind,
            pencil_opacity,
        };
        result.push(InkStroke::restore(
            id, points, attributes, transform, selected,
        )?);
    }
    if reader.remaining() != 0 {
        return Err(InkError::InvalidData("trailing data"));
    }
    Ok(result)
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn write_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn write_matrix(out: &mut Vec<u8>, value: Matrix) {
    for v in [
        value.m11, value.m12, value.m21, value.m22, value.m31, value.m32,
    ] {
        write_f64(out, v);
    }
}
struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
    fn remaining(&self) -> usize {
        self.data.len() - self.position
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], InkError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(InkError::InvalidData("record length overflow"))?;
        let result = self
            .data
            .get(self.position..end)
            .ok_or(InkError::InvalidData("truncated document"))?;
        self.position = end;
        Ok(result)
    }
    fn byte(&mut self) -> Result<u8, InkError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, InkError> {
        let mut b = [0; 4];
        b.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(b))
    }
    fn u64(&mut self) -> Result<u64, InkError> {
        let mut b = [0; 8];
        b.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(b))
    }
    fn f32(&mut self) -> Result<f32, InkError> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn f64(&mut self) -> Result<f64, InkError> {
        Ok(f64::from_bits(self.u64()?))
    }
    fn matrix(&mut self) -> Result<Matrix, InkError> {
        Ok(Matrix::new(
            self.f64()?,
            self.f64()?,
            self.f64()?,
            self.f64()?,
            self.f64()?,
            self.f64()?,
        ))
    }
}

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut value = i as u32;
        let mut j = 0;
        while j < 8 {
            value = if value & 1 != 0 {
                0xedb8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
            j += 1;
        }
        table[i] = value;
        i += 1;
    }
    table
}
const CRC_TABLE: [u32; 256] = crc_table();
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc = CRC_TABLE[((crc ^ u32::from(*byte)) & 255) as usize] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stroke() -> Rc<InkStroke> {
        let p = InkPoint {
            position: Point::new(10.0, -4.0),
            pressure: 0.75,
            tilt_x: 30.0,
            tilt_y: -20.0,
            twist: 359.0,
            timestamp: 12345,
        };
        let a = InkDrawingAttributes {
            color: Color::from_argb(128, 255, 80, 20),
            size: Size::new(7.0, 3.0),
            kind: InkDrawingAttributesKind::Pencil,
            pencil_opacity: 0.65,
            ..Default::default()
        };
        let s = InkStroke::new(vec![p], a).unwrap();
        s.set_selected(true);
        s.set_point_transform(Matrix::create_translation(4.0, 9.0))
            .unwrap();
        s
    }
    #[test]
    fn round_trip_preserves_all_samples_and_attributes() {
        let s = stroke();
        let bytes = encode(&[s.clone()], InkPersistenceLimits::DEFAULT).unwrap();
        let loaded = decode(&bytes, InkPersistenceLimits::DEFAULT).unwrap();
        let copy = &loaded[0];
        assert_eq!(copy.id(), s.id());
        assert_eq!(copy.ink_points(), s.ink_points());
        assert_eq!(copy.drawing_attributes(), s.drawing_attributes());
        assert_eq!(copy.point_transform(), s.point_transform());
        assert!(copy.selected());
    }
    #[test]
    fn every_truncation_is_rejected() {
        let bytes = encode(&[stroke()], InkPersistenceLimits::DEFAULT).unwrap();
        for n in 0..bytes.len() {
            assert!(
                decode(&bytes[..n], InkPersistenceLimits::DEFAULT).is_err(),
                "length {n}"
            );
        }
    }
    #[test]
    fn corruption_is_detected_and_limits_precede_allocations() {
        let mut bytes = encode(&[stroke()], InkPersistenceLimits::DEFAULT).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        assert_eq!(
            decode(&bytes, InkPersistenceLimits::DEFAULT).unwrap_err(),
            InkError::InvalidData("checksum mismatch")
        );
        assert!(matches!(
            decode(
                &bytes,
                InkPersistenceLimits {
                    max_bytes: 8,
                    ..Default::default()
                }
            ),
            Err(InkError::LimitExceeded)
        ));
    }
    #[test]
    fn hostile_count_with_valid_checksum_is_rejected() {
        let mut bytes = encode(&[stroke()], InkPersistenceLimits::DEFAULT).unwrap();
        bytes[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        let crc = crc32(&bytes[HEADER_SIZE..]);
        bytes[16..20].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(
            decode(&bytes, InkPersistenceLimits::DEFAULT).unwrap_err(),
            InkError::LimitExceeded
        );
    }
    #[test]
    fn arbitrary_input_never_panics() {
        for len in 0..256 {
            let bytes: Vec<_> = (0..len)
                .map(|i| ((i * 137 + len * 19) % 256) as u8)
                .collect();
            let _ = decode(&bytes, InkPersistenceLimits::DEFAULT);
        }
    }
    #[test]
    fn empty_document_round_trips() {
        let bytes = encode(&[], InkPersistenceLimits::DEFAULT).unwrap();
        assert!(decode(&bytes, InkPersistenceLimits::DEFAULT)
            .unwrap()
            .is_empty());
    }
}
