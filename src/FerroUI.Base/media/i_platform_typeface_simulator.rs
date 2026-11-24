use std::rc::Rc;

use crate::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface};

/// Implemented by an [`IPlatformTypeface`] that can make variants of itself with other
/// [`FontSimulations`], reached through [`IPlatformTypeface::as_simulator`].
///
/// A font collection that needs a synthetic bold or oblique face asks the platform typeface it
/// already holds for a simulated variant. The variant shares the font data of its source, so no
/// copy of the font file is made. When the variant is refused, the collection uses the source
/// face without the simulations. Only a platform typeface without a simulator makes the
/// collection create the synthetic face from the font stream of
/// [`IPlatformTypeface::try_get_stream`].
pub trait IPlatformTypefaceSimulator {
    /// The platform typeface of the same font face with `font_simulations`, reporting `style`,
    /// `weight` and `stretch`.
    ///
    /// `font_simulations` are all the simulations of the variant: a simulation this typeface
    /// carries is kept only when it is listed. The variant shares the font data of this
    /// typeface.
    ///
    /// `None` when the variant would leave out a simulation the platform applied inside this
    /// typeface, which cannot be removed.
    fn try_create_simulated_typeface(
        &self,
        font_simulations: FontSimulations,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>>;
}
