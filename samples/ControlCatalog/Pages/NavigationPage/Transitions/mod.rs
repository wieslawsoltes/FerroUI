//! The custom page transitions of the `NavigationPage` samples (namespace `ControlCatalog.Pages`,
//! directory `Pages/NavigationPage/Transitions`): one module per upstream file.

mod composite_transition;
mod fade_through_transition;
mod page_slide_transition;
mod parallax_slide_transition;

pub use composite_transition::CompositeTransition;
pub use fade_through_transition::FadeThroughTransition;
pub use page_slide_transition::{PageSlideTransition, PageSlideTransitionAxis};
pub use parallax_slide_transition::ParallaxSlideTransition;
