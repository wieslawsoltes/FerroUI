//! The page transitions of the catalog (namespace `ControlCatalog.Pages.Transitions`,
//! directory `Pages/Transitions`): one module per upstream file.

mod card_stack_page_transition;
mod wave_reveal_page_transition;

pub use card_stack_page_transition::CardStackPageTransition;
pub use wave_reveal_page_transition::WaveRevealPageTransition;
