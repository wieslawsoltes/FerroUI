use std::cell::Cell;
use std::io::{Cursor, Read};
use std::rc::Rc;

use crate::media::fonts::tables::cmap::{CharacterToGlyphMap, CmapTable};
use crate::media::fonts::tables::name::NameTable;
use crate::media::fonts::tables::{FontSelectionFlags, OS2Table};
use crate::media::fonts::{OpenTypeTag, UnmanagedFontMemory};
use crate::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IFontMemory, IPlatformTypeface};
use crate::utilities::ReadOnlyMemory;

/// A platform typeface over font file bytes held in memory, as a font
/// backend would create it from a stream.
///
/// The identity (family name, weight, style, stretch) is read from the
/// `name` and `OS/2` tables; a bold simulation reports a bold weight and an
/// oblique simulation an italic style.
pub struct TestPlatformTypeface {
    memory: UnmanagedFontMemory,
    family_name: String,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    font_simulations: FontSimulations,
    character_map: CharacterToGlyphMap,
    is_disposed: Cell<bool>,
}

#[allow(dead_code)] // the full harness API is kept for the text tests built on top
impl TestPlatformTypeface {
    /// Creates a typeface from font file bytes; `None` when the bytes are not
    /// a font with a readable character map.
    pub fn from_bytes(bytes: Vec<u8>, font_simulations: FontSimulations) -> Option<Rc<TestPlatformTypeface>> {
        let memory = UnmanagedFontMemory::create_from_bytes(bytes);

        let character_map = CmapTable::load(&memory).ok()?;

        let os2_table = OS2Table::try_load(&memory).ok().flatten();

        let family_name = NameTable::load(&memory).map(|names| names.font_family_name(0x007F)).unwrap_or_default();

        let mut weight = os2_table.as_ref().map_or(FontWeight::Normal, |os2| FontWeight(os2.weight_class as i32));

        let mut style = match &os2_table {
            Some(os2) if os2.selection.contains(FontSelectionFlags::OBLIQUE) => FontStyle::Oblique,
            Some(os2) if os2.selection.contains(FontSelectionFlags::ITALIC) => FontStyle::Italic,
            _ => FontStyle::Normal,
        };

        let stretch = os2_table
            .as_ref()
            .and_then(|os2| FontStretch::from_i32(os2.width_class as i32))
            .unwrap_or(FontStretch::Normal);

        if font_simulations.contains(FontSimulations::Bold) {
            weight = FontWeight::Bold;
        }

        if font_simulations.contains(FontSimulations::Oblique) {
            style = FontStyle::Italic;
        }

        Some(Rc::new(TestPlatformTypeface {
            memory,
            family_name,
            weight,
            style,
            stretch,
            font_simulations,
            character_map,
            is_disposed: Cell::new(false),
        }))
    }

    /// Creates a typeface from a stream of font file bytes.
    pub fn from_stream(stream: &mut dyn Read, font_simulations: FontSimulations) -> Option<Rc<TestPlatformTypeface>> {
        let mut bytes = Vec::new();

        stream.read_to_end(&mut bytes).ok()?;

        Self::from_bytes(bytes, font_simulations)
    }

    /// Whether the font maps the codepoint to a glyph.
    pub fn covers(&self, codepoint: i32) -> bool {
        self.character_map.try_get_glyph(codepoint).is_some()
    }

    /// Whether the typeface was disposed.
    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    /// The font file bytes; empty after disposal.
    pub fn bytes(&self) -> ReadOnlyMemory<u8> {
        self.memory.memory()
    }
}

impl IFontMemory for TestPlatformTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.memory.try_get_table(tag)
    }

    fn dispose(&self) {
        self.is_disposed.set(true);
        self.memory.dispose();
    }
}

impl IPlatformTypeface for TestPlatformTypeface {
    fn family_name(&self) -> String {
        self.family_name.clone()
    }

    fn weight(&self) -> FontWeight {
        self.weight
    }

    fn style(&self) -> FontStyle {
        self.style
    }

    fn stretch(&self) -> FontStretch {
        self.stretch
    }

    fn font_simulations(&self) -> FontSimulations {
        self.font_simulations
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        let bytes = self.memory.memory();

        if bytes.is_empty() {
            return None;
        }

        Some(Box::new(Cursor::new(bytes.span().to_vec())))
    }
}
