//! Fragments of the drawing context: what the content of a visual drew,
//! kept by the compositor and drawn again in later frames without the
//! commands that drew it ([`IDrawingContextImplWithFragments`]).
//!
//! The scene of the GPU mode keeps them: a fragment is what its scene
//! recorded, which `vello` appends whole to the scene of a later frame
//! (`Scene::append`), as the widgets of Xilem keep theirs. Everything a
//! command does on the processor (flattening, the outlines of strokes, the
//! snapping of rectangles, the glyphs of text) is done once. A fragment is
//! drawn again only where it gives the pixels the commands would give: at
//! the transform it was made at moved by whole pixels, with the opacity and
//! the options it was made with; the scene refuses one that paints with an
//! image or blends with what lies outside it. The other modes keep none:
//! their scenes compute coverage while they are recorded, and a scene of
//! `vello_gpu` or `vello_cpu` cannot take what another one recorded.

use super::DrawingContextImpl;
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::{RenderOptions, TextOptions};
use ferroui_base::platform::{IDrawingContextImplWithFragments, IDrawingFragment};
use std::any::Any;
use std::rc::Rc;

/// The state of the drawing context a fragment was made in: what the
/// commands it was made of drew with besides the transform, which the scene
/// compares itself.
#[derive(Clone, Copy, PartialEq)]
struct FragmentState {
    opacity: f64,
    render_options: RenderOptions,
    text_options: TextOptions,
    anti_alias: bool,
}

/// A fragment that is being made.
pub(super) struct FragmentRecording {
    state: FragmentState,
    /// How deep the states of the context were when it started.
    depth: usize,
    /// Whether the scene keeps it: false when the context keeps none at
    /// this point (inside an effect).
    started: bool,
    /// Whether the scene was cleared while it was made.
    cleared: bool,
}

/// A fragment of the drawing context: the scene's, and the state it was
/// made in.
struct VelloDrawingFragment {
    scene: Rc<dyn Any>,
    state: FragmentState,
}

impl IDrawingFragment for VelloDrawingFragment {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl DrawingContextImpl {
    /// Whether the scene of the context keeps fragments.
    pub(super) fn keeps_fragments(&mut self) -> bool {
        self.scene().rendering_mode() == VelloRenderingMode::Gpu
    }

    fn fragment_state(&self) -> FragmentState {
        FragmentState {
            opacity: self.current_opacity,
            render_options: self.render_options,
            text_options: self.text_options,
            anti_alias: self.anti_alias(),
        }
    }

    /// The scene is cleared: a fragment that is being made is not kept.
    pub(super) fn fragment_cleared(&mut self) {
        if let Some(recording) = &mut self.fragment {
            recording.cleared = true;
        }
    }
}

impl IDrawingContextImplWithFragments for DrawingContextImpl {
    fn begin_fragment(&mut self) {
        if self.fragment.is_some() {
            return;
        }
        // What the target held is put under the scene before the fragment
        // starts: it is no part of it.
        if self.backdrop.is_some() {
            self.start_with_backdrop();
        }
        let transform = self.device_transform();
        let started = self.effect_stack.is_empty() && self.scene().begin_fragment(transform);
        self.fragment = Some(FragmentRecording {
            state: self.fragment_state(),
            depth: self.state_stack.len(),
            started,
            cleared: false,
        });
    }

    fn end_fragment(&mut self) -> Option<Rc<dyn IDrawingFragment>> {
        let recording = self.fragment.take()?;
        if !recording.started {
            return None;
        }
        let scene = self.scene().end_fragment();
        let balanced = self.state_stack.len() == recording.depth && self.effect_stack.is_empty();
        if !balanced || recording.cleared || recording.state != self.fragment_state() {
            return None;
        }
        Some(Rc::new(VelloDrawingFragment { scene: scene?, state: recording.state }))
    }

    fn draw_fragment(&mut self, fragment: &Rc<dyn IDrawingFragment>) -> bool {
        let Some(fragment) = fragment.as_any().downcast_ref::<VelloDrawingFragment>() else {
            return false;
        };
        if self.fragment.is_some() || !self.effect_stack.is_empty() || fragment.state != self.fragment_state() {
            return false;
        }
        let transform = self.device_transform();
        // `sink` puts what the target held under the scene first.
        self.sink().draw_fragment(&*fragment.scene, transform)
    }
}
