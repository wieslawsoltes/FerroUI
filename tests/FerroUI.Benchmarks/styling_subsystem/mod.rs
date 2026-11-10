//! The styling subsystem, one mechanism a benchmark: how a selector of each kind matches, what the
//! styles in scope that do not match cost, how a style is switched on and off, how setters reach
//! the values of a control, how a control theme and its template are applied, what a change of a
//! styles collection restyles, how a resource is found and followed, and what a subtree that moves
//! pays for its styles. FerroUI's own regression benchmarks, run by the local performance gate
//! (`scripts/perf/gate.py`, suite `framework`): each guards a finding of the performance notes
//! (`docs/porting/desktop-performance.md`, sections 8.1 and 9), and the sizes are parameters where
//! the cost grows with them.
//!
//! Beside them: `trees::Styling` and `trees::Propagation` measure styles on whole screens (a wall
//! attached under six styles, a class toggled on every tile, a resource replaced, a theme variant
//! switched), and `styling` holds the upstream micro benchmarks of one control. The benchmarks
//! here stand between the two: many controls, one mechanism, and a measured body that does that
//! mechanism only. Most of them measure a styling pass of attached controls: the iteration setup
//! takes the styles off the controls (`invalidate_styles`), the body applies them again
//! (`apply_styling`), and the layout the pass queues runs in the next iteration setup.
//!
//! The services are those of the tree benchmarks (`trees::fixture`): the Fluent theme in its light
//! variant, Skia's font manager and HarfBuzz; the roots render nothing. The global styles of the
//! application are the styling parent of every root, so every styling pass also walks them.

use crate::harness::Registry;

pub mod activation;
pub mod control_theme_apply;
pub mod detach_reattach;
pub mod resource_lookup;
pub mod selector_match;
pub mod setter_apply;
pub mod styles_changed;
pub mod styles_in_scope;
pub mod wall;

pub fn register(registry: &mut Registry) {
    selector_match::register(registry);
    styles_in_scope::register(registry);
    activation::register(registry);
    setter_apply::register(registry);
    control_theme_apply::register(registry);
    styles_changed::register(registry);
    resource_lookup::register(registry);
    detach_reattach::register(registry);
}
