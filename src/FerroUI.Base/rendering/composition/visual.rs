//! `CompositionVisual`.
//!
//! Upstream this is the root of a class hierarchy
//! (`CompositionContainerVisual`, `CompositionDrawListVisual`,
//! `CompositionSolidColorVisual`, ...). Here every visual is one
//! [`CompositionVisual`] behind an `Rc`: the members of the abstract class
//! and of the container class (which every concrete class derives from),
//! plus a [`CompositionVisualKind`] that holds what the concrete class
//! adds. The concrete classes are typed handles around the same `Rc`
//! (`CompositionDrawListVisual`, `CompositionSolidColorVisual`, ...).

use super::animations::{ICompositionAnimation, ICompositionAnimationBase, ImplicitAnimationCollection};
use super::composition_draw_list_visual::DrawListData;
use super::composition_object::ICompositionObjectAnimations;
use super::drawing::brush_get_server_resource;
use super::expressions::ExpressionVariant;
use super::generated::{
    CompositionContainerVisualHooks, CompositionContainerVisualProps, CompositionSolidColorVisualProps,
    CompositionSurfaceVisualProps, CompositionVisualHooks, CompositionVisualProps,
    ServerCompositionContainerVisualProps,
};
use super::hit_testing::CompositionHitTestAabbTree;
use super::server::{
    CompositionProperty, IServerVisualContent, ReadbackData, ServerCompositionVisual,
    ServerCompositionVisualCollection, ServerObjectId, VisualReadback,
};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::{
    AsCompositionObject, CompositionBitmapCache, CompositionObject, CompositionTarget, CompositionVisualCollection, Compositor,
    ICompositionObject, ICompositionObjectHost, ICompositorSerializable, PendingAnimations,
};
use crate::media::{BrushExtensions, Geometry, IBrush, IImmutableEffect, IntersectionResult, RenderOptions, TextOptions};
use crate::numerics::Quaternion;
use crate::platform::IGeometryImpl;
use crate::{Matrix, Point, Ref, Vector, Vector3D};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// What the concrete class of a visual adds to [`CompositionVisual`].
pub(crate) enum CompositionVisualKind {
    Container(CompositionContainerVisualProps),
    DrawList(DrawListData),
    SolidColor(CompositionSolidColorVisualProps),
    Surface(CompositionSurfaceVisualProps),
}

/// A node of the composition visual tree.
pub struct CompositionVisual {
    pub(super) this: Weak<CompositionVisual>,
    object: CompositionObject,
    pub(super) kind: CompositionVisualKind,
    children: Rc<CompositionVisualCollection>,
    pub(super) hit_test_children: RefCell<Option<CompositionHitTestAabbTree>>,
    opacity_mask: RefCell<Option<Rc<dyn IBrush>>>,
    pub(super) custom_hit_test_count_in_sub_tree: Cell<i32>,
    tag: RefCell<Option<Rc<dyn Any>>>,
    readback: Arc<VisualReadback>,
}

macro_rules! visual_value_properties {
    ($($(#[$meta:meta])* $get:ident / $set:ident : $ty:ty;)*) => {
        impl CompositionVisual {$(
            $(#[$meta])*
            pub fn $get(&self) -> $ty {
                self.visual_props().$get()
            }

            pub fn $set(&self, value: $ty) {
                self.visual_props().$set(self, value)
            }
        )*}
    };
}

visual_value_properties! {
    visible / set_visible: bool;
    opacity / set_opacity: f32;
    /// The geometry the visual is clipped with.
    clip / set_clip: Option<Rc<dyn IGeometryImpl>>;
    clip_to_bounds / set_clip_to_bounds: bool;
    offset / set_offset: Vector3D;
    translation / set_translation: Vector3D;
    size / set_size: Vector;
    anchor_point / set_anchor_point: Vector;
    center_point / set_center_point: Vector3D;
    rotation_angle / set_rotation_angle: f32;
    orientation / set_orientation: Quaternion;
    scale / set_scale: Vector3D;
    transform_matrix / set_transform_matrix: Matrix;
    /// Whether the adorner is clipped by the clips of the adorned visual
    /// and its ancestors.
    adorner_is_clipped / set_adorner_is_clipped: bool;
    effect / set_effect: Option<Rc<dyn IImmutableEffect>>;
    render_options / set_render_options: RenderOptions;
    text_options / set_text_options: TextOptions;
}

impl CompositionVisual {
    /// Creates a visual of the given kind. `content` creates the concrete
    /// class of the server-side visual, on the render thread.
    pub(crate) fn create(
        compositor: &Rc<Compositor>,
        kind: CompositionVisualKind,
        content: impl FnOnce() -> Box<dyn IServerVisualContent> + 'static,
    ) -> Rc<CompositionVisual> {
        let children_server =
            compositor.create_server_object(|compositor, _| ServerCompositionVisualCollection::new(compositor));
        let readback = Arc::new(VisualReadback::new());
        let server_readback = readback.clone();
        let server = compositor.create_server_object(move |compositor, _| {
            ServerCompositionVisual::new(
                compositor,
                compositor.get::<ServerCompositionVisualCollection>(children_server),
                server_readback,
                content(),
            )
        });
        let visual = Rc::new_cyclic(|this: &Weak<CompositionVisual>| CompositionVisual {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            kind,
            children: CompositionVisualCollection::new(compositor, this.clone(), children_server),
            hit_test_children: RefCell::new(None),
            opacity_mask: RefCell::new(None),
            custom_hit_test_count_in_sub_tree: Cell::new(0),
            tag: RefCell::new(None),
            readback,
        });
        visual.initialize_defaults();
        visual
    }

    fn initialize_defaults(&self) {
        match &self.kind {
            CompositionVisualKind::Container(props) => props.initialize_defaults(self),
            CompositionVisualKind::DrawList(data) => data.initialize_defaults(self),
            CompositionVisualKind::SolidColor(props) => props.initialize_defaults(self),
            CompositionVisualKind::Surface(props) => props.initialize_defaults(self),
        }
    }

    pub(super) fn container_props(&self) -> &CompositionContainerVisualProps {
        match &self.kind {
            CompositionVisualKind::Container(props) => props,
            CompositionVisualKind::DrawList(data) => data.props(),
            CompositionVisualKind::SolidColor(props) => props.base(),
            CompositionVisualKind::Surface(props) => props.base(),
        }
    }

    /// The generated properties of the visual class.
    pub fn visual_props(&self) -> &CompositionVisualProps {
        self.container_props().base()
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The collection of implicit animations attached to this object.
    pub fn implicit_animations(&self) -> Option<Rc<ImplicitAnimationCollection>> {
        self.object.implicit_animations()
    }

    pub fn set_implicit_animations(&self, value: Option<Rc<ImplicitAnimationCollection>>) {
        self.object.set_implicit_animations(value)
    }

    /// The id of the server-side visual.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }

    /// The children of the visual.
    pub fn children(&self) -> &Rc<CompositionVisualCollection> {
        &self.children
    }

    // --- generated reference properties, typed --------------------------------

    /// The composition target the visual is attached to.
    pub fn root(&self) -> Option<Rc<CompositionTarget>> {
        self.visual_props().root().and_then(|o| o.into_any_rc().downcast::<CompositionTarget>().ok())
    }

    pub(crate) fn set_root(&self, value: Option<Rc<CompositionTarget>>) {
        self.visual_props().set_root(self, value.map(|v| v as Rc<dyn ICompositionObject>));
    }

    pub fn parent(&self) -> Option<Rc<CompositionVisual>> {
        self.visual_props().parent().and_then(|o| o.into_any_rc().downcast::<CompositionVisual>().ok())
    }

    pub(crate) fn set_parent(&self, value: Option<Rc<CompositionVisual>>) {
        self.visual_props().set_parent(self, value.map(|v| v as Rc<dyn ICompositionObject>));
    }

    /// The visual this visual is an adorner of: its transform follows that
    /// visual.
    pub fn adorned_visual(&self) -> Option<Rc<CompositionVisual>> {
        self.visual_props().adorned_visual().and_then(|o| o.into_any_rc().downcast::<CompositionVisual>().ok())
    }

    pub fn set_adorned_visual(&self, value: Option<Rc<CompositionVisual>>) {
        self.visual_props().set_adorned_visual(self, value.map(|v| v as Rc<dyn ICompositionObject>));
    }

    pub fn cache_mode(&self) -> Option<Rc<CompositionBitmapCache>> {
        self.visual_props().cache_mode().and_then(|o| o.into_any_rc().downcast::<CompositionBitmapCache>().ok())
    }

    pub fn set_cache_mode(&self, value: Option<Rc<CompositionBitmapCache>>) {
        self.visual_props().set_cache_mode(self, value.map(|v| v as Rc<dyn ICompositionObject>));
    }

    // --- Visual.cs ---------------------------------------------------------------

    pub fn disable_sub_tree_bounds_hit_test_optimization(&self) -> bool {
        self.custom_hit_test_count_in_sub_tree.get() != 0
    }

    pub fn opacity_mask(&self) -> Option<Rc<dyn IBrush>> {
        self.opacity_mask.borrow().clone()
    }

    pub fn set_opacity_mask(&self, value: Option<Rc<dyn IBrush>>) {
        let current = self.opacity_mask.borrow().clone();
        let same = match (&current, &value) {
            (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }

        let compositor = self.compositor().clone();
        // Release the previous compositor-resource based brush
        if let Some(old) = &current {
            if let Some(resource) = old.as_composition_render_resource() {
                resource.release_on_compositor(&compositor);
                *self.opacity_mask.borrow_mut() = None;
                self.visual_props().set_opacity_mask_brush_transport_field(self, None);
            }
        }

        match &value {
            Some(brush) if brush.as_composition_render_resource().is_some() => {
                if let Some(resource) = brush.as_composition_render_resource() {
                    resource.add_ref_on_compositor(&compositor);
                }
                let transport = brush_get_server_resource(Some(brush), Some(&compositor));
                self.visual_props().set_opacity_mask_brush_transport_field(self, transport);
                *self.opacity_mask.borrow_mut() = value;
            }
            _ => {
                let immutable = value.as_ref().map(|brush| {
                    let immutable: Rc<dyn IBrush> = BrushExtensions::to_immutable(brush);
                    super::transport::BatchResource::Value(immutable)
                });
                *self.opacity_mask.borrow_mut() = value;
                self.visual_props().set_opacity_mask_brush_transport_field(self, immutable);
            }
        }
    }

    /// The data the server wrote back for this visual, if it is valid for
    /// the current read revision.
    pub fn try_get_valid_readback(&self) -> Option<ReadbackData> {
        let root = self.root()?;

        let indices = self.compositor().server().readback().clone();
        let readback = self.readback.get_readback(indices.read_revision())?;

        // CompositionVisual wasn't visible or wasn't even attached to the composition target during the lat frame
        if !readback.visible || readback.target_id != root.id() {
            return None;
        }

        Some(readback)
    }

    pub fn tag(&self) -> Option<Rc<dyn Any>> {
        self.tag.borrow().clone()
    }

    pub fn set_tag(&self, value: Option<Rc<dyn Any>>) {
        *self.tag.borrow_mut() = value;
    }

    /// Whether the content of the visual is hit at a point in its own
    /// coordinates.
    pub fn hit_test(&self, point: Point) -> bool {
        match &self.kind {
            CompositionVisualKind::DrawList(data) => data.hit_test(point),
            _ => true,
        }
    }

    /// How the content of the visual intersects a geometry in its own
    /// coordinates.
    pub fn hit_test_geometry(&self, geometry: &Ref<Geometry>) -> IntersectionResult {
        match &self.kind {
            CompositionVisualKind::DrawList(data) => data.hit_test_geometry(geometry),
            _ => IntersectionResult::Intersects,
        }
    }

    fn serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        match &self.kind {
            CompositionVisualKind::Container(props) => props.serialize_changes_core(self, writer),
            CompositionVisualKind::DrawList(data) => data.serialize_changes_core(self, writer),
            CompositionVisualKind::SolidColor(props) => props.serialize_changes_core(self, writer),
            CompositionVisualKind::Surface(props) => props.serialize_changes_core(self, writer),
        }
    }
}

impl CompositionVisualHooks for CompositionVisual {
    fn on_root_changed(&self) {
        // OnRootChangedCore of the container visual.
        let root = self.root();
        for child in self.children.items() {
            child.set_root(root.clone());
        }
    }

    fn on_parent_changing(&self) {
        // Propagate the blight
        let count = self.custom_hit_test_count_in_sub_tree.get();
        if count != 0 {
            let mut parent = self.parent();
            while let Some(p) = parent {
                p.custom_hit_test_count_in_sub_tree.set(p.custom_hit_test_count_in_sub_tree.get() - count);
                parent = p.parent();
            }
        }
    }

    fn on_parent_changed(&self) {
        self.set_root(self.parent().and_then(|parent| parent.root()));

        // Propagate the blight
        let count = self.custom_hit_test_count_in_sub_tree.get();
        if count != 0 {
            let mut parent = self.parent();
            while let Some(p) = parent {
                p.custom_hit_test_count_in_sub_tree.set(p.custom_hit_test_count_in_sub_tree.get() + count);
                parent = p.parent();
            }
        }
    }
}

impl CompositionContainerVisualHooks for CompositionVisual {}

impl IRegisterForSerialization for CompositionVisual {
    fn register_for_serialization(&self) {
        self.object
            .register_for_serialization(|| self.this.upgrade().map(|this| this as Rc<dyn ICompositorSerializable>));
    }
}

impl ICompositionObjectHost for CompositionVisual {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn pending_animations(&self) -> &PendingAnimations {
        self.object.pending_animations()
    }

    fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.object.implicit_animation(property_name)
    }

    fn start_animation_group(
        &self,
        grp: &Rc<dyn ICompositionAnimationBase>,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        self.start_animation_group_for(&**grp, target, final_value)
    }
}

impl ICompositionObjectAnimations for CompositionVisual {
    fn try_start_animation(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) -> bool {
        match &self.kind {
            CompositionVisualKind::Container(props) => props.start_animation(self, property_name, animation, final_value),
            CompositionVisualKind::DrawList(data) => {
                data.props().start_animation(self, property_name, animation, final_value)
            }
            CompositionVisualKind::SolidColor(props) => props.start_animation(self, property_name, animation, final_value),
            CompositionVisualKind::Surface(props) => props.start_animation(self, property_name, animation, final_value),
        }
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        match &self.kind {
            CompositionVisualKind::SolidColor(_) => {
                super::generated::ServerCompositionSolidColorVisualProps::get_composition_property(property_name)
                    .or_else(|| ServerCompositionContainerVisualProps::get_composition_property(property_name))
            }
            _ => ServerCompositionContainerVisualProps::get_composition_property(property_name),
        }
    }

    fn composition_object(&self) -> &CompositionObject {
        &self.object
    }
}

impl AsCompositionObject for CompositionVisual {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        match &self.kind {
            CompositionVisualKind::Container(_) => "CompositionContainerVisual",
            CompositionVisualKind::DrawList(_) => "CompositionDrawListVisual",
            CompositionVisualKind::SolidColor(_) => "CompositionSolidColorVisual",
            CompositionVisualKind::Surface(_) => "CompositionSurfaceVisual",
        }
    }
}

impl ICompositionObject for CompositionVisual {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl ICompositorSerializable for CompositionVisual {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.object.try_get_server(c)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.object.begin_serialize_changes(c);
        self.serialize_changes_core(writer);
    }
}
