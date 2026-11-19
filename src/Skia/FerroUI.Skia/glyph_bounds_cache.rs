//! Not from upstream: the ink bounds of glyphs as the measuring font of a glyph run gives them,
//! kept by the typeface for the sizes it was last asked for.
//!
//! A glyph run measures the bounds of its glyphs when it is created
//! ([`GlyphRunImpl::new`](crate::glyph_run_impl::GlyphRunImpl::new)), on the thread that formats
//! text. Skia answers through its strike cache, one cache of the whole process under one lock,
//! which the threads that draw frames take for every text blob they draw: a frame that formats
//! text while bands of the frame before draw waits on that lock (docs/porting/desktop-performance.md).
//! The bounds of a glyph at a size do not change, so what Skia answered is kept: a run whose
//! glyphs were all measured before asks Skia nothing, and one with new glyphs asks only for those.
//! The bounds are Skia's own, so a glyph run is the same as before.
//!
//! What is kept is bounded: the last [`SIZES`] sizes of the typeface, each with at most
//! [`PAGES_PER_SIZE`] pages of [`PAGE`] glyphs (the glyphs of a page outside the bound are measured
//! by Skia every time, as before). A size that is not kept takes the place of the one used least
//! recently, and its pages are used again.

use skia_safe::Rect;
use std::cell::RefCell;

/// The glyphs of a page.
const PAGE: usize = 256;

/// The sizes of a typeface that are kept.
const SIZES: usize = 8;

/// The pages a size keeps: 4,096 glyphs, the glyphs of the text of most scripts.
const PAGES_PER_SIZE: usize = 16;

/// The bounds of the glyphs of a page that were measured.
struct Page {
    /// One bit a glyph of the page: its bounds are known.
    known: [u64; PAGE / 64],
    bounds: [Rect; PAGE],
}

impl Page {
    fn new() -> Box<Self> {
        Box::new(Self { known: [0; PAGE / 64], bounds: [Rect::default(); PAGE] })
    }

    fn get(&self, slot: usize) -> Option<Rect> {
        (self.known[slot / 64] & (1 << (slot % 64)) != 0).then(|| self.bounds[slot])
    }

    fn set(&mut self, slot: usize, bounds: Rect) {
        self.known[slot / 64] |= 1 << (slot % 64);
        self.bounds[slot] = bounds;
    }
}

/// The pages of one size.
struct Size {
    /// The size, by its bits (a size is a key, not compared as a number).
    size: u32,
    /// The pages by the high byte of the glyph index; `None` where no glyph was measured.
    pages: Vec<Option<Box<Page>>>,
    /// The pages that are kept.
    kept: usize,
}

impl Size {
    fn get(&self, glyph: u16) -> Option<Rect> {
        let index = glyph as usize;
        self.pages.get(index / PAGE)?.as_ref()?.get(index % PAGE)
    }

    fn set(&mut self, glyph: u16, bounds: Rect) {
        let index = glyph as usize;
        let page = index / PAGE;
        if page >= self.pages.len() {
            if self.kept >= PAGES_PER_SIZE {
                return;
            }
            self.pages.resize_with(page + 1, || None);
        }
        let slot = &mut self.pages[page];
        if slot.is_none() {
            if self.kept >= PAGES_PER_SIZE {
                return;
            }
            *slot = Some(Page::new());
            self.kept += 1;
        }
        if let Some(page) = slot {
            page.set(index % PAGE, bounds);
        }
    }

    /// The size used again for `size`: its pages stay allocated and are forgotten.
    fn reuse(&mut self, size: u32) {
        self.size = size;
        for page in self.pages.iter_mut().flatten() {
            page.known = [0; PAGE / 64];
        }
    }
}

/// The sizes missed while every size was kept, that are remembered: a size takes the place of
/// another only when it is missed again while remembered, so text whose size keeps changing (an
/// animated size) is measured by Skia as before instead of replacing the kept sizes every run.
const REMEMBERED: usize = SIZES;

/// The glyph bounds of a typeface by size (see the module).
#[derive(Default)]
pub(crate) struct GlyphBoundsCache {
    /// The sizes, the one used last first.
    sizes: RefCell<Vec<Size>>,
    /// The sizes missed lately while all sizes were kept (`REMEMBERED`), by their bits; 0 is free
    /// (the bits of the size 0.0, which no glyph run has).
    remembered: RefCell<[u32; REMEMBERED]>,
}

impl GlyphBoundsCache {
    /// Whether `size` was missed lately while all sizes were kept (it is forgotten then, as it
    /// takes a place); otherwise it is remembered, in the place of the one remembered longest.
    fn remembered_again(&self, size: u32) -> bool {
        let mut remembered = self.remembered.borrow_mut();
        if let Some(position) = remembered.iter().position(|bits| *bits == size) {
            remembered[position] = 0;
            return true;
        }
        remembered.rotate_right(1);
        remembered[0] = size;
        false
    }

    /// Writes the bounds of `glyphs` at `size` into `bounds`: the kept ones, and for the others
    /// what `measure` writes for them (it is called once, with the glyphs that are not kept and a
    /// buffer of their length, and only when there are such glyphs).
    ///
    /// # Panics
    /// Panics when `bounds` is shorter than `glyphs`.
    pub(crate) fn get(&self, size: f32, glyphs: &[u16], bounds: &mut [Rect], measure: impl FnOnce(&[u16], &mut [Rect])) {
        assert!(bounds.len() >= glyphs.len(), "a bound for every glyph");
        let mut sizes = self.sizes.borrow_mut();
        let size = size.to_bits();
        match sizes.iter().position(|entry| entry.size == size) {
            Some(0) => {}
            Some(position) => sizes[..=position].rotate_right(1),
            None if sizes.len() < SIZES => sizes.insert(0, Size { size, pages: Vec::new(), kept: 0 }),
            None if !self.remembered_again(size) => {
                // Missed for the first time lately: measured, not kept.
                measure(glyphs, &mut bounds[..glyphs.len()]);
                return;
            }
            None => {
                let mut least = sizes.pop().expect("the cache keeps sizes");
                least.reuse(size);
                sizes.insert(0, least);
            }
        }
        let entry = &mut sizes[0];

        let mut missing_count = 0;
        for (glyph, bound) in glyphs.iter().zip(bounds.iter_mut()) {
            match entry.get(*glyph) {
                Some(kept) => *bound = kept,
                None => missing_count += 1,
            }
        }
        if missing_count == 0 {
            return;
        }
        if missing_count == glyphs.len() {
            // None kept (a new size, or new glyphs): Skia measures into the bounds themselves.
            let bounds = &mut bounds[..glyphs.len()];
            measure(glyphs, bounds);
            for (glyph, bound) in glyphs.iter().zip(bounds.iter()) {
                entry.set(*glyph, *bound);
            }
            return;
        }

        let missing: Vec<usize> = (0..glyphs.len()).filter(|index| entry.get(glyphs[*index]).is_none()).collect();
        let missing_glyphs: Vec<u16> = missing.iter().map(|index| glyphs[*index]).collect();
        let mut measured = vec![Rect::default(); missing.len()];
        measure(&missing_glyphs, &mut measured);
        for ((index, glyph), bound) in missing.iter().zip(&missing_glyphs).zip(&measured) {
            bounds[*index] = *bound;
            entry.set(*glyph, *bound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GlyphBoundsCache, PAGE, PAGES_PER_SIZE, SIZES};
    use skia_safe::Rect;
    use std::cell::Cell;

    /// A measure that gives every glyph a box of its index and counts the glyphs it was asked for.
    fn measured(asked: &Cell<usize>) -> impl FnOnce(&[u16], &mut [Rect]) + '_ {
        move |glyphs, bounds| {
            asked.set(asked.get() + glyphs.len());
            for (glyph, bound) in glyphs.iter().zip(bounds) {
                *bound = Rect::new(0.0, -(*glyph as f32), 1.0, 0.0);
            }
        }
    }

    #[test]
    fn a_glyph_is_measured_once_a_size_and_its_bounds_are_the_measured_ones() {
        let cache = GlyphBoundsCache::default();
        let asked = Cell::new(0);
        let mut bounds = [Rect::default(); 3];
        cache.get(12.0, &[5, 6, 5], &mut bounds, measured(&asked));
        assert_eq!(asked.get(), 3, "the glyphs not kept, duplicates measured as asked");
        assert_eq!(bounds, [Rect::new(0.0, -5.0, 1.0, 0.0), Rect::new(0.0, -6.0, 1.0, 0.0), Rect::new(0.0, -5.0, 1.0, 0.0)]);

        let mut again = [Rect::default(); 2];
        cache.get(12.0, &[6, 5], &mut again, |_, _| panic!("all kept"));
        assert_eq!(again, [Rect::new(0.0, -6.0, 1.0, 0.0), Rect::new(0.0, -5.0, 1.0, 0.0)]);

        // Another size measures again; a new glyph of a kept size is the only one asked for.
        cache.get(13.0, &[5], &mut bounds[..1], measured(&asked));
        assert_eq!(asked.get(), 4);
        cache.get(12.0, &[5, 7], &mut bounds[..2], measured(&asked));
        assert_eq!(asked.get(), 5);
    }

    #[test]
    fn a_new_size_takes_the_place_of_the_least_recently_used_one_when_missed_again() {
        let cache = GlyphBoundsCache::default();
        let asked = Cell::new(0);
        let mut bound = [Rect::default(); 1];
        for size in 0..SIZES {
            cache.get(10.0 + size as f32, &[1], &mut bound, measured(&asked));
        }
        // The first size is used again, so the second (11) is the least recently used one.
        cache.get(10.0, &[1], &mut bound, |_, _| panic!("kept"));
        // A new size missed once is measured and not kept; missed again, it takes a place.
        cache.get(99.0, &[2], &mut bound, measured(&asked));
        assert_eq!(bound[0], Rect::new(0.0, -2.0, 1.0, 0.0));
        let before = asked.get();
        cache.get(99.0, &[2], &mut bound, measured(&asked));
        assert_eq!(asked.get(), before + 1, "the second miss measures and keeps");
        cache.get(99.0, &[2], &mut bound, |_, _| panic!("kept now"));
        cache.get(10.0, &[1], &mut bound, |_, _| panic!("still kept"));
        // 11 gave way: measured again.
        cache.get(11.0, &[1], &mut bound, measured(&asked));
        assert_eq!(asked.get(), before + 2);
        // The reused size forgets what the size before it kept.
        cache.get(99.0, &[1], &mut bound, measured(&asked));
        assert_eq!(bound[0], Rect::new(0.0, -1.0, 1.0, 0.0));
    }

    #[test]
    fn sizes_that_keep_changing_do_not_replace_the_kept_ones() {
        let cache = GlyphBoundsCache::default();
        let asked = Cell::new(0);
        let mut bound = [Rect::default(); 1];
        for size in 0..SIZES {
            cache.get(10.0 + size as f32, &[1], &mut bound, measured(&asked));
        }
        // Forty other sizes in turn, twice: each is measured every time and none is kept.
        for _ in 0..2 {
            for size in 0..40 {
                cache.get(50.0 + size as f32, &[3], &mut bound, measured(&asked));
                assert_eq!(bound[0], Rect::new(0.0, -3.0, 1.0, 0.0));
            }
        }
        assert_eq!(asked.get(), SIZES + 80);
        for size in 0..SIZES {
            cache.get(10.0 + size as f32, &[1], &mut bound, |_, _| panic!("the kept sizes stay"));
        }
    }

    #[test]
    fn a_size_keeps_a_bounded_number_of_pages() {
        let cache = GlyphBoundsCache::default();
        let asked = Cell::new(0);
        let glyphs: Vec<u16> = (0..=PAGES_PER_SIZE).map(|page| (page * PAGE) as u16).collect();
        let mut bounds = vec![Rect::default(); glyphs.len()];
        cache.get(12.0, &glyphs, &mut bounds, measured(&asked));
        assert_eq!(asked.get(), glyphs.len());
        // The glyph of the page beyond the bound is measured every time, and right.
        let last = *glyphs.last().unwrap();
        cache.get(12.0, &[last], &mut bounds[..1], measured(&asked));
        assert_eq!(asked.get(), glyphs.len() + 1);
        assert_eq!(bounds[0], Rect::new(0.0, -(last as f32), 1.0, 0.0));
        cache.get(12.0, &glyphs[..PAGES_PER_SIZE], &mut bounds[..PAGES_PER_SIZE], |_, _| panic!("kept"));
    }
}
