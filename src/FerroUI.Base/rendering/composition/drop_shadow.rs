use super::composition_object_impls::composition_object_impls;
use super::generated::{DropShadowHooks, DropShadowProps, ServerDropShadowProps};
use super::server::{ServerDropShadow, ServerObjectId};
use super::{CompositionBrush, CompositionDropShadowSourcePolicy, CompositionObject, Compositor, ICompositionObject};
use crate::media::Color;
use crate::Vector3D;
use std::rc::{Rc, Weak};

/// The shadow of a sprite or layer visual (WinUI
/// `Microsoft.UI.Composition.DropShadow`), with animatable `BlurRadius`,
/// `Color`, `Offset` and `Opacity`.
///
/// On a sprite visual the shadow is cast by the rectangle of the visual (a
/// box shadow of the drawing context), by the alpha of its
/// [`mask`](Self::mask), or, with the `InheritFromVisualContent`
/// [`source_policy`](Self::source_policy), by the alpha of the brush of the
/// visual. On a layer visual it is cast by the composed children, or by the
/// mask. The shadow of a mask or of the content goes through a drop shadow
/// effect, when the drawing context has effects.
///
/// Deviation (winui-composition.md): without effects in the drawing context
/// the shadow is the box shadow of the rectangle; a colour or gradient mask
/// scales the box shadow; the Z of the offset is ignored. WinUI's
/// `CompositionShadow` base class has no members and is not a separate type.
pub struct DropShadow {
    this: Weak<DropShadow>,
    object: CompositionObject,
    props: DropShadowProps,
}

impl DropShadow {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<DropShadow> {
        let server = compositor.create_server_object(|compositor, _| ServerDropShadow::new(compositor));
        let shadow = Rc::new_cyclic(|this: &Weak<DropShadow>| DropShadow {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: DropShadowProps::new(),
        });
        shadow.props.initialize_defaults(&*shadow);
        shadow
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side shadow.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }

    /// The generated properties of the shadow.
    pub fn props(&self) -> &DropShadowProps {
        &self.props
    }

    /// The radius of the blur (default 9).
    pub fn blur_radius(&self) -> f32 {
        self.props.blur_radius()
    }

    pub fn set_blur_radius(&self, value: f32) {
        self.props.set_blur_radius(self, value)
    }

    /// The colour of the shadow (default black).
    pub fn color(&self) -> Color {
        self.props.color()
    }

    pub fn set_color(&self, value: Color) {
        self.props.set_color(self, value)
    }

    /// The offset of the shadow from the visual (WinUI `Vector3`; the Z
    /// component is not used by the 2D renderers).
    pub fn offset(&self) -> Vector3D {
        self.props.offset()
    }

    pub fn set_offset(&self, value: Vector3D) {
        self.props.set_offset(self, value)
    }

    /// The opacity of the shadow (default 1).
    pub fn opacity(&self) -> f32 {
        self.props.opacity()
    }

    pub fn set_opacity(&self, value: f32) {
        self.props.set_opacity(self, value)
    }

    /// The brush whose alpha shapes the shadow, painted over the area of the
    /// visual; `None` for the shape of the visual. Ignored with the
    /// `InheritFromVisualContent` source policy.
    pub fn mask(&self) -> Option<Rc<CompositionBrush>> {
        self.props.mask().and_then(|mask| mask.into_any_rc().downcast::<CompositionBrush>().ok())
    }

    pub fn set_mask(&self, value: Option<Rc<CompositionBrush>>) {
        self.props.set_mask(self, value.map(|mask| mask as Rc<dyn ICompositionObject>))
    }

    /// What the shadow is cast from: `Default` for the mask (or the shape of
    /// the visual), `InheritFromVisualContent` for the alpha of the content
    /// of the visual (the brush of a sprite visual, the children of a layer
    /// visual).
    pub fn source_policy(&self) -> CompositionDropShadowSourcePolicy {
        self.props.source_policy()
    }

    pub fn set_source_policy(&self, value: CompositionDropShadowSourcePolicy) {
        self.props.set_source_policy(self, value)
    }
}

/// `InitializeDefaultsExtra` activates the server object: done when the
/// server object is created.
impl DropShadowHooks for DropShadow {}

composition_object_impls!(
    DropShadow, "DropShadow",
    serialize: |this, writer| this.props.serialize_changes_core(this, writer),
    start_animation: |this, name, animation, final_value| this.props.start_animation(this, name, animation, final_value),
    composition_property: |_this, name| ServerDropShadowProps::get_composition_property(name),
);
