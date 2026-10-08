use std::cell::Cell;
use std::io::Read;
use std::rc::Rc;

use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont, SyntheticFontMemory};
use crate::media::fonts::OpenTypeTag;
use crate::media::{
    FontSimulations, FontStretch, FontStyle, FontWeight, IFontMemory, IPlatformTypeface, Typeface,
};
use crate::platform::IFontManagerImpl;
use crate::utilities::{CultureInfo, ReadOnlyMemory};

use super::{ASCENDER, DESCENDER, DESIGN_EM_HEIGHT, GLYPH_ADVANCE, LINE_GAP};

/// The number of glyphs of every test font. A BMP codepoint maps to the glyph
/// with its own value; a supplementary codepoint to its low 16 bits.
const GLYPH_COUNT: u16 = 0xFFFF;

/// The description of a synthetic test font.
#[derive(Clone)]
pub struct TestFont {
    family: String,
    ranges: Vec<(u32, u32)>,
    advance: u16,
    line_gap: i16,
    script_tags: Vec<&'static str>,
    language: Option<&'static str>,
}

impl TestFont {
    /// A font without coverage and with the default advance.
    pub fn new(family: &str) -> Self {
        Self { family: family.to_owned(), ranges: Vec::new(), advance: GLYPH_ADVANCE, line_gap: LINE_GAP, script_tags: Vec::new(), language: None }
    }

    /// The (inclusive) codepoint ranges the font maps.
    pub fn with_ranges(mut self, ranges: &[(u32, u32)]) -> Self {
        self.ranges = ranges.to_vec();
        self
    }

    /// The advance of every glyph, in design units.
    pub fn with_advance(mut self, advance: u16) -> Self {
        self.advance = advance;
        self
    }

    /// The two letter language the font is designed for: the font manager
    /// prefers it when a character is matched for a culture of that language.
    pub fn with_language(mut self, language: &'static str) -> Self {
        self.language = Some(language);
        self
    }

    /// The line gap, in design units.
    pub fn with_line_gap(mut self, line_gap: i16) -> Self {
        self.line_gap = line_gap;
        self
    }

    /// The OpenType script tags the font declares in its `GSUB` table (what
    /// makes it able to shape a complex script).
    pub fn with_script_tags(mut self, script_tags: &[&'static str]) -> Self {
        self.script_tags = script_tags.to_vec();
        self
    }

    fn covers(&self, codepoint: i32) -> bool {
        codepoint >= 0 && self.ranges.iter().any(|&(start, end)| (start..=end).contains(&(codepoint as u32)))
    }

    /// Builds the font: `head`, `hhea`, `hmtx`, `maxp`, `cmap` (format 12),
    /// `name` and, when script tags are given, `GSUB`.
    fn build(&self) -> SyntheticFont {
        let mut font = SyntheticFont::new();

        let mut head = BigEndianBuffer::new();
        head.uint32(0x0001_0000) // version
            .uint32(0x0001_0000) // fontRevision
            .uint32(0) // checkSumAdjustment
            .uint32(0x5F0F_3CF5) // magicNumber
            .uint16(0) // flags
            .uint16(DESIGN_EM_HEIGHT as i32) // unitsPerEm
            .zeros(16) // created, modified
            .int16(0) // xMin
            .int16(DESCENDER as i32) // yMin
            .int16(self.advance as i32) // xMax
            .int16(ASCENDER as i32) // yMax
            .uint16(0) // macStyle
            .uint16(6) // lowestRecPPEM
            .int16(2) // fontDirectionHint
            .int16(0) // indexToLocFormat
            .int16(0); // glyphDataFormat
        font.replace("head", head.to_array());

        let mut hhea = BigEndianBuffer::new();
        hhea.uint32(0x0001_0000)
            .int16(ASCENDER as i32)
            .int16(DESCENDER as i32)
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
            .uint16(1); // numberOfHMetrics
        font.replace("hhea", hhea.to_array());

        // One long metric; every other glyph repeats its advance.
        let mut hmtx = BigEndianBuffer::new();
        hmtx.uint16(self.advance as i32).int16(0).zeros((GLYPH_COUNT as usize - 1) * 2);
        font.replace("hmtx", hmtx.to_array());

        let mut maxp = BigEndianBuffer::new();
        maxp.uint32(0x0000_5000).uint16(GLYPH_COUNT as i32);
        font.replace("maxp", maxp.to_array());

        let mut cmap = BigEndianBuffer::new();
        cmap.uint16(0) // version
            .uint16(1) // numTables
            .uint16(3) // platformID = Windows
            .uint16(10) // encodingID = UCS-4
            .uint32(12) // offset
            .uint16(12) // format
            .uint16(0) // reserved
            .uint32((16 + self.ranges.len() * 12) as u32) // length
            .uint32(0) // language
            .uint32(self.ranges.len() as u32); // numGroups
        for &(start, end) in &self.ranges {
            cmap.uint32(start).uint32(end).uint32((start & 0xFFFF).max(1));
        }
        font.replace("cmap", cmap.to_array());

        let mut name = BigEndianBuffer::new();
        let records: [(i32, &str); 2] = [(1, &self.family), (2, "Regular")];
        let mut storage: Vec<u8> = Vec::new();
        name.uint16(0).uint16(records.len() as i32).uint16((6 + records.len() * 12) as i32);
        for (name_id, text) in records {
            let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
            name.uint16(3) // platformID = Windows
                .uint16(1) // encodingID = Unicode BMP
                .uint16(0x0409) // languageID = en-US
                .uint16(name_id)
                .uint16(bytes.len() as i32)
                .uint16(storage.len() as i32);
            storage.extend_from_slice(&bytes);
        }
        name.bytes(&storage);
        font.replace("name", name.to_array());

        if !self.script_tags.is_empty() {
            // GSUB header (version 1.0) with a script list that only names the scripts.
            let mut gsub = BigEndianBuffer::new();
            gsub.uint16(1).uint16(0).uint16(10).uint16(0).uint16(0);
            gsub.uint16(self.script_tags.len() as i32);
            for tag in &self.script_tags {
                gsub.tag(tag).uint16(0);
            }
            font.replace("GSUB", gsub.to_array());
        }

        if let Some(language) = self.language {
            // A 'meta' table declaring the design language, which is what the
            // culture-aware font fallback reads.
            let mut meta = BigEndianBuffer::new();
            meta.uint32(1) // version
                .uint32(0) // flags
                .uint32(0) // reserved
                .uint32(1) // dataMapsCount
                .tag("dlng")
                .uint32(28) // dataOffset: header (16) + one map (12)
                .uint32(language.len() as u32)
                .bytes(language.as_bytes());
            font.replace("meta", meta.to_array());
        }

        font
    }
}

/// A platform typeface over a synthetic font.
pub struct TestPlatformTypeface {
    memory: SyntheticFontMemory,
    font: TestFont,
    is_disposed: Cell<bool>,
}

#[allow(dead_code)] // the full harness API is kept for the tests built on top
impl TestPlatformTypeface {
    pub fn new(font: TestFont) -> Self {
        Self { memory: font.build().to_font_memory(), font, is_disposed: Cell::new(false) }
    }

    /// The font description the typeface was built from.
    pub fn font(&self) -> &TestFont {
        &self.font
    }

    /// Whether the typeface was disposed.
    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }
}

impl IFontMemory for TestPlatformTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.memory.try_get_table(tag)
    }

    fn dispose(&self) {
        self.is_disposed.set(true);
    }
}

impl IPlatformTypeface for TestPlatformTypeface {
    fn family_name(&self) -> String {
        self.font.family.clone()
    }

    fn weight(&self) -> FontWeight {
        FontWeight::Normal
    }

    fn style(&self) -> FontStyle {
        FontStyle::Normal
    }

    fn stretch(&self) -> FontStretch {
        FontStretch::Normal
    }

    fn font_simulations(&self) -> FontSimulations {
        FontSimulations::None
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// A font manager backend that serves synthetic fonts. The first font is the
/// default family; a character is matched by the first font that covers it.
pub struct TestFontManagerImpl {
    typefaces: Vec<Rc<TestPlatformTypeface>>,
    match_character_calls: Cell<usize>,
}

#[allow(dead_code)] // the full harness API is kept for the tests built on top
impl TestFontManagerImpl {
    pub fn new(fonts: Vec<TestFont>) -> Self {
        assert!(!fonts.is_empty(), "the test font manager needs a default font");

        Self {
            typefaces: fonts.into_iter().map(|font| Rc::new(TestPlatformTypeface::new(font))).collect(),
            match_character_calls: Cell::new(0),
        }
    }

    /// How often a fallback font was searched.
    pub fn match_character_calls(&self) -> usize {
        self.match_character_calls.get()
    }
}

impl IFontManagerImpl for TestFontManagerImpl {
    fn get_default_font_family_name(&self) -> String {
        self.typefaces[0].font.family.clone()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        self.typefaces.iter().map(|typeface| typeface.font.family.clone()).collect()
    }

    fn try_match_character(
        &self,
        codepoint: i32,
        _font_style: FontStyle,
        _font_weight: FontWeight,
        _font_stretch: FontStretch,
        _family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.match_character_calls.set(self.match_character_calls.get() + 1);

        let language = culture.map(|culture| culture.two_letter_iso_language_name());

        let mut covering = self.typefaces.iter().filter(|typeface| typeface.font.covers(codepoint));

        let preferred = covering
            .clone()
            .find(|typeface| typeface.font.language.is_some() && typeface.font.language == language)
            .or_else(|| covering.next());

        preferred.map(|typeface| typeface.clone() as Rc<dyn IPlatformTypeface>)
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.typefaces
            .iter()
            .find(|typeface| typeface.font.family.eq_ignore_ascii_case(family_name))
            .map(|typeface| typeface.clone() as Rc<dyn IPlatformTypeface>)
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        _stream: &mut dyn Read,
        _font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        None
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        self.typefaces
            .iter()
            .any(|typeface| typeface.font.family.eq_ignore_ascii_case(family_name))
            .then(|| vec![Typeface::from_name(family_name)])
    }
}
