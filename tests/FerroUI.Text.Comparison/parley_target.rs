//! parley: a `FontContext` whose fontique collection has Roboto Regular registered from memory and
//! no system fonts, and one `LayoutContext`, both kept for the life of the target as an
//! application keeps them (they are meant to be shared globally). A string is a `Layout` built
//! by a `RangedBuilder` (font size and family as the default style); a width is
//! `Layout::break_all_lines`, which also aligns the lines (to the start, the default alignment)
//! relative to the width.

use crate::font::{FAMILY, FONT_SIZE, ROBOTO_REGULAR};
use crate::target::{TextTarget, Totals};
use crate::LibraryKind;
use parley::fontique::{Blob, Collection, CollectionOptions, SourceCache};
use parley::{FontContext, FontFamilyName, Layout, LayoutContext, StyleProperty};
use std::borrow::Cow;
use std::sync::Arc;

/// The display scale: one pixel a unit, as for every library.
const SCALE: f32 = 1.0;
/// Pixel-aligned line metrics, what parley recommends for laying out text to paint.
const QUANTIZE: bool = true;

pub struct ParleyTarget {
    font_cx: FontContext,
    layout_cx: LayoutContext<()>,
    texts: Vec<String>,
    /// The prepared layouts, one per string: shaped once.
    layouts: Vec<Layout<()>>,
}

/// A layout of `text`, shaped and not yet broken into lines.
fn build(font_cx: &mut FontContext, layout_cx: &mut LayoutContext<()>, text: &str) -> Layout<()> {
    let mut builder = layout_cx.ranged_builder(font_cx, text, SCALE, QUANTIZE);
    builder.push_default(StyleProperty::FontSize(FONT_SIZE));
    builder.push_default(FontFamilyName::Named(Cow::Borrowed(FAMILY)));
    builder.build(text)
}

fn totals(layout: &Layout<()>) -> Totals {
    Totals::new(layout.len(), f64::from(layout.height()))
}

impl TextTarget for ParleyTarget {
    const KIND: LibraryKind = LibraryKind::Parley;

    fn new(texts: Vec<String>) -> Self {
        let mut collection = Collection::new(CollectionOptions { shared: false, system_fonts: false });
        collection.register_fonts(Blob::new(Arc::new(ROBOTO_REGULAR.to_vec())), None);
        assert!(collection.family_id(FAMILY).is_some(), "the collection has Roboto");
        let font_cx = FontContext { collection, source_cache: SourceCache::default() };
        Self { font_cx, layout_cx: LayoutContext::new(), texts, layouts: Vec::new() }
    }

    fn len(&self) -> usize {
        self.texts.len()
    }

    fn cold_one(&mut self, index: usize, width: f32) -> Totals {
        let mut layout = build(&mut self.font_cx, &mut self.layout_cx, &self.texts[index]);
        layout.break_all_lines(Some(width));
        totals(&layout)
    }

    fn prepare(&mut self) {
        let mut layouts = Vec::with_capacity(self.texts.len());
        for text in &self.texts {
            layouts.push(build(&mut self.font_cx, &mut self.layout_cx, text));
        }
        self.layouts = layouts;
        let _ = self.relayout(crate::corpus::LAYOUT_WIDTH);
    }

    fn relayout(&mut self, width: f32) -> Totals {
        let mut result = Totals::default();
        for layout in &mut self.layouts {
            layout.break_all_lines(Some(width));
            result += std::hint::black_box(totals(layout));
        }
        result
    }
}
