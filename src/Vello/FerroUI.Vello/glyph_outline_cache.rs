//! Per-immutable-run curve preparation. Translation never changes the error of
//! a flattened curve; scale, rotation and shear do and belong in the key.
use kurbo::{Affine, BezPath, PathEl};
use std::{collections::HashMap, mem::size_of, sync::Arc};

// Bound both path payload and metadata, independently of the text length or
// the number of camera transforms encountered over the run's lifetime.
const MAX_PATH_BYTES: usize = 128 * 1024;
const MAX_ENTRIES: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key { glyph: usize, linear: [u64; 4] }

struct Entry {
    // None remembers an over-budget contour without retaining its vertices.
    path: Option<Arc<BezPath>>,
    bytes: usize,
    used: u64,
}

#[derive(Default)]
pub(super) struct GlyphOutlineCache {
    entries: HashMap<Key, Entry>,
    bytes: usize,
    clock: u64,
    #[cfg(test)]
    hits: u64,
    #[cfg(test)]
    misses: u64,
}

impl GlyphOutlineCache {
    pub(super) fn prepare(&mut self, glyph: usize, path: &BezPath, transform: Affine)
        -> Option<(Arc<BezPath>, Affine)> {
        if !transform.is_finite() || transform.determinant() == 0.0 {
            return None;
        }
        let [a, b, c, d, x, y] = transform.as_coeffs();
        let key = Key { glyph, linear: [a.to_bits(), b.to_bits(), c.to_bits(), d.to_bits()] };
        let translation = Affine::translate((x, y));
        self.clock = self.clock.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.used = self.clock;
            #[cfg(test)] { self.hits += 1; }
            return entry.path.as_ref().map(|path| (path.clone(), translation));
        }
        if !path.elements().iter().any(|item| matches!(item, PathEl::QuadTo(..) | PathEl::CurveTo(..))) {
            return None;
        }
        #[cfg(test)] { self.misses += 1; }
        let linear = Affine::new([a, b, c, d, 0.0, 0.0]);
        let mut lines = Vec::new();
        let mut over_budget = false;
        let limit = MAX_PATH_BYTES / size_of::<PathEl>();
        {
            let _perf = crate::perf::scope(crate::perf::Phase::Flatten, path.elements().len() as u64);
            kurbo::flatten(path.iter().map(|item| linear * item), crate::scene::glyph_curve_tolerance(), |item| {
                if lines.len() < limit { lines.push(item); }
                else { over_budget = true; }
            });
        }
        let prepared = if over_budget { None } else {
            lines.shrink_to_fit();
            Some(Arc::new(BezPath::from_vec(lines)))
        };
        let bytes = prepared.as_ref().map_or(0, |path| path.elements().len() * size_of::<PathEl>());
        while self.entries.len() >= MAX_ENTRIES || self.bytes + bytes > MAX_PATH_BYTES {
            let Some(oldest) = self.entries.iter().min_by_key(|(_, entry)| entry.used).map(|(key, _)| *key) else { break; };
            self.bytes -= self.entries.remove(&oldest).expect("the oldest cache entry exists").bytes;
        }
        self.bytes += bytes;
        self.entries.insert(key, Entry { path: prepared.clone(), bytes, used: self.clock });
        prepared.map(|path| (path, translation))
    }

    #[cfg(test)]
    pub(super) fn stats(&self) -> (usize, usize, u64, u64) {
        (self.entries.len(), self.bytes, self.hits, self.misses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> BezPath {
        let mut path = BezPath::new();
        path.move_to((0.125, 0.375));
        path.curve_to((4.25, 19.875), (18.375, -7.625), (23.125, 8.375));
        path.close_path();
        path
    }

    #[test]
    fn unhinted_preparation_cache_reuses_translation_but_not_scale_shear_or_another_glyph() {
        let mut cache = GlyphOutlineCache::default();
        let path = curve();
        let (first, _) = cache.prepare(0, &path, Affine::IDENTITY).unwrap();
        let (panned, placement) = cache.prepare(0, &path, Affine::translate((7.125, -9.375))).unwrap();
        assert!(Arc::ptr_eq(&first, &panned));
        assert_eq!(placement, Affine::translate((7.125, -9.375)));
        let (scaled, _) = cache.prepare(0, &path, Affine::scale(1.25)).unwrap();
        let (sheared, _) = cache.prepare(0, &path, Affine::new([1.0, 0.25, -0.125, 1.0, 0.0, 0.0])).unwrap();
        let (other, _) = cache.prepare(1, &path, Affine::IDENTITY).unwrap();
        assert!(!Arc::ptr_eq(&first, &scaled));
        assert!(!Arc::ptr_eq(&first, &sheared));
        assert!(!Arc::ptr_eq(&first, &other));
        assert_eq!(cache.stats().2, 1);
        assert_eq!(cache.stats().3, 4);
        assert!(first.elements().iter().all(|item| !matches!(item, PathEl::QuadTo(..) | PathEl::CurveTo(..))));
    }

    #[test]
    fn unhinted_preparation_cache_evicts_old_transforms_and_bounds_path_bytes() {
        let mut cache = GlyphOutlineCache::default();
        let path = curve();
        let (first, _) = cache.prepare(0, &path, Affine::IDENTITY).unwrap();
        for glyph in 1..MAX_ENTRIES + 2 {
            cache.prepare(glyph, &path, Affine::IDENTITY).unwrap();
            assert!(cache.entries.len() <= MAX_ENTRIES);
            assert!(cache.bytes <= MAX_PATH_BYTES);
        }
        let (recomputed, _) = cache.prepare(0, &path, Affine::IDENTITY).unwrap();
        assert!(!Arc::ptr_eq(&first, &recomputed));
        assert_eq!(*first, *recomputed, "Eviction cannot change the prepared geometry");
        let mut oversized = BezPath::new();
        for _ in 0..MAX_PATH_BYTES / size_of::<PathEl>() { oversized.extend(path.iter()); }
        assert!(cache.prepare(9999, &oversized, Affine::IDENTITY).is_none());
        assert!(cache.entries.len() <= MAX_ENTRIES && cache.bytes <= MAX_PATH_BYTES);
        let misses = cache.stats().3;
        assert!(cache.prepare(9999, &oversized, Affine::translate((2.0, 4.0))).is_none());
        assert_eq!(cache.stats().3, misses, "An oversized contour must not be flattened speculatively again");
    }
}
