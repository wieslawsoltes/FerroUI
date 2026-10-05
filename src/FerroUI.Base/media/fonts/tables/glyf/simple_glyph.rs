use super::glyph_flag::GlyphFlag;

/// A decoded simple (non-composite) glyph outline.
///
/// The default value is the empty outline, which is also what malformed or
/// truncated glyph data decodes to.
#[derive(Clone, Debug, Default)]
pub struct SimpleGlyph<'a> {
    end_pts_of_contours: Vec<u16>,
    instructions: &'a [u8],
    flags: Vec<GlyphFlag>,
    x_coordinates: Vec<i16>,
    y_coordinates: Vec<i16>,
}

impl<'a> SimpleGlyph<'a> {
    pub fn end_pts_of_contours(&self) -> &[u16] {
        &self.end_pts_of_contours
    }

    pub fn instructions(&self) -> &'a [u8] {
        self.instructions
    }

    pub fn flags(&self) -> &[GlyphFlag] {
        &self.flags
    }

    pub fn x_coordinates(&self) -> &[i16] {
        &self.x_coordinates
    }

    pub fn y_coordinates(&self) -> &[i16] {
        &self.y_coordinates
    }

    /// Decodes the glyph body that follows the 10 byte glyph header.
    pub fn create(data: &'a [u8], number_of_contours: i32) -> SimpleGlyph<'a> {
        Self::try_create(data, number_of_contours).unwrap_or_default()
    }

    fn try_create(data: &'a [u8], number_of_contours: i32) -> Option<SimpleGlyph<'a>> {
        if number_of_contours <= 0 {
            return None;
        }

        let number_of_contours = number_of_contours as usize;

        // The contour count comes from the untrusted glyph header; a body too short to hold
        // the endpoint array plus the instruction-length field degrades to an empty outline.
        if number_of_contours * 2 + 2 > data.len() {
            return None;
        }

        // Endpoints of contours
        let mut end_pts_of_contours = Vec::with_capacity(number_of_contours);

        let mut previous_end_pt = 0u16;

        for i in 0..number_of_contours {
            let end_pt = read_uint16(data, i * 2)?;

            // Contour endpoints must be strictly increasing: numPoints below is derived from
            // the last endpoint alone, so an out-of-order endpoint would send every consumer
            // walk indexing past the point buffers. Reject the glyph up front instead — the
            // default value renders as an empty outline.
            if i > 0 && end_pt <= previous_end_pt {
                return None;
            }

            end_pts_of_contours.push(end_pt);
            previous_end_pt = end_pt;
        }

        // Instructions
        let instructions_offset = number_of_contours * 2;
        let instructions_length = read_uint16(data, instructions_offset)? as usize;

        // The declared instruction run must also fit — its length is untrusted too.
        let flags_offset = instructions_offset + 2 + instructions_length;

        let instructions = data.get(instructions_offset + 2..flags_offset)?;

        // Number of points
        let num_points = previous_end_pt as usize + 1;
        let mut flags = Vec::with_capacity(num_points);

        // Decode flags. The flag and coordinate streams are variable-length, so every
        // read is bounds-checked: running out of data mid-decode degrades to an empty
        // outline instead of failing.
        let mut offset = flags_offset;

        while flags.len() < num_points {
            let flag = GlyphFlag::from_bits_retain(*data.get(offset)?);
            offset += 1;
            flags.push(flag);

            // Repeat flag
            if flag.contains(GlyphFlag::Repeat) {
                // Read repeat count
                let repeat_count = *data.get(offset)?;
                offset += 1;

                // Repeat the flag
                for _ in 0..repeat_count {
                    if flags.len() >= num_points {
                        break;
                    }

                    flags.push(flag);
                }
            }
        }

        let x_coordinates = Self::decode_coordinates(
            data,
            &mut offset,
            &flags,
            GlyphFlag::XShortVector,
            GlyphFlag::XIsSameOrPositiveXShortVector,
        )?;

        let y_coordinates = Self::decode_coordinates(
            data,
            &mut offset,
            &flags,
            GlyphFlag::YShortVector,
            GlyphFlag::YIsSameOrPositiveYShortVector,
        )?;

        Some(SimpleGlyph { end_pts_of_contours, instructions, flags, x_coordinates, y_coordinates })
    }

    /// Decodes one coordinate stream (deltas accumulated into absolute values).
    fn decode_coordinates(
        data: &[u8],
        offset: &mut usize,
        flags: &[GlyphFlag],
        short_vector: GlyphFlag,
        is_same_or_positive_short_vector: GlyphFlag,
    ) -> Option<Vec<i16>> {
        let mut coordinates = Vec::with_capacity(flags.len());
        let mut value = 0i16;

        for flag in flags {
            if flag.contains(short_vector) {
                // Short vector
                let delta = *data.get(*offset)? as i16;
                *offset += 1;

                if flag.contains(is_same_or_positive_short_vector) {
                    value = value.wrapping_add(delta);
                } else {
                    value = value.wrapping_sub(delta);
                }
            } else if !flag.contains(is_same_or_positive_short_vector) {
                // Not a short vector
                let delta = read_uint16(data, *offset)? as i16;
                *offset += 2;
                value = value.wrapping_add(delta);
            }

            coordinates.push(value);
        }

        Some(coordinates)
    }
}

#[inline]
fn read_uint16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset.checked_add(2)?)?;

    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_u16(data: &mut Vec<u8>, value: u16) {
        data.extend_from_slice(&value.to_be_bytes());
    }

    fn write_i16(data: &mut Vec<u8>, value: i16) {
        data.extend_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn create_returns_default_when_endpoint_array_is_truncated() {
        // Four contours promise 8 bytes of endpoints (plus the instruction length field);
        // only 4 bytes are present.
        let data = [0u8; 4];
        let glyph = SimpleGlyph::create(&data, 4);

        assert_eq!(glyph.end_pts_of_contours().len(), 0);
        assert_eq!(glyph.flags().len(), 0);
    }

    #[test]
    fn create_returns_default_when_instructions_are_truncated() {
        let mut data = Vec::new();
        write_u16(&mut data, 0); // endPtsOfContours[0] -> 1 point
        write_u16(&mut data, 100); // instructionLength promises 100 bytes; none follow

        let glyph = SimpleGlyph::create(&data, 1);

        assert_eq!(glyph.end_pts_of_contours().len(), 0);
    }

    #[test]
    fn create_returns_default_when_flags_are_truncated() {
        let mut data = Vec::new();
        write_u16(&mut data, 2); // endPtsOfContours[0] -> 3 points
        write_u16(&mut data, 0); // instructionLength
        data.push(GlyphFlag::OnCurvePoint.bits()); // one flag; two more points need flags

        let glyph = SimpleGlyph::create(&data, 1);

        assert_eq!(glyph.end_pts_of_contours().len(), 0);
    }

    #[test]
    fn create_returns_default_when_coordinates_are_truncated() {
        let mut data = Vec::new();
        write_u16(&mut data, 0); // endPtsOfContours[0] -> 1 point
        write_u16(&mut data, 0); // instructionLength
        data.push(GlyphFlag::XShortVector.bits()); // the point promises a 1-byte x delta; none follows

        let glyph = SimpleGlyph::create(&data, 1);

        assert_eq!(glyph.end_pts_of_contours().len(), 0);
    }

    #[test]
    fn create_parses_a_minimal_complete_glyph() {
        let mut data = Vec::new();
        write_u16(&mut data, 1); // endPtsOfContours[0] -> 2 points
        write_u16(&mut data, 0); // instructionLength
        data.push(GlyphFlag::OnCurvePoint.bits());
        data.push(GlyphFlag::OnCurvePoint.bits());
        write_i16(&mut data, 10); // x deltas
        write_i16(&mut data, 20);
        write_i16(&mut data, 30); // y deltas
        write_i16(&mut data, 40);

        let glyph = SimpleGlyph::create(&data, 1);

        assert_eq!(glyph.end_pts_of_contours().len(), 1);
        assert_eq!(glyph.flags().len(), 2);
        assert_eq!(glyph.x_coordinates()[0], 10);
        assert_eq!(glyph.x_coordinates()[1], 30);
        assert_eq!(glyph.y_coordinates()[0], 30);
        assert_eq!(glyph.y_coordinates()[1], 70);
    }

    #[test]
    fn create_returns_default_for_non_positive_contour_counts() {
        let data = [0u8; 16];

        assert!(SimpleGlyph::create(&data, 0).flags().is_empty());
        assert!(SimpleGlyph::create(&data, -1).flags().is_empty());
        assert!(SimpleGlyph::create(&data, i32::MAX).flags().is_empty());
    }

    #[test]
    fn create_rejects_non_increasing_endpoints() {
        let mut data = Vec::new();
        write_u16(&mut data, 3);
        write_u16(&mut data, 3); // not strictly increasing
        write_u16(&mut data, 0);
        data.extend_from_slice(&[0x31; 8]);

        assert!(SimpleGlyph::create(&data, 2).end_pts_of_contours().is_empty());
    }

    #[test]
    fn create_decodes_repeats_short_vectors_and_instructions() {
        let mut data = Vec::new();
        write_u16(&mut data, 3); // 4 points
        write_u16(&mut data, 2); // instructionLength
        data.extend_from_slice(&[0xAA, 0xBB]);

        // Point 0: short positive x (5), y is same (no data).
        let first = GlyphFlag::OnCurvePoint
            | GlyphFlag::XShortVector
            | GlyphFlag::XIsSameOrPositiveXShortVector
            | GlyphFlag::YIsSameOrPositiveYShortVector;
        // Points 1..3: short negative x, short positive y, repeated twice.
        let rest = GlyphFlag::XShortVector
            | GlyphFlag::YShortVector
            | GlyphFlag::YIsSameOrPositiveYShortVector
            | GlyphFlag::Repeat;

        data.push(first.bits());
        data.push(rest.bits());
        data.push(2); // repeat count
        data.extend_from_slice(&[5, 1, 2, 3]); // x deltas
        data.extend_from_slice(&[10, 20, 30]); // y deltas

        let glyph = SimpleGlyph::create(&data, 1);

        assert_eq!(glyph.instructions(), [0xAA, 0xBB]);
        assert_eq!(glyph.flags().len(), 4);
        assert!(glyph.flags()[0].contains(GlyphFlag::OnCurvePoint));
        assert!(!glyph.flags()[3].contains(GlyphFlag::OnCurvePoint));
        assert_eq!(glyph.x_coordinates(), [5, 4, 2, -1]);
        assert_eq!(glyph.y_coordinates(), [0, 10, 30, 60]);
    }
}
