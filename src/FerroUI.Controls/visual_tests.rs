//! Port of `VisualTests.cs` (base unit tests), with `TestVisual.cs`. The
//! trees are built from decorators, canvases and stack panels under a test
//! root, so the tests live with the controls.
//!
//! The renderer mock of upstream (`Mock<IRenderer>`, `RendererMocks`) is
//! [`RecordingRenderer`], which records the visuals passed to `AddDirty` and
//! `RecalculateChildren`; a verified call is a recorded visual. An
//! `InvalidOperationException` is a panic.

use crate::test_support::{test_scope, TestRoot};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Canvas, Decorator, StackPanel};
use ferroui_base::media::ScaleTransform;
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::rendering::{IRenderer, RendererDiagnostics, SceneInvalidatedEventArgs};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectExtensions, FerroObjectImpl, Matrix, Point, Rect, Ref,
    Size, StyledElementImpl, Visual, VisualImpl,
};
use std::any::TypeId;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// Declares one test per data row of a parameterized test.
macro_rules! theory {
    ($func:ident: $($name:ident($($arg:expr),* $(,)?));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                $func($($arg),*)
            }
        )+
    };
}

// --- TestVisual.cs -----------------------------------------------------------------

#[repr(C)]
struct TestVisual {
    base: Visual,
}

ferro_class!(TestVisual: Visual);
ferro_impl_classes!(TestVisual: FerroObjectImpl, StyledElementImpl, VisualImpl);

impl TestVisual {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Visual::construct() })
    }

    fn add_child(&self, v: &Visual) {
        self.visual_children().add(v.to_ref());
    }

    fn add_children(&self, v: &[&Visual]) {
        self.visual_children().add_range(v.iter().map(|v| v.to_ref()));
    }

    fn remove_child(&self, v: &Visual) {
        self.visual_children().remove(&v.to_ref());
    }

    fn clear_children(&self) {
        self.visual_children().clear();
    }
}

// --- RendererMocks.cs --------------------------------------------------------------

/// `RendererMocks.CreateRenderer()`: a renderer whose members do nothing and
/// record the visuals they are called with.
#[derive(Default)]
struct RecordingRenderer {
    dirty: RefCell<Vec<Ref<Visual>>>,
    recalculated: RefCell<Vec<Ref<Visual>>>,
}

impl RecordingRenderer {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `renderer.Verify(x => x.AddDirty(visual))`.
    fn verify_add_dirty(&self, visual: &Visual) {
        assert!(self.dirty.borrow().iter().any(|v| *v == visual.to_ref()), "AddDirty was not called with the visual");
    }

    /// `renderer.Verify(x => x.RecalculateChildren(visual))`.
    fn verify_recalculate_children(&self, visual: &Visual) {
        assert!(
            self.recalculated.borrow().iter().any(|v| *v == visual.to_ref()),
            "RecalculateChildren was not called with the visual"
        );
    }

    /// `renderer.Invocations.Clear()`.
    fn clear_invocations(&self) {
        self.dirty.borrow_mut().clear();
        self.recalculated.borrow_mut().clear();
    }
}

impl IRenderer for RecordingRenderer {
    fn diagnostics(&self) -> Rc<RendererDiagnostics> {
        RendererDiagnostics::new()
    }
    fn scene_invalidated(&self, _handler: Rc<dyn Fn(&SceneInvalidatedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: TypeId) -> Option<ferroui_base::rendering::composition::RenderInterfaceFeature> {
        None
    }
    fn add_dirty(&self, visual: &Visual) {
        self.dirty.borrow_mut().push(visual.to_ref());
    }
    fn recalculate_children(&self, visual: &Visual) {
        self.recalculated.borrow_mut().push(visual.to_ref());
    }
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

fn v(visual: &Visual) -> Ref<Visual> {
    visual.to_ref()
}

// --- VisualTests.cs ----------------------------------------------------------------

#[test]
fn added_child_should_have_visual_parent_set() {
    let _scope = test_scope();
    let target = TestVisual::new();
    let child = Visual::new();

    target.add_child(&child);

    assert_eq!(Some(v(&target)), child.get_visual_parent());
}

#[test]
fn added_child_should_notify_visual_parent_changed() {
    let _scope = test_scope();
    let target = TestVisual::new();
    let child = TestVisual::new();
    let parents = Rc::new(RefCell::new(Vec::<Option<Ref<Visual>>>::new()));

    let recorded = parents.clone();
    let _subscription =
        child.get_observable(Visual::visual_parent_property()).subscribe_fn(move |x| recorded.borrow_mut().push(x));
    target.add_child(&child);
    target.remove_child(&child);

    assert_eq!(vec![None, Some(v(&target)), None], *parents.borrow());
}

#[test]
fn removed_child_should_have_visual_parent_cleared() {
    let _scope = test_scope();
    let target = TestVisual::new();
    let child = Visual::new();

    target.add_child(&child);
    target.remove_child(&child);

    assert_eq!(None, child.get_visual_parent());
}

#[test]
fn clearing_children_should_clear_visual_parent() {
    let _scope = test_scope();
    let children = [Visual::new(), Visual::new()];
    let target = TestVisual::new();

    target.add_children(&[&children[0], &children[1]]);
    target.clear_children();

    let result: Vec<_> = children.iter().map(|x| x.get_visual_parent()).collect();

    assert_eq!(vec![None, None], result);
}

#[test]
fn adding_children_should_fire_on_attached_to_visual_tree() {
    let _scope = test_scope();
    let child2 = Decorator::new();
    let child1 = Decorator::new();
    child1.set_child(child2.clone());
    let root = TestRoot::new();
    let called1 = Rc::new(RefCell::new(false));
    let called2 = Rc::new(RefCell::new(false));

    let (r, c) = (v(&root), called1.clone());
    let _h1 = child1.attached_to_visual_tree(move |e| {
        // TODO: Tests are running against TestRoot, so behavior DOES NOT match the actual TopLevel.
        assert_eq!(e.attachment_point(), Some(&r));
        assert_eq!(e.root_visual(), &r);
        *c.borrow_mut() = true;
    });

    let (r, c) = (v(&root), called2.clone());
    let _h2 = child2.attached_to_visual_tree(move |e| {
        assert_eq!(e.attachment_point(), Some(&r));
        assert_eq!(e.root_visual(), &r);
        *c.borrow_mut() = true;
    });

    root.set_child(child1.clone());

    assert!(*called1.borrow());
    assert!(*called2.borrow());
}

#[test]
fn removing_children_should_fire_on_detached_from_visual_tree() {
    let _scope = test_scope();
    let child2 = Decorator::new();
    let child1 = Decorator::new();
    child1.set_child(child2.clone());
    let root = TestRoot::new();
    let called1 = Rc::new(RefCell::new(false));
    let called2 = Rc::new(RefCell::new(false));

    root.set_child(child1.clone());

    let (r, c) = (v(&root), called1.clone());
    let _h1 = child1.detached_from_visual_tree(move |e| {
        // TODO: Tests are running against TestRoot, so behavior DOES NOT match the actual TopLevel.
        assert_eq!(e.attachment_point(), Some(&r));
        assert_eq!(e.root_visual(), &r);
        *c.borrow_mut() = true;
    });

    let (r, c) = (v(&root), called2.clone());
    let _h2 = child2.detached_from_visual_tree(move |e| {
        assert_eq!(e.attachment_point(), Some(&r));
        assert_eq!(e.root_visual(), &r);
        *c.borrow_mut() = true;
    });

    root.set_child(None);

    assert!(*called1.borrow());
    assert!(*called2.borrow());
}

#[test]
fn visual_children_can_be_added_during_attached_to_visual_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = TestVisual::new();
    let child1 = TestVisual::new();
    let child2 = TestVisual::new();

    parent.add_child(&child1);

    let (p, c2) = (parent.clone(), child2.clone());
    let _h = child1.attached_to_visual_tree(move |_| p.add_child(&c2));

    root.visual_children().add(v(&parent));

    assert_eq!(vec![v(&child1), v(&child2)], parent.visual_children().to_vec());
    assert!(child1.is_attached_to_visual_tree());
    assert!(child2.is_attached_to_visual_tree());
}

#[test]
fn visual_children_can_be_removed_during_attached_to_visual_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = TestVisual::new();
    let child1 = TestVisual::new();
    let child2 = TestVisual::new();

    parent.add_children(&[&child1, &child2]);

    let (p, c2) = (parent.clone(), child2.clone());
    let _h = child1.attached_to_visual_tree(move |_| p.remove_child(&c2));

    root.visual_children().add(v(&parent));

    assert_eq!(vec![v(&child1)], parent.visual_children().to_vec());
    assert!(child1.is_attached_to_visual_tree());
    assert!(!child2.is_attached_to_visual_tree());
}

#[test]
fn visual_children_can_be_added_during_detached_from_visual_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = TestVisual::new();
    let child1 = TestVisual::new();
    let child2 = TestVisual::new();

    parent.add_child(&child1);
    root.visual_children().add(v(&parent));

    let (p, c2) = (parent.clone(), child2.clone());
    let _h = child1.detached_from_visual_tree(move |_| p.add_child(&c2));

    root.visual_children().remove(&v(&parent));

    assert_eq!(vec![v(&child1), v(&child2)], parent.visual_children().to_vec());
    assert!(!child1.is_attached_to_visual_tree());
    assert!(!child2.is_attached_to_visual_tree());
}

#[test]
fn visual_children_can_be_removed_during_detached_from_visual_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = TestVisual::new();
    let child1 = TestVisual::new();
    let child2 = TestVisual::new();
    let child2_detached = Rc::new(RefCell::new(0));

    parent.add_children(&[&child1, &child2]);
    root.visual_children().add(v(&parent));

    let (p, c2) = (parent.clone(), child2.clone());
    let _h1 = child1.detached_from_visual_tree(move |_| p.remove_child(&c2));
    let counter = child2_detached.clone();
    let _h2 = child2.detached_from_visual_tree(move |_| *counter.borrow_mut() += 1);

    root.visual_children().remove(&v(&parent));

    assert_eq!(vec![v(&child1)], parent.visual_children().to_vec());
    assert!(!child1.is_attached_to_visual_tree());
    assert!(!child2.is_attached_to_visual_tree());
    assert_eq!(1, *child2_detached.borrow());
}

#[test]
fn root_should_return_self_as_visual_root() {
    let _scope = test_scope();
    let root = TestRoot::new();

    assert_eq!(Some(v(&root)), root.visual_root());
}

#[test]
fn descendants_should_return_visual_root() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let child1 = Decorator::new();
    let child2 = Decorator::new();

    root.set_child(child1.clone());
    child1.set_child(child2.clone());

    assert_eq!(Some(v(&root)), child1.visual_root());
    assert_eq!(Some(v(&root)), child2.visual_root());
}

#[test]
fn attaching_to_visual_tree_should_invalidate_visual() {
    let _scope = test_scope();
    let renderer = RecordingRenderer::new();
    let child = Decorator::new();
    let root = TestRoot::new();
    root.set_renderer(renderer.clone());

    root.set_child(child.clone());

    renderer.verify_add_dirty(&child);
}

#[test]
fn detaching_from_visual_tree_should_invalidate_visual() {
    let _scope = test_scope();
    let renderer = RecordingRenderer::new();
    let child = Decorator::new();
    let root = TestRoot::new();
    root.set_renderer(renderer.clone());

    root.set_child(child.clone());
    renderer.clear_invocations();
    root.set_child(None);

    renderer.verify_add_dirty(&child);
}

#[test]
fn adding_already_parented_control_should_throw() {
    let _scope = test_scope();
    let root1 = TestRoot::new();
    let root2 = TestRoot::new();
    let child = Canvas::new();

    root1.set_child(child.clone());

    assert!(catch_unwind(AssertUnwindSafe(|| root2.set_child(child.clone()))).is_err());
    assert!(root2.get_visual_children().is_empty());
}

#[test]
fn transform_to_visual_should_work() {
    let _scope = test_scope();
    let child = Decorator::new();
    child.set_width(100.0);
    child.set_height(100.0);
    let root = TestRoot::with_child(child.clone());
    root.set_width(400.0);
    root.set_height(400.0);

    root.measure(Size::INFINITY);
    root.arrange(Rect::from_position_size(Point::default(), root.desired_size()));

    let tr = child.transform_to_visual(&root);

    assert!(tr.is_some());

    let point = root.bounds().top_left() * tr.unwrap();

    //child is centered (400 - 100)/2
    assert_eq!(Point::new(150.0, 150.0), point);
}

#[test]
fn transform_to_visual_with_render_transform_should_work() {
    let _scope = test_scope();
    let child = Decorator::new();
    child.set_width(100.0);
    child.set_height(100.0);
    child.set_render_transform(Some(ScaleTransform::with_scale(2.0, 2.0).into()));
    let root = TestRoot::with_child(child.clone());
    root.set_width(400.0);
    root.set_height(400.0);

    root.measure(Size::INFINITY);
    root.arrange(Rect::from_position_size(Point::default(), root.desired_size()));

    let tr = child.transform_to_visual(&root);

    assert!(tr.is_some());

    let point = root.bounds().top_left() * tr.unwrap();

    //child is centered (400 - 100*2 scale)/2
    assert_eq!(Point::new(100.0, 100.0), point);
}

#[test]
fn transform_to_visual_with_non_invertible_render_transform_should_work() {
    let _scope = test_scope();
    let child = Decorator::new();
    child.set_width(100.0);
    child.set_height(100.0);
    child.set_render_transform(Some(ScaleTransform::with_scale(0.0, 0.0).into()));
    let root = TestRoot::with_child(child.clone());
    root.set_width(400.0);
    root.set_height(400.0);

    root.measure(Size::INFINITY);
    root.arrange(Rect::from_position_size(Point::default(), root.desired_size()));

    let tr: Option<Matrix> = root.transform_to_visual(&child);

    assert!(tr.is_none());
}

/// The tree of the z-index tests: a stack panel with two canvases.
fn z_index_tree() -> (Ref<TestRoot>, Ref<StackPanel>, Ref<Canvas>) {
    let canvas1 = Canvas::new();
    let stack_panel = StackPanel::new();
    stack_panel.children().add(canvas1.clone());
    stack_panel.children().add(Canvas::new());
    let root = TestRoot::with_child(stack_panel.clone());
    (root, stack_panel, canvas1)
}

#[test]
fn changing_z_index_should_invalidate_visual() {
    let _scope = test_scope();
    let renderer = RecordingRenderer::new();
    let (root, _stack_panel, canvas1) = z_index_tree();

    root.set_renderer(renderer.clone());
    canvas1.set_z_index(10);

    renderer.verify_add_dirty(&canvas1);
}

#[test]
fn changing_z_index_should_recalculate_parent_children() {
    let _scope = test_scope();
    let renderer = RecordingRenderer::new();
    let (root, stack_panel, canvas1) = z_index_tree();

    root.set_renderer(renderer.clone());
    canvas1.set_z_index(10);

    renderer.verify_recalculate_children(&stack_panel);
}

#[allow(clippy::too_many_arguments)]
fn is_effectively_visible_propagates_to_visual_children(
    assign_order: &[i32],
    root_v: bool,
    child1_v: bool,
    child2_v: bool,
    root_expected: bool,
    child1_expected: bool,
    child2_expected: bool,
    initial_set_to_false: bool,
) {
    let _app = UnitTestApplication::start(TestServices::new());
    let child2 = Decorator::new();
    let child1 = Decorator::new();
    child1.set_child(child2.clone());
    let root = TestRoot::with_child(child1.clone());

    assert!(child2.is_effectively_visible());

    if initial_set_to_false {
        root.set_is_visible(false);
        child1.set_is_visible(false);
        child2.set_is_visible(false);
    }

    for order in assign_order {
        match order {
            1 => root.set_is_visible(root_v),
            2 => child1.set_is_visible(child1_v),
            3 => child2.set_is_visible(child2_v),
            _ => {}
        }
    }

    assert_eq!(root_expected, root.is_effectively_visible());
    assert_eq!(child1_expected, child1.is_effectively_visible());
    assert_eq!(child2_expected, child2.is_effectively_visible());
}

theory!(is_effectively_visible_propagates_to_visual_children:
    is_effectively_visible_propagates_to_visual_children_1(&[1, 2, 3], true, true, true, true, true, true, false);
    is_effectively_visible_propagates_to_visual_children_2(&[3, 2, 1], true, true, true, true, true, true, false);
    is_effectively_visible_propagates_to_visual_children_3(&[1], false, true, true, false, false, false, false);
    is_effectively_visible_propagates_to_visual_children_4(&[2], true, false, true, true, false, false, false);
    is_effectively_visible_propagates_to_visual_children_5(&[3], true, true, false, true, true, false, false);
    is_effectively_visible_propagates_to_visual_children_6(&[3, 1], true, true, false, true, true, false, false);
    is_effectively_visible_propagates_to_visual_children_7(&[2, 3, 1], true, false, true, true, false, false, true);
    is_effectively_visible_propagates_to_visual_children_8(&[3, 1, 2], true, true, false, true, true, false, true);
    is_effectively_visible_propagates_to_visual_children_9(&[3, 2, 1], true, true, false, true, true, false, true));

#[test]
fn added_child_has_correct_is_effectively_visible() {
    let _app = UnitTestApplication::start(TestServices::new());
    let root = TestRoot::new();
    root.set_is_visible(false);
    let child = Decorator::new();

    root.set_child(child.clone());
    assert!(!child.is_effectively_visible());
}

#[test]
fn added_grandchild_has_correct_is_effectively_visible() {
    let _app = UnitTestApplication::start(TestServices::new());
    let child = Decorator::new();
    let grandchild = Decorator::new();
    let root = TestRoot::new();
    root.set_is_visible(false);
    root.set_child(child.clone());

    child.set_child(grandchild.clone());
    assert!(!grandchild.is_effectively_visible());
}

#[test]
fn removing_child_resets_is_effectively_visible() {
    let _app = UnitTestApplication::start(TestServices::new());
    let child = Decorator::new();
    let root = TestRoot::with_child(child.clone());
    root.set_is_visible(false);

    assert!(!child.is_effectively_visible());

    root.set_child(None);

    assert!(child.is_effectively_visible());
}

#[test]
fn removing_child_resets_is_effectively_visible_of_grandchild() {
    let _app = UnitTestApplication::start(TestServices::new());
    let grandchild = Decorator::new();
    let child = Decorator::new();
    child.set_child(grandchild.clone());
    let root = TestRoot::with_child(child.clone());
    root.set_is_visible(false);

    assert!(!child.is_effectively_visible());
    assert!(!grandchild.is_effectively_visible());

    root.set_child(None);

    assert!(child.is_effectively_visible());
    assert!(grandchild.is_effectively_visible());
}
