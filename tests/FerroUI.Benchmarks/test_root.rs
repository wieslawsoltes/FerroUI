//! The root of the trees of the benchmarks: the port of the test root of
//! the upstream unit test library, which the benchmarks are built with. A
//! decorator that is the root of a logical and visual tree, with a layout
//! manager and a presentation source of its own.

use ferroui_base::input::InputElementImpl;
use ferroui_base::input::{FocusManager, IInputRoot, InputElement, KeyboardNavigation, KeyboardNavigationMode};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{Geometry, GeometryHitTestResult};
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::composition::RenderInterfaceFeature;
use ferroui_base::rendering::{
    IHitTester, IPresentationSource, IRenderer, RendererDiagnostics, SceneInvalidatedEventArgs,
};
use ferroui_base::styling::StyleHostRef;
use ferroui_base::*;
use ferroui_controls::{Application, Control, ControlImpl, Decorator};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The renderer of a root nothing else was given to: it does nothing.
#[derive(Default)]
pub struct TestRenderer;

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<RendererDiagnostics> {
        RendererDiagnostics::new()
    }
    fn scene_invalidated(&self, _handler: Rc<dyn Fn(&SceneInvalidatedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: TypeId) -> Option<RenderInterfaceFeature> {
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

/// A hit tester that finds nothing.
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
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        None
    }
}

/// The presentation source, the layout root and the input root of a
/// [`TestRoot`] (the contracts the upstream class implements itself).
pub struct TestSource {
    this: Weak<TestSource>,
    root: WeakRef<TestRoot>,
    renderer: RefCell<Rc<dyn IRenderer>>,
    hit_tester: RefCell<Rc<dyn IHitTester>>,
    focus_manager: Rc<FocusManager>,
    layout_manager: RefCell<Option<Rc<dyn ILayoutManager>>>,
}

impl TestSource {
    fn new(root: WeakRef<TestRoot>) -> Rc<Self> {
        let source = Rc::new_cyclic(|this: &Weak<TestSource>| TestSource {
            this: this.clone(),
            root,
            renderer: RefCell::new(Rc::new(TestRenderer)),
            hit_tester: RefCell::new(Rc::new(NullHitTester)),
            focus_manager: FocusManager::new(),
            layout_manager: RefCell::new(None),
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
        self.renderer.borrow().clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().expect("the source is alive")
    }
    fn client_size(&self) -> Size {
        self.root.upgrade().map_or(Size::new(1000.0, 1000.0), |root| root.client_size())
    }
    fn platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        FerroLocator::current().get_service::<dyn IPlatformSettings>()
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        self.hit_tester.borrow().clone()
    }
    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().expect("the source is alive")
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
        self.layout_manager.borrow().clone().expect("the source has a layout manager")
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
    styling_parent: RefCell<Option<StyleHostRef>>,
    source: RefCell<Option<Rc<TestSource>>>,
}

ferro_class!(TestRoot: Decorator);
ferro_impl_classes!(TestRoot: VisualImpl, InteractiveImpl, ControlImpl);

/// The root is a focus scope.
impl InputElementImpl for TestRoot {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }
}

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

    fn styling_parent(this: &Self) -> Option<StyleHostRef> {
        this.styling_parent.borrow().clone()
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
            styling_parent: RefCell::new(None),
            source: RefCell::new(None),
        }
    }

    /// The parameterless constructor.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The constructor with a child (no global styles).
    pub fn with_child(child: impl IntoRef<Control>) -> Ref<Self> {
        Self::with_global_styles(false, Some(child.into_ref()))
    }

    /// The constructor with both arguments: with `use_global_styles` the
    /// styling parent of the root is the application of the test.
    pub fn with_global_styles(use_global_styles: bool, child: Option<Ref<Control>>) -> Ref<Self> {
        let root = Self::new();
        if use_global_styles {
            let application = Application::current().map(|application| StyleHostRef::Other(application.as_style_host()));
            root.set_styling_parent(application);
        }
        if let Some(child) = child {
            root.set_child(child);
        }
        root
    }

    fn source(&self) -> Rc<TestSource> {
        self.source.borrow().clone().expect("the test root has a presentation source")
    }

    /// The renderer of the root; one that does nothing unless one is set.
    pub fn renderer(&self) -> Rc<dyn IRenderer> {
        self.source().renderer.borrow().clone()
    }

    pub fn set_renderer(&self, value: Rc<dyn IRenderer>) {
        *self.source().renderer.borrow_mut() = value;
    }

    /// Replaces the hit tester of the root, which finds nothing by default.
    pub fn set_hit_tester(&self, value: Rc<dyn IHitTester>) {
        *self.source().hit_tester.borrow_mut() = value;
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

    /// The styling parent of the root; none unless one is set.
    pub fn set_styling_parent(&self, value: Option<StyleHostRef>) {
        *self.styling_parent.borrow_mut() = value;
    }

    pub fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.source().layout_manager()
    }

    pub fn set_layout_manager(&self, value: Rc<dyn ILayoutManager>) {
        *self.source().layout_manager.borrow_mut() = Some(value);
    }

    pub fn execute_initial_layout_pass(&self) {
        self.layout_manager().execute_initial_layout_pass();
    }

    /// The focus manager of the root.
    pub fn focus_manager(&self) -> Rc<FocusManager> {
        self.source().focus_manager.clone()
    }
}
