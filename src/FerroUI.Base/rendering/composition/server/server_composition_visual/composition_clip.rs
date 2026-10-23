//! The WinUI clip of a visual (`Visual.Clip`, here `composition_clip`): the
//! clip object is a render resource the visual observes, and the area it
//! leaves visible is part of the own clip of the visual and is pushed onto
//! the drawing context by the render walk.

use super::ServerCompositionVisual;
use crate::rendering::composition::server::{
    CompositionClipShape, IServerObject, IServerRenderResource, IServerRenderResourceObserver,
};
use std::rc::Rc;

impl ServerCompositionVisual {
    /// The WinUI clip object of the visual (a server-side `InsetClip` or
    /// `RectangleClip`).
    pub fn composition_clip(&self) -> Option<Rc<dyn IServerObject>> {
        self.props.base().composition_clip()
    }

    /// The area the WinUI clip of the visual leaves visible, in the
    /// coordinates of the visual.
    pub fn composition_clip_shape(&self) -> Option<CompositionClipShape> {
        let clip = self.composition_clip()?;
        CompositionClipShape::of(&*clip, self.size())
    }

    fn composition_clip_resource(&self) -> Option<Rc<dyn IServerRenderResource>> {
        self.composition_clip().and_then(|clip| clip.as_render_resource())
    }

    /// Starts or stops observing the clip object: its changes (its
    /// animations included) change the clip of the visual.
    pub(super) fn observe_composition_clip(&self, observe: bool) {
        let (Some(resource), Some(this)) = (self.composition_clip_resource(), self.rc()) else { return };
        let observer: Rc<dyn IServerRenderResourceObserver> = this;
        if observe {
            resource.add_observer(&observer);
        } else {
            resource.remove_observer(&observer);
        }
    }

    /// Whether `sender` is the clip object of the visual.
    pub(super) fn is_composition_clip(&self, sender: &dyn IServerRenderResource) -> bool {
        self.composition_clip_resource()
            .is_some_and(|resource| std::ptr::addr_eq(Rc::as_ptr(&resource), sender as *const dyn IServerRenderResource))
    }

    /// The clip object, or a property of it, changed: what a change of the
    /// geometry clip does.
    pub(super) fn composition_clip_changed(&self) {
        self.trigger_composition_fields_dirty();
        self.trigger_clip_size_dirty();
        self.own_bounds_dirty.set(true);
    }
}
