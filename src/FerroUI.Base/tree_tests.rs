//! Integration tests for the logical tree, the visual tree and layout.

use crate::layout::{
    HorizontalAlignment, ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl, VerticalAlignment,
};
use crate::media::MediaContext;
use crate::input::{FocusManager, IInputRoot, InputElement, InputElementImpl};
use crate::interactivity::InteractiveImpl;
use crate::rendering::{IHitTester, IPresentationSource, IRenderer, ManagedHitTester};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// An input element that is a logical root and reports a fixed content size.
#[repr(C)]
pub struct TestRoot {
    base: InputElement,
}

ferro_class!(TestRoot: InputElement);
ferro_impl_classes!(TestRoot: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl StyledElementImpl for TestRoot {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl TestRoot {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: InputElement::construct() })
    }
}

/// A leaf that wants a fixed size.
#[repr(C)]
pub struct Leaf {
    base: Layoutable,
    wanted: Cell<Size>,
    measured_with: Cell<Option<Size>>,
    arranged_with: Cell<Option<Size>>,
}

ferro_class!(Leaf: Layoutable);
ferro_impl_classes!(Leaf: FerroObjectImpl, StyledElementImpl, VisualImpl);

impl LayoutableImpl for Leaf {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measured_with.set(Some(available_size));
        this.wanted.get()
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arranged_with.set(Some(final_size));
        final_size
    }
}

impl Leaf {
    pub fn new(width: f64, height: f64) -> Ref<Self> {
        instantiate(Self {
            base: Layoutable::construct(),
            wanted: Cell::new(Size::new(width, height)),
            measured_with: Cell::new(None),
            arranged_with: Cell::new(None),
        })
    }
}

#[derive(Default)]
struct TestRenderer {
    dirty: Cell<u32>,
}

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<crate::rendering::RendererDiagnostics> {
        crate::rendering::RendererDiagnostics::new()
    }
    fn scene_invalidated(
        &self,
        _handler: Rc<dyn Fn(&crate::rendering::SceneInvalidatedEventArgs)>,
    ) -> Rc<dyn crate::reactive::IDisposable> {
        crate::reactive::Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: std::any::TypeId) -> Option<crate::rendering::composition::RenderInterfaceFeature> {
        None
    }
    fn add_dirty(&self, _visual: &Visual) {
        self.dirty.set(self.dirty.get() + 1);
    }
    fn recalculate_children(&self, _visual: &Visual) {}
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

pub(crate) struct TestSource {
    this: Weak<TestSource>,
    root: Ref<TestRoot>,
    renderer: Rc<TestRenderer>,
    layout_manager: RefCell<Option<Rc<LayoutManager>>>,
    scaling: f64,
}

impl TestSource {
    pub(crate) fn new(root: Ref<TestRoot>, scaling: f64) -> Rc<Self> {
        let source = Rc::new_cyclic(|this: &Weak<TestSource>| TestSource {
            this: this.clone(),
            root,
            renderer: Rc::new(TestRenderer::default()),
            layout_manager: RefCell::new(None),
            scaling,
        });
        let as_layout_root: Rc<dyn ILayoutRoot> = source.clone();
        *source.layout_manager.borrow_mut() = Some(LayoutManager::new(Rc::downgrade(&as_layout_root)));
        source
    }

    pub(crate) fn manager(&self) -> Rc<LayoutManager> {
        self.layout_manager.borrow().clone().unwrap()
    }
}

impl IPresentationSource for TestSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        Some(self.root.clone().upcast())
    }
    fn render_scaling(&self) -> f64 {
        self.scaling
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().unwrap()
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        Rc::new(ManagedHitTester::new())
    }
    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }
    fn client_size(&self) -> Size {
        Size::new(800.0, 600.0)
    }
}

impl IInputRoot for TestSource {
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
        self.root.clone().upcast()
    }
    fn focus_root(&self) -> Ref<InputElement> {
        self.root.clone().upcast()
    }
    fn pointer_over_invalidated(&self) {}
}

impl ILayoutRoot for TestSource {
    fn layout_scaling(&self) -> f64 {
        self.scaling
    }
    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.manager()
    }
    fn root_visual(&self) -> Ref<Layoutable> {
        self.root.clone().upcast()
    }
}

fn add_child(parent: &Layoutable, child: &Ref<Leaf>) {
    parent.logical_children().add(child.clone().upcast());
    parent.visual_children().add(child.clone().upcast());
}

#[test]
fn logical_tree_attachment_follows_parent() {
    let root = TestRoot::new();
    let child = Leaf::new(10.0, 10.0);
    let attached = Rc::new(Cell::new(0));
    let detached = Rc::new(Cell::new(0));
    let (a, d) = (attached.clone(), detached.clone());
    child.attached_to_logical_tree(move |_| a.set(a.get() + 1));
    child.detached_from_logical_tree(move |_| d.set(d.get() + 1));

    assert!(root.is_attached_to_logical_tree());
    assert!(!child.is_attached_to_logical_tree());

    root.logical_children().add(child.clone().upcast());
    assert!(child.is_attached_to_logical_tree());
    assert_eq!(child.parent().unwrap(), root);
    assert_eq!(child.inheritance_parent().unwrap(), root);
    assert_eq!(attached.get(), 1);

    root.logical_children().remove(&child.clone().upcast());
    assert!(!child.is_attached_to_logical_tree());
    assert!(child.parent().is_none());
    assert_eq!(detached.get(), 1);
}

#[test]
fn data_context_is_inherited_through_logical_tree() {
    let root = TestRoot::new();
    let child = Leaf::new(10.0, 10.0);
    root.logical_children().add(child.clone().upcast());
    let changed = Rc::new(Cell::new(0));
    let c = changed.clone();
    child.data_context_changed(move || c.set(c.get() + 1));

    let context: BoxedValue = Rc::new("context".to_string());
    root.set_data_context(Some(context.clone()));
    assert_eq!(child.data_context(), Some(context));
    assert_eq!(changed.get(), 1);
}

#[test]
fn visual_tree_attachment_and_effective_visibility() {
    // The layout manager verifies that it is called on the UI thread.
    let _dispatcher = crate::threading::Dispatcher::unit_test_scope();
    let root = TestRoot::new();
    let child = Leaf::new(10.0, 10.0);
    add_child(&root, &child);
    assert!(!child.is_attached_to_visual_tree());
    assert_eq!(child.visual_parent().unwrap(), root);

    let source = TestSource::new(root.clone(), 1.0);
    root.set_presentation_source_for_root_visual(Some(source.clone()));
    assert!(root.is_attached_to_visual_tree());
    assert!(child.is_attached_to_visual_tree());
    assert_eq!(child.visual_root().unwrap(), root);

    root.set_is_visible(false);
    assert!(!child.is_effectively_visible());
    root.set_is_visible(true);
    assert!(child.is_effectively_visible());

    root.visual_children().remove(&child.clone().upcast());
    assert!(!child.is_attached_to_visual_tree());
    assert!(child.visual_parent().is_none());

    root.set_presentation_source_for_root_visual(None);
    assert!(!root.is_attached_to_visual_tree());
}

#[test]
#[should_panic(expected = "already has a visual parent")]
fn adding_visual_with_parent_panics() {
    let a = TestRoot::new();
    let b = TestRoot::new();
    let child = Leaf::new(1.0, 1.0);
    a.visual_children().add(child.clone().upcast());
    b.visual_children().add(child.upcast());
}

#[test]
fn layout_pass_measures_and_arranges_tree() {
    // The layout manager verifies that it is called on the UI thread.
    let _dispatcher = crate::threading::Dispatcher::unit_test_scope();
    let root = TestRoot::new();
    root.set_width(200.0);
    root.set_height(100.0);
    let child = Leaf::new(50.0, 20.0);
    child.set_margin(Thickness::uniform(5.0));
    child.set_horizontal_alignment(HorizontalAlignment::Left);
    child.set_vertical_alignment(VerticalAlignment::Bottom);
    add_child(&root, &child);

    let source = TestSource::new(root.clone(), 1.0);
    root.set_presentation_source_for_root_visual(Some(source.clone()));
    source.manager().execute_initial_layout_pass();

    assert_eq!(root.desired_size(), Size::new(200.0, 100.0));
    assert_eq!(root.bounds(), Rect::new(0.0, 0.0, 200.0, 100.0));
    assert_eq!(child.measured_with.get(), Some(Size::new(190.0, 90.0)));
    assert_eq!(child.desired_size(), Size::new(60.0, 30.0));
    assert_eq!(child.bounds(), Rect::new(5.0, 75.0, 50.0, 20.0));
    assert!(child.is_measure_valid() && child.is_arrange_valid());
}

#[test]
fn invalidating_measure_queues_layout_pass_on_render() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let root = TestRoot::new();
    let child = Leaf::new(50.0, 20.0);
    child.set_horizontal_alignment(HorizontalAlignment::Left);
    child.set_vertical_alignment(VerticalAlignment::Top);
    add_child(&root, &child);
    let source = TestSource::new(root.clone(), 1.0);
    root.set_presentation_source_for_root_visual(Some(source.clone()));
    source.manager().execute_initial_layout_pass();
    assert_eq!(child.bounds(), Rect::new(0.0, 0.0, 50.0, 20.0));
    assert_eq!(root.bounds().size(), Size::new(50.0, 20.0));

    let updated = Rc::new(Cell::new(0));
    let u = updated.clone();
    child.layout_updated(move || u.set(u.get() + 1));

    child.set_width(80.0);
    assert!(!child.is_measure_valid());
    assert!(MediaContext::instance().is_render_scheduled());
    // Nothing happens until the frame runs.
    assert_eq!(child.bounds().width, 50.0);

    crate::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(!MediaContext::instance().is_render_scheduled());
    assert_eq!(child.bounds(), Rect::new(0.0, 0.0, 80.0, 20.0));
    assert_eq!(root.bounds().size(), Size::new(80.0, 20.0));
    assert_eq!(updated.get(), 1);
    assert!(source.renderer.dirty.get() > 0);
}

#[test]
fn layout_rounding_snaps_to_device_pixels() {
    // The layout manager verifies that it is called on the UI thread.
    let _dispatcher = crate::threading::Dispatcher::unit_test_scope();
    let root = TestRoot::new();
    let child = Leaf::new(10.3, 10.3);
    child.set_horizontal_alignment(HorizontalAlignment::Left);
    child.set_vertical_alignment(VerticalAlignment::Top);
    add_child(&root, &child);
    let source = TestSource::new(root.clone(), 2.0);
    root.set_presentation_source_for_root_visual(Some(source.clone()));
    source.manager().execute_initial_layout_pass();
    assert_eq!(child.bounds().size(), Size::new(10.5, 10.5));
}

#[test]
fn hidden_control_has_no_desired_size() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let root = TestRoot::new();
    let child = Leaf::new(50.0, 20.0);
    add_child(&root, &child);
    let source = TestSource::new(root.clone(), 1.0);
    root.set_presentation_source_for_root_visual(Some(source.clone()));
    source.manager().execute_initial_layout_pass();
    assert_eq!(root.desired_size(), Size::new(50.0, 20.0));

    child.set_is_visible(false);
    MediaContext::instance().render();
    assert_eq!(child.desired_size(), Size::default());
    assert_eq!(root.desired_size(), Size::default());
}
