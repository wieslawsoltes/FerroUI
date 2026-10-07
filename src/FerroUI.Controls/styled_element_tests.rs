//! Port of `StyledElementTests.cs` (base unit tests). The trees are built
//! from decorators, borders, canvases, panels and stack panels under a test
//! root, so the tests live with the controls.
//!
//! An `InvalidOperationException` is a panic. The settable `StylingParent`
//! of the upstream test root is [`StylingRoot`], a test root whose styling
//! parent can be set. `Mock.Of<IGlobalStyles>()` is [`MockGlobalStyles`], a
//! global style host whose members return defaults, and the mocked
//! `IResourceDictionary` is [`RecordingResourceDictionary`], which records
//! the owners it is added to. `Skip(1)` on an observable is a flag that
//! drops the first value.

use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{
    Border, Canvas, ContentControl, Control, ControlImpl, Decorator, Panel, PanelImpl, StackPanel, StackPanelImpl,
    TextBlock,
};
use ferroui_base::controls::{
    IResourceHost, IResourceNode, ResourceDictionary, ResourceHostRef, ResourceKey, ResourceProviderImpl,
    ResourceProviderImplExt, ResourceValue, ResourcesChangedEventArgs,
};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::styling::{
    GlobalStylesHandler, IGlobalStyles, IStyle, IStyleHost, Selectors, Setter, Style, StyleHostRef, Styles,
    ThemeVariant,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    Rect, Ref, Size, StyledElement, StyledElementImpl, StyledElementImplExt, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

// --- test doubles --------------------------------------------------------------------

/// `Mock.Of<IGlobalStyles>()`: a global style host whose members return
/// their defaults.
struct MockGlobalStyles {
    styles: Ref<Styles>,
}

impl MockGlobalStyles {
    fn new() -> Rc<Self> {
        Rc::new(Self { styles: Styles::new() })
    }
}

impl IResourceNode for MockGlobalStyles {
    fn has_resources(&self) -> bool {
        false
    }

    fn try_get_resource(&self, _key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        None
    }
}

impl IResourceHost for MockGlobalStyles {
    fn resources_changed(&self, _handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn notify_hosted_resources_changed(&self, _e: ResourcesChangedEventArgs) {}

    fn as_style_host(&self) -> Option<&dyn IStyleHost> {
        Some(self)
    }
}

impl IStyleHost for MockGlobalStyles {
    fn is_styles_initialized(&self) -> bool {
        false
    }

    fn styles(&self) -> Ref<Styles> {
        self.styles.clone()
    }

    fn styling_parent(&self) -> Option<StyleHostRef> {
        None
    }

    fn styles_added(&self, _styles: &[Rc<dyn IStyle>]) {}

    fn styles_removed(&self, _styles: &[Rc<dyn IStyle>]) {}
}

impl IGlobalStyles for MockGlobalStyles {
    fn global_styles_added(&self, _handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn global_styles_removed(&self, _handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

/// The test root of upstream with its settable `StylingParent`.
// Deviation (DEVIATIONS.md, Tests and test support): `TestRoot` of this crate has no settable styling parent.
#[repr(C)]
pub(crate) struct StylingRoot {
    base: TestRoot,
    styling_parent: RefCell<Option<StyleHostRef>>,
}

ferro_class!(StylingRoot: TestRoot);
ferro_impl_classes!(
    StylingRoot: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl StyledElementImpl for StylingRoot {
    fn is_logical_root(this: &Self) -> bool {
        Self::parent_is_logical_root(this)
    }

    fn styling_parent(this: &Self) -> Option<StyleHostRef> {
        this.styling_parent.borrow().clone()
    }
}

impl StylingRoot {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: TestRoot::construct(), styling_parent: RefCell::new(None) })
    }

    pub(crate) fn set_styling_parent(&self, value: Option<StyleHostRef>) {
        *self.styling_parent.borrow_mut() = value;
    }
}

/// `new Mock<IResourceDictionary>()`: a resource dictionary that records the
/// owners it is added to (`Verify(x => x.AddOwner(target))`).
#[repr(C)]
struct RecordingResourceDictionary {
    base: ResourceDictionary,
    added_owners: RefCell<Vec<ResourceHostRef>>,
}

ferro_class!(RecordingResourceDictionary: ResourceDictionary);
ferro_impl_classes!(RecordingResourceDictionary: FerroObjectImpl);

impl ResourceProviderImpl for RecordingResourceDictionary {
    fn has_resources(this: &Self) -> bool {
        Self::parent_has_resources(this)
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        Self::parent_try_get_resource(this, key, theme)
    }

    fn on_add_owner(this: &Self, owner: &ResourceHostRef) {
        this.added_owners.borrow_mut().push(owner.clone());
        Self::parent_on_add_owner(this, owner);
    }

    fn on_remove_owner(this: &Self, owner: &ResourceHostRef) {
        Self::parent_on_remove_owner(this, owner);
    }
}

impl RecordingResourceDictionary {
    fn new() -> Ref<Self> {
        instantiate(Self { base: ResourceDictionary::construct(), added_owners: RefCell::new(Vec::new()) })
    }
}

type SenderHandler = Rc<dyn Fn(&StyledElement)>;

/// `IDataContextEvents`.
trait IDataContextEvents {
    fn data_context_begin_update(&self, handler: impl Fn(&StyledElement) + 'static);
    fn data_context_end_update(&self, handler: impl Fn(&StyledElement) + 'static);
}

fn raise(handlers: &RefCell<Vec<SenderHandler>>, sender: &StyledElement) {
    let handlers = handlers.borrow().clone();
    for handler in handlers {
        handler(sender);
    }
}

#[repr(C)]
struct TestControl {
    base: Decorator,
    data_context_begin_update: RefCell<Vec<SenderHandler>>,
    data_context_end_update: RefCell<Vec<SenderHandler>>,
}

ferro_class!(TestControl: Decorator);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl StyledElementImpl for TestControl {
    fn on_data_context_begin_update(this: &Self) {
        raise(&this.data_context_begin_update, this);
        Self::parent_on_data_context_begin_update(this);
    }

    fn on_data_context_end_update(this: &Self) {
        raise(&this.data_context_end_update, this);
        Self::parent_on_data_context_end_update(this);
    }
}

impl TestControl {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Decorator::construct(),
            data_context_begin_update: RefCell::new(Vec::new()),
            data_context_end_update: RefCell::new(Vec::new()),
        })
    }

    fn named(name: &str) -> Ref<Self> {
        let result = Self::new();
        result.set_name(Some(name.to_string()));
        result
    }
}

impl IDataContextEvents for TestControl {
    fn data_context_begin_update(&self, handler: impl Fn(&StyledElement) + 'static) {
        self.data_context_begin_update.borrow_mut().push(Rc::new(handler));
    }

    fn data_context_end_update(&self, handler: impl Fn(&StyledElement) + 'static) {
        self.data_context_end_update.borrow_mut().push(Rc::new(handler));
    }
}

#[repr(C)]
struct TestStackPanel {
    base: StackPanel,
    data_context_begin_update: RefCell<Vec<SenderHandler>>,
    data_context_end_update: RefCell<Vec<SenderHandler>>,
}

ferro_class!(TestStackPanel: StackPanel);
ferro_impl_classes!(
    TestStackPanel: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    StackPanelImpl
);

impl StyledElementImpl for TestStackPanel {
    fn on_data_context_begin_update(this: &Self) {
        raise(&this.data_context_begin_update, this);
        Self::parent_on_data_context_begin_update(this);
    }

    fn on_data_context_end_update(this: &Self) {
        raise(&this.data_context_end_update, this);
        Self::parent_on_data_context_end_update(this);
    }
}

impl TestStackPanel {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            data_context_begin_update: RefCell::new(Vec::new()),
            data_context_end_update: RefCell::new(Vec::new()),
        })
    }
}

impl IDataContextEvents for TestStackPanel {
    fn data_context_begin_update(&self, handler: impl Fn(&StyledElement) + 'static) {
        self.data_context_begin_update.borrow_mut().push(Rc::new(handler));
    }

    fn data_context_end_update(&self, handler: impl Fn(&StyledElement) + 'static) {
        self.data_context_end_update.borrow_mut().push(Rc::new(handler));
    }
}

/// Subscribes the three data context notifications of `c` (the
/// `IDataContextEvents` cast of upstream), recording them in `called`.
fn record_data_context_events(c: &Ref<StyledElement>, called: &Rc<RefCell<Vec<String>>>) {
    fn name(s: &StyledElement) -> String {
        s.name().unwrap_or_default()
    }

    let (begin, end) = (called.clone(), called.clone());
    if let Some(c) = c.clone().cast::<TestControl>() {
        c.data_context_begin_update(move |s| begin.borrow_mut().push(format!("begin {}", name(s))));
        c.data_context_end_update(move |s| end.borrow_mut().push(format!("end {}", name(s))));
    } else if let Some(c) = c.clone().cast::<TestStackPanel>() {
        c.data_context_begin_update(move |s| begin.borrow_mut().push(format!("begin {}", name(s))));
        c.data_context_end_update(move |s| end.borrow_mut().push(format!("end {}", name(s))));
    } else {
        panic!("not an IDataContextEvents");
    }

    let changed = called.clone();
    let sender = c.downgrade();
    c.data_context_changed(move || {
        let s = sender.upgrade().unwrap();
        changed.borrow_mut().push(format!("changed {}", name(&s)));
    });
}

fn red() -> Option<Rc<dyn IBrush>> {
    Some(Brushes::red())
}

fn tag_style() -> Ref<Style> {
    Style::with_setters(Selectors::is::<Control>(), [Setter::new(Control::tag_property(), boxed_str("foo"))])
}

fn tag_of(control: &Control) -> Option<String> {
    control.tag().as_ref().and_then(string_of)
}

// --- tests ----------------------------------------------------------------------------

#[test]
fn classes_should_initially_be_empty() {
    let _scope = test_scope();
    let target = StyledElement::new();

    assert_eq!(target.classes().count(), 0);
}

#[test]
fn setting_parent_should_also_set_inheritance_parent() {
    let _scope = test_scope();
    let parent = Decorator::new();
    let target = TestControl::new();

    parent.set_child(target.clone());

    assert_eq!(target.parent(), Some(parent.clone().upcast()));
    assert_eq!(target.inheritance_parent(), Some(parent.upcast()));
}

#[test]
fn inheritance_parent_should_be_cleared_when_removed_from_parent() {
    let _scope = test_scope();
    let parent = Decorator::new();
    let target = TestControl::new();

    parent.set_child(target.clone());
    parent.set_child(None);

    assert!(target.inheritance_parent().is_none());
}

#[test]
fn adding_element_with_null_parent_to_logical_tree_should_throw() {
    let _scope = test_scope();
    let target = Border::new();
    let visual_parent = Panel::new();
    let logical_parent = Panel::new();
    let root = TestRoot::new();

    // Set the logical parent...
    target.set_parent(logical_parent.clone());

    // ...so that when it's added to `visualParent`, the parent won't be set again.
    visual_parent.children().add(target.clone());

    // Clear the logical parent. It's now a logical child of `visualParent` but doesn't have
    // a logical parent itself.
    target.set_parent(None);

    // In this case, attaching the control to a logical tree should throw.
    logical_parent.children().add(visual_parent.clone());
    assert!(catch_unwind(AssertUnwindSafe(|| root.set_child(logical_parent.clone()))).is_err());
}

#[test]
fn attached_to_logical_tree_should_be_called_when_added_to_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = Border::new();
    let child = Border::new();
    let grandchild = Border::new();
    let parent_raised = Rc::new(Cell::new(false));
    let child_raised = Rc::new(Cell::new(false));
    let grandchild_raised = Rc::new(Cell::new(false));

    let r = parent_raised.clone();
    parent.attached_to_logical_tree(move |_| r.set(true));
    let r = child_raised.clone();
    child.attached_to_logical_tree(move |_| r.set(true));
    let r = grandchild_raised.clone();
    grandchild.attached_to_logical_tree(move |_| r.set(true));

    parent.set_child(child.clone());
    child.set_child(grandchild.clone());

    assert!(!parent_raised.get());
    assert!(!child_raised.get());
    assert!(!grandchild_raised.get());

    root.set_child(parent.clone());

    assert!(parent_raised.get());
    assert!(child_raised.get());
    assert!(grandchild_raised.get());
}

#[test]
fn attached_to_logical_tree_should_be_called_before_parent_change_signalled() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let child = Border::new();
    let raised = Rc::new(RefCell::new(Vec::<String>::new()));

    let (r, c, expected_parent) = (raised.clone(), child.downgrade(), root.downgrade());
    child.attached_to_logical_tree(move |_| {
        assert_eq!(
            c.upgrade().unwrap().parent(),
            Some(expected_parent.upgrade().unwrap().upcast::<StyledElement>())
        );
        r.borrow_mut().push("attached".to_string());
    });

    let r = raised.clone();
    let skipped = Cell::new(false);
    let child_object: &FerroObject = &child;
    FerroObjectExtensions::get_observable(child_object, StyledElement::parent_property()).subscribe_fn(move |_| {
        // Skip(1)
        if skipped.replace(true) {
            r.borrow_mut().push("parent".to_string());
        }
    });

    root.set_child(child.clone());

    assert_eq!(vec!["attached", "parent"], *raised.borrow());
}

#[test]
fn attached_to_logical_tree_should_not_be_called_with_global_styles_as_root() {
    let _scope = test_scope();
    let global_styles = MockGlobalStyles::new();
    // Deviation (DEVIATIONS.md, Tests and test support): the test root of this crate has no settable
    // styling parent; a local derived root supplies it.
    let root = StylingRoot::new();
    root.set_styling_parent(Some(StyleHostRef::Other(global_styles)));
    let child = Border::new();
    let raised = Rc::new(Cell::new(false));

    let (r, expected_root) = (raised.clone(), root.downgrade());
    child.attached_to_logical_tree(move |e| {
        assert_eq!(*e.root(), expected_root.upgrade().unwrap().upcast::<StyledElement>());
        r.set(true);
    });

    root.set_child(child.clone());

    assert!(raised.get());
}

#[test]
fn attached_to_logical_tree_should_have_source_set() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let canvas = Canvas::new();
    let border = Border::new();
    border.set_child(canvas.clone());
    let raised = Rc::new(Cell::new(0));

    let attached = {
        let (border, raised) = (border.downgrade(), raised.clone());
        move |e: &LogicalTreeAttachmentEventArgs| {
            assert_eq!(*e.source(), border.upgrade().unwrap().upcast::<StyledElement>());
            raised.set(raised.get() + 1);
        }
    };

    border.attached_to_logical_tree(attached.clone());
    canvas.attached_to_logical_tree(attached);

    root.set_child(border.clone());

    assert_eq!(2, raised.get());
}

#[test]
fn attached_to_logical_tree_should_have_parent_set() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let canvas = Canvas::new();
    let border = Border::new();
    border.set_child(canvas.clone());
    let raised = Rc::new(Cell::new(0));

    let attached = {
        let (root, raised) = (root.downgrade(), raised.clone());
        move |e: &LogicalTreeAttachmentEventArgs| {
            assert_eq!(e.parent(), Some(&root.upgrade().unwrap().upcast::<StyledElement>()));
            raised.set(raised.get() + 1);
        }
    };

    border.attached_to_logical_tree(attached.clone());
    canvas.attached_to_logical_tree(attached);

    root.set_child(border.clone());

    assert_eq!(2, raised.get());
}

#[test]
fn detached_from_logical_tree_should_be_called_when_removed_from_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = Border::new();
    let child = Border::new();
    let grandchild = Border::new();
    let parent_raised = Rc::new(Cell::new(false));
    let child_raised = Rc::new(Cell::new(false));
    let grandchild_raised = Rc::new(Cell::new(false));

    parent.set_child(child.clone());
    child.set_child(grandchild.clone());
    root.set_child(parent.clone());

    let r = parent_raised.clone();
    parent.detached_from_logical_tree(move |_| r.set(true));
    let r = child_raised.clone();
    child.detached_from_logical_tree(move |_| r.set(true));
    let r = grandchild_raised.clone();
    grandchild.detached_from_logical_tree(move |_| r.set(true));

    root.set_child(None);

    assert!(parent_raised.get());
    assert!(child_raised.get());
    assert!(grandchild_raised.get());
}

#[test]
fn detached_from_logical_tree_should_not_be_called_with_global_styles_as_root() {
    let _scope = test_scope();
    let global_styles = MockGlobalStyles::new();
    // Deviation (DEVIATIONS.md, Tests and test support): the test root of this crate has no settable
    // styling parent; a local derived root supplies it.
    let root = StylingRoot::new();
    root.set_styling_parent(Some(StyleHostRef::Other(global_styles)));
    let child = Border::new();
    let raised = Rc::new(Cell::new(false));

    let (r, expected_root) = (raised.clone(), root.downgrade());
    child.detached_from_logical_tree(move |e| {
        assert_eq!(*e.root(), expected_root.upgrade().unwrap().upcast::<StyledElement>());
        r.set(true);
    });

    root.set_child(child.clone());
    root.set_child(None);

    assert!(raised.get());
}

#[test]
fn logical_children_can_be_added_during_attached_to_logical_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = StyledElement::new();
    let child1 = StyledElement::new();
    let child2 = StyledElement::new();

    parent.logical_children().add(child1.clone());

    let (p, c2) = (parent.downgrade(), child2.clone());
    child1.attached_to_logical_tree(move |_| p.upgrade().unwrap().logical_children().add(c2.clone()));

    root.logical_children().add(parent.clone());

    assert_eq!(vec![child1.clone(), child2.clone()], parent.logical_children().to_vec());
    assert!(child1.is_attached_to_logical_tree());
    assert!(child2.is_attached_to_logical_tree());
}

#[test]
fn logical_children_can_be_removed_during_attached_to_logical_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = StyledElement::new();
    let child1 = StyledElement::new();
    let child2 = StyledElement::new();

    parent.logical_children().add_range([child1.clone(), child2.clone()]);

    let (p, c2) = (parent.downgrade(), child2.clone());
    child1.attached_to_logical_tree(move |_| {
        p.upgrade().unwrap().logical_children().remove(&c2);
    });

    root.logical_children().add(parent.clone());

    assert_eq!(vec![child1.clone()], parent.logical_children().to_vec());
    assert!(child1.is_attached_to_logical_tree());
    assert!(!child2.is_attached_to_logical_tree());
}

#[test]
fn logical_children_can_be_added_during_detached_from_logical_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = StyledElement::new();
    let child1 = StyledElement::new();
    let child2 = StyledElement::new();

    parent.logical_children().add(child1.clone());
    root.logical_children().add(parent.clone());

    let (p, c2) = (parent.downgrade(), child2.clone());
    child1.detached_from_logical_tree(move |_| p.upgrade().unwrap().logical_children().add(c2.clone()));

    root.logical_children().remove(&parent.clone());

    assert_eq!(vec![child1.clone(), child2.clone()], parent.logical_children().to_vec());
    assert!(!child1.is_attached_to_logical_tree());
    assert!(!child2.is_attached_to_logical_tree());
}

#[test]
fn logical_children_can_be_removed_during_detached_from_logical_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let parent = StyledElement::new();
    let child1 = StyledElement::new();
    let child2 = StyledElement::new();
    let child2_detached = Rc::new(Cell::new(0));

    parent.logical_children().add_range([child1.clone(), child2.clone()]);
    root.logical_children().add(parent.clone());

    let (p, c2) = (parent.downgrade(), child2.clone());
    child1.detached_from_logical_tree(move |_| {
        p.upgrade().unwrap().logical_children().remove(&c2);
    });
    let d = child2_detached.clone();
    child2.detached_from_logical_tree(move |_| d.set(d.get() + 1));

    root.logical_children().remove(&parent.clone());

    assert_eq!(vec![child1.clone()], parent.logical_children().to_vec());
    assert!(!child1.is_attached_to_logical_tree());
    assert!(!child2.is_attached_to_logical_tree());
    assert_eq!(1, child2_detached.get());
}

#[test]
fn parent_should_be_null_when_detached_from_logical_tree_called() {
    let _scope = test_scope();
    let target = TestControl::new();
    let root = TestRoot::with_child(target.clone());
    let called = Rc::new(Cell::new(0));

    let (t, c) = (target.downgrade(), called.clone());
    target.detached_from_logical_tree(move |_| {
        let target = t.upgrade().unwrap();
        assert!(target.parent().is_none());
        assert!(target.inheritance_parent().is_none());
        c.set(c.get() + 1);
    });

    root.set_child(None);

    assert_eq!(1, called.get());
}

#[test]
fn adding_tree_to_root_should_style_controls() {
    let _scope = test_scope();
    let root = TestRoot::new();
    root.styles().add(tag_style());

    let grandchild = Control::new();
    let child = Border::new();
    child.set_child(grandchild.clone());
    let parent = Border::new();
    parent.set_child(child.clone());

    assert!(parent.tag().is_none());
    assert!(child.tag().is_none());
    assert!(grandchild.tag().is_none());

    root.set_child(parent.clone());

    assert_eq!(Some("foo".to_string()), tag_of(&parent));
    assert_eq!(Some("foo".to_string()), tag_of(&child));
    assert_eq!(Some("foo".to_string()), tag_of(&grandchild));
}

#[test]
fn styles_not_applied_until_initialization_finished() {
    let _scope = test_scope();
    let root = TestRoot::new();
    root.styles().add(tag_style());

    let child = Border::new();

    child.begin_init();
    root.set_child(child.clone());
    assert!(child.tag().is_none());

    child.end_init();
    assert_eq!(Some("foo".to_string()), tag_of(&child));
}

#[test]
fn name_cannot_be_set_after_added_to_logical_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let child = Border::new();

    root.set_child(child.clone());

    assert!(catch_unwind(AssertUnwindSafe(|| child.set_name(Some("foo".to_string())))).is_err());
}

#[test]
fn name_can_be_set_while_initializing() {
    let _scope = test_scope();
    let locator_scope = FerroLocator::enter_scope();
    {
        let root = TestRoot::new();
        let child = Border::new();

        child.begin_init();
        root.set_child(child.clone());
        child.set_name(Some("foo".to_string()));
        child.end_init();
    }
    locator_scope.dispose();
}

#[test]
fn style_is_removed_when_control_removed_from_logical_tree() {
    let _app = UnitTestApplication::start(TestServices::new());
    let target = Border::new();
    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Border>(),
        [Setter::new(Border::background_property(), red())],
    ));
    root.set_child(target.clone());

    assert_eq!(red(), target.background());
    root.set_child(None);
    assert!(target.background().is_none());
}

#[test]
fn end_init_should_raise_initialized() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let target = Border::new();
    let called = Rc::new(Cell::new(false));

    let c = called.clone();
    target.initialized(move || c.set(true));
    target.begin_init();
    root.set_child(target.clone());
    target.end_init();

    assert!(called.get());
    assert!(target.is_initialized());
}

#[test]
fn attaching_to_visual_tree_should_raise_initialized() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let target = Border::new();
    let called = Rc::new(Cell::new(false));

    let c = called.clone();
    target.initialized(move || c.set(true));
    root.set_child(target.clone());

    assert!(called.get());
    assert!(target.is_initialized());
}

/// The tree of the data context tests: a stack panel named `root` with two
/// test controls, `a1` (with a child `b1`) and `a2` (with its own data
/// context).
fn data_context_tree() -> Ref<TestStackPanel> {
    let root = TestStackPanel::new();
    root.set_name(Some("root".to_string()));
    let a1 = TestControl::named("a1");
    a1.set_child(TestControl::named("b1"));
    let a2 = TestControl::named("a2");
    a2.set_data_context(boxed_str("foo"));
    root.children().add(a1);
    root.children().add(a2);
    root
}

#[test]
fn data_context_changed_should_be_called() {
    let _scope = test_scope();
    let root = data_context_tree();

    let called = Rc::new(RefCell::new(Vec::<Option<String>>::new()));
    let record = |sender: &Ref<StyledElement>| {
        let (called, sender) = (called.clone(), sender.downgrade());
        move || called.borrow_mut().push(sender.upgrade().unwrap().name())
    };

    let root_element: Ref<StyledElement> = root.clone().upcast();
    root.data_context_changed(record(&root_element));

    for c in root.get_logical_descendants() {
        assert!(c.is::<TestControl>());
        c.data_context_changed(record(&c));
    }

    root.set_data_context(boxed_str("foo"));

    assert_eq!(
        vec![Some("root".to_string()), Some("a1".to_string()), Some("b1".to_string())],
        *called.borrow()
    );
}

#[test]
fn data_context_notifications_should_be_called_in_correct_order() {
    let _scope = test_scope();
    let root = data_context_tree();

    let called = Rc::new(RefCell::new(Vec::<String>::new()));

    for c in root.get_self_and_logical_descendants() {
        record_data_context_events(&c, &called);
    }

    root.set_data_context(boxed_str("foo"));

    assert_eq!(
        vec![
            "begin root",
            "begin a1",
            "begin b1",
            "changed root",
            "changed a1",
            "changed b1",
            "end b1",
            "end a1",
            "end root",
        ],
        *called.borrow()
    );
}

#[test]
fn data_context_notifications_should_be_called_in_correct_order_when_setting_parent() {
    let _scope = test_scope();
    let root = TestStackPanel::new();
    root.set_name(Some("root".to_string()));
    root.set_data_context(boxed_str("foo"));

    let a1 = TestControl::named("a1");
    a1.set_child(TestControl::named("b1"));
    let a2 = TestControl::named("a2");
    a2.set_data_context(boxed_str("foo"));
    let children = [a1, a2];

    let called = Rc::new(RefCell::new(Vec::<String>::new()));

    for c in [
        children[0].clone().upcast::<StyledElement>(),
        children[0].child().unwrap().upcast(),
        children[1].clone().upcast(),
    ] {
        record_data_context_events(&c, &called);
    }

    root.children().add_range(children.iter().map(|c| c.clone().upcast::<Control>()));

    assert_eq!(vec!["begin a1", "begin b1", "changed a1", "changed b1", "end b1", "end a1"], *called.borrow());
}

#[test]
fn resources_owner_is_set() {
    let _scope = test_scope();
    let target = TestControl::new();

    let owner = target.resources().owner();
    assert_eq!(owner.as_ref().and_then(ResourceHostRef::as_element), Some(&target.clone().upcast::<StyledElement>()));
}

#[test]
fn assigned_resources_parent_is_set() {
    let _scope = test_scope();
    // Deviation (DEVIATIONS.md, Tests and test support): `Resources` is a concrete resource dictionary, not an
    // interface that can be mocked; a derived dictionary records its owners from the `on_add_owner` hook.
    let resources = RecordingResourceDictionary::new();
    let target = TestControl::new();
    target.set_resources(resources.clone().upcast());

    let target_host = ResourceHostRef::Element(target.upcast());
    assert!(resources.added_owners.borrow().iter().any(|owner| *owner == target_host));
}

#[test]
fn assigning_resources_raises_resources_changed() {
    let _scope = test_scope();
    let resources = ResourceDictionary::new();
    resources.add_value("foo", "bar".to_string());
    let target = TestControl::new();
    let raised = Rc::new(Cell::new(0));

    let r = raised.clone();
    target.resources_changed(move |_| r.set(r.get() + 1));
    target.set_resources(resources);

    assert_eq!(1, raised.get());
}

#[test]
fn styles_owner_is_set() {
    let _scope = test_scope();
    let target = TestControl::new();

    let owner = target.styles().owner();
    assert_eq!(owner.as_ref().and_then(ResourceHostRef::as_element), Some(&target.clone().upcast::<StyledElement>()));
}

#[test]
fn adding_to_logical_tree_raises_resources_changed() {
    let _scope = test_scope();
    let target = TestRoot::new();
    let parent = Decorator::new();
    parent.resources().add_value("foo", "bar".to_string());
    let raised = Rc::new(Cell::new(0));

    let r = raised.clone();
    target.resources_changed(move |_| r.set(r.get() + 1));

    parent.set_child(target.clone());

    assert_eq!(1, raised.get());
}

#[test]
fn set_parent_does_not_crash_due_to_reentrancy() {
    // Issue #3708
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let target = ContentControl::new();
    target.styles().add(Style::with_setters(
        Selectors::of_type::<ContentControl>(),
        [Setter::new_template(
            ContentControl::content_property().as_property(),
            crate::templates::FuncTemplate::new(|| {
                let text_block = TextBlock::new();
                text_block.set_text(Some("Enabled"));
                Some(Control::boxed(text_block))
            }),
        )],
    ));
    target.styles().add(Style::with_setters(
        Selectors::of_type::<ContentControl>().class(":disabled"),
        [Setter::new_template(
            ContentControl::content_property().as_property(),
            crate::templates::FuncTemplate::new(|| {
                let text_block = TextBlock::new();
                text_block.set_text(Some("Disabled"));
                Some(Control::boxed(text_block))
            }),
        )],
    ));
    target.bind_binding(InputElement::is_enabled_property().as_property(), &ReflectionBinding::empty());

    let root = TestRoot::new();
    root.set_data_context(Some(Rc::new(false)));
    root.set_child(target.clone());

    root.measure(Size::INFINITY);
    root.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    let content = target.content().expect("content");
    let text_block = Control::from_boxed(&content).and_then(|c| c.cast::<TextBlock>()).expect("a TextBlock");
    assert_eq!(Some("Disabled".to_string()), text_block.text());

    // #3708 was crashing here with an internal exception.
    root.set_child(None);
}

