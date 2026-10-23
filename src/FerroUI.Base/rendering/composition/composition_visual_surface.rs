use super::composition_object_impls::composition_object_impls;
use super::generated::{
    CompositionVisualSurfaceHooks, CompositionVisualSurfaceProps, ServerCompositionVisualSurfaceProps,
};
use super::server::{ServerCompositionVisualSurface, ServerObjectId};
use super::transport::IRegisterForSerialization;
use super::{CompositionObject, CompositionVisual, Compositor, ICompositionObject, ICompositionSurface};
use crate::Vector;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// A live image of the subtree of a visual, shown by a surface brush
/// (WinUI `Microsoft.UI.Composition.CompositionVisualSurface`).
///
/// The surface shows the area `(source_offset, source_size)` of
/// `source_visual`, in the coordinates of the source; a zero `source_size`
/// is the size of the source visual. The subtree is rendered as a bitmap
/// cache renders it: the visibility, opacity, transform and clip of the
/// source visual itself are ignored, so a hidden source still shows (the
/// connected animation hides its elements and shows them through visual
/// surfaces). Changes of the subtree are followed while the source is
/// attached to a composition target.
pub struct CompositionVisualSurface {
    this: Weak<CompositionVisualSurface>,
    object: CompositionObject,
    props: CompositionVisualSurfaceProps,
    /// Whether the next batch asks the server to freeze the surface.
    freeze_requested: Cell<bool>,
}

impl CompositionVisualSurface {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<CompositionVisualSurface> {
        let server = compositor.create_server_object(|compositor, _| ServerCompositionVisualSurface::new(compositor));
        let surface = Rc::new_cyclic(|this: &Weak<CompositionVisualSurface>| CompositionVisualSurface {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: CompositionVisualSurfaceProps::new(),
            freeze_requested: Cell::new(false),
        });
        surface.props.initialize_defaults(&*surface);
        surface
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side surface.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }

    /// The generated properties of the surface.
    pub fn props(&self) -> &CompositionVisualSurfaceProps {
        &self.props
    }

    /// The visual whose subtree the surface shows.
    pub fn source_visual(&self) -> Option<Rc<CompositionVisual>> {
        self.props.source_visual().and_then(|visual| visual.into_any_rc().downcast::<CompositionVisual>().ok())
    }

    pub fn set_source_visual(&self, value: Option<Rc<CompositionVisual>>) {
        self.props.set_source_visual(self, value.map(|visual| visual as Rc<dyn ICompositionObject>))
    }

    /// The top left corner of the area shown, in the coordinates of the
    /// source visual (WinUI `Vector2`; animatable as `SourceOffset`).
    pub fn source_offset(&self) -> Vector {
        self.props.source_offset()
    }

    pub fn set_source_offset(&self, value: Vector) {
        self.props.set_source_offset(self, value)
    }

    /// The size of the area shown; zero for the size of the source visual
    /// (WinUI `Vector2`; animatable as `SourceSize`).
    pub fn source_size(&self) -> Vector {
        self.props.source_size()
    }

    pub fn set_source_size(&self, value: Vector) {
        self.props.set_source_size(self, value)
    }

    /// Freezes the surface: the frame that applies the next batch renders
    /// the source into an offscreen layer once, and the surface shows that
    /// image from then on, whatever happens to the source.
    ///
    /// Not a public member in WinUI, where the connected animation freezes
    /// its snapshot surface internally. Best effort: the image is rendered
    /// with the bounds of the source subtree as of the last rendered frame.
    pub fn freeze(&self) {
        self.freeze_requested.set(true);
        self.register_for_serialization();
    }
}

/// `InitializeDefaultsExtra` activates the server object: done when the
/// server object is created.
impl CompositionVisualSurfaceHooks for CompositionVisualSurface {}

impl ICompositionSurface for CompositionVisualSurface {}

composition_object_impls!(
    CompositionVisualSurface, "CompositionVisualSurface",
    // The request to freeze precedes the properties; the server reads it in
    // `deserialize_changes_extra`.
    serialize: |this, writer| {
        writer.write(this.freeze_requested.replace(false));
        this.props.serialize_changes_core(this, writer)
    },
    start_animation: |this, name, animation, final_value| this.props.start_animation(this, name, animation, final_value),
    composition_property: |_this, name| ServerCompositionVisualSurfaceProps::get_composition_property(name),
);
