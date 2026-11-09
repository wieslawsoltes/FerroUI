//! FerroUI: `TextLayout` (`src/FerroUI.Base/media/text_formatting/text_layout.rs`) shaped by
//! HarfBuzz (`ferroui-harfbuzz`), with Roboto Regular served from memory by the font backend of
//! the font tests (`media::fonts::testing::TestFontManagerImpl`), on a test application whose
//! services are those of the text benchmarks (`tests/FerroUI.Benchmarks/text/`).
//!
//! A layout is built as `TextBlock` builds one (`src/FerroUI.Controls/text_block.rs`,
//! `create_text_layout`): a new `TextLayout` for the text and the width, with the `TextRunCache`
//! of the text block when there is one. The cache holds the shaped runs of a paragraph (by its
//! index in the text) so that a new width does not shape again; a text block has one cache, so a
//! prepared string here has one.

use crate::font::{FONT_SIZE, ROBOTO_REGULAR};
use crate::target::{TextTarget, Totals};
use crate::LibraryKind;
use ferroui_base::media::fonts::testing::{TestFontManagerImpl, TestPlatformTypeface};
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions, TextRunCache};
use ferroui_base::media::{Brushes, FontSimulations, IPlatformTypeface, TextTrimming, TextWrapping, Typeface};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_harfbuzz::HarfBuzzTextShaper;
use std::rc::Rc;

pub struct FerroUiTarget {
    typeface: Typeface,
    texts: Vec<String>,
    /// The texts in UTF-16, the text of a `TextBlock`: a prepared layout takes them without
    /// transcoding (the cold path transcodes, as a new text does).
    utf16: Vec<ReadOnlyMemory<u16>>,
    /// One cache of shaped runs per prepared string, as each text block has one.
    caches: Vec<Rc<TextRunCache>>,
    /// Disposed when the target is dropped (the fields drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl FerroUiTarget {
    fn options(width: f32, cache: Option<Rc<TextRunCache>>) -> TextLayoutOptions {
        TextLayoutOptions {
            font_size: f64::from(FONT_SIZE),
            foreground: Some(Brushes::black()),
            max_width: f64::from(width),
            text_trimming: Some(<dyn TextTrimming>::none()),
            text_wrapping: TextWrapping::Wrap,
            text_run_cache: cache,
            ..TextLayoutOptions::default()
        }
    }

    fn totals(layout: TextLayout) -> Totals {
        let totals = Totals::new(layout.text_lines().len(), layout.height());
        // As a text block disposes the layout it replaces: its shaped buffers go back to their
        // pool (the cache keeps references of its own).
        layout.dispose();
        totals
    }
}

impl TextTarget for FerroUiTarget {
    const KIND: LibraryKind = LibraryKind::FerroUi;

    fn new(texts: Vec<String>) -> Self {
        let roboto = TestPlatformTypeface::from_bytes(ROBOTO_REGULAR.to_vec(), FontSimulations::None)
            .expect("Roboto Regular loads");
        let family = roboto.family_name();
        let font_manager = TestFontManagerImpl::new(&family);
        font_manager.add_font(roboto);

        // The services of the text benchmarks, with HarfBuzz and a font backend that has Roboto
        // and nothing else.
        let services = TestServices::styled_window()
            .with_text_shaper_impl(Rc::new(HarfBuzzTextShaper::new()))
            .with_font_manager_impl(font_manager);
        let app = UnitTestApplication::start(services);

        let typeface = Typeface::from_name(&family);
        let resolved = typeface.glyph_typeface();
        assert!(
            resolved.family_name().eq_ignore_ascii_case(&family),
            "the typeface resolves to Roboto, not to a fallback: {}",
            resolved.family_name()
        );

        let utf16 = texts.iter().map(|text| ReadOnlyMemory::<u16>::from_str(text)).collect();
        Self { typeface, texts, utf16, caches: Vec::new(), _app: app }
    }

    fn len(&self) -> usize {
        self.texts.len()
    }

    fn cold_one(&mut self, index: usize, width: f32) -> Totals {
        // A new text: transcoded and shaped, with no cache of shaped runs.
        Self::totals(TextLayout::new(&self.texts[index], self.typeface.clone(), Self::options(width, None)))
    }

    fn prepare(&mut self) {
        self.caches = (0..self.utf16.len()).map(|_| Rc::new(TextRunCache::new())).collect();
        let _ = self.relayout(crate::corpus::LAYOUT_WIDTH);
    }

    fn relayout(&mut self, width: f32) -> Totals {
        let mut totals = Totals::default();
        for (text, cache) in self.utf16.iter().zip(&self.caches) {
            let layout =
                TextLayout::from_utf16(text.clone(), self.typeface.clone(), Self::options(width, Some(cache.clone())));
            totals += std::hint::black_box(Self::totals(layout));
        }
        totals
    }
}

impl Drop for FerroUiTarget {
    fn drop(&mut self) {
        for cache in &self.caches {
            cache.dispose();
        }
    }
}
