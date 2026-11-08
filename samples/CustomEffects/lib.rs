//! custom-effects: the gallery of the custom effects of FerroUI (#455, E8;
//! `docs/porting/custom-effects.md`, section 3.7), the counterpart of Effector's
//! `Effector.Sample.App` and `Effector.Compiz.Sample.App`.
//!
//! The effects are those of the library (`ferroui_base::media::effects::library`), Effector's
//! sample set ported in E2 to E7: each is a value of `CustomEffect` set as `Visual.Effect`, drawn
//! by the backend on the GPU (an effect graph or a runtime shader compiled once, images uploaded
//! once), with a CPU reference for Vello's CPU mode and the render tests. The gallery shows them in
//! one section per family ([`Family`]): the colour effects, the image effects, the shader effects,
//! the interactive effects and the page transitions. Each tile ([`EffectTile`]) shows the code
//! that makes its effect, the effect string markup writes for the current value
//! (`Effect="tint(color=#0F9D8E, strength=0.55)"`) and an animation: the transition of the effect
//! of its host and key frames of `Visual.Effect`.
//!
//! The sections are the pages of the section "Custom effects" of the ControlCatalog, which depends
//! on the crate without its default feature `app`. With it the crate is also the application:
//!
//! ```sh
//! cargo run -p custom-effects
//! FERROUI_SMOKE_EXIT_MS=2000 cargo run -p custom-effects
//! ```

mod families;
mod gallery;
mod tiles;
mod transition_images;

#[cfg(test)]
mod tests;

pub use families::{Family, PRESENTER_MS};
pub use gallery::{authoring_tiles, gallery, main_window, AUTHORING_TITLE, GLOW_MS, TINT_MARKUP};
pub use tiles::{label_of, markup_of, EffectTile, CAPTION_MS, SWEEP_MS, TRANSITION_MS};
pub use transition_images::{Picture, TransitionImages, PAGE_HEIGHT, PAGE_WIDTH};
