use crate::animation::{IPageTransition, PageTransitionItem};
use crate::{Ref, Visual};

/// An [`IPageTransition`] that supports progress-driven updates, for
/// example while the user drags between pages.
///
/// Transitions implementing this interface publish everything they need
/// through the visual's properties and its render transform.
pub trait IProgressPageTransition: IPageTransition {
    /// Updates the transition to reflect the specified progress.
    ///
    /// `progress` is the normalized progress from 0 to 1; `from` and `to`
    /// are the source and target visuals; `forward` the direction;
    /// `page_length` the size of a page along the transition axis, and
    /// `visible_items` the currently visible realized pages, if more than
    /// one page is visible.
    fn update(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    );

    /// Resets any visual state applied to the given visual by this
    /// transition.
    fn reset(&self, visual: &Ref<Visual>);
}
