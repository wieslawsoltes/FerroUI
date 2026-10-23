use super::composition_object_impls::composition_object_impls;
use super::generated::{CompositionViewBoxHooks, CompositionViewBoxProps, ServerCompositionViewBoxProps};
use super::server::{ServerCompositionViewBox, ServerObjectId};
use super::{CompositionObject, CompositionStretch, Compositor};
use crate::Vector;
use std::rc::{Rc, Weak};

/// The area of the shape coordinates a shape visual or a geometric clip
/// shows (WinUI `Microsoft.UI.Composition.CompositionViewBox`): the
/// rectangle `(offset, size)` of the shapes is mapped onto the visual by
/// `stretch` and placed by the alignment ratios, as a surface brush lays
/// out its surface. `Offset`, `Size` and the ratios are animatable.
///
/// Deviation (winui-composition.md): what is outside the mapped view box is
/// clipped (the port's reading of WinUI); a view box whose size is zero maps
/// nothing (the shapes are drawn as if there were no view box).
pub struct CompositionViewBox {
    this: Weak<CompositionViewBox>,
    object: CompositionObject,
    props: CompositionViewBoxProps,
}

impl CompositionViewBox {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<CompositionViewBox> {
        let server = compositor.create_server_object(|compositor, _| ServerCompositionViewBox::new(compositor));
        let view_box = Rc::new_cyclic(|this: &Weak<CompositionViewBox>| CompositionViewBox {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: CompositionViewBoxProps::new(),
        });
        view_box.props.initialize_defaults(&*view_box);
        view_box
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side view box.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }

    /// The generated properties of the view box.
    pub fn props(&self) -> &CompositionViewBoxProps {
        &self.props
    }

    /// The part of the free horizontal space left of the mapped view box
    /// (0 left, 0.5 centre, 1 right; default 0.5).
    pub fn horizontal_alignment_ratio(&self) -> f32 {
        self.props.horizontal_alignment_ratio()
    }

    pub fn set_horizontal_alignment_ratio(&self, value: f32) {
        self.props.set_horizontal_alignment_ratio(self, value)
    }

    /// The top left corner of the view box, in shape coordinates.
    pub fn offset(&self) -> Vector {
        self.props.offset()
    }

    pub fn set_offset(&self, value: Vector) {
        self.props.set_offset(self, value)
    }

    /// The size of the view box, in shape coordinates.
    pub fn size(&self) -> Vector {
        self.props.size()
    }

    pub fn set_size(&self, value: Vector) {
        self.props.set_size(self, value)
    }

    /// How the view box is scaled onto the visual (default `Fill`).
    pub fn stretch(&self) -> CompositionStretch {
        self.props.stretch()
    }

    pub fn set_stretch(&self, value: CompositionStretch) {
        self.props.set_stretch(self, value)
    }

    /// The part of the free vertical space above the mapped view box
    /// (0 top, 0.5 centre, 1 bottom; default 0.5).
    pub fn vertical_alignment_ratio(&self) -> f32 {
        self.props.vertical_alignment_ratio()
    }

    pub fn set_vertical_alignment_ratio(&self, value: f32) {
        self.props.set_vertical_alignment_ratio(self, value)
    }
}

impl CompositionViewBoxHooks for CompositionViewBox {}

composition_object_impls!(
    CompositionViewBox, "CompositionViewBox",
    serialize: |this, writer| this.props.serialize_changes_core(this, writer),
    start_animation: |this, name, animation, final_value| this.props.start_animation(this, name, animation, final_value),
    composition_property: |_this, name| ServerCompositionViewBoxProps::get_composition_property(name),
);
