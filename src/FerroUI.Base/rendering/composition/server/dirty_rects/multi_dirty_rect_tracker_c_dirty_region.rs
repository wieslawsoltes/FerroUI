use super::i_dirty_rect_tracker::contains_rect;
use crate::platform::LtrbRect;

/// This is a port of CDirtyRegion2 from WPF
pub(super) struct CDirtyRegion2 {
    max_dirty_region_count: usize,
    dirty_regions: Vec<LtrbRect>,
    resolved_regions: Vec<LtrbRect>,
    /// `[max_dirty_region_count + 1, max_dirty_region_count]`, row-major.
    overhead: Vec<f64>,
    surface_bounds: LtrbRect,
    allowed_dirty_region_overhead: f64,
    region_count: usize,
    optimized: bool,
    max_surface_fallback: bool,
}

struct UnionResult {
    overhead: f64,
    // Left here for debugging purposes
    #[allow(dead_code)]
    area: f64,
    union: LtrbRect,
}

fn rect_area(r: LtrbRect) -> f64 {
    (r.right - r.left) * (r.bottom - r.top)
}

fn rect_union(left: LtrbRect, right: LtrbRect) -> LtrbRect {
    if left.is_zero_size() {
        return right;
    }
    if right.is_zero_size() {
        return left;
    }
    left.union(right)
}

fn compute_union(r0: LtrbRect, r1: LtrbRect) -> UnionResult {
    let unioned = rect_union(r0, r1);
    let intersected = r0.intersect_or_empty(r1);

    let area_of_union = rect_area(unioned);
    let mut overhead = area_of_union - (rect_area(r0) + rect_area(r1) - rect_area(intersected));

    // Use 0 as overhead if computed overhead is negative or overhead
    // computation returns a nan.  (If more than one of the previous
    // area computations overflowed then overhead could be not a
    // number.)
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(overhead > 0.0) {
        overhead = 0.0;
    }

    UnionResult { overhead, area: area_of_union, union: unioned }
}

impl CDirtyRegion2 {
    pub(super) fn new(max_dirty_region_count: i32) -> Self {
        // A negative count is a programmer error (the array allocation
        // throws in the C# source).
        let count = usize::try_from(max_dirty_region_count).expect("max_dirty_region_count must not be negative");
        Self {
            max_dirty_region_count: count,
            dirty_regions: vec![LtrbRect::default(); count],
            resolved_regions: vec![LtrbRect::default(); count],
            overhead: vec![0.0; (count + 1) * count],
            surface_bounds: LtrbRect::default(),
            allowed_dirty_region_overhead: 0.0,
            region_count: 0,
            optimized: false,
            max_surface_fallback: false,
        }
    }

    fn set_overhead(&mut self, i: usize, j: usize, value: f64) {
        let n = self.max_dirty_region_count;
        if i > j {
            self.overhead[i * n + j] = value;
        } else if i < j {
            self.overhead[j * n + i] = value;
        }
    }

    fn get_overhead(&self, i: usize, j: usize) -> f64 {
        let n = self.max_dirty_region_count;
        if i > j {
            return self.overhead[i * n + j];
        }

        if i < j {
            return self.overhead[j * n + i];
        }

        f64::MAX
    }

    fn update_overhead(&mut self, region_index: usize) {
        let region_at_index = self.dirty_regions[region_index];
        for i in 0..self.max_dirty_region_count {
            if region_index != i {
                let ur = compute_union(self.dirty_regions[i], region_at_index);
                self.set_overhead(i, region_index, ur.overhead);
            }
        }
    }

    /// Initialize must be called before adding dirty rects. Initialize can also be called to
    /// reset the dirty region.
    pub(super) fn initialize(&mut self, surface_bounds: LtrbRect, allowed_dirty_region_overhead: f64) {
        self.allowed_dirty_region_overhead = allowed_dirty_region_overhead;
        self.dirty_regions.fill(LtrbRect::default());
        self.overhead.fill(0.0);
        self.optimized = false;
        self.max_surface_fallback = false;
        self.region_count = 0;

        self.surface_bounds = surface_bounds;
    }

    /// Adds a new dirty rectangle to the dirty region.
    pub(super) fn add(&mut self, new_region: LtrbRect) {
        // We've already fallen back to setting the whole surface as a dirty region
        // because of invalid dirty rects, so no need to add any new ones
        if self.max_surface_fallback {
            return;
        }

        // Check if rectangle is well formed before we try to intersect it,
        // because Intersect will fail for badly formed rects
        if !new_region.is_well_ordered() {
            // If we're here it means that we've been passed an invalid rectangle as a dirty
            // region, containing NAN or a non well ordered rectangle.
            // In this case, make the dirty region the full surface size
            // since this could cause a serious perf regression.
            //
            // (The C# source additionally raises a debug-build assertion
            // here; it is left out so that bad input never aborts.)

            //
            // Remove all dirty regions from this object, since
            // they're no longer relevant.
            //
            self.initialize(self.surface_bounds, self.allowed_dirty_region_overhead);
            self.max_surface_fallback = true;
            self.region_count = 1;
            return;
        }

        let mut clipped_new_region = new_region.intersect_or_empty(self.surface_bounds);

        if clipped_new_region.is_empty() {
            return;
        }

        // Always keep bounding boxes device space integer.
        clipped_new_region = LtrbRect::new(
            clipped_new_region.left.floor(),
            clipped_new_region.top.floor(),
            clipped_new_region.right.ceil(),
            clipped_new_region.bottom.ceil(),
        );

        let max = self.max_dirty_region_count;

        // Compute the overhead for the new region combined with all existing regions
        for n in 0..max {
            let ur = compute_union(self.dirty_regions[n], clipped_new_region);
            self.set_overhead(max, n, ur.overhead);
        }

        // Find the pair of dirty regions that if merged create the minimal overhead. A overhead
        // of 0 is perfect in the sense that it can not get better. In that case we break early
        // out of the loop.
        let mut minimal_overhead = f64::MAX;
        let mut best_match_n = 0;
        let mut best_match_k = 0;
        let mut match_found = false;

        'search: for n in (1..=max).rev() {
            for k in 0..n {
                let overhead_nk = self.get_overhead(n, k);
                if minimal_overhead >= overhead_nk {
                    minimal_overhead = overhead_nk;
                    best_match_n = n;
                    best_match_k = k;
                    match_found = true;

                    if overhead_nk < self.allowed_dirty_region_overhead {
                        // If the overhead is very small, we bail out early since this
                        // saves us some valuable cycles. Note that "small" means really
                        // nothing here. In fact we don't always know if that number is
                        // actually small. However, it the algorithm stays still correct
                        // in the sense that we render everything that is necessary. It
                        // might just be not optimal.
                        break 'search;
                    }
                }
            }
        }

        if !match_found {
            return;
        }

        // Case A: The new dirty region can be combined with an existing one
        if best_match_n == max {
            let ur = compute_union(clipped_new_region, self.dirty_regions[best_match_k]);
            let unioned = ur.union;

            if contains_rect(&self.dirty_regions[best_match_k], unioned) {
                // newDirtyRegion is enclosed by dirty region bestMatchK
                return;
            }

            self.dirty_regions[best_match_k] = unioned;
            self.update_overhead(best_match_k);
        } else {
            // Case B: Merge region N with region K, store new region slot K
            let ur = compute_union(self.dirty_regions[best_match_n], self.dirty_regions[best_match_k]);
            self.dirty_regions[best_match_n] = ur.union;
            self.dirty_regions[best_match_k] = clipped_new_region;
            self.update_overhead(best_match_n);
            self.update_overhead(best_match_k);
        }
    }

    /// Returns an array of dirty rectangles describing the dirty region.
    pub(super) fn get_uninflated_dirty_regions(&mut self) -> &[LtrbRect] {
        if self.max_surface_fallback {
            return std::slice::from_ref(&self.surface_bounds);
        }

        if !self.optimized {
            self.resolved_regions.fill(LtrbRect::default());

            // Consolidate the dirtyRegions array
            let mut added_dirty_region_count = 0;
            for i in 0..self.max_dirty_region_count {
                if !self.dirty_regions[i].is_empty() {
                    if i != added_dirty_region_count {
                        self.dirty_regions[added_dirty_region_count] = self.dirty_regions[i];
                        self.update_overhead(added_dirty_region_count);
                    }

                    added_dirty_region_count += 1;
                }
            }

            // Merge all dirty rects that we can
            let mut could_merge = true;
            while could_merge {
                could_merge = false;
                for n in 0..added_dirty_region_count {
                    for k in n + 1..added_dirty_region_count {
                        if !self.dirty_regions[n].is_empty()
                            && !self.dirty_regions[k].is_empty()
                            && self.get_overhead(n, k) < self.allowed_dirty_region_overhead
                        {
                            let ur = compute_union(self.dirty_regions[n], self.dirty_regions[k]);
                            self.dirty_regions[n] = ur.union;
                            self.dirty_regions[k] = LtrbRect::default();
                            self.update_overhead(n);
                            could_merge = true;
                        }
                    }
                }
            }

            // Consolidate and copy into resolvedRegions
            let mut final_region_count = 0;
            for i in 0..added_dirty_region_count {
                if !self.dirty_regions[i].is_empty() {
                    self.resolved_regions[final_region_count] = self.dirty_regions[i];
                    final_region_count += 1;
                }
            }

            self.region_count = final_region_count;
            self.optimized = true;
        }

        &self.resolved_regions[..self.region_count]
    }

    /// Checks if the dirty region is empty.
    pub(super) fn is_empty(&self) -> bool {
        self.dirty_regions.iter().all(|r| r.is_empty())
    }

    /// Returns the dirty region count.
    /// NOTE: The region count is NOT VALID until GetUninflatedDirtyRegions is called.
    #[allow(dead_code)]
    pub(super) fn region_count(&self) -> i32 {
        self.region_count as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SURFACE: LtrbRect = LtrbRect::new(0.0, 0.0, 1000.0, 1000.0);

    fn region(max: i32, allowed_overhead: f64) -> CDirtyRegion2 {
        let mut region = CDirtyRegion2::new(max);
        region.initialize(SURFACE, allowed_overhead);
        region
    }

    fn sorted(rects: &[LtrbRect]) -> Vec<LtrbRect> {
        let mut rects = rects.to_vec();
        rects.sort_by(|a, b| (a.left, a.top).partial_cmp(&(b.left, b.top)).unwrap());
        rects
    }

    #[test]
    fn compute_union_overhead() {
        // Disjoint: union 30x10, areas 100 + 100.
        let ur = compute_union(LtrbRect::new(0.0, 0.0, 10.0, 10.0), LtrbRect::new(20.0, 0.0, 30.0, 10.0));
        assert_eq!(100.0, ur.overhead);
        assert_eq!(300.0, ur.area);
        assert_eq!(LtrbRect::new(0.0, 0.0, 30.0, 10.0), ur.union);

        // Overlapping: union 15x15 = 225, covered 100 + 100 - 25.
        let ur = compute_union(LtrbRect::new(0.0, 0.0, 10.0, 10.0), LtrbRect::new(5.0, 5.0, 15.0, 15.0));
        assert_eq!(50.0, ur.overhead);

        // Contained and adjacent rectangles waste nothing.
        let ur = compute_union(LtrbRect::new(0.0, 0.0, 10.0, 10.0), LtrbRect::new(2.0, 2.0, 4.0, 4.0));
        assert_eq!(0.0, ur.overhead);
        let ur = compute_union(LtrbRect::new(0.0, 0.0, 10.0, 10.0), LtrbRect::new(10.0, 0.0, 20.0, 10.0));
        assert_eq!(0.0, ur.overhead);

        // An empty rectangle is ignored by the union.
        let ur = compute_union(LtrbRect::default(), LtrbRect::new(50.0, 50.0, 60.0, 60.0));
        assert_eq!(0.0, ur.overhead);
        assert_eq!(LtrbRect::new(50.0, 50.0, 60.0, 60.0), ur.union);

        // NaN overhead (inf - inf) is reported as zero.
        let huge = LtrbRect::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::INFINITY);
        assert_eq!(0.0, compute_union(huge, huge).overhead);
    }

    #[test]
    fn overhead_matrix_is_symmetric_with_max_on_the_diagonal() {
        let mut region = region(3, 0.0);
        region.set_overhead(2, 1, 5.0);
        assert_eq!(5.0, region.get_overhead(2, 1));
        assert_eq!(5.0, region.get_overhead(1, 2));
        region.set_overhead(0, 3, 7.0);
        assert_eq!(7.0, region.get_overhead(3, 0));
        region.set_overhead(1, 1, 9.0);
        assert_eq!(f64::MAX, region.get_overhead(1, 1));
    }

    #[test]
    fn starts_empty() {
        let mut region = region(4, 0.0);
        assert!(region.is_empty());
        assert!(region.get_uninflated_dirty_regions().is_empty());
        assert_eq!(0, region.region_count());
    }

    #[test]
    fn keeps_up_to_max_disjoint_rects() {
        let mut region = region(4, 0.0);
        let rects = [
            LtrbRect::new(0.0, 0.0, 10.0, 10.0),
            LtrbRect::new(100.0, 0.0, 110.0, 10.0),
            LtrbRect::new(0.0, 100.0, 10.0, 110.0),
            LtrbRect::new(100.0, 100.0, 110.0, 110.0),
        ];
        for rect in rects {
            region.add(rect);
        }
        assert!(!region.is_empty());
        assert_eq!(sorted(&rects), sorted(region.get_uninflated_dirty_regions()));
        assert_eq!(4, region.region_count());
    }

    #[test]
    fn merges_the_pair_with_the_least_added_area_when_full() {
        let mut region = region(4, 0.0);
        region.add(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
        region.add(LtrbRect::new(500.0, 0.0, 510.0, 10.0));
        region.add(LtrbRect::new(0.0, 500.0, 10.0, 510.0));
        region.add(LtrbRect::new(500.0, 500.0, 510.0, 510.0));
        // The fifth rectangle is closest to the first one: merging those two
        // wastes 20x10 - 200 + ... = 100, far less than any other pair.
        region.add(LtrbRect::new(20.0, 0.0, 30.0, 10.0));

        assert_eq!(
            sorted(&[
                LtrbRect::new(0.0, 0.0, 30.0, 10.0),
                LtrbRect::new(500.0, 0.0, 510.0, 10.0),
                LtrbRect::new(0.0, 500.0, 10.0, 510.0),
                LtrbRect::new(500.0, 500.0, 510.0, 510.0),
            ]),
            sorted(region.get_uninflated_dirty_regions())
        );
    }

    #[test]
    fn merges_two_existing_rects_when_that_is_cheaper_than_growing_one() {
        let mut region = region(3, 0.0);
        // Two close rectangles and a distant one.
        region.add(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
        region.add(LtrbRect::new(12.0, 0.0, 22.0, 10.0));
        region.add(LtrbRect::new(900.0, 900.0, 910.0, 910.0));
        // The new one is far from everything: the cheapest merge is between
        // the two existing close rectangles (case B), and the new rectangle
        // takes the freed slot.
        region.add(LtrbRect::new(0.0, 900.0, 10.0, 910.0));

        assert_eq!(
            sorted(&[
                LtrbRect::new(0.0, 0.0, 22.0, 10.0),
                LtrbRect::new(0.0, 900.0, 10.0, 910.0),
                LtrbRect::new(900.0, 900.0, 910.0, 910.0),
            ]),
            sorted(region.get_uninflated_dirty_regions())
        );
    }

    #[test]
    fn enclosed_rect_changes_nothing_when_some_overhead_is_allowed() {
        let mut region = region(2, 1.0);
        region.add(LtrbRect::new(0.0, 0.0, 100.0, 100.0));
        region.add(LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        assert_eq!(vec![LtrbRect::new(0.0, 0.0, 100.0, 100.0)], region.get_uninflated_dirty_regions().to_vec());
    }

    #[test]
    fn enclosed_rect_is_kept_separately_when_no_overhead_is_allowed() {
        // With an allowed overhead of zero the search never exits early and
        // ties go to the last pair examined, which is a pair of existing
        // slots: the enclosed rectangle takes a slot of its own, and
        // resolving does not merge it either (0 < 0 is false).
        let mut region = region(2, 0.0);
        region.add(LtrbRect::new(0.0, 0.0, 100.0, 100.0));
        region.add(LtrbRect::new(10.0, 10.0, 20.0, 20.0));
        assert_eq!(
            vec![LtrbRect::new(10.0, 10.0, 20.0, 20.0), LtrbRect::new(0.0, 0.0, 100.0, 100.0)],
            region.get_uninflated_dirty_regions().to_vec()
        );
    }

    /// Reference results obtained by running the C# implementation of the
    /// algorithm on the same pseudo random input: (max region count, allowed
    /// overhead, number of rectangles added, resolved regions).
    const REFERENCE: &[(i32, f64, usize, &str)] = &[
        (4, 0.0, 3, "[811,343,855,401][750,466,755,520][150,376,233,439]"),
        (4, 0.0, 5, "[139,645,191,656][750,343,855,520][542,186,559,191][150,376,233,439]"),
        (4, 0.0, 20, "[979,0,1000,46][655,343,1000,800][542,146,1000,261][139,356,490,800]"),
        (4, 0.0, 60, "[540,0,1000,46][125,126,226,195][542,146,1000,261][0,276,1000,800]"),
        (8, 500.0, 3, "[150,376,233,439][811,343,855,401][750,466,755,520]"),
        (8, 500.0, 5, "[150,376,233,439][811,343,855,401][750,466,755,520][542,186,559,191][139,645,191,656]"),
        (8, 500.0, 20, "[979,0,1000,46][811,343,855,401][351,356,755,520][916,571,1000,658][139,376,233,656][542,186,618,261][339,766,742,800][775,146,1000,165]"),
        (8, 500.0, 60, "[0,276,84,709][125,126,226,195][320,344,758,582][698,298,1000,750][139,352,334,722][524,17,721,374][0,766,1000,800][775,0,1000,207]"),
        (1, 0.0, 3, "[150,343,855,520]"),
        (1, 0.0, 5, "[139,186,855,656]"),
        (1, 0.0, 20, "[139,0,1000,800]"),
        (1, 0.0, 60, "[0,0,1000,800]"),
        (3, 50000.0, 3, "[150,376,233,439][750,343,855,520]"),
        (3, 50000.0, 5, "[139,376,233,656][750,343,855,520][542,186,559,191]"),
        (3, 50000.0, 20, "[139,356,490,800][655,0,1000,800][542,146,1000,261]"),
        (3, 50000.0, 60, "[125,126,226,195][0,247,746,800][540,0,1000,800]"),
    ];

    #[test]
    fn matches_the_reference_implementation() {
        for &(max, overhead, count, expected) in REFERENCE {
            let mut region = CDirtyRegion2::new(max);
            region.initialize(LtrbRect::new(0.0, 0.0, 1000.0, 800.0), overhead);
            let mut seed = 12345u32;
            let mut next = |modulus: u32| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 8) % modulus) as f64
            };
            for _ in 0..count {
                let x = next(1100);
                let y = next(900);
                let w = next(120);
                let h = next(90);
                region.add(LtrbRect::new(
                    x - 50.0 + 0.25,
                    y - 50.0 + 0.5,
                    x - 50.0 + w + 0.75,
                    y - 50.0 + h + 0.625,
                ));
            }
            let actual: String = region
                .get_uninflated_dirty_regions()
                .iter()
                .map(|r| format!("[{},{},{},{}]", r.left, r.top, r.right, r.bottom))
                .collect();
            assert_eq!(expected, actual, "max {max}, overhead {overhead}, count {count}");
        }
    }

    #[test]
    fn clips_to_the_surface_and_snaps_outwards_to_integers() {
        let mut region = region(4, 0.0);
        region.add(LtrbRect::new(-50.0, 10.3, 20.2, 30.7));
        assert_eq!(vec![LtrbRect::new(0.0, 10.0, 21.0, 31.0)], region.get_uninflated_dirty_regions().to_vec());

        // Entirely outside the surface: ignored.
        let mut region = self::region(4, 0.0);
        region.add(LtrbRect::new(2000.0, 2000.0, 2010.0, 2010.0));
        // Zero-sized: ignored.
        region.add(LtrbRect::new(5.0, 5.0, 5.0, 50.0));
        assert!(region.is_empty());
        assert!(region.get_uninflated_dirty_regions().is_empty());
    }

    #[test]
    fn allowed_overhead_merges_nearby_rects_early() {
        // Merging the two rectangles wastes 100 units, which is allowed.
        let mut region = region(4, 150.0);
        region.add(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
        region.add(LtrbRect::new(20.0, 0.0, 30.0, 10.0));
        region.add(LtrbRect::new(500.0, 500.0, 510.0, 510.0));
        assert_eq!(
            sorted(&[LtrbRect::new(0.0, 0.0, 30.0, 10.0), LtrbRect::new(500.0, 500.0, 510.0, 510.0)]),
            sorted(region.get_uninflated_dirty_regions())
        );
        assert_eq!(2, region.region_count());
    }

    #[test]
    fn resolved_regions_are_cached_until_initialize() {
        let mut region = region(4, 0.0);
        region.add(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(1, region.get_uninflated_dirty_regions().len());
        // Once resolved, later additions are not reflected (as in the
        // original algorithm) until the region is re-initialized.
        region.add(LtrbRect::new(500.0, 500.0, 510.0, 510.0));
        assert_eq!(1, region.get_uninflated_dirty_regions().len());

        region.initialize(SURFACE, 0.0);
        assert!(region.is_empty());
        assert!(region.get_uninflated_dirty_regions().is_empty());
    }

    #[test]
    fn invalid_rect_falls_back_to_the_whole_surface() {
        for invalid in [
            LtrbRect::new(10.0, 10.0, 5.0, 20.0),
            LtrbRect::new(f64::NAN, 0.0, 10.0, 10.0),
            LtrbRect::new(0.0, 0.0, 10.0, f64::NAN),
        ] {
            let mut region = region(4, 0.0);
            region.add(LtrbRect::new(0.0, 0.0, 10.0, 10.0));
            region.add(invalid);
            assert_eq!(1, region.region_count());
            // Further rectangles are ignored.
            region.add(LtrbRect::new(500.0, 500.0, 510.0, 510.0));
            assert_eq!(vec![SURFACE], region.get_uninflated_dirty_regions().to_vec());
            // The individual regions were dropped.
            assert!(region.is_empty());

            region.initialize(SURFACE, 0.0);
            assert!(region.get_uninflated_dirty_regions().is_empty());
        }
    }

    #[test]
    fn many_rects_never_exceed_max_and_always_cover_the_input() {
        let mut region = region(6, 0.0);
        let mut input = Vec::new();
        // A deterministic scatter of small rectangles.
        let mut seed = 12345u32;
        for _ in 0..200 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = (seed >> 8) % 950;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let y = (seed >> 8) % 950;
            let rect = LtrbRect::new(x as f64, y as f64, x as f64 + 37.5, y as f64 + 21.25);
            input.push(rect);
            region.add(rect);
        }
        let resolved = region.get_uninflated_dirty_regions().to_vec();
        assert!(!resolved.is_empty() && resolved.len() <= 6);
        for rect in input {
            assert!(
                resolved.iter().any(|r| contains_rect(r, rect)),
                "{rect:?} is not covered by {resolved:?}"
            );
        }
    }
}
