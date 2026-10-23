//! Spatial broad phase for prepared roots. Geometry remains owned by FerroUI.
//!
//! The balanced tree partitions by spatial centers, independently of stacking.
//! Queries restore preorder before painting, so overlapping alpha/blend layers
//! retain exactly the document order. A scoped edit refits only its leaf and
//! ancestors; unchanged viewports reuse the candidate buffer without allocation.

use super::{PreparedNode, Rect};

pub(super) const MIN_INDEXED_ROOTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    // None is an unbounded conservative fallback for nonfinite derived bounds.
    rect: Option<Rect>,
    nodes: usize,
}

impl Bounds {
    fn new(rect: Rect, nodes: usize) -> Self {
        Self {
            rect: (rect.is_finite() && rect.width >= 0.0 && rect.height >= 0.0).then_some(rect),
            nodes,
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            rect: self.rect.zip(other.rect).and_then(|(a, b)| {
                let union = a.union(b);
                union.is_finite().then_some(union)
            }),
            nodes: self.nodes.saturating_add(other.nodes),
        }
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Leaf { prepared: usize },
    Branch { left: usize, right: usize },
}

struct Entry {
    bounds: Bounds,
    parent: Option<usize>,
    kind: Kind,
}

#[derive(Clone, Copy)]
struct Leaf {
    slot: usize,
    prepared: usize,
    bounds: Bounds,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct QueryStats {
    pub(super) bounds_tests: usize,
    pub(super) culled_nodes: usize,
    pub(super) reused: bool,
}

#[derive(Default)]
pub(super) struct RootPaintIndex {
    entries: Vec<Entry>,
    leaves: Vec<usize>,
    ordered_roots: Vec<usize>,
    candidates: Vec<usize>,
    all_visible: bool,
    viewport: Option<Rect>,
    query_stats: QueryStats,
}

impl RootPaintIndex {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.leaves.clear();
        self.ordered_roots.clear();
        self.candidates.clear();
        self.all_visible = false;
        self.viewport = None;
        self.query_stats = QueryStats::default();
    }

    pub(super) fn rebuild(&mut self, scene: &[PreparedNode], roots: &[usize]) {
        self.clear();
        if roots.len() < MIN_INDEXED_ROOTS {
            return;
        }
        let mut leaves: Vec<_> = roots
            .iter()
            .enumerate()
            .map(|(slot, &prepared)| Leaf {
                slot,
                prepared,
                bounds: Bounds::new(
                    scene[prepared].world_subtree_bounds,
                    scene[prepared].subtree_nodes,
                ),
            })
            .collect();
        self.build_leaves(&mut leaves);
    }

    fn build_leaves(&mut self, leaves: &mut [Leaf]) {
        self.ordered_roots
            .extend(leaves.iter().map(|leaf| leaf.prepared));
        self.leaves.resize(leaves.len(), 0);
        self.entries
            .reserve(leaves.len().saturating_mul(2).saturating_sub(1));
        if !leaves.is_empty() {
            self.build_branch(leaves, None);
        }
    }

    fn build_branch(&mut self, leaves: &mut [Leaf], parent: Option<usize>) -> usize {
        let bounds = leaves
            .iter()
            .skip(1)
            .fold(leaves[0].bounds, |bounds, leaf| bounds.union(leaf.bounds));
        let index = self.entries.len();
        self.entries.push(Entry {
            bounds,
            parent,
            kind: Kind::Leaf {
                prepared: leaves[0].prepared,
            },
        });
        if leaves.len() == 1 {
            self.leaves[leaves[0].slot] = index;
        } else {
            // Median partition bounds depth even for overlapping or degenerate
            // artboards. A stable slot tie-breaker makes construction repeatable.
            let horizontal = bounds.rect.is_none_or(|rect| rect.width >= rect.height);
            let center = |leaf: &Leaf| {
                leaf.bounds
                    .rect
                    .map(|rect| {
                        if horizontal {
                            rect.x + rect.width * 0.5
                        } else {
                            rect.y + rect.height * 0.5
                        }
                    })
                    .unwrap_or(0.0)
            };
            let middle = leaves.len() / 2;
            leaves.select_nth_unstable_by(middle, |a, b| {
                center(a).total_cmp(&center(b)).then(a.slot.cmp(&b.slot))
            });
            let (left, right) = leaves.split_at_mut(middle);
            let left = self.build_branch(left, Some(index));
            let right = self.build_branch(right, Some(index));
            self.entries[index].kind = Kind::Branch { left, right };
        }
        index
    }

    pub(super) fn is_active(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Refit a known stable root slot. Structural changes use rebuild instead.
    /// Returns the number of bounds entries actually changed, including the leaf.
    pub(super) fn update(&mut self, slot: usize, rect: Rect, nodes: usize) -> usize {
        let Some(&leaf) = self.leaves.get(slot) else {
            return 0;
        };
        let bounds = Bounds::new(rect, nodes);
        if self.entries[leaf].bounds == bounds {
            return 0;
        }
        self.viewport = None;
        self.entries[leaf].bounds = bounds;
        let mut changed = 1;
        let mut parent = self.entries[leaf].parent;
        while let Some(index) = parent {
            let Kind::Branch { left, right } = self.entries[index].kind else {
                unreachable!()
            };
            let bounds = self.entries[left].bounds.union(self.entries[right].bounds);
            if self.entries[index].bounds == bounds {
                break;
            }
            self.entries[index].bounds = bounds;
            changed += 1;
            parent = self.entries[index].parent;
        }
        changed
    }

    pub(super) fn query(&mut self, viewport: Rect) -> QueryStats {
        if self.viewport == Some(viewport) {
            return QueryStats {
                bounds_tests: 0,
                reused: true,
                ..self.query_stats
            };
        }
        self.candidates.clear();
        let mut stats = QueryStats::default();
        self.all_visible = self
            .entries
            .first()
            .and_then(|entry| entry.bounds.rect)
            .is_some_and(|bounds| viewport.contains_rect(bounds));
        if self.all_visible {
            // Fit-to-page is common and all roots are already in paint order.
            // Do not traverse or sort a dense scene just to rediscover them.
            stats.bounds_tests = 1;
        } else if !self.entries.is_empty() {
            Self::query_branch(&self.entries, 0, viewport, &mut self.candidates, &mut stats);
            // Prepared indices are monotonically increasing document paint order.
            self.candidates.sort_unstable();
        }
        self.viewport = Some(viewport);
        self.query_stats = stats;
        stats
    }

    fn query_branch(
        entries: &[Entry],
        index: usize,
        viewport: Rect,
        output: &mut Vec<usize>,
        stats: &mut QueryStats,
    ) {
        let entry = &entries[index];
        stats.bounds_tests += 1;
        if entry
            .bounds
            .rect
            .is_some_and(|bounds| !bounds.intersects(viewport))
        {
            stats.culled_nodes += entry.bounds.nodes;
            return;
        }
        match entry.kind {
            Kind::Leaf { prepared } => output.push(prepared),
            Kind::Branch { left, right } => {
                Self::query_branch(entries, left, viewport, output, stats);
                Self::query_branch(entries, right, viewport, output, stats);
            }
        }
    }

    pub(super) fn candidates(&self) -> &[usize] {
        if self.all_visible {
            &self.ordered_roots
        } else {
            &self.candidates
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(rects: &[Rect]) -> RootPaintIndex {
        let mut index = RootPaintIndex::default();
        let mut leaves: Vec<_> = rects
            .iter()
            .enumerate()
            .map(|(slot, &rect)| Leaf {
                slot,
                prepared: slot * 3,
                bounds: Bounds::new(rect, slot % 3 + 1),
            })
            .collect();
        index.build_leaves(&mut leaves);
        index
    }

    fn assert_matches_scan(index: &mut RootPaintIndex, rects: &[Rect], viewport: Rect) {
        let stats = index.query(viewport);
        let expected: Vec<_> = rects
            .iter()
            .enumerate()
            .filter(|(_, rect)| rect.intersects(viewport))
            .map(|(slot, _)| slot * 3)
            .collect();
        let culled: usize = rects
            .iter()
            .enumerate()
            .filter(|(_, rect)| !rect.intersects(viewport))
            .map(|(slot, _)| slot % 3 + 1)
            .sum();
        assert_eq!(index.candidates(), expected);
        assert_eq!(stats.culled_nodes, culled);
    }

    #[test]
    fn spatial_partition_culls_scrambled_stacking_without_reordering_paint() {
        let rects: Vec<_> = (0..10_000)
            .map(|slot| {
                let cell = (slot * 7919) % 10_000;
                Rect::new(
                    (cell % 100) as f64 * 30.0,
                    (cell / 100) as f64 * 30.0,
                    20.0,
                    20.0,
                )
            })
            .collect();
        let mut index = index(&rects);
        let viewport = Rect::new(1495.0, 1495.0, 70.0, 70.0);
        assert_matches_scan(&mut index, &rects, viewport);
        assert!(
            index.query_stats.bounds_tests < 150,
            "{} bounds tests",
            index.query_stats.bounds_tests
        );
        let cached = index.query(viewport);
        assert!(cached.reused);
        assert_eq!(cached.bounds_tests, 0);
        for x in [0.0, 300.0, 2990.0, 4000.0] {
            assert_matches_scan(&mut index, &rects, Rect::new(x, 100.0, 90.0, 90.0));
        }
        assert_matches_scan(&mut index, &rects, Rect::new(-10.0, -10.0, 3020.0, 3020.0));
        assert_eq!(index.query_stats.bounds_tests, 1);
        assert!(
            index.candidates.is_empty(),
            "dense viewport reuses the retained paint-order roots"
        );
    }

    #[test]
    fn refitting_moved_roots_invalidates_candidates_and_preserves_order_and_counts() {
        let mut rects: Vec<_> = (0..256)
            .map(|i| Rect::new(i as f64 * 20.0, 0.0, 12.0, 12.0))
            .collect();
        let mut index = index(&rects);
        let viewport = Rect::new(0.0, 0.0, 128.0, 128.0);
        assert_matches_scan(&mut index, &rects, viewport);
        for slot in [200, 0, 255, 1, 120] {
            let old = rects[slot];
            rects[slot] = Rect::new(35.0, 15.0, 15.0, 15.0);
            let changed = index.update(slot, rects[slot], slot % 3 + 1);
            assert!((1..=9).contains(&changed));
            assert_matches_scan(&mut index, &rects, viewport);
            assert_eq!(index.update(slot, rects[slot], slot % 3 + 1), 0);
            assert!(index.query(viewport).reused);
            rects[slot] = old;
            index.update(slot, old, slot % 3 + 1);
            assert_matches_scan(&mut index, &rects, viewport);
        }
    }

    #[test]
    fn overlap_boundary_contact_empty_tree_and_nonfinite_bounds_are_conservative() {
        let rects = [
            Rect::new(0.0, 0.0, 0.0, 0.0),
            Rect::new(5.0, 5.0, 10.0, 10.0),
            Rect::new(5.0, 5.0, 10.0, 10.0),
        ];
        let mut index = index(&rects);
        for viewport in [
            Rect::new(15.0, 15.0, 0.0, 0.0),
            Rect::new(-5.0, -5.0, 5.0, 5.0),
            Rect::new(0.0, 0.0, 50.0, 50.0),
        ] {
            assert_matches_scan(&mut index, &rects, viewport);
        }
        index.update(1, Rect::new(f64::NAN, 0.0, 1.0, 1.0), 2);
        index.query(Rect::new(5000.0, 5000.0, 10.0, 10.0));
        assert_eq!(index.candidates(), &[3]);
        index.clear();
        assert!(!index.is_active());
        assert_eq!(index.query(Rect::default()).culled_nodes, 0);
        assert!(index.candidates().is_empty());
    }
}
