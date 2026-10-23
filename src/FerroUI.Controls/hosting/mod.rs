//! `Microsoft.UI.Xaml.Hosting`: access to the composition visuals of elements.

mod element_composition_preview;

pub use element_composition_preview::ElementCompositionPreview;

#[cfg(test)]
mod element_composition_preview_tests;
