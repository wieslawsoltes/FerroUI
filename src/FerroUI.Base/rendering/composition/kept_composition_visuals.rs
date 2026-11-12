//! The composition visuals kept for visuals that are out of their visual tree (FerroUI
//! addition, docs/porting/desktop-performance.md, section 9.32).

use super::{CompositionDrawItem, CompositionDrawListVisual};
use crate::{Visual, WeakRef};
use std::rc::Rc;

/// What is kept of a visual that left its tree: its composition visual, or, for a visual
/// without one, what it drew (section 9.42).
pub(crate) enum KeptComposition {
    Visual(CompositionDrawListVisual),
    Item(Rc<CompositionDrawItem>),
}

/// A composition visual, or a drawing, kept for its visual. The visual is referred to weakly:
/// what keeps a composition visual keeps no visual alive.
pub(crate) type KeptCompositionVisual = (WeakRef<Visual>, KeptComposition);

/// The composition visuals of the visuals that left a visual tree inside
/// [`Visual::leave_tree_keeping_composition_visuals`], kept until this is released or dropped.
///
/// A visual among them that is attached to a tree of the same compositor before that rejoins
/// with the composition visual it had. Whatever is not back in a tree when this is released
/// lets go of its composition visual, exactly as it would have when it left. What holds this
/// (the recycle pool of a virtualizing panel) therefore decides how long the composition
/// visuals of what it took out of the tree live, and nothing lives longer than the holder.
pub struct KeptCompositionVisuals {
    entries: Vec<KeptCompositionVisual>,
}

impl KeptCompositionVisuals {
    pub(crate) fn new(entries: Vec<KeptCompositionVisual>) -> Self {
        Self { entries }
    }

    /// The number of composition visuals that were kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no composition visual was kept.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Releases the composition visuals of the visuals that are not in a tree again. Dropping
    /// does the same.
    pub fn release(mut self) {
        release_kept_composition_visuals(&mut self.entries);
    }
}

impl Drop for KeptCompositionVisuals {
    fn drop(&mut self) {
        release_kept_composition_visuals(&mut self.entries);
    }
}

/// Empties `entries`: a visual that is still out of its tree releases the composition visual
/// kept for it, and the composition visual of a visual that is gone is released as it was left.
/// A visual that is back in a tree, or holds another composition visual, is left alone. The
/// drawing kept for a visual without a composition visual goes the same way.
pub(crate) fn release_kept_composition_visuals(entries: &mut Vec<KeptCompositionVisual>) {
    for (visual, kept) in entries.drain(..) {
        match kept {
            KeptComposition::Visual(composition_visual) => match visual.upgrade() {
                Some(visual) => visual.release_departed_composition_visual(&composition_visual),
                // The visual is gone and its composition visual is still as it left it.
                None if composition_visual.is_departed() => {
                    Visual::release_orphaned_composition_visual(&composition_visual);
                }
                None => {}
            },
            KeptComposition::Item(item) => match visual.upgrade() {
                Some(visual) => visual.release_departed_composition_item(&item),
                None if item.is_departed() => item.release(),
                None => {}
            },
        }
    }
}
