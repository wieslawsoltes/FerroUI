use std::rc::Rc;

use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};
use crate::media::fonts::FontCodePageCoverage;
use crate::media::{FontSimulations, FontStretch, FontStyle, FontWeight};

use super::TestPlatformTypeface;

/// Builds a minimal but valid OpenType font in code: `head`, `hhea`, `hmtx`,
/// `maxp`, `cmap` (format 12), `name`, `OS/2` and `post`, plus optional
/// `GSUB` (script list only) and `meta` tables.
///
/// Glyph 0 is `.notdef`; every codepoint of the given ranges gets its own
/// glyph, numbered in range order starting at 1. All glyphs have the same
/// advance unless [`glyph_advance`](Self::glyph_advance) says otherwise.
///
/// Defaults: 1000 units per em, ascender 800, descender -200, line gap 0,
/// advance 600, weight 400, normal style and stretch, codepoints U+0020 to
/// U+007E, an en-US family name record.
#[derive(Clone, Debug)]
pub struct TestFontBuilder {
    family_name: String,
    typographic_family_name: Option<String>,
    localized_family_names: Vec<(u16, String)>,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    ranges: Vec<(u32, u32)>,
    advance: u16,
    glyph_advances: Vec<(u32, u16)>,
    units_per_em: u16,
    ascender: i16,
    descender: i16,
    line_gap: i16,
    is_fixed_pitch: bool,
    is_last_resort: bool,
    code_page_coverage: FontCodePageCoverage,
    unicode_range_bits: Vec<u32>,
    shaping_scripts: Vec<String>,
    design_languages: Option<String>,
    supported_languages: Option<String>,
}

#[allow(dead_code)] // the full builder API is kept for the text tests built on top
impl TestFontBuilder {
    pub fn new(family_name: &str) -> Self {
        Self {
            family_name: family_name.to_owned(),
            typographic_family_name: None,
            localized_family_names: Vec::new(),
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
            ranges: vec![(0x20, 0x7E)],
            advance: 600,
            glyph_advances: Vec::new(),
            units_per_em: 1000,
            ascender: 800,
            descender: -200,
            line_gap: 0,
            is_fixed_pitch: false,
            is_last_resort: false,
            code_page_coverage: FontCodePageCoverage::None,
            unicode_range_bits: Vec::new(),
            shaping_scripts: Vec::new(),
            design_languages: None,
            supported_languages: None,
        }
    }

    /// The OS/2 weight class.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Italic and oblique set the matching OS/2 selection bits (and the
    /// italic angle).
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// The OS/2 width class.
    pub fn stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// The covered codepoints as inclusive ranges; replaces the default range.
    pub fn codepoints(mut self, ranges: &[(u32, u32)]) -> Self {
        self.ranges = ranges.to_vec();
        self
    }

    /// The advance width of every glyph, in design units.
    pub fn advance(mut self, advance: u16) -> Self {
        self.advance = advance;
        self
    }

    /// The advance width of the glyph of one codepoint, in design units.
    pub fn glyph_advance(mut self, codepoint: u32, advance: u16) -> Self {
        self.glyph_advances.push((codepoint, advance));
        self
    }

    pub fn units_per_em(mut self, units_per_em: u16) -> Self {
        self.units_per_em = units_per_em;
        self
    }

    /// The `hhea` (and OS/2 typographic) ascender, descender (negative below
    /// the baseline) and line gap, in design units.
    pub fn vertical_metrics(mut self, ascender: i16, descender: i16, line_gap: i16) -> Self {
        self.ascender = ascender;
        self.descender = descender;
        self.line_gap = line_gap;
        self
    }

    pub fn fixed_pitch(mut self, is_fixed_pitch: bool) -> Self {
        self.is_fixed_pitch = is_fixed_pitch;
        self
    }

    /// Marks the font as a last resort font (`head` flag).
    pub fn last_resort(mut self, is_last_resort: bool) -> Self {
        self.is_last_resort = is_last_resort;
        self
    }

    /// The typographic family name (name id 16).
    pub fn typographic_family_name(mut self, name: &str) -> Self {
        self.typographic_family_name = Some(name.to_owned());
        self
    }

    /// An additional family name record for a Windows language id (e.g. `0x0411` for ja-JP).
    pub fn localized_family_name(mut self, lcid: u16, name: &str) -> Self {
        self.localized_family_names.push((lcid, name.to_owned()));
        self
    }

    /// The OS/2 code page range bits.
    pub fn code_page_coverage(mut self, coverage: FontCodePageCoverage) -> Self {
        self.code_page_coverage = coverage;
        self
    }

    /// Sets a bit (0..=127) of the OS/2 Unicode range fields.
    pub fn unicode_range_bit(mut self, bit: u32) -> Self {
        self.unicode_range_bits.push(bit);
        self
    }

    /// The OpenType script tags the font declares in `GSUB` (e.g. `"arab"`).
    pub fn shaping_scripts(mut self, scripts: &[&str]) -> Self {
        self.shaping_scripts = scripts.iter().map(|script| (*script).to_owned()).collect();
        self
    }

    /// The `meta` table design languages (`dlng`), e.g. `"ja"` or `"zh-Hans, ja"`.
    pub fn design_languages(mut self, languages: &str) -> Self {
        self.design_languages = Some(languages.to_owned());
        self
    }

    /// The `meta` table supported languages (`slng`).
    pub fn supported_languages(mut self, languages: &str) -> Self {
        self.supported_languages = Some(languages.to_owned());
        self
    }

    fn glyph_count(&self) -> usize {
        let codepoints: u64 =
            self.ranges.iter().map(|(start, end)| (*end as u64).saturating_sub(*start as u64) + 1).sum();

        assert!(codepoints < u16::MAX as u64, "a test font holds less than 65535 codepoints");

        codepoints as usize + 1
    }

    fn glyph_of(&self, codepoint: u32) -> Option<usize> {
        let mut first_glyph = 1usize;

        for (start, end) in &self.ranges {
            if (*start..=*end).contains(&codepoint) {
                return Some(first_glyph + (codepoint - start) as usize);
            }

            first_glyph += (end - start) as usize + 1;
        }

        None
    }

    fn build_head(&self) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        let mut mac_style = 0;

        if self.weight.value() >= 600 {
            mac_style |= 1;
        }

        if self.style == FontStyle::Italic {
            mac_style |= 2;
        }

        buffer
            .uint32(0x0001_0000) // version
            .uint32(0x0001_0000) // fontRevision
            .uint32(0) // checkSumAdjustment
            .uint32(0x5F0F_3CF5) // magicNumber
            .uint16(if self.is_last_resort { 1 << 10 } else { 0 }) // flags
            .uint16(self.units_per_em as i32)
            .zeros(16) // created, modified
            .int16(0) // xMin
            .int16(self.descender as i32) // yMin
            .int16(self.advance as i32) // xMax
            .int16(self.ascender as i32) // yMax
            .uint16(mac_style)
            .uint16(8) // lowestRecPPEM
            .int16(2) // fontDirectionHint
            .int16(0) // indexToLocFormat
            .int16(0); // glyphDataFormat

        buffer.to_array()
    }

    fn build_hhea(&self, number_of_h_metrics: usize) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer
            .uint32(0x0001_0000)
            .int16(self.ascender as i32)
            .int16(self.descender as i32)
            .int16(self.line_gap as i32)
            .uint16(self.advance as i32) // advanceWidthMax
            .int16(0) // minLeftSideBearing
            .int16(0) // minRightSideBearing
            .int16(self.advance as i32) // xMaxExtent
            .int16(1) // caretSlopeRise
            .int16(0) // caretSlopeRun
            .int16(0) // caretOffset
            .zeros(8)
            .int16(0) // metricDataFormat
            .uint16(number_of_h_metrics as i32);

        buffer.to_array()
    }

    fn build_hmtx(&self, glyph_count: usize) -> (Vec<u8>, usize) {
        let mut buffer = BigEndianBuffer::new();

        if self.glyph_advances.is_empty() {
            // One advance shared by all glyphs, then the left side bearings of the others.
            buffer.uint16(self.advance as i32).int16(0);
            buffer.zeros((glyph_count - 1) * 2);

            return (buffer.to_array(), 1);
        }

        let mut advances = vec![self.advance; glyph_count];

        for (codepoint, advance) in &self.glyph_advances {
            let glyph = self.glyph_of(*codepoint).expect("the codepoint of a glyph advance is covered by the font");
            advances[glyph] = *advance;
        }

        for advance in advances {
            buffer.uint16(advance as i32).int16(0);
        }

        (buffer.to_array(), glyph_count)
    }

    fn build_maxp(glyph_count: usize) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        // Version 0.5: only the glyph count (a font without TrueType outlines).
        buffer.uint32(0x0000_5000).uint16(glyph_count as i32);

        buffer.to_array()
    }

    fn build_cmap(&self) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        // Header: version, one encoding record (Windows, UCS-4) pointing at the subtable.
        buffer.uint16(0).uint16(1).uint16(3).uint16(10).uint32(12);

        // Format 12 subtable.
        buffer
            .uint16(12)
            .uint16(0)
            .uint32(16 + self.ranges.len() as u32 * 12)
            .uint32(0)
            .uint32(self.ranges.len() as u32);

        let mut ranges = self.ranges.clone();
        let mut first_glyphs = Vec::with_capacity(ranges.len());
        let mut first_glyph = 1u32;

        for (start, end) in &ranges {
            first_glyphs.push(first_glyph);
            first_glyph += end - start + 1;
        }

        // Groups are sorted by start code; glyph ids follow the order the ranges were given in.
        let mut order: Vec<usize> = (0..ranges.len()).collect();
        order.sort_by_key(|index| ranges[*index].0);

        for index in order {
            let (start, end) = ranges[index];
            buffer.uint32(start).uint32(end).uint32(first_glyphs[index]);
        }

        ranges.clear();

        buffer.to_array()
    }

    fn build_name(&self) -> Vec<u8> {
        const US_ENGLISH: u16 = 0x0409;

        let sub_family_name = match (self.weight.value() >= 600, self.style != FontStyle::Normal) {
            (false, false) => "Regular",
            (true, false) => "Bold",
            (false, true) => "Italic",
            (true, true) => "Bold Italic",
        };

        let mut records: Vec<(u16, u16, String)> = vec![
            (US_ENGLISH, 1, self.family_name.clone()),
            (US_ENGLISH, 2, sub_family_name.to_owned()),
            (US_ENGLISH, 4, format!("{} {}", self.family_name, sub_family_name)),
        ];

        if let Some(typographic_family_name) = &self.typographic_family_name {
            records.push((US_ENGLISH, 16, typographic_family_name.clone()));
        }

        for (lcid, name) in &self.localized_family_names {
            records.push((*lcid, 1, name.clone()));
        }

        let mut buffer = BigEndianBuffer::new();
        let mut storage = Vec::new();

        buffer.uint16(0).uint16(records.len() as i32).uint16((6 + records.len() * 12) as i32);

        for (language, name_id, text) in &records {
            let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();

            buffer
                .uint16(3) // Windows
                .uint16(1) // Unicode BMP
                .uint16(*language as i32)
                .uint16(*name_id as i32)
                .uint16(bytes.len() as i32)
                .uint16(storage.len() as i32);

            storage.extend_from_slice(&bytes);
        }

        buffer.bytes(&storage);
        buffer.to_array()
    }

    fn build_os2(&self) -> Vec<u8> {
        let mut selection = 0;

        match self.style {
            FontStyle::Italic => selection |= 1,
            FontStyle::Oblique => selection |= 1 << 9,
            FontStyle::Normal => {}
        }

        if self.weight.value() >= 600 {
            selection |= 1 << 5;
        }

        if selection == 0 {
            selection = 1 << 6; // REGULAR
        }

        let mut unicode_ranges = [0u32; 4];

        for bit in &self.unicode_range_bits {
            unicode_ranges[(*bit / 32) as usize] |= 1 << (bit % 32);
        }

        let first_char = self.ranges.iter().map(|(start, _)| *start).min().unwrap_or(0).min(0xFFFF);
        let last_char = self.ranges.iter().map(|(_, end)| *end).max().unwrap_or(0).min(0xFFFF);

        let mut buffer = BigEndianBuffer::new();

        buffer
            .uint16(4) // version
            .int16(self.advance as i32) // xAvgCharWidth
            .uint16(self.weight.value()) // usWeightClass
            .uint16(self.stretch as i32) // usWidthClass
            .uint16(0) // fsType
            .zeros(16) // subscript and superscript sizes and offsets
            .int16((self.units_per_em / 20) as i32) // yStrikeoutSize
            .int16((self.ascender / 3) as i32) // yStrikeoutPosition
            .int16(0) // sFamilyClass
            .zeros(10) // panose
            .uint32(unicode_ranges[0])
            .uint32(unicode_ranges[1])
            .uint32(unicode_ranges[2])
            .uint32(unicode_ranges[3])
            .tag("TEST") // achVendID
            .uint16(selection)
            .uint16(first_char as i32)
            .uint16(last_char as i32)
            .int16(self.ascender as i32) // sTypoAscender
            .int16(self.descender as i32) // sTypoDescender
            .int16(self.line_gap as i32) // sTypoLineGap
            .uint16(self.ascender.max(0) as i32) // usWinAscent
            .uint16(-(self.descender.min(0) as i32)) // usWinDescent
            .uint32(self.code_page_coverage.bits() as u32) // ulCodePageRange1
            .uint32((self.code_page_coverage.bits() >> 32) as u32) // ulCodePageRange2
            .int16((self.ascender / 2) as i32) // sxHeight
            .int16(self.ascender as i32) // sCapHeight
            .uint16(0) // usDefaultChar
            .uint16(0x20) // usBreakChar
            .uint16(1); // usMaxContext

        buffer.to_array()
    }

    fn build_post(&self) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer
            .uint32(0x0003_0000)
            .fixed(if self.style == FontStyle::Normal { 0.0 } else { -12.0 })
            .int16(-((self.units_per_em / 10) as i32)) // underlinePosition
            .int16((self.units_per_em / 20) as i32) // underlineThickness
            .uint32(u32::from(self.is_fixed_pitch))
            .zeros(16);

        buffer.to_array()
    }

    fn build_gsub(&self) -> Vec<u8> {
        let mut buffer = BigEndianBuffer::new();

        buffer.uint16(1).uint16(0).uint16(10).uint16(0).uint16(0);
        buffer.uint16(self.shaping_scripts.len() as i32);

        for script in &self.shaping_scripts {
            buffer.tag(script).uint16(0);
        }

        buffer.to_array()
    }

    fn build_meta(&self) -> Vec<u8> {
        let maps: Vec<(&str, &str)> = [("dlng", &self.design_languages), ("slng", &self.supported_languages)]
            .into_iter()
            .filter_map(|(tag, value)| value.as_deref().map(|value| (tag, value)))
            .collect();

        let mut buffer = BigEndianBuffer::new();

        buffer.uint32(1).uint32(0).uint32(0).uint32(maps.len() as u32);

        let mut payload_offset = 16 + 12 * maps.len();

        for (tag, value) in &maps {
            buffer.tag(tag).uint32(payload_offset as u32).uint32(value.len() as u32);
            payload_offset += value.len();
        }

        for (_, value) in &maps {
            buffer.bytes(value.as_bytes());
        }

        buffer.to_array()
    }

    /// The font as an editable table set.
    pub(crate) fn build_font(&self) -> SyntheticFont {
        let glyph_count = self.glyph_count();
        let (hmtx, number_of_h_metrics) = self.build_hmtx(glyph_count);

        let mut font = SyntheticFont::new();

        font.replace("head", self.build_head())
            .replace("hhea", self.build_hhea(number_of_h_metrics))
            .replace("hmtx", hmtx)
            .replace("maxp", Self::build_maxp(glyph_count))
            .replace("cmap", self.build_cmap())
            .replace("name", self.build_name())
            .replace("OS/2", self.build_os2())
            .replace("post", self.build_post());

        if !self.shaping_scripts.is_empty() {
            font.replace("GSUB", self.build_gsub());
        }

        if self.design_languages.is_some() || self.supported_languages.is_some() {
            font.replace("meta", self.build_meta());
        }

        font
    }

    /// The font file bytes.
    pub fn build_bytes(&self) -> Vec<u8> {
        self.build_font().to_bytes()
    }

    /// The font as a platform typeface.
    pub fn build(&self) -> Rc<TestPlatformTypeface> {
        TestPlatformTypeface::from_bytes(self.build_bytes(), FontSimulations::None)
            .expect("a built test font is a valid font")
    }
}
