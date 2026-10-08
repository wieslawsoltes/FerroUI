//! Tests of the generated property blocks in isolation: a generic client
//! object and a generic server object embed a block each, the client is
//! committed on a compositor, the server compositor ticks, and the server
//! block is inspected.

use super::*;
use crate::media::immutable::{ImmutableSolidColorBrush, ImmutableTransform};
use crate::media::{Color, Colors, IBrush, ITransform, MediaContext, RenderOptions};
use crate::numerics::Quaternion;
use crate::rendering::composition::animations::{
    IAnimationInstance, ICompositionAnimation, ICompositionAnimationBase, IInterpolator,
};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::{
    CompositionProperty, IServerAnimatedPropertyHost, IServerClockItem, IServerObject, IServerPropertyHost,
    ServerCompositor, ServerObjectId, ServerValueChange,
};
use crate::rendering::composition::transport::{
    BatchResource, BatchStreamData, BatchStreamReader, BatchStreamWriter, BatchValue, IRegisterForSerialization,
};
use crate::rendering::composition::{
    CompositionBlendMode, CompositionGradientExtendMode, CompositionStretch, CompositionTileMode,
    CompositionTransparencyLevel, Compositor, ICompositionObject, ICompositionObjectHost, ICompositorSerializable,
    PendingAnimations,
};
use crate::rendering::testing::{ManualRenderLoop, MockPlatformRenderInterface};
use crate::rendering::{LayoutPassTiming, RendererDebugOverlays};
use crate::threading::Dispatcher;
use crate::{CornerRadius, Matrix, RelativePoint, RelativeRect, RelativeScalar, RelativeUnit, Size, Vector, Vector3D};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

// --- harness -----------------------------------------------------------------

struct Fixture {
    _dispatcher_scope: crate::threading::UnitTestDispatcherScope,
    locator_scope: Rc<dyn crate::reactive::IDisposable>,
    render_loop: std::sync::Arc<ManualRenderLoop>,
    compositor: Rc<Compositor>,
}

impl Fixture {
    fn new() -> Fixture {
        let dispatcher_scope = Dispatcher::unit_test_scope();
        let (locator_scope, _) = MockPlatformRenderInterface::install();
        let render_loop = ManualRenderLoop::new();
        let compositor = Compositor::with_scheduler(
            render_loop.clone(),
            None,
            false,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        Fixture { _dispatcher_scope: dispatcher_scope, locator_scope, render_loop, compositor }
    }

    /// Commits the pending changes and applies them on the server.
    fn run(&self) {
        self.compositor.commit();
        self.render_loop.tick();
    }

    fn create<C: ClientBlock, S: ServerBlock>(&self) -> Rc<TestClient<C>> {
        let server = self.compositor.create_server_object(|compositor, _| TestServer::<S>::new(compositor));
        Rc::new_cyclic(|this| TestClient {
            this: this.clone(),
            compositor: Rc::downgrade(&self.compositor),
            server,
            registered: Cell::new(false),
            pending_animations: PendingAnimations::new(),
            implicit_animations: RefCell::new(HashMap::new()),
            started_groups: RefCell::new(Vec::new()),
            log: RefCell::new(Vec::new()),
            props: C::create(),
        })
    }

    fn server<S: ServerBlock>(&self, id: ServerObjectId) -> Rc<TestServer<S>> {
        self.compositor.server().get::<TestServer<S>>(id).expect("the server object exists")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.locator_scope.dispose();
    }
}

/// A generated UI-thread block as the test client embeds it.
trait ClientBlock: Sized + 'static {
    fn create() -> Self;
    fn serialize(host: &TestClient<Self>, writer: &mut BatchStreamWriter<'_>);
}

/// A generated server block as the test server embeds it.
trait ServerBlock: Sized + 'static {
    fn create() -> Self;
    fn deserialize(host: &TestServer<Self>, reader: &mut BatchStreamReader<'_>, committed_at: Duration);
    fn find(&self, type_id: TypeId) -> Option<&dyn Any>;
}

/// The hand-written half of a composition object, reduced to what the
/// generated block needs.
struct TestClient<C: ClientBlock> {
    this: Weak<TestClient<C>>,
    compositor: Weak<Compositor>,
    server: ServerObjectId,
    registered: Cell<bool>,
    pending_animations: PendingAnimations,
    implicit_animations: RefCell<HashMap<String, Rc<dyn ICompositionAnimationBase>>>,
    started_groups: RefCell<Vec<(String, ExpressionVariant)>>,
    log: RefCell<Vec<String>>,
    props: C,
}

impl<C: ClientBlock> TestClient<C> {
    fn log(&self, entry: &str) {
        self.log.borrow_mut().push(entry.to_string());
    }

    fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

impl<C: ClientBlock> IRegisterForSerialization for TestClient<C> {
    fn register_for_serialization(&self) {
        if self.registered.replace(true) {
            return;
        }
        if let (Some(compositor), Some(this)) = (self.compositor.upgrade(), self.this.upgrade()) {
            compositor.register_for_serialization(this);
        }
    }
}

impl<C: ClientBlock> ICompositionObjectHost for TestClient<C> {
    fn server(&self) -> ServerObjectId {
        self.server
    }

    fn pending_animations(&self) -> &PendingAnimations {
        &self.pending_animations
    }

    fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.implicit_animations.borrow().get(property_name).cloned()
    }

    fn start_animation_group(
        &self,
        _grp: &Rc<dyn ICompositionAnimationBase>,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        self.started_groups.borrow_mut().push((target.to_string(), final_value));
        true
    }
}

impl<C: ClientBlock> ICompositionObject for TestClient<C> {
    fn server(&self) -> ServerObjectId {
        self.server
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl<C: ClientBlock> ICompositorSerializable for TestClient<C> {
    fn try_get_server(&self, _c: &Compositor) -> Option<ServerObjectId> {
        Some(self.server)
    }

    fn serialize_changes(&self, _c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.registered.set(false);
        C::serialize(self, writer);
    }
}

struct StartedAnimation {
    property: &'static CompositionProperty,
    current_value: ExpressionVariant,
    committed_at: Duration,
    animation: Rc<dyn IAnimationInstance>,
}

/// The hand-written half of a server object, reduced to what the generated
/// block needs. It records what the block asks of it.
struct TestServer<S: ServerBlock> {
    compositor: Weak<ServerCompositor>,
    log: RefCell<Vec<String>>,
    animations: RefCell<Vec<StartedAnimation>>,
    props: S,
}

impl<S: ServerBlock> TestServer<S> {
    fn new(compositor: &Rc<ServerCompositor>) -> Rc<dyn IServerObject> {
        Rc::new(TestServer::<S> {
            compositor: Rc::downgrade(compositor),
            log: RefCell::new(Vec::new()),
            animations: RefCell::new(Vec::new()),
            props: S::create(),
        })
    }

    fn log(&self, entry: &str) {
        self.log.borrow_mut().push(entry.to_string());
    }

    fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

impl<S: ServerBlock> IServerObject for TestServer<S> {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        S::deserialize(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.log("values_invalidated");
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl<S: ServerBlock> IServerPropertyHost for TestServer<S> {
    fn server_compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.upgrade()
    }

    fn set_value(&self, property: &'static CompositionProperty, change: ServerValueChange<'_>) {
        self.log(&format!("set_value {}{}", property.name(), if change.equal { " (equal)" } else { "" }));
        (change.assign)();
    }
}

impl<S: ServerBlock> IServerAnimatedPropertyHost for TestServer<S> {
    fn set_animated_value(
        &self,
        property: &'static CompositionProperty,
        current_value: ExpressionVariant,
        committed_at: Duration,
        animation: Rc<dyn IAnimationInstance>,
    ) {
        // What the animations of a server object do when an animation
        // starts.
        animation.initialize(committed_at, current_value, property);
        animation.activate();
        self.log(&format!("set_animated_value {}", property.name()));
        self.animations.borrow_mut().push(StartedAnimation { property, current_value, committed_at, animation });
    }

    fn remove_animation_for_property(&self, property: &'static CompositionProperty) {
        self.log(&format!("remove_animation {}", property.name()));
    }

    fn notify_animated_value_changed(&self, property: &'static CompositionProperty) {
        self.log(&format!("animated_value_changed {}", property.name()));
    }
}

macro_rules! client_block {
    ($props:ty) => {
        impl ClientBlock for $props {
            fn create() -> Self {
                <$props>::new()
            }
            fn serialize(host: &TestClient<Self>, writer: &mut BatchStreamWriter<'_>) {
                host.props.serialize_changes_core(host, writer);
            }
        }
    };
}

macro_rules! server_block {
    ($props:ty) => {
        impl ServerBlock for $props {
            fn create() -> Self {
                <$props>::new()
            }
            fn deserialize(host: &TestServer<Self>, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
                host.props.deserialize_changes_core(host, reader, committed_at);
            }
            fn find(&self, type_id: TypeId) -> Option<&dyn Any> {
                self.find_props(type_id)
            }
        }
    };
}

// --- the blocks under test ----------------------------------------------------

// CompositionTarget: plain values, an object reference and a payload.
client_block!(CompositionTargetProps);
server_block!(ServerCompositionTargetProps);

impl CompositionTargetHooks for TestClient<CompositionTargetProps> {
    fn validate_scaling_change(&self, old_value: f64, new_value: f64) {
        self.log(&format!("validate_scaling {old_value} -> {new_value}"));
    }
    fn on_scaling_changing(&self) {
        self.log("scaling_changing");
    }
    fn on_scaling_changed(&self) {
        self.log("scaling_changed");
    }
}

impl ServerCompositionTargetHooks for TestServer<ServerCompositionTargetProps> {
    fn on_fields_deserialized(&self, changed: CompositionTargetChangedFields) {
        self.log(&format!("fields_deserialized {:#x}", changed.bits()));
    }
    fn on_scaling_changing(&self) {
        self.log("scaling_changing");
    }
    fn on_scaling_changed(&self) {
        self.log("scaling_changed");
    }
    fn on_size_changed(&self) {
        self.log("size_changed");
    }
    fn on_root_changed(&self) {
        self.log("root_changed");
    }
}

// CompositionVisual: animated properties and defaults.
client_block!(CompositionVisualProps);
server_block!(ServerCompositionVisualProps);

impl CompositionVisualHooks for TestClient<CompositionVisualProps> {
    fn initialize_defaults_extra(&self) {
        self.log("initialize_defaults_extra");
    }
}

impl ServerCompositionVisualHooks for TestServer<ServerCompositionVisualProps> {}

// CompositionSolidColorVisual: a generated base on the UI thread
// (visual -> container visual -> solid color visual) and a hand-written
// base on the server (the size dependant visual, which derives from the
// container visual).
client_block!(CompositionSolidColorVisualProps);

impl CompositionVisualHooks for TestClient<CompositionSolidColorVisualProps> {}
impl CompositionContainerVisualHooks for TestClient<CompositionSolidColorVisualProps> {}
impl CompositionSolidColorVisualHooks for TestClient<CompositionSolidColorVisualProps> {}

struct SolidColorVisualServer {
    base: ServerCompositionContainerVisualProps,
    props: ServerCompositionSolidColorVisualProps,
}

impl ServerBlock for SolidColorVisualServer {
    fn create() -> Self {
        Self { base: ServerCompositionContainerVisualProps::new(), props: ServerCompositionSolidColorVisualProps::new() }
    }
    fn deserialize(host: &TestServer<Self>, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        host.props.props.deserialize_changes_core(host, reader, committed_at);
    }
    fn find(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id).or_else(|| self.base.find_props(type_id))
    }
}

impl ServerCompositionVisualHooks for TestServer<SolidColorVisualServer> {}
impl ServerCompositionContainerVisualHooks for TestServer<SolidColorVisualServer> {}
impl ServerCompositionSolidColorVisualHooks for TestServer<SolidColorVisualServer> {
    fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.log("base_deserialize");
        self.props.base.deserialize_changes_core(self, reader, committed_at);
    }
}

// CompositionSolidColorBrush: generated bases on both sides and resource
// references (the transforms of the brush).
client_block!(CompositionSolidColorBrushProps);
server_block!(ServerCompositionSolidColorBrushProps);

impl CompositionBrushHooks for TestClient<CompositionSolidColorBrushProps> {
    fn initialize_defaults_extra(&self) {
        self.log("brush_defaults_extra");
    }
}
impl CompositionSolidColorBrushHooks for TestClient<CompositionSolidColorBrushProps> {
    fn initialize_defaults_extra(&self) {
        self.log("solid_color_brush_defaults_extra");
    }
}
impl ServerCompositionBrushHooks for TestServer<ServerCompositionSolidColorBrushProps> {}
impl ServerCompositionSolidColorBrushHooks for TestServer<ServerCompositionSolidColorBrushProps> {}

// CompositionVisualCollection: the list proxy.
type TestVisual = TestClient<CompositionVisualProps>;

impl ClientBlock for CompositionVisualCollectionProps<TestVisual> {
    fn create() -> Self {
        CompositionVisualCollectionProps::new()
    }
    fn serialize(host: &TestClient<Self>, writer: &mut BatchStreamWriter<'_>) {
        host.props.serialize_changes_core(host, writer);
    }
}
server_block!(ServerCompositionVisualCollectionProps);

impl CompositionVisualCollectionHooks<TestVisual> for TestClient<CompositionVisualCollectionProps<TestVisual>> {
    fn on_before_added(&self, _item: &Rc<TestVisual>) {
        self.log("before_added");
    }
    fn on_added(&self, _item: &Rc<TestVisual>) {
        self.log("added");
    }
    fn on_removed(&self, _item: &Rc<TestVisual>) {
        self.log("removed");
    }
    fn on_before_clear(&self) {
        self.log("before_clear");
    }
    fn on_before_replace(&self, _old_item: &Rc<TestVisual>, _new_item: &Rc<TestVisual>) {
        self.log("before_replace");
    }
    fn on_replace(&self, _old_item: &Rc<TestVisual>, _new_item: &Rc<TestVisual>) {
        self.log("replace");
    }
    fn on_clear(&self) {
        self.log("clear");
    }
}

impl ServerCompositionVisualCollectionHooks for TestServer<ServerCompositionVisualCollectionProps> {}

// --- a fake animation -----------------------------------------------------------

#[derive(Default)]
struct FakeAnimationState {
    initialized: Option<(Duration, ExpressionVariant, i32)>,
    activated: u32,
}

struct FakeAnimationInstance {
    target: ServerObjectId,
    final_value: Option<ExpressionVariant>,
    state: RefCell<FakeAnimationState>,
}

impl IServerClockItem for FakeAnimationInstance {
    fn on_tick(&self) {}
}

impl IAnimationInstance for FakeAnimationInstance {
    fn target_object(&self) -> ServerObjectId {
        self.target
    }

    fn evaluate(&self, _now: Duration, current_value: ExpressionVariant) -> ExpressionVariant {
        self.final_value.unwrap_or(current_value)
    }

    fn initialize(&self, started_at: Duration, starting_value: ExpressionVariant, property: &'static CompositionProperty) {
        self.state.borrow_mut().initialized = Some((started_at, starting_value, property.id()));
    }

    fn activate(&self) {
        self.state.borrow_mut().activated += 1;
    }

    fn deactivate(&self) {}

    fn invalidate(&self) {}
}

#[derive(Default)]
struct FakeAnimation {
    created: RefCell<Vec<Rc<FakeAnimationInstance>>>,
}

impl ICompositionAnimationBase for FakeAnimation {
    fn as_composition_animation(&self) -> Option<&dyn ICompositionAnimation> {
        Some(self)
    }
}

impl ICompositionAnimation for FakeAnimation {
    fn target(&self) -> Option<String> {
        Some("Opacity".to_string())
    }

    fn create_instance(
        &self,
        target_object: ServerObjectId,
        final_value: Option<ExpressionVariant>,
    ) -> crate::rendering::composition::animations::AnimationInstanceFactory {
        let instance =
            Rc::new(FakeAnimationInstance { target: target_object, final_value, state: RefCell::default() });
        self.created.borrow_mut().push(instance.clone());
        // The tests look at the instance before the server has it, and run
        // the server on their own thread: the factory hands out the instance
        // made here.
        let instance = crate::utilities::ThreadBound::new(instance);
        crate::rendering::composition::animations::AnimationInstanceFactory::new(move || {
            instance.get().clone() as Rc<dyn IAnimationInstance>
        })
    }
}

// --- tests ------------------------------------------------------------------------

#[test]
fn only_changed_members_are_sent() {
    let fixture = Fixture::new();
    let target = fixture.create::<CompositionTargetProps, ServerCompositionTargetProps>();
    target.props.initialize_defaults(&*target);
    // Nothing changed: the object is not registered and nothing is sent.
    assert!(!target.registered.get());

    target.props.set_scaling(&*target, 2.0);
    assert_eq!(target.take_log(), ["validate_scaling 0 -> 2", "scaling_changing", "scaling_changed"]);
    assert!(target.registered.get());
    assert_eq!(target.props.scaling(), 2.0);
    fixture.run();

    let server = fixture.server::<ServerCompositionTargetProps>(target.server);
    assert_eq!(server.props.scaling(), 2.0);
    let scaling_bit = CompositionTargetChangedFields::SCALING.bits();
    assert_eq!(
        server.take_log(),
        [
            "scaling_changing".to_string(),
            "set_value Scaling".to_string(),
            "scaling_changed".to_string(),
            format!("fields_deserialized {scaling_bit:#x}"),
            "values_invalidated".to_string(),
        ]
    );

    // A second change sends only that member; the mask was reset.
    target.props.set_size(&*target, Size::new(30.0, 40.0));
    target.props.set_is_enabled(&*target, true);
    fixture.run();
    assert_eq!(server.props.size(), Size::new(30.0, 40.0));
    assert!(server.props.is_enabled());
    assert_eq!(server.props.scaling(), 2.0);
    let mask = (CompositionTargetChangedFields::SIZE | CompositionTargetChangedFields::IS_ENABLED).bits();
    assert_eq!(
        server.take_log(),
        [
            "set_value IsEnabled".to_string(),
            "set_value Size".to_string(),
            "size_changed".to_string(),
            format!("fields_deserialized {mask:#x}"),
            "values_invalidated".to_string(),
        ]
    );

    // Setting the current value again changes nothing and sends nothing.
    target.take_log();
    target.props.set_scaling(&*target, 2.0);
    assert!(target.take_log().is_empty());
    assert!(!target.registered.get());
}

#[test]
fn the_serialized_form_holds_the_mask_and_the_changed_members_only() {
    let fixture = Fixture::new();
    let target = fixture.create::<CompositionTargetProps, ServerCompositionTargetProps>();
    target.props.set_scaling(&*target, 1.5);

    let mut data = BatchStreamData::new();
    {
        let mut writer = BatchStreamWriter::new(&mut data);
        target.props.serialize_changes_core(&*target, &mut writer);
    }
    let mut reader = BatchStreamReader::new(&mut data);
    assert_eq!(reader.read::<CompositionTargetChangedFields>(), CompositionTargetChangedFields::SCALING);
    assert_eq!(reader.read::<f64>(), 1.5);
    assert!(reader.is_struct_eof());
    assert!(reader.is_object_eof());

    // The mask was reset: a second serialization writes an empty mask.
    let mut data = BatchStreamData::new();
    {
        let mut writer = BatchStreamWriter::new(&mut data);
        target.props.serialize_changes_core(&*target, &mut writer);
    }
    let mut reader = BatchStreamReader::new(&mut data);
    assert_eq!(reader.read::<CompositionTargetChangedFields>(), CompositionTargetChangedFields::empty());
    assert!(reader.is_struct_eof());
}

#[test]
fn changed_fields_masks_follow_the_schema_order() {
    assert_eq!(CompositionTargetChangedFields::ROOT.bits(), 1);
    assert_eq!(CompositionTargetChangedFields::IS_ENABLED.bits(), 2);
    assert_eq!(CompositionTargetChangedFields::PLATFORM_SPECIFIC_SCENE_INFO.bits(), 1 << 7);
    // An animated property takes two bits; the mask grows to fit.
    assert_eq!(CompositionVisualChangedFields::ROOT.bits(), 1);
    assert_eq!(CompositionVisualChangedFields::VISIBLE.bits(), 1 << 2);
    assert_eq!(CompositionVisualChangedFields::VISIBLE_ANIMATED.bits(), 1 << 3);
    // 22 properties, 12 of them animated: 34 bits. The bit of the skipped
    // text options property stays reserved.
    assert_eq!(CompositionVisualChangedFields::CACHE_MODE.bits(), 1u64 << 33);
    assert_eq!(std::mem::size_of::<CompositionVisualChangedFields>(), 8);
    assert_eq!(std::mem::size_of::<CompositionTargetChangedFields>(), 1);
    assert_eq!(std::mem::size_of::<CompositionSimplePenChangedFields>(), 1);
}

#[test]
fn object_references_cross_by_id() {
    let fixture = Fixture::new();
    let target = fixture.create::<CompositionTargetProps, ServerCompositionTargetProps>();
    let visual = fixture.create::<CompositionVisualProps, ServerCompositionVisualProps>();

    let root: Rc<dyn ICompositionObject> = visual.clone();
    target.props.set_root(&*target, Some(root.clone()));
    assert!(Rc::ptr_eq(&target.props.root().unwrap().into_any_rc().downcast::<TestVisual>().unwrap(), &visual));
    // The same reference again is not a change.
    target.registered.set(false);
    target.props.set_root(&*target, Some(root));
    assert!(!target.registered.get());
    target.registered.set(true);
    fixture.run();

    let server = fixture.server::<ServerCompositionTargetProps>(target.server);
    let server_visual = fixture.compositor.server().get_object(visual.server).unwrap();
    let server_root = server.props.root().expect("the root was resolved");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&server_root), Rc::as_ptr(&server_visual)));
    assert!(server.take_log().contains(&"root_changed".to_string()));

    // A payload crosses as it is.
    let info: std::sync::Arc<dyn Any + Send + Sync> = std::sync::Arc::new(42i32);
    target.props.set_platform_specific_scene_info(&*target, Some(info.clone()));
    target.props.set_root(&*target, None);
    fixture.run();
    assert!(server.props.root().is_none());
    assert!(std::sync::Arc::ptr_eq(&server.props.platform_specific_scene_info().unwrap(), &info));
}

#[test]
fn default_values_reach_the_server() {
    let fixture = Fixture::new();
    let visual = fixture.create::<CompositionVisualProps, ServerCompositionVisualProps>();
    visual.props.initialize_defaults(&*visual);
    assert_eq!(visual.take_log(), ["initialize_defaults_extra"]);
    assert!(visual.props.visible());
    assert_eq!(visual.props.opacity(), 1.0);
    assert!(visual.props.clip_to_bounds());
    assert_eq!(visual.props.orientation(), Quaternion::IDENTITY);
    assert_eq!(visual.props.scale(), Vector3D::new(1.0, 1.0, 1.0));
    assert_eq!(visual.props.transform_matrix(), Matrix::IDENTITY);
    // Properties without a default keep the default of their type.
    assert_eq!(visual.props.offset(), Vector3D::default());
    assert!(visual.props.clip().is_none());

    // Before the batch the server block holds the defaults of the types.
    fixture.compositor.commit();
    fixture.render_loop.tick();
    let server = fixture.server::<ServerCompositionVisualProps>(visual.server);
    assert!(server.props.visible());
    assert_eq!(server.props.opacity(), 1.0);
    assert!(server.props.clip_to_bounds());
    assert_eq!(server.props.orientation(), Quaternion::IDENTITY);
    assert_eq!(server.props.scale(), Vector3D::new(1.0, 1.0, 1.0));
    assert_eq!(server.props.transform_matrix(), Matrix::IDENTITY);
    assert_eq!(server.props.offset(), Vector3D::default());
    // An animated property set directly removes the animation of the
    // property.
    let log = server.take_log();
    assert!(log.contains(&"remove_animation Opacity".to_string()));
    assert!(!log.iter().any(|entry| entry.starts_with("set_value")));

    let fresh = ServerCompositionVisualProps::new();
    assert!(!fresh.visible());
    assert_eq!(fresh.opacity(), 0.0);
}

#[test]
fn other_value_types_round_trip() {
    let fixture = Fixture::new();
    let visual = fixture.create::<CompositionVisualProps, ServerCompositionVisualProps>();
    let options = RenderOptions { requires_full_opacity_handling: Some(true), ..Default::default() };
    visual.props.set_render_options(&*visual, options);
    visual.props.set_size(&*visual, Vector::new(3.0, 4.0));
    visual.props.set_adorner_is_clipped(&*visual, true);
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    visual.props.set_opacity_mask_brush_transport_field(&*visual, Some(BatchResource::Value(brush.clone())));
    fixture.run();
    let server = fixture.server::<ServerCompositionVisualProps>(visual.server);
    assert_eq!(server.props.render_options(), options);
    assert_eq!(server.props.size(), Vector::new(3.0, 4.0));
    assert!(server.props.adorner_is_clipped());
    let mask = server.props.opacity_mask_brush().expect("the brush crossed");
    // What crosses is the shared form of the brush, which the brush keeps.
    assert_eq!(mask.value.reference_id(), IBrush::reference_id(&brush.to_shared().unwrap()));
    assert!(mask.resource.is_none());
}

#[test]
fn derived_blocks_serialize_their_base_first() {
    let fixture = Fixture::new();
    let visual = fixture.create::<CompositionSolidColorVisualProps, SolidColorVisualServer>();
    visual.props.initialize_defaults(&*visual);
    // The base properties are set through the base block.
    visual.props.base().base().set_offset(&*visual, Vector3D::new(1.0, 2.0, 3.0));
    visual.props.set_color(&*visual, Colors::BLUE);
    fixture.run();

    let server = fixture.server::<SolidColorVisualServer>(visual.server);
    assert_eq!(server.props.props.color(), Colors::BLUE);
    assert_eq!(server.props.base.base().offset(), Vector3D::new(1.0, 2.0, 3.0));
    assert!(server.props.base.base().visible());
    // The hand-written server base read its part before the class did.
    let log = server.take_log();
    assert_eq!(log.first().map(String::as_str), Some("base_deserialize"));
    let offset = log.iter().position(|e| e == "remove_animation Offset").unwrap();
    let color = log.iter().position(|e| e == "remove_animation Color").unwrap();
    assert!(offset < color);

    // Composition properties find their fields through the blocks.
    let offset_property = ServerCompositionVisualProps::id_of_offset_property();
    assert_eq!(offset_property.get_field(&*server), Vector3D::new(1.0, 2.0, 3.0));
    let color_property = ServerCompositionSolidColorVisualProps::id_of_color_property();
    assert_eq!(color_property.get_field(&*server), Colors::BLUE);
    color_property.set_field(&*server, Colors::GREEN);
    assert_eq!(server.props.props.color(), Colors::GREEN);
    let get_variant = color_property.get_variant().expect("a color has a variant form");
    assert_eq!(get_variant(&*server), ExpressionVariant::Color(Colors::GREEN));
}

#[test]
fn constructor_chain_initializes_defaults_base_first() {
    let fixture = Fixture::new();
    let brush = fixture.create::<CompositionSolidColorBrushProps, ServerCompositionSolidColorBrushProps>();
    brush.props.initialize_defaults(&*brush);
    assert_eq!(brush.take_log(), ["brush_defaults_extra", "solid_color_brush_defaults_extra"]);
    assert_eq!(brush.props.base().opacity(), 1.0);

    let matrix = Matrix::create_translation(3.0, 4.0);
    let transform: Rc<dyn ITransform> = Rc::new(ImmutableTransform::new(matrix));
    brush.props.base().set_relative_transform(&*brush, Some(BatchResource::Value(transform)));
    brush.props.base().set_transform_origin(&*brush, RelativePoint::CENTER);
    brush.props.set_color(&*brush, Colors::RED);
    fixture.run();

    let server = fixture.server::<ServerCompositionSolidColorBrushProps>(brush.server);
    assert_eq!(server.props.color(), Colors::RED);
    assert_eq!(server.props.base().opacity(), 1.0);
    assert_eq!(server.props.base().transform_origin(), RelativePoint::CENTER);
    assert_eq!(server.props.base().relative_transform().unwrap().value.value(), matrix);
    assert!(server.props.base().transform().is_none());
}

#[test]
fn composition_properties_are_registered_once_with_distinct_ids() {
    let opacity = ServerCompositionVisualProps::id_of_opacity_property();
    let visible = ServerCompositionVisualProps::id_of_visible_property();
    let brush_opacity = ServerCompositionBrushProps::id_of_opacity_property();
    assert_eq!(opacity.name(), "Opacity");
    assert_eq!(opacity.owner_name(), "ServerCompositionVisual");
    assert_eq!(opacity.owner(), TypeId::of::<ServerCompositionVisualProps>());
    assert!(opacity.id() > 0);
    assert_ne!(opacity.id(), visible.id());
    assert_ne!(opacity.id(), brush_opacity.id());
    assert_eq!(opacity.id(), ServerCompositionVisualProps::id_of_opacity_property().id());
    assert!(std::ptr::eq(opacity, ServerCompositionVisualProps::id_of_opacity_property()));

    // Lookup by name covers the properties with a variant form, through
    // the generated bases.
    let found = ServerCompositionVisualProps::get_composition_property("Opacity").unwrap();
    assert_eq!(found.id(), opacity.id());
    assert!(ServerCompositionVisualProps::get_composition_property("Clip").is_none());
    // The matrix is declared with a qualified type name, which upstream's
    // table of variant types does not list.
    assert!(ServerCompositionVisualProps::get_composition_property("TransformMatrix").is_none());
    assert!(ServerCompositionVisualProps::id_of_transform_matrix_property().get_variant().is_none());
    assert!(ServerCompositionVisualProps::id_of_transform_matrix_property().set_variant().is_some());
    assert_eq!(
        ServerCompositionContainerVisualProps::get_composition_property("Visible").map(|p| p.id()),
        Some(visible.id())
    );
    assert_eq!(
        ServerCompositionSolidColorBrushProps::get_composition_property("Opacity").map(|p| p.id()),
        Some(brush_opacity.id())
    );
    assert!(ServerCompositionSolidColorBrushProps::get_composition_property("Color").is_some());
    assert!(ServerCompositionSolidColorBrushProps::get_composition_property("Nope").is_none());
    // Only an animated property can be written from a variant.
    assert!(visible.set_variant().is_some());
    assert!(ServerCompositionTargetProps::id_of_scaling_property().set_variant().is_none());
}

#[test]
fn start_animation_sends_the_instance_through_the_seam() {
    let fixture = Fixture::new();
    let visual = fixture.create::<CompositionVisualProps, ServerCompositionVisualProps>();
    visual.props.initialize_defaults(&*visual);
    fixture.run();
    let server = fixture.server::<ServerCompositionVisualProps>(visual.server);
    server.take_log();

    let animation = FakeAnimation::default();
    assert!(!visual.props.start_animation(&*visual, "Clip", &animation, None));
    assert!(!visual.props.start_animation(&*visual, "Nope", &animation, None));
    assert!(animation.created.borrow().is_empty());

    assert!(visual.props.start_animation(&*visual, "Opacity", &animation, Some(ExpressionVariant::Double(0.25))));
    assert_eq!(animation.created.borrow().len(), 1);
    assert_eq!(animation.created.borrow()[0].target, visual.server);
    assert_eq!(visual.pending_animations.count(), 1);
    assert!(visual.registered.get());
    fixture.run();
    assert_eq!(visual.pending_animations.count(), 0);

    // The server host was asked to start the animation with the current
    // value of the field; the field itself is untouched.
    assert_eq!(server.take_log(), ["set_animated_value Opacity", "values_invalidated"]);
    assert_eq!(server.props.opacity(), 1.0);
    let started = server.animations.borrow_mut().pop().unwrap();
    let opacity = ServerCompositionVisualProps::id_of_opacity_property();
    assert_eq!(started.property.id(), opacity.id());
    assert_eq!(started.current_value, ExpressionVariant::Double(1.0));
    let instance = animation.created.borrow()[0].clone();
    let state = instance.state.borrow();
    assert_eq!(state.initialized, Some((started.committed_at, ExpressionVariant::Double(1.0), opacity.id())));
    assert_eq!(state.activated, 1);

    // The animation writes the field through the property, type-erased.
    let value = started.animation.evaluate(Duration::ZERO, started.current_value);
    let set_variant = started.property.set_variant().expect("an animated property");
    set_variant(&*server, value);
    server.notify_animated_value_changed(started.property);
    assert_eq!(server.props.opacity(), 0.25);
    assert_eq!(server.take_log(), ["animated_value_changed Opacity"]);

    // A direct set afterwards removes the animation.
    visual.props.set_opacity(&*visual, 0.5);
    fixture.run();
    assert_eq!(server.props.opacity(), 0.5);
    assert_eq!(server.take_log(), ["remove_animation Opacity", "values_invalidated"]);
}

#[test]
fn implicit_animations_replace_the_value_with_an_animation() {
    let fixture = Fixture::new();
    let visual = fixture.create::<CompositionVisualProps, ServerCompositionVisualProps>();
    visual.props.initialize_defaults(&*visual);
    fixture.run();
    let server = fixture.server::<ServerCompositionVisualProps>(visual.server);
    server.take_log();

    let animation = Rc::new(FakeAnimation::default());
    visual.implicit_animations.borrow_mut().insert("Opacity".to_string(), animation.clone());
    visual.props.set_opacity(&*visual, 0.5);
    // The UI-thread value is the new value right away.
    assert_eq!(visual.props.opacity(), 0.5);
    assert_eq!(animation.created.borrow().len(), 1);
    assert_eq!(animation.created.borrow()[0].final_value, Some(ExpressionVariant::Double(0.5)));
    assert_eq!(*visual.started_groups.borrow(), [("Opacity".to_string(), ExpressionVariant::Double(0.5))]);
    fixture.run();
    // The server got the animation instead of the value.
    assert_eq!(server.take_log(), ["set_animated_value Opacity", "values_invalidated"]);
    assert_eq!(server.props.opacity(), 1.0);

    // A property without an implicit animation is sent as a value, and a
    // pending animation of a property is dropped by a later direct set.
    visual.implicit_animations.borrow_mut().clear();
    assert!(visual.props.start_animation(&*visual, "Opacity", &*animation, None));
    assert_eq!(visual.pending_animations.count(), 1);
    visual.props.set_opacity(&*visual, 0.75);
    assert_eq!(visual.pending_animations.count(), 0);
    fixture.run();
    assert_eq!(server.props.opacity(), 0.75);
    assert_eq!(server.take_log(), ["remove_animation Opacity", "values_invalidated"]);
}

#[test]
fn list_proxy_round_trips() {
    let fixture = Fixture::new();
    let list = fixture
        .create::<CompositionVisualCollectionProps<TestVisual>, ServerCompositionVisualCollectionProps>();
    list.props.initialize_defaults(&*list);
    let visuals: Vec<Rc<TestVisual>> =
        (0..4).map(|_| fixture.create::<CompositionVisualProps, ServerCompositionVisualProps>()).collect();
    let server_ids = |server: &TestServer<ServerCompositionVisualCollectionProps>| -> Vec<*const ()> {
        server.props.list().list().iter().map(|o| Rc::as_ptr(o) as *const ()).collect()
    };
    let expected = |indices: &[usize]| -> Vec<*const ()> {
        indices
            .iter()
            .map(|i| Rc::as_ptr(&fixture.compositor.server().get_object(visuals[*i].server).unwrap()) as *const ())
            .collect()
    };

    // Add.
    list.props.add(&*list, visuals[0].clone());
    list.props.add(&*list, visuals[1].clone());
    assert_eq!(list.take_log(), ["before_added", "added", "before_added", "added"]);
    assert_eq!(list.props.count(), 2);
    assert!(list.registered.get());
    fixture.run();
    let server = fixture.server::<ServerCompositionVisualCollectionProps>(list.server);
    assert_eq!(server_ids(&server), expected(&[0, 1]));
    assert_eq!(server.props.list().items::<TestServer<ServerCompositionVisualProps>>().len(), 2);

    // Insert.
    list.props.insert(&*list, 1, visuals[2].clone());
    assert_eq!(list.props.index_of(&visuals[2]), Some(1));
    assert!(list.props.contains(&visuals[1]));
    assert!(!list.props.contains(&visuals[3]));
    fixture.run();
    assert_eq!(server_ids(&server), expected(&[0, 2, 1]));

    // Replace.
    list.take_log();
    list.props.set(&*list, 0, visuals[3].clone());
    assert_eq!(list.take_log(), ["before_replace", "replace"]);
    assert!(Rc::ptr_eq(&list.props.get(0), &visuals[3]));
    fixture.run();
    assert_eq!(server_ids(&server), expected(&[3, 2, 1]));

    // Remove, by item and by index.
    assert!(list.props.remove(&*list, &visuals[2]));
    assert!(!list.props.remove(&*list, &visuals[0]));
    assert_eq!(list.take_log(), ["removed"]);
    list.props.remove_at(&*list, 0);
    assert_eq!(list.take_log(), ["removed"]);
    fixture.run();
    assert_eq!(server_ids(&server), expected(&[1]));

    // An unchanged list is not resent: the server keeps what it has.
    let mut data = BatchStreamData::new();
    {
        let mut writer = BatchStreamWriter::new(&mut data);
        list.props.serialize_changes_core(&*list, &mut writer);
    }
    let mut reader = BatchStreamReader::new(&mut data);
    assert_eq!(reader.read::<u8>(), 0);
    assert!(reader.is_struct_eof() && reader.is_object_eof());

    // Clear.
    let mut copy = vec![None, None];
    list.props.copy_to(&mut copy, 1);
    assert!(copy[0].is_none() && Rc::ptr_eq(copy[1].as_ref().unwrap(), &visuals[1]));
    assert_eq!(list.props.items().len(), 1);
    assert!(!list.props.is_read_only());
    list.props.clear(&*list);
    assert_eq!(list.take_log(), ["before_clear", "clear"]);
    fixture.run();
    assert!(server_ids(&server).is_empty());
    assert_eq!(server.props.list().count(), 0);
}

#[test]
fn server_only_classes_serialize_all_their_properties() {
    let mut data = BatchStreamData::new();
    {
        let mut writer = BatchStreamWriter::new(&mut data);
        ServerCompositionSimpleTileBrushProps::serialize_all_changes(
            &mut writer,
            crate::media::AlignmentX::Right,
            crate::media::AlignmentY::Bottom,
            RelativeRect::FILL,
            RelativeRect::new(1.0, 2.0, 3.0, 4.0, RelativeUnit::Absolute),
            crate::media::Stretch::UniformToFill,
            crate::media::TileMode::FlipXY,
        );
    }
    let mut reader = BatchStreamReader::new(&mut data);
    assert_eq!(reader.read::<CompositionSimpleTileBrushChangedFields>(), CompositionSimpleTileBrushChangedFields::all());
    assert_eq!(reader.read::<crate::media::AlignmentX>(), crate::media::AlignmentX::Right);
    assert_eq!(reader.read::<crate::media::AlignmentY>(), crate::media::AlignmentY::Bottom);
    assert_eq!(reader.read::<RelativeRect>(), RelativeRect::FILL);
    assert_eq!(reader.read::<RelativeRect>(), RelativeRect::new(1.0, 2.0, 3.0, 4.0, RelativeUnit::Absolute));
    assert_eq!(reader.read::<crate::media::Stretch>(), crate::media::Stretch::UniformToFill);
    assert_eq!(reader.read::<crate::media::TileMode>(), crate::media::TileMode::FlipXY);
    assert!(reader.is_struct_eof() && reader.is_object_eof());
}

#[test]
fn schema_value_types_cross_the_value_stream() {
    fn round_trip<T: BatchValue + PartialEq + std::fmt::Debug + Copy>(value: T) {
        let mut data = BatchStreamData::new();
        BatchStreamWriter::new(&mut data).write(value);
        let mut reader = BatchStreamReader::new(&mut data);
        assert_eq!(reader.read::<T>(), value);
        assert!(reader.is_struct_eof());
    }
    round_trip(Quaternion::new(1.0, 2.0, 3.0, 4.0));
    round_trip(crate::numerics::Vector2::new(1.0, 2.0));
    round_trip(crate::numerics::Vector3::new(1.0, 2.0, 3.0));
    round_trip(crate::numerics::Vector4::new(1.0, 2.0, 3.0, 4.0));
    round_trip(crate::numerics::Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0));
    round_trip(crate::numerics::Matrix4x4::IDENTITY);
    round_trip(CornerRadius::new(1.0, 2.0, 3.0, 4.0));
    round_trip(RelativePoint::new(0.25, 0.75, RelativeUnit::Absolute));
    round_trip(RelativeScalar::new(0.5, RelativeUnit::Relative));
    round_trip(RelativeRect::new(1.0, 2.0, 3.0, 4.0, RelativeUnit::Absolute));
    round_trip(RelativeUnit::Absolute);
    round_trip(LayoutPassTiming::new(3, Duration::from_millis(7)));
    round_trip(RendererDebugOverlays::FPS | RendererDebugOverlays::DIRTY_RECTS);
    round_trip(CompositionTransparencyLevel::Mica);
    round_trip(CompositionBlendMode::Luminosity);
    round_trip(CompositionGradientExtendMode::Mirror);
    round_trip(CompositionTileMode::TILE | CompositionTileMode::FLIP_X);
    round_trip(CompositionStretch::Fill);
    round_trip(crate::media::PenLineCap::Square);
    round_trip(crate::media::PenLineJoin::Round);
    round_trip(crate::media::GradientSpreadMethod::Repeat);
    round_trip(RenderOptions {
        edge_mode: crate::media::EdgeMode::Aliased,
        requires_full_opacity_handling: Some(false),
        ..Default::default()
    });
    round_trip(Color::from_uint32(0x80112233));
}

#[test]
fn key_frame_animation_table_matches_the_schema() {
    macro_rules! collect {
        ($(($class:ident, $factory:ident, $ty:ty, $interpolator:path)),* $(,)?) => {{
            let mut entries: Vec<(&str, &str, &str)> = Vec::new();
            $(
                // The interpolator of an entry interpolates the value type
                // of the entry.
                let _: &dyn IInterpolator<$ty> = &$interpolator;
                entries.push((stringify!($class), stringify!($factory), std::any::type_name::<$ty>()));
            )*
            entries
        }};
    }
    let entries = for_each_composition_key_frame_animation!(collect);
    assert_eq!(entries.len(), KEY_FRAME_ANIMATIONS.len());
    assert_eq!(entries.len(), 12);
    for ((class, factory, _), (name, _)) in entries.iter().zip(KEY_FRAME_ANIMATIONS) {
        assert_eq!(*class, format!("{name}KeyFrameAnimation"));
        assert!(factory.starts_with("create_") && factory.ends_with("_key_frame_animation"));
    }
    assert_eq!(entries[0], ("ScalarKeyFrameAnimation", "create_scalar_key_frame_animation", "f32"));
    assert_eq!(entries[7].1, "create_vector3d_key_frame_animation");
    assert_eq!(KEY_FRAME_ANIMATIONS[3], ("Color", "Color"));
}
