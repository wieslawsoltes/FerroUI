//! Port of upstream's render tests (`tests/*.RenderTests`, compiled by
//! `tests/*.Skia.RenderTests`): scenes are rendered offscreen, once through
//! the immediate renderer and once through the compositor, and compared
//! with the expected images of upstream (`TestFiles`) by upstream's
//! measure and threshold.
//!
//! The harness is `test_base` (upstream's `TestBase`), `test_render_helper`
//! (`TestRenderHelper`), `test_render_root` (`TestRenderRoot`) and
//! `manual_render_timer`. The suites keep upstream's structure: one module
//! for each upstream file, the tests under upstream's names in snake case.
//!
//! See `docs/porting/render-tests.md`.

pub mod assets;
pub mod manual_render_timer;
pub mod test_base;
pub mod test_render_helper;
pub mod test_render_root;

#[cfg(test)]
mod bug_repros;
#[cfg(test)]
mod composition;
#[cfg(test)]
mod controls;
#[cfg(test)]
mod cross_test_base;
#[cfg(test)]
mod cross_tests;
#[cfg(test)]
mod cross_ui;
#[cfg(test)]
mod geometry_clipping_tests;
#[cfg(test)]
mod media;
#[cfg(test)]
mod opacity_mask_tests;
#[cfg(test)]
mod shapes;
#[cfg(test)]
mod svg_path_tests;
