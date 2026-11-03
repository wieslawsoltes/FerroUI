//! Optional emoji fonts using native FerroUI font collections and Rust byte readers.
use ferroui_base::{
    FerroLocator, LocatorExtensions,
    media::{
        FontFallback, FontFamily, FontManager, FontManagerOptions, Typeface, UnicodeRange,
        UnicodeRangeSegment,
        fonts::{FontCollectionBase, FontCollectionBaseImpl, FontCollectionKey},
    },
    platform::IFontManagerImpl,
    utilities::{Uri, UriKind},
};
use std::{
    io::{self, Cursor, Read},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

// Keep the upstream emoji classifier and the native fallback filter in one place.
pub(super) const EMOJI_RANGES: &[(i32, i32)] = &[
    (0x1F000, 0x1FAFF),
    (0x2600, 0x27BF),
    (0x2300, 0x23FF),
    (0x2B00, 0x2BFF),
    (0xE0020, 0xE007F),
    (0x200D, 0x200D),
    (0xFE0F, 0xFE0F),
    (0x20E3, 0x20E3),
    (0x00A9, 0x00A9),
    (0x00AE, 0x00AE),
    (0x203C, 0x203C),
    (0x2049, 0x2049),
    (0x2122, 0x2122),
    (0x2139, 0x2139),
    (0x2194, 0x21AA),
    (0x24C2, 0x24C2),
    (0x3030, 0x3030),
    (0x303D, 0x303D),
    (0x3297, 0x3297),
    (0x3299, 0x3299),
];
const ALIAS: &str = "FerroBoard Emoji";

#[derive(Debug)]
pub enum EmojiFontError {
    Read(io::Error),
    Empty,
    BackendUnavailable,
    UnsupportedFont,
}
impl std::fmt::Display for EmojiFontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(e) => write!(f, "could not read emoji font: {e}"),
            Self::Empty => f.write_str("emoji font is empty"),
            Self::BackendUnavailable => {
                f.write_str("initialize FerroUI's font backend before installing the emoji font")
            }
            Self::UnsupportedFont => {
                f.write_str("font backend could not load the supplied emoji font")
            }
        }
    }
}
impl std::error::Error for EmojiFontError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            _ => None,
        }
    }
}

/// A reusable font asset. Read bytes once, configure fallbacks before application setup,
/// then install with `AppBuilder::configure_fonts` on the UI thread.
///
/// File/network access belongs to the application's composition root. No font is downloaded.
/// Registered glyph data is owned by FerroUI's font manager; dropping this asset is safe.
#[derive(Clone)]
pub struct EmojiFont {
    bytes: Arc<[u8]>,
    key: Uri,
    family: FontFamily,
}
impl EmojiFont {
    pub fn from_reader(mut reader: impl Read) -> Result<Self, EmojiFontError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(EmojiFontError::Read)?;
        Self::from_bytes(bytes)
    }
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Result<Self, EmojiFontError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(EmojiFontError::Empty);
        }
        static NEXT_KEY: AtomicU64 = AtomicU64::new(1);
        let key = Uri::try_create(
            &format!(
                "fonts:FerroBoardEmoji{}",
                NEXT_KEY.fetch_add(1, Ordering::Relaxed)
            ),
            UriKind::Absolute,
        )
        .expect("native font collection URI");
        let family = FontFamily::new(&format!("{key}#{ALIAS}"));
        Ok(Self { bytes, key, family })
    }
    pub fn font_family(&self) -> FontFamily {
        self.family.clone()
    }
    pub fn fallback(&self) -> FontFallback {
        FontFallback {
            font_family: self.font_family(),
            unicode_range: UnicodeRange::from_segments(
                EMOJI_RANGES
                    .iter()
                    .map(|&(a, b)| UnicodeRangeSegment::new(a, b))
                    .collect(),
            ),
        }
    }
    /// Give the custom font first choice for emoji, preserving every existing fallback and
    /// default/mapping option. Characters absent from the font continue through FerroUI's chain.
    pub fn configure(&self, options: &mut FontManagerOptions) {
        let fallback = self.fallback();
        let existing = options.font_fallbacks.get_or_insert_with(Vec::new);
        if !existing.contains(&fallback) {
            existing.insert(0, fallback);
        }
    }
    /// Register a decoded native glyph typeface. Repeat installation is idempotent while
    /// the font is registered; after explicit removal the cached bytes can install it again.
    pub fn install(&self, manager: &FontManager) -> Result<(), EmojiFontError> {
        if FerroLocator::current()
            .get_service::<dyn IFontManagerImpl>()
            .is_none()
        {
            return Err(EmojiFontError::BackendUnavailable);
        }
        let face = Typeface::new(self.font_family());
        if manager.try_get_glyph_typeface(&face).is_some() {
            return Ok(());
        }
        let collection = Rc::new(ByteFontCollection {
            base: FontCollectionBase::new(),
            key: self.key.clone(),
        });
        let glyph = FontCollectionBase::try_add_glyph_typeface_from_stream(
            &*collection,
            &mut Cursor::new(self.bytes.clone()),
        )
        .ok_or(EmojiFontError::UnsupportedFont)?;
        if glyph.family_name() != ALIAS
            && glyph.typographic_family_name() != ALIAS
            && !FontCollectionBase::try_add_glyph_typeface_by_name(
                &*collection,
                ALIAS,
                FontCollectionKey::from(&*glyph),
                Some(glyph),
            )
        {
            return Err(EmojiFontError::UnsupportedFont);
        }
        manager.add_font_collection(collection);
        Ok(())
    }
}
struct ByteFontCollection {
    base: FontCollectionBase,
    key: Uri,
}
impl FontCollectionBaseImpl for ByteFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }
    fn key(this: &Self) -> Uri {
        this.key.clone()
    }
}
