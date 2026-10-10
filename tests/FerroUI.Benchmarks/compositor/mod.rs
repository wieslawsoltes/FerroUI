//! The compositor benchmarks.
//!
//! Not ported: the composition target benchmarks of upstream (the classes
//! `CompositionTargetUpdateOnly` and `CompositionTargetUpdateWithRender`,
//! each with the benchmark `TargetUpdate`). They build a pyramid of 87380
//! draw list visuals with internal members the compositor of the framework
//! has other forms of, or none:
//!
//! - a draw list visual made from a server visual of its own and no visual
//!   of the visual tree: `CompositionDrawListVisual::new` takes the visual
//!   it draws (`&Visual`) and creates its server visual itself
//!   (`src/FerroUI.Base/rendering/composition/composition_draw_list_visual.rs`);
//! - the locked framebuffer class over memory of the caller, which the
//!   framebuffer surface of the benchmark hands out: the framework has the
//!   interface (`ILockedFramebuffer`) and no public class for it;
//! - the compositor constructor with the flag that makes the batch pools
//!   reclaim their buffers at once: `Compositor::with_scheduler` has no
//!   such flag (`src/FerroUI.Base/rendering/composition/compositor.rs`);
//! - `ServerCompositor.Render(catchExceptions)` and
//!   `ServerCompositionTarget.Update()`: here `ServerCompositor::render()`
//!   takes no argument and `ServerCompositionTarget::update` takes the time
//!   the global passes took.

use crate::harness::Registry;

pub fn register(_registry: &mut Registry) {}
