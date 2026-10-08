//! Server-side `CompositionVisual` counterpart.
//!
//! Upstream this is a class hierarchy rooted at `ServerCompositionVisual`.
//! Here every server visual is one [`ServerCompositionVisual`]: the state
//! and behaviour of the base class (and of the container class, which every
//! concrete class derives from), plus a [`IServerVisualContent`] that holds
//! what a concrete class adds and overrides. The parts of the upstream
//! partial class live in the submodules of this module.

mod act;
mod adorners;
mod computed_properties;
mod dirty_inputs;
mod readback;
mod render;
mod server_composition_visual_cache;
mod update;
mod walker;

pub use readback::{ReadbackData, VisualReadback};
pub use server_composition_visual_cache::ServerCompositionVisualCache;
pub use walker::TreeWalkerFrame;

use super::{
    CompositionProperty, IAnimatedServerObject, IServerAnimatedPropertyHost, IServerObject, IServerPropertyHost,
    IServerRenderResource, IServerRenderResourceObserver, ServerCompositionBitmapCache, ServerCompositionTarget,
    ServerCompositionVisualCollection, ServerCompositor, ServerObject, ServerValueChange, ServerVisualRenderContext,
};
use crate::media::{IBrush, IImmutableEffect, RenderOptions, TextOptions};
use crate::numerics::Quaternion;
use crate::platform::{IDrawingContextImpl, IGeometryImpl, LtrbRect};
use crate::rendering::composition::animations::IAnimationInstance;
use crate::rendering::composition::expressions::{ExpressionVariant, IExpressionObject};
use crate::rendering::composition::generated::{
    ServerCompositionContainerVisualHooks, ServerCompositionContainerVisualProps, ServerCompositionVisualHooks,
    ServerCompositionVisualProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Matrix, Rect, Vector, Vector3D};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// What a concrete server visual class adds to [`ServerCompositionVisual`]:
/// its own properties and its overrides of the virtual members.
pub trait IServerVisualContent: 'static {
    /// `DeserializeChangesCore` of the class; the default is the one of the
    /// container visual.
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        visual.base_deserialize_changes_core(reader, committed_at);
    }

    fn compute_own_content_bounds(&self, _visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        None
    }

    fn render_core(
        &self,
        _visual: &ServerCompositionVisual,
        _context: &mut ServerVisualRenderContext<'_>,
        _current_transformed_clip: LtrbRect,
    ) {
    }

    fn size_changed(&self, _visual: &ServerCompositionVisual) {}

    fn on_attached_to_root(&self, _visual: &ServerCompositionVisual, _target: &Rc<ServerCompositionTarget>) {}

    fn on_detached_from_root(&self, _visual: &ServerCompositionVisual, _target: &Rc<ServerCompositionTarget>) {}

    fn push_clip_to_bounds(&self, visual: &ServerCompositionVisual, canvas: &mut dyn IDrawingContextImpl) {
        let size = visual.size();
        canvas.push_clip(Rect::new(0.0, 0.0, size.x, size.y));
    }

    fn has_effect(&self, visual: &ServerCompositionVisual) -> bool {
        visual.effect().is_some()
    }

    /// A render resource the content depends on changed.
    fn dependency_queued_invalidate(&self, _visual: &ServerCompositionVisual) {}

    /// The generated property block of the given type, if the class has
    /// one of its own.
    fn find_props(&self, _type_id: TypeId) -> Option<&dyn Any> {
        None
    }

    fn get_composition_property(&self, name: &str) -> Option<&'static CompositionProperty> {
        ServerCompositionContainerVisualProps::get_composition_property(name)
    }

    /// The visual has been disposed on the UI thread.
    fn dispose(&self, _visual: &ServerCompositionVisual) {}

    fn as_any(&self) -> &dyn Any;
}

/// Server-side `CompositionVisual` counterpart. Is responsible for
/// computing the transformation matrix, for applying various visual
/// properties before calling visual-specific drawing code and for notifying
/// the [`ServerCompositionTarget`] for new dirty rects.
pub struct ServerCompositionVisual {
    this: Weak<ServerCompositionVisual>,
    object: ServerObject,
    compositor: Weak<ServerCompositor>,
    props: ServerCompositionContainerVisualProps,
    content: Box<dyn IServerVisualContent>,
    children: Option<Rc<ServerCompositionVisualCollection>>,
    cache: RefCell<Option<Rc<ServerCompositionVisualCache>>>,
    disposed: Cell<bool>,

    // Act
    att_helper: act::AttHelper,

    // Dirty flags, handled by RecomputeOwnProperties
    combined_transform_dirty: Cell<bool>,
    clip_size_dirty: Cell<bool>,
    own_bounds_dirty: Cell<bool>,
    composition_fields_dirty: Cell<bool>,
    content_changed: Cell<bool>,
    delay_propagate_needs_bounds_update: Cell<bool>,
    delay_propagate_is_dirty_for_render: Cell<bool>,
    delay_propagate_has_extra_dirty_rects: Cell<bool>,

    // Dirty rect, re-render flags, set by PropagateFlags
    needs_bounding_box_update: Cell<bool>,
    is_dirty_for_render: Cell<bool>,
    is_dirty_for_render_in_subgraph: Cell<bool>,

    // Transform that accounts for offset, RenderTransform and other
    // properties of _this_ visual that is used to transform to parent's
    // coordinate space. Updated by RecomputeOwnProperties pass
    own_transform: Cell<Option<Matrix>>,
    // The bounds of this visual's own content, excluding children.
    // Coordinate space: local. Updated by RecomputeOwnProperties pass
    own_content_bounds: Cell<Option<LtrbRect>>,
    // The bounds of this visual and its subtree. Coordinate space: local.
    // Updated by: PreSubgraph, PostSubraph (recursive)
    sub_tree_bounds: Cell<Option<LtrbRect>>,
    // The bounds of this visual and its subtree. Coordinate space: parent.
    // Updated by: PostSubgraph
    transformed_sub_tree_bounds: Cell<Option<LtrbRect>>,
    // Visual's own clip area. Coordinate space: local
    own_clip_rect: Cell<Option<LtrbRect>>,
    needs_to_add_extra_dirty_rect_to_dirty_region: Cell<bool>,
    extra_dirty_rect: Cell<LtrbRect>,
    combined_transform_matrix: Cell<Matrix>,

    // Dirty inputs
    enqueued_for_own_properties_recompute: Cell<bool>,

    // Readback
    readback: Arc<VisualReadback>,
    enqueued_for_readback_update: Cell<bool>,
}

impl ServerCompositionVisual {
    /// Creates a server visual. `children` is the server side of the
    /// children collection (created before the visual by the UI thread),
    /// `readback` the readback slots shared with the UI-thread visual, and
    /// `content` the concrete class.
    pub fn new(
        compositor: &Rc<ServerCompositor>,
        children: Option<Rc<ServerCompositionVisualCollection>>,
        readback: Arc<VisualReadback>,
        content: Box<dyn IServerVisualContent>,
    ) -> Rc<ServerCompositionVisual> {
        Rc::new_cyclic(|this: &Weak<ServerCompositionVisual>| {
            let owner: Weak<dyn IAnimatedServerObject> = this.clone();
            ServerCompositionVisual {
                this: this.clone(),
                object: ServerObject::new(compositor, owner),
                compositor: Rc::downgrade(compositor),
                props: ServerCompositionContainerVisualProps::new(),
                content,
                children,
                cache: RefCell::new(None),
                disposed: Cell::new(false),
                att_helper: act::AttHelper::default(),
                combined_transform_dirty: Cell::new(false),
                clip_size_dirty: Cell::new(false),
                own_bounds_dirty: Cell::new(false),
                composition_fields_dirty: Cell::new(false),
                content_changed: Cell::new(false),
                delay_propagate_needs_bounds_update: Cell::new(false),
                delay_propagate_is_dirty_for_render: Cell::new(false),
                delay_propagate_has_extra_dirty_rects: Cell::new(false),
                needs_bounding_box_update: Cell::new(false),
                is_dirty_for_render: Cell::new(false),
                is_dirty_for_render_in_subgraph: Cell::new(false),
                own_transform: Cell::new(None),
                own_content_bounds: Cell::new(None),
                sub_tree_bounds: Cell::new(None),
                transformed_sub_tree_bounds: Cell::new(None),
                own_clip_rect: Cell::new(None),
                needs_to_add_extra_dirty_rect_to_dirty_region: Cell::new(false),
                extra_dirty_rect: Cell::new(LtrbRect::default()),
                combined_transform_matrix: Cell::new(Matrix::IDENTITY),
                enqueued_for_own_properties_recompute: Cell::new(false),
                readback,
                enqueued_for_readback_update: Cell::new(false),
            }
        })
    }

    /// A weak handle of the visual.
    pub fn weak(&self) -> Weak<ServerCompositionVisual> {
        self.this.clone()
    }

    fn rc(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.this.upgrade()
    }

    /// The compositor of the visual, while it is alive.
    pub fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.upgrade()
    }

    /// The `ServerObject` part: activation and animations.
    pub fn object(&self) -> &ServerObject {
        &self.object
    }

    pub fn activate(&self) {
        self.object.activate();
    }

    pub fn deactivate(&self) {
        self.object.deactivate();
    }

    /// The concrete class of the visual.
    pub fn content(&self) -> &dyn IServerVisualContent {
        &*self.content
    }

    /// The concrete class of the visual as `T`.
    pub fn content_as<T: IServerVisualContent>(&self) -> Option<&T> {
        self.content.as_any().downcast_ref::<T>()
    }

    pub fn children(&self) -> Option<&Rc<ServerCompositionVisualCollection>> {
        self.children.as_ref()
    }

    pub fn cache(&self) -> Option<Rc<ServerCompositionVisualCache>> {
        self.cache.borrow().clone()
    }

    /// The generated properties of the visual class.
    pub fn visual_props(&self) -> &ServerCompositionVisualProps {
        self.props.base()
    }

    // --- generated properties, typed ---------------------------------------

    pub fn root(&self) -> Option<Rc<ServerCompositionTarget>> {
        self.props.base().root().and_then(|o| o.into_any_rc().downcast::<ServerCompositionTarget>().ok())
    }

    pub fn parent(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.props.base().parent().and_then(|o| o.into_any_rc().downcast::<ServerCompositionVisual>().ok())
    }

    pub fn adorned_visual(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.props.base().adorned_visual().and_then(|o| o.into_any_rc().downcast::<ServerCompositionVisual>().ok())
    }

    pub fn cache_mode(&self) -> Option<Rc<ServerCompositionBitmapCache>> {
        self.props.base().cache_mode().and_then(|o| o.into_any_rc().downcast::<ServerCompositionBitmapCache>().ok())
    }

    pub fn visible(&self) -> bool {
        self.props.base().visible()
    }

    pub fn opacity(&self) -> f32 {
        self.props.base().opacity()
    }

    pub fn clip(&self) -> Option<Arc<dyn IGeometryImpl>> {
        self.props.base().clip()
    }

    pub fn clip_to_bounds(&self) -> bool {
        self.props.base().clip_to_bounds()
    }

    pub fn offset(&self) -> Vector3D {
        self.props.base().offset()
    }

    pub fn translation(&self) -> Vector3D {
        self.props.base().translation()
    }

    pub fn size(&self) -> Vector {
        self.props.base().size()
    }

    pub fn anchor_point(&self) -> Vector {
        self.props.base().anchor_point()
    }

    pub fn center_point(&self) -> Vector3D {
        self.props.base().center_point()
    }

    pub fn rotation_angle(&self) -> f32 {
        self.props.base().rotation_angle()
    }

    pub fn orientation(&self) -> Quaternion {
        self.props.base().orientation()
    }

    pub fn scale(&self) -> Vector3D {
        self.props.base().scale()
    }

    pub fn transform_matrix(&self) -> Matrix {
        self.props.base().transform_matrix()
    }

    pub fn adorner_is_clipped(&self) -> bool {
        self.props.base().adorner_is_clipped()
    }

    pub fn opacity_mask_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.props.base().opacity_mask_brush().map(|brush| brush.value)
    }

    pub fn effect(&self) -> Option<Rc<dyn IImmutableEffect>> {
        self.props.base().effect()
    }

    pub fn render_options(&self) -> RenderOptions {
        self.props.base().render_options()
    }

    pub fn text_options(&self) -> TextOptions {
        self.props.base().text_options()
    }

    // --- ServerCompositionVisual.cs ------------------------------------------

    pub fn on_cache_mode_state_changed(&self) {
        if let Some(cache) = self.cache() {
            cache.invalidate_properties();
        }
        self.invalidate_content();
    }

    fn render_core(&self, context: &mut ServerVisualRenderContext<'_>, current_transformed_clip: LtrbRect) {
        self.content.render_core(self, context, current_transformed_clip);
    }

    /// `DeserializeChangesCore` of the container visual class: what the
    /// concrete classes call as their base.
    pub fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }
}

impl ServerCompositionVisualHooks for ServerCompositionVisual {
    fn on_fields_deserialized(
        &self,
        changed: crate::rendering::composition::generated::CompositionVisualChangedFields,
    ) {
        self.on_fields_deserialized_core(changed);
    }

    fn on_root_changing(&self) {
        if let (Some(root), Some(this)) = (self.root(), self.rc()) {
            root.remove_visual(&this);
            self.content.on_detached_from_root(self, &root);
        }
    }

    fn on_root_changed(&self) {
        if let (Some(root), Some(this)) = (self.root(), self.rc()) {
            root.add_visual(&this);
            self.content.on_attached_to_root(self, &root);
            self.adorner_helper_attached_to_root();
        }
        if let Some(cache) = self.cache() {
            cache.free_resources();
        }
    }

    fn on_parent_changing(&self) {
        self.on_parent_changing_core();
    }

    fn on_parent_changed(&self) {
        self.on_parent_changed_core();
    }

    fn on_adorned_visual_changing(&self) {
        self.on_adorned_visual_changing_core();
    }

    fn on_adorned_visual_changed(&self) {
        self.on_adorned_visual_changed_core();
    }

    fn on_cache_mode_changing(&self) {
        if let (Some(cache_mode), Some(this)) = (self.cache_mode(), self.rc()) {
            cache_mode.unsubscribe(&this);
        }
        if let Some(cache) = self.cache.borrow_mut().take() {
            cache.free_resources();
        }
    }

    fn on_cache_mode_changed(&self) {
        let cache_mode = self.cache_mode();
        *self.cache.borrow_mut() =
            cache_mode.as_ref().map(|mode| ServerCompositionVisualCache::new(self.this.clone(), mode.clone()));
        if let (Some(cache_mode), Some(this)) = (cache_mode, self.rc()) {
            cache_mode.subscribe(&this);
        }
        self.on_cache_mode_state_changed();
    }
}

impl ServerCompositionContainerVisualHooks for ServerCompositionVisual {}

impl IServerPropertyHost for ServerCompositionVisual {
    fn server_compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.upgrade()
    }

    fn set_value(&self, property: &'static CompositionProperty, change: ServerValueChange<'_>) {
        self.object.set_value(property, change);
    }
}

impl IServerAnimatedPropertyHost for ServerCompositionVisual {
    fn set_animated_value(
        &self,
        property: &'static CompositionProperty,
        current_value: ExpressionVariant,
        committed_at: Duration,
        animation: Rc<dyn IAnimationInstance>,
    ) {
        self.object.set_animated_value(property, current_value, committed_at, animation);
    }

    fn remove_animation_for_property(&self, property: &'static CompositionProperty) {
        self.object.remove_animation_for_property(property);
    }

    fn notify_animated_value_changed(&self, property: &'static CompositionProperty) {
        self.notify_animated_value_changed_core(property);
    }
}

impl IAnimatedServerObject for ServerCompositionVisual {
    fn server_object(&self) -> &ServerObject {
        &self.object
    }

    fn get_composition_property(&self, field_name: &str) -> Option<&'static CompositionProperty> {
        self.content.get_composition_property(field_name)
    }

    fn as_server_object_dyn(&self) -> &dyn IServerObject {
        self
    }
}

impl IExpressionObject for ServerCompositionVisual {
    fn get_property(&self, name: &str) -> ExpressionVariant {
        self.object.get_property(name)
    }
}

impl IServerRenderResourceObserver for ServerCompositionVisual {
    fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {
        self.content.dependency_queued_invalidate(self);
    }
}

impl IServerObject for ServerCompositionVisual {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.content.deserialize_changes_core(self, reader, committed_at);
    }

    /// Releases what the visual holds. Upstream the garbage collector
    /// reclaims a server visual once nothing refers to it; here the UI
    /// thread disposes it by id, and the references that would keep a
    /// cycle alive are dropped without notifications.
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.content.dispose(self);
        if let (Some(root), Some(this)) = (self.root(), self.rc()) {
            root.remove_visual(&this);
        }
        if let (Some(cache_mode), Some(this)) = (self.cache_mode(), self.rc()) {
            cache_mode.unsubscribe(&this);
        }
        if let Some(cache) = self.cache.borrow_mut().take() {
            cache.free_resources();
        }
        if let (Some(parent), Some(bounds)) = (self.parent(), self.transformed_sub_tree_bounds.get()) {
            if !parent.disposed.get() {
                parent.add_extra_dirty_rect(bounds);
            }
        }
        ServerCompositionVisualProps::id_of_root_property().set_field(self, None);
        ServerCompositionVisualProps::id_of_parent_property().set_field(self, None);
        ServerCompositionVisualProps::id_of_adorned_visual_property().set_field(self, None);
        ServerCompositionVisualProps::id_of_cache_mode_property().set_field(self, None);
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id).or_else(|| self.content.find_props(type_id))
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// The content of a plain container visual: nothing of its own.
pub struct ServerCompositionContainerVisual;

impl IServerVisualContent for ServerCompositionContainerVisual {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
