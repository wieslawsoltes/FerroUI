use std::cell::{Cell, OnceCell};

use super::character_to_glyph_map::CharacterToGlyphMap;
use super::codepoint_range::CodepointRange;

/// A read-only dictionary view (code point to glyph) over a
/// [`CharacterToGlyphMap`].
pub struct CharacterToGlyphMapDictionary {
    map: CharacterToGlyphMap,
    cached_ranges: OnceCell<Vec<CodepointRange>>,
    cached_count: Cell<i32>,
}

impl CharacterToGlyphMapDictionary {
    pub fn new(map: CharacterToGlyphMap) -> Self {
        Self { map, cached_ranges: OnceCell::new(), cached_count: Cell::new(-1) }
    }

    /// The glyph for the code point (the reference's indexer).
    ///
    /// Panics when the code point is not in the map; use
    /// [`try_get_value`](Self::try_get_value) for untrusted keys.
    pub fn get(&self, key: i32) -> u16 {
        match self.map.try_get_glyph(key) {
            Some(glyph_id) => glyph_id,
            None => panic!("The code point {key} was not found in the character map."),
        }
    }

    /// The mapped code points, in range order.
    pub fn keys(&self) -> impl Iterator<Item = i32> + '_ {
        // Membership must match try_get_value (try_get_glyph), not contains_glyph:
        // the two predicates disagree for mappings that resolve to glyph 0, and
        // keys yielding a key the indexer rejects breaks the dictionary contract.
        self.iter().map(|(code_point, _)| code_point)
    }

    /// The glyphs of the mapped code points, in range order.
    pub fn values(&self) -> impl Iterator<Item = u16> + '_ {
        self.iter().map(|(_, glyph_id)| glyph_id)
    }

    /// The number of mapped code points (computed once).
    pub fn count(&self) -> i32 {
        let cached_count = self.cached_count.get();

        if cached_count >= 0 {
            return cached_count;
        }

        let count = self.iter().count() as i32;

        self.cached_count.set(count);

        count
    }

    pub fn contains_key(&self, key: i32) -> bool {
        self.map.try_get_glyph(key).is_some()
    }

    pub fn try_get_value(&self, key: i32) -> Option<u16> {
        self.map.try_get_glyph(key)
    }

    /// Enumerates the `(code point, glyph)` pairs, in range order.
    pub fn iter(&self) -> impl Iterator<Item = (i32, u16)> + '_ {
        self.get_ranges().iter().flat_map(move |range| {
            (range.start..=range.end)
                .filter_map(move |code_point| self.map.try_get_glyph(code_point).map(|glyph_id| (code_point, glyph_id)))
        })
    }

    fn get_ranges(&self) -> &[CodepointRange] {
        self.cached_ranges.get_or_init(|| self.map.get_mapped_ranges().collect())
    }
}
