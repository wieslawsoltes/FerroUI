//! Port of `HeadlessPlatformStubs.cs`: the clipboard, the cursor factory,
//! the fonts, the icon loader and the screens of the headless platform.
//!
//! `TextTestHelper` of the upstream file is not ported: it asks whether a
//! `ReadOnlyMemory<char>` is a slice of a string (`MemoryMarshal.TryGetString`),
//! which has no counterpart, and nothing in the project uses it.

use ferroui_base::input::platform::{ClipboardError, IClipboardImpl, IOwnedClipboardImpl};
use ferroui_base::input::{DataFormat, IAsyncDataTransfer, IAsyncDataTransferItem, LocalBoxFuture, StandardCursorType};
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{
    FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface, IFontMemory, IPlatformTypeface, Typeface,
};
use ferroui_base::platform::{IBitmapImpl, ICursorFactory, ICursorImpl, IFontManagerImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_base::{PixelPoint, PixelRect};
use ferroui_controls::platform::{
    IPlatformIconLoader, IWindowIconImpl, PlatformHandle, PlatformScreen, ScreensBase, ScreensBaseImpl,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{self, Cursor, Read};
use std::rc::Rc;

/// The font every family name resolves to.
const BARE_MINIMUM_FONT: &[u8] = include_bytes!("BareMinimum.ttf");

/// The clipboard of the headless platform: it keeps the data placed on it.
#[derive(Default)]
pub(crate) struct HeadlessClipboardImplStub {
    data: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
}

impl IClipboardImpl for HeadlessClipboardImplStub {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        // Return an instance that won't be disposed (we're keeping the ownership).
        let data = self.data.borrow().clone().map(|data| {
            let wrapped: Rc<dyn IAsyncDataTransfer> = Rc::new(NonDisposingDataTransfer { wrapped: data });
            wrapped
        });
        Box::pin(std::future::ready(Ok(data)))
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        *self.data.borrow_mut() = Some(data_transfer);
        Box::pin(std::future::ready(Ok(())))
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let data = self.data.borrow_mut().take();
        if let Some(data) = data {
            data.dispose();
        }
        Box::pin(std::future::ready(Ok(())))
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        Some(self)
    }
}

impl IOwnedClipboardImpl for HeadlessClipboardImplStub {
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        Box::pin(std::future::ready(Ok(self.data.borrow().is_some())))
    }
}

struct NonDisposingDataTransfer {
    wrapped: Rc<dyn IAsyncDataTransfer>,
}

impl IDisposable for NonDisposingDataTransfer {
    fn dispose(&self) {}
}

impl IAsyncDataTransfer for NonDisposingDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.wrapped.formats()
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        self.wrapped.items()
    }
}

/// A cursor factory whose cursors do nothing.
#[derive(Default)]
pub(crate) struct HeadlessCursorFactoryStub;

struct CursorStub;

impl ICursorImpl for CursorStub {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ICursorFactory for HeadlessCursorFactoryStub {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }

    fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }
}

// Deviation (DEVIATIONS.md, Headless platform): upstream keeps the font in an
// `UnmanagedFontMemory`, which is internal to the base assembly and visible to the headless
// assembly; in the port that type is private to `ferroui-base`, so the table lookup of the sfnt
// table directory is repeated here.
/// The bytes of a font file and the lookup of its tables.
struct FontMemory {
    memory: RefCell<ReadOnlyMemory<u8>>,
    table_cache: RefCell<HashMap<u32, ReadOnlyMemory<u8>>>,
}

impl FontMemory {
    fn load_from_stream(stream: &mut dyn Read) -> io::Result<FontMemory> {
        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;
        Ok(Self::create_from_bytes(data))
    }

    fn create_from_bytes(data: Vec<u8>) -> FontMemory {
        FontMemory { memory: RefCell::new(ReadOnlyMemory::from_vec(data)), table_cache: RefCell::new(HashMap::new()) }
    }

    fn memory(&self) -> ReadOnlyMemory<u8> {
        self.memory.borrow().clone()
    }

    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        // Validate tag
        if tag.value() == 0 {
            return None;
        }

        let memory = self.memory.borrow();
        let font_data = memory.span();

        // Minimal SFNT header: 4 (sfnt) + 2 (numTables) + 6 (rest) = 12
        if font_data.len() < 12 {
            return None;
        }

        // Check cache first
        if let Some(cached) = self.table_cache.borrow().get(&tag.value()) {
            return Some(cached.clone());
        }

        // Parse table directory
        let num_tables = u16::from_be_bytes([font_data[4], font_data[5]]) as usize;
        let records_start = 12;
        let required_directory_bytes = records_start + num_tables * 16;

        if font_data.len() < required_directory_bytes {
            return None;
        }

        let read_u32 = |offset: usize| {
            u32::from_be_bytes([font_data[offset], font_data[offset + 1], font_data[offset + 2], font_data[offset + 3]])
        };

        for i in 0..num_tables {
            let entry_offset = records_start + i * 16;

            if read_u32(entry_offset) != tag.value() {
                continue;
            }

            let offset = read_u32(entry_offset + 8) as u64;
            let length = read_u32(entry_offset + 12) as u64;

            // Bounds checks - ensure values fit within the data
            if offset + length > font_data.len() as u64 {
                return None;
            }

            let table = memory.slice(offset as usize, length as usize);

            // Cache the result for faster subsequent lookups
            self.table_cache.borrow_mut().insert(tag.value(), table.clone());

            return Some(table);
        }

        None
    }

    fn dispose(&self) {
        *self.memory.borrow_mut() = ReadOnlyMemory::empty();
    }
}

/// The typeface the identity of a font file is read with: the "dummy"
/// glyph typeface of the upstream constructor is created over it, since the
/// headless typeface itself does not exist yet at that point.
struct IdentityProbeTypeface {
    memory: FontMemory,
}

impl IFontMemory for IdentityProbeTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.memory.try_get_table(tag)
    }

    fn dispose(&self) {
        self.memory.dispose();
    }
}

impl IPlatformTypeface for IdentityProbeTypeface {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn family_name(&self) -> String {
        String::new()
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
}

/// A platform typeface over the bytes of a font file.
pub(crate) struct HeadlessPlatformTypeface {
    font_memory: FontMemory,
    family_name: String,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
}

impl HeadlessPlatformTypeface {
    /// Creates a typeface from a stream of font file bytes; the family name
    /// of the font is used unless `family_name` is given.
    ///
    /// # Panics
    /// Panics if the stream cannot be read or does not hold a font the
    /// glyph typeface can read (the exceptions of the original).
    pub(crate) fn new(stream: &mut dyn Read, family_name: Option<&str>) -> Rc<HeadlessPlatformTypeface> {
        let font_memory = match FontMemory::load_from_stream(stream) {
            Ok(font_memory) => font_memory,
            Err(error) => panic!("{error}"),
        };

        let probe: Rc<dyn IPlatformTypeface> =
            Rc::new(IdentityProbeTypeface { memory: FontMemory::create_from_bytes(font_memory.memory().to_vec()) });
        let dummy = match GlyphTypeface::new(probe, FontSimulations::None) {
            Ok(dummy) => dummy,
            Err(error) => panic!("{error}"),
        };

        Rc::new(HeadlessPlatformTypeface {
            font_memory,
            family_name: match family_name {
                Some(family_name) => family_name.to_owned(),
                None => dummy.family_name().to_owned(),
            },
            weight: dummy.weight(),
            style: dummy.style(),
            stretch: dummy.stretch(),
        })
    }
}

impl IFontMemory for HeadlessPlatformTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.font_memory.try_get_table(tag)
    }

    fn dispose(&self) {
        self.font_memory.dispose();
    }
}

impl IPlatformTypeface for HeadlessPlatformTypeface {
    fn as_any(&self) -> &dyn Any {
        self
    }

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
        FontSimulations::None
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        let data = self.font_memory.memory();

        if data.is_empty() {
            return None;
        }

        Some(Box::new(Cursor::new(data.span().to_vec())))
    }
}

/// A font manager with one installed family, which every family name
/// resolves to: the embedded `BareMinimum.ttf`.
pub(crate) struct HeadlessFontManagerStub {
    default_family_name: String,
}

impl HeadlessFontManagerStub {
    pub(crate) fn new() -> Self {
        Self::with_default_family_name("Default")
    }

    pub(crate) fn with_default_family_name(default_family_name: &str) -> Self {
        Self { default_family_name: default_family_name.to_owned() }
    }
}

impl IFontManagerImpl for HeadlessFontManagerStub {
    fn get_default_font_family_name(&self) -> String {
        self.default_family_name.clone()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        vec![self.default_family_name.clone()]
    }

    fn try_match_character(
        &self,
        _codepoint: i32,
        _font_style: FontStyle,
        _font_weight: FontWeight,
        _font_stretch: FontStretch,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        None
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        // Upstream opens the embedded resource `BareMinimum.ttf` of the assembly with the asset
        // loader: the font is embedded in the crate.
        let mut stream = Cursor::new(BARE_MINIMUM_FONT);

        let platform_typeface: Rc<dyn IPlatformTypeface> = HeadlessPlatformTypeface::new(&mut stream, Some(family_name));

        Some(platform_typeface)
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        _font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let platform_typeface: Rc<dyn IPlatformTypeface> = HeadlessPlatformTypeface::new(stream, None);

        Some(platform_typeface)
    }

    fn try_get_family_typefaces(&self, _family_name: &str) -> Option<Vec<Typeface>> {
        None
    }
}

/// A font manager that reports several installed families and creates no
/// typeface.
pub(crate) struct HeadlessFontManagerWithMultipleSystemFontsStub {
    installed_font_family_names: Vec<String>,
    default_family_name: String,
    try_create_glyph_typeface_count: Cell<i32>,
}

#[allow(dead_code)] // as upstream, the class is offered to the tests of the project and not used by the platform
impl HeadlessFontManagerWithMultipleSystemFontsStub {
    pub(crate) fn new(installed_font_family_names: Vec<String>) -> Self {
        Self::with_default_family_name(installed_font_family_names, "Default")
    }

    pub(crate) fn with_default_family_name(installed_font_family_names: Vec<String>, default_family_name: &str) -> Self {
        Self {
            installed_font_family_names,
            default_family_name: default_family_name.to_owned(),
            try_create_glyph_typeface_count: Cell::new(0),
        }
    }

    /// As upstream, the count is never incremented.
    pub(crate) fn try_create_glyph_typeface_count(&self) -> i32 {
        self.try_create_glyph_typeface_count.get()
    }
}

impl IFontManagerImpl for HeadlessFontManagerWithMultipleSystemFontsStub {
    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        self.installed_font_family_names.clone()
    }

    fn get_default_font_family_name(&self) -> String {
        self.default_family_name.clone()
    }

    fn try_create_glyph_typeface(
        &self,
        _family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        None
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        _stream: &mut dyn Read,
        _font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        None
    }

    fn try_get_family_typefaces(&self, _family_name: &str) -> Option<Vec<Typeface>> {
        None
    }

    fn try_match_character(
        &self,
        _codepoint: i32,
        _font_style: FontStyle,
        _font_weight: FontWeight,
        _font_stretch: FontStretch,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        None
    }
}

/// An icon loader whose icons save nothing.
#[derive(Default)]
pub(crate) struct HeadlessIconLoaderStub;

struct IconStub;

impl IWindowIconImpl for IconStub {
    fn save(&self, _output_stream: &mut dyn io::Write) -> io::Result<()> {
        Ok(())
    }
}

impl IPlatformIconLoader for HeadlessIconLoaderStub {
    fn load_icon_from_file(&self, _file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_stream(&self, _stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_bitmap(&self, _bitmap: std::sync::Arc<ferroui_base::platform::SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        Rc::new(IconStub)
    }
}

/// The screens of the headless platform: one primary 1920x1280 screen.
pub(crate) struct HeadlessScreensStub {
    base: ScreensBase<i32, PlatformScreen>,
}

impl HeadlessScreensStub {
    pub(crate) fn new() -> Rc<HeadlessScreensStub> {
        Rc::new(HeadlessScreensStub { base: ScreensBase::new() })
    }
}

impl ScreensBaseImpl for HeadlessScreensStub {
    type Key = i32;
    type Screen = PlatformScreen;

    fn screens_base(&self) -> &ScreensBase<i32, PlatformScreen> {
        &self.base
    }

    fn get_all_screen_keys(&self) -> Vec<i32> {
        vec![1]
    }

    fn create_screen_from_key(&self, key: &i32) -> Rc<PlatformScreen> {
        // `PlatformScreenStub`.
        let screen = PlatformScreen::new(Rc::new(PlatformHandle::new(*key as isize, Some("HeadlessScreensStub"))));
        screen.set_scaling(1.0);
        let bounds = PixelRect::new(0, 0, 1920, 1280);
        screen.set_bounds(bounds);
        screen.set_working_area(bounds);
        screen.set_is_primary(true);
        Rc::new(screen)
    }
}
