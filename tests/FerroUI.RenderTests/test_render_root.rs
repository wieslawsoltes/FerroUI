//! Port of upstream's `TestRenderRoot.cs` of the render tests: a decorator
//! that is the root of a visual tree rendered by a compositing renderer.
//!
//! Upstream's class implements the contracts of a root itself
//! (`IPresentationSource`, `IInputRoot`, `ILayoutRoot`); a class of the
//! port cannot be a contract handle, so the contracts are implemented by
//! the source object the root owns.

use ferroui_base::input::{FocusManager, IInputRoot, InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl};
use ferroui_base::media::{Geometry, GeometryHitTestResult};
use ferroui_base::rendering::composition::CompositingRenderer;
use ferroui_base::rendering::{IHitTester, IPresentationSource, IRenderer};
use ferroui_base::*;
use ferroui_controls::{Control, ControlImpl, Decorator};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[repr(C)]
pub struct TestRenderRoot {
    base: Decorator,
    source: RefCell<Option<Rc<TestRenderSource>>>,
}

ferro_class!(TestRenderRoot: Decorator);
ferro_impl_classes!(TestRenderRoot: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl StyledElementImpl for TestRenderRoot {
    // `ILogicalRoot`.
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

struct NullHitTester;

impl IHitTester for NullHitTester {
    fn hit_test(&self, _p: Point, _root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        Vec::new()
    }

    fn hit_test_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first(&self, _p: Point, _root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Option<Ref<Visual>> {
        None
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

/// The contracts of the root.
struct TestRenderSource {
    this: Weak<TestRenderSource>,
    root: WeakRef<TestRenderRoot>,
    render_scaling: f64,
    client_size: Cell<Size>,
    renderer: RefCell<Option<Rc<CompositingRenderer>>>,
    layout_manager: RefCell<Option<Rc<LayoutManager>>>,
}

impl TestRenderSource {
    fn root(&self) -> Ref<TestRenderRoot> {
        self.root.upgrade().expect("the root is alive")
    }
}

impl IPresentationSource for TestRenderSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        self.root.upgrade().map(Ref::upcast)
    }

    fn render_scaling(&self) -> f64 {
        self.render_scaling
    }

    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.borrow().clone().expect("the root was initialized")
    }

    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().unwrap()
    }

    fn client_size(&self) -> Size {
        self.client_size.get()
    }

    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        Rc::new(NullHitTester)
    }

    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }

    fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
        Some(PixelPoint::from_point(point, self.render_scaling))
    }

    fn point_to_client(&self, point: PixelPoint) -> Option<Point> {
        Some(point.to_point(self.render_scaling))
    }
}

impl IInputRoot for TestRenderSource {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        None
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

impl ILayoutRoot for TestRenderSource {
    fn layout_scaling(&self) -> f64 {
        1.0
    }

    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.borrow().clone().unwrap()
    }

    fn root_visual(&self) -> Ref<Layoutable> {
        self.root().upcast()
    }
}

impl TestRenderRoot {
    pub fn new(scaling: f64) -> Ref<Self> {
        let root = instantiate(Self { base: Decorator::construct(), source: RefCell::new(None) });
        let source = Rc::new_cyclic(|this: &Weak<TestRenderSource>| TestRenderSource {
            this: this.clone(),
            root: root.downgrade(),
            render_scaling: scaling,
            client_size: Cell::new(Size::default()),
            renderer: RefCell::new(None),
            layout_manager: RefCell::new(None),
        });
        let as_layout_root: Rc<dyn ILayoutRoot> = source.clone();
        *source.layout_manager.borrow_mut() = Some(LayoutManager::new(Rc::downgrade(&as_layout_root)));
        *root.source.borrow_mut() = Some(source);
        root
    }

    /// The root as the presentation source a renderer is created for.
    pub fn source(&self) -> Rc<dyn IPresentationSource> {
        self.source.borrow().clone().expect("the source is created with the root")
    }

    pub fn client_size(&self) -> Size {
        self.source.borrow().as_ref().unwrap().client_size.get()
    }

    pub fn renderer(&self) -> Rc<CompositingRenderer> {
        self.source.borrow().as_ref().unwrap().renderer.borrow().clone().expect("the root was initialized")
    }

    pub fn initialize(&self, renderer: &Rc<CompositingRenderer>, child: &Ref<Control>) {
        let source = self.source.borrow().clone().unwrap();
        *source.renderer.borrow_mut() = Some(renderer.clone());
        self.set_presentation_source_for_root_visual(Some(source.clone() as Rc<dyn IPresentationSource>));
        renderer.set_root(Some(self.to_ref().upcast()));
        self.set_child(Some(child.clone()));
        self.set_width(child.width());
        self.set_height(child.height());
        source.client_size.set(Size::new(self.width(), self.height()));
        self.measure(source.client_size.get());
        self.arrange(Rect::from_size(source.client_size.get()));
    }

    /// Releases what the root holds of its renderer. Upstream leaves this
    /// to the garbage collector.
    pub fn release(&self) {
        self.set_presentation_source_for_root_visual(None);
        if let Some(source) = self.source.borrow().as_ref() {
            source.renderer.borrow_mut().take();
        }
    }
}
