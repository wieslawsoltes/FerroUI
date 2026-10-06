//! Classes shared by the tests of this crate.

use crate::{Control, ControlImpl, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::input::{FocusManager, IInputRoot, InputElement, KeyboardNavigation, KeyboardNavigationMode};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::{
    IHitTester, IPresentationSource, IRenderer, ManagedHitTester, RendererDiagnostics, SceneInvalidatedEventArgs,
};
use std::any::{Any, TypeId};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Isolates the dispatcher and the loaded queue of the calling test.
pub struct TestScope {
    _dispatcher: UnitTestDispatcherScope,
    // Fonts and a text shaper: text controls are measured in most trees.
    _text: TextTestScope,
}

/// Starts a test scope: the calling thread gets its own dispatcher and an
/// empty loaded queue for the lifetime of the returned value.
pub fn test_scope() -> TestScope {
    Control::reset_loaded_queue_for_unit_tests();
    TestScope { _dispatcher: Dispatcher::unit_test_scope(), _text: TextTestScope::new() }
}

impl Drop for TestScope {
    fn drop(&mut self) {
        Control::reset_loaded_queue_for_unit_tests();
    }
}

/// A renderer that does nothing.
#[derive(Default)]
pub struct TestRenderer;

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<RendererDiagnostics> {
        RendererDiagnostics::new()
    }
    fn scene_invalidated(&self, _handler: Rc<dyn Fn(&SceneInvalidatedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
    fn add_dirty(&self, _visual: &Visual) {}
    fn recalculate_children(&self, _visual: &Visual) {}
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

/// The presentation source and layout root of a [`TestRoot`].
pub struct TestSource {
    this: Weak<TestSource>,
    root: WeakRef<TestRoot>,
    renderer: Rc<TestRenderer>,
    focus_manager: Rc<FocusManager>,
    layout_manager: RefCell<Option<Rc<dyn ILayoutManager>>>,
    hit_tester: RefCell<Option<Rc<dyn IHitTester>>>,
    platform_settings: RefCell<Option<Rc<dyn ferroui_base::platform::IPlatformSettings>>>,
}

impl TestSource {
    fn new(root: WeakRef<TestRoot>) -> Rc<Self> {
        let source = Rc::new_cyclic(|this: &Weak<TestSource>| TestSource {
            this: this.clone(),
            root,
            renderer: Rc::new(TestRenderer),
            focus_manager: FocusManager::new(),
            layout_manager: RefCell::new(None),
            hit_tester: RefCell::new(None),
            platform_settings: RefCell::new(None),
        });
        let as_layout_root: Rc<dyn ILayoutRoot> = source.clone();
        let layout_manager: Rc<dyn ILayoutManager> = LayoutManager::new(Rc::downgrade(&as_layout_root));
        *source.layout_manager.borrow_mut() = Some(layout_manager);
        source
    }

    fn root(&self) -> Ref<TestRoot> {
        self.root.upgrade().expect("the test root has been dropped")
    }
}

impl IPresentationSource for TestSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        self.root.upgrade().map(Ref::upcast)
    }
    fn render_scaling(&self) -> f64 {
        1.0
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().unwrap()
    }
    fn client_size(&self) -> Size {
        self.root.upgrade().map_or(Size::new(1000.0, 1000.0), |root| root.client_size())
    }
    fn platform_settings(&self) -> Option<Rc<dyn ferroui_base::platform::IPlatformSettings>> {
        self.platform_settings.borrow().clone()
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        match self.hit_tester.borrow().clone() {
            Some(hit_tester) => hit_tester,
            None => Rc::new(ManagedHitTester::new()),
        }
    }
    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }

    fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
        Some(PixelPoint::from_point(point, 1.0))
    }

    fn point_to_client(&self, point: PixelPoint) -> Option<Point> {
        Some(point.to_point(1.0))
    }
}

impl IInputRoot for TestSource {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        Some(self.focus_manager.clone())
    }
    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}
    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}
    fn root_element(&self) -> Ref<InputElement> {
        self.root().upcast()
    }
    fn focus_root(&self) -> Ref<InputElement> {
        self.root().upcast()
    }
    fn pointer_over_invalidated(&self) {}
}

impl ILayoutRoot for TestSource {
    fn layout_scaling(&self) -> f64 {
        self.root.upgrade().map_or(1.0, |root| root.layout_scaling())
    }
    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.borrow().clone().unwrap()
    }
    fn root_visual(&self) -> Ref<Layoutable> {
        self.root().upcast()
    }
}

/// A decorator that is the root of a logical and visual tree, with its own
/// layout manager.
#[repr(C)]
pub struct TestRoot {
    base: Decorator,
    client_size: Cell<Size>,
    layout_scaling: Cell<f64>,
    source: RefCell<Option<Rc<TestSource>>>,
}

ferro_class!(TestRoot: Decorator);
ferro_impl_classes!(TestRoot: VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for TestRoot {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let source = TestSource::new(this.to_ref().downgrade());
        *this.source.borrow_mut() = Some(source.clone());
        source.focus_manager.set_content_root(Some(this.to_ref().upcast()));
        this.set_is_visible(true);
        KeyboardNavigation::set_tab_navigation(this, KeyboardNavigationMode::Cycle);
        this.set_presentation_source_for_root_visual(Some(source));
    }
}

/// A layout pass queued on the dispatcher must not outlive the root: the
/// layout manager is disposed with it.
impl Drop for TestRoot {
    fn drop(&mut self) {
        if let Some(source) = self.source.borrow().as_ref() {
            source.layout_manager().dispose();
        }
    }
}

impl StyledElementImpl for TestRoot {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl LayoutableImpl for TestRoot {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        Self::parent_measure_override(this, this.client_size())
    }
}

impl TestRoot {
    /// Field initialisation, for classes that derive from the test root.
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            client_size: Cell::new(Size::new(1000.0, 1000.0)),
            layout_scaling: Cell::new(1.0),
            source: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn with_child(child: impl IntoRef<Control>) -> Ref<Self> {
        let root = Self::new();
        root.set_child(child.into_ref());
        root
    }

    pub fn client_size(&self) -> Size {
        self.client_size.get()
    }

    pub fn set_client_size(&self, value: Size) {
        self.client_size.set(value)
    }

    pub fn layout_scaling(&self) -> f64 {
        self.layout_scaling.get()
    }

    pub fn set_layout_scaling(&self, value: f64) {
        self.layout_scaling.set(value)
    }

    pub fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.source.borrow().as_ref().unwrap().layout_manager()
    }

    /// Replaces the layout manager of the root (the settable `LayoutManager`
    /// of the upstream test root).
    pub fn set_layout_manager(&self, value: Rc<dyn ILayoutManager>) {
        *self.source.borrow().as_ref().unwrap().layout_manager.borrow_mut() = Some(value);
    }

    /// Sets the platform settings of the root (none by default): the
    /// gestures of the base crate read the hold duration and the tap size
    /// from them.
    pub fn set_platform_settings(&self, value: Option<Rc<dyn ferroui_base::platform::IPlatformSettings>>) {
        *self.source.borrow().as_ref().unwrap().platform_settings.borrow_mut() = value;
    }

    pub fn execute_initial_layout_pass(&self) {
        self.layout_manager().execute_initial_layout_pass();
    }

    /// The focus manager of the root.
    pub fn focus_manager(&self) -> Rc<FocusManager> {
        self.source.borrow().as_ref().unwrap().focus_manager.clone()
    }

    /// Replaces the hit tester of the root; `None` restores the default one,
    /// which hit tests the visuals of the tree.
    pub fn set_hit_tester(&self, value: Option<Rc<dyn IHitTester>>) {
        *self.source.borrow().as_ref().unwrap().hit_tester.borrow_mut() = value;
    }

}

/// A hit tester that finds nothing, as a renderer that does not hit test.
pub struct NullHitTester;

impl IHitTester for NullHitTester {
    fn hit_test(&self, _p: Point, _root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        Vec::new()
    }

    fn hit_test_first(
        &self,
        _p: Point,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<Ref<Visual>> {
        None
    }

    fn hit_test_geometry(
        &self,
        _geometry: &ferroui_base::media::Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<ferroui_base::media::GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &ferroui_base::media::Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<ferroui_base::media::GeometryHitTestResult> {
        None
    }
}

/// The text control created by the default data templates.
///
/// Kept under the name of the stand-in the tests used before the text
/// controls existed.
pub type TestTextBlock = crate::TextBlock;

/// The string held by an untyped value, if it holds one.
pub fn string_of(value: &BoxedValue) -> Option<String> {
    let value: &dyn AnyValue = &**value;
    value.downcast_ref::<String>().cloned()
}

/// An untyped string value.
pub fn boxed_str(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

/// Formerly installed the stand-in text control of the default data
/// templates; they create real text blocks now. Kept so that tests written
/// against the stand-in keep compiling.
pub fn use_test_text_block() {}
