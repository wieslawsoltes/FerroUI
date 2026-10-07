//! Port of `ContentPresenterTests_Standalone.cs`: tests for content controls
//! that aren't hosted in a control template.

use crate::documents::TextElement;
use crate::presenters::{register_content_presenter_host, ContentPresenter, IContentPresenterHost};
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Border, Canvas, ContentControl, Control, ControlImpl, TextBlock};
use ferroui_base::collections::FerroList;
use ferroui_base::data::{BindingPriority, ReflectionBinding};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElement,
    StyledElementImpl, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn should_set_childs_parent_to_itself_standalone() {
    let _scope = test_scope();
    let content = Border::new();
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();

    assert_eq!(content.parent().unwrap(), target);
}

#[test]
fn should_add_child_to_own_logical_children_standalone() {
    let _scope = test_scope();
    let content = Border::new();
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();

    let logical_children = target.logical_children().to_vec();

    assert_eq!(logical_children.len(), 1);
    assert_eq!(logical_children[0], content);
}

#[test]
fn should_raise_detached_from_logical_tree_on_content_changed_standalone() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_content_template(Some(content_control_template()));

    // Deviation (DEVIATIONS.md, Tests and test support): upstream sets a `Mock<Control>` that also
    // implements `IContentPresenterHost`, `IPresentationSource` and `ILogicalRoot` as the parent; the
    // port uses `MockHostParent`, a control that is a logical root and a content presenter host.
    let parent_mock = MockHostParent::new();

    target.set_parent(&parent_mock);

    target.set_content(boxed_str("foo"));

    target.update_child();

    let foo = target.child().and_then(|x| x.cast::<ContentControl>());

    let foo_detached = Rc::new(Cell::new(false));

    assert!(foo.is_some());
    let foo = foo.unwrap();
    assert_eq!(foo.content().and_then(|x| string_of(&x)).as_deref(), Some("foo"));

    let detached = foo_detached.clone();
    foo.detached_from_logical_tree(move |_| detached.set(true));

    target.set_content(boxed_str("bar"));
    target.update_child();

    let bar = target.child().and_then(|x| x.cast::<ContentControl>());

    assert!(bar.is_some());
    assert!(bar.unwrap() != foo);
    assert!(!foo.is_attached_to_logical_tree());
    assert!(foo_detached.get());
}

#[test]
fn should_raise_detached_from_logical_tree_in_content_control_on_content_changed_standalone() {
    let _scope = test_scope();
    let content_control = ContentControl::new();
    content_control.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|c, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_indexer(
            &ContentPresenter::content_property().bind().with_priority(BindingPriority::Template),
            &c.indexer(&ContentControl::content_property().bind().with_priority(BindingPriority::Template)),
        );
        presenter.bind_indexer(
            &ContentPresenter::content_template_property().bind().with_priority(BindingPriority::Template),
            &c.indexer(&ContentControl::content_template_property().bind().with_priority(BindingPriority::Template)),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    })));
    content_control.set_content_template(Some(content_control_template()));

    // Deviation (DEVIATIONS.md, Tests and test support): upstream sets a `Mock<Control>` that also
    // implements `IPresentationSource` and `ILogicalRoot`, and reports `IsAttachedToLogicalTree`, as the
    // parent; the port uses `MockParent`, a control that is a logical root (and so is attached).
    let parent_mock = MockParent::new();

    content_control.set_parent(&parent_mock);

    content_control.apply_template();
    let target = content_control.presenter();
    assert!(target.is_some());
    let target = target.unwrap();

    content_control.set_content(boxed_str("foo"));

    target.update_child();

    let tbfoo = target.child().and_then(|x| x.cast::<ContentControl>());

    let foo_detached = Rc::new(Cell::new(false));

    assert!(tbfoo.is_some());
    let tbfoo = tbfoo.unwrap();
    assert_eq!(tbfoo.content().and_then(|x| string_of(&x)).as_deref(), Some("foo"));

    let detached = foo_detached.clone();
    tbfoo.detached_from_logical_tree(move |_| detached.set(true));

    content_control.set_content(boxed_str("bar"));
    target.update_child();

    let tbbar = target.child().and_then(|x| x.cast::<ContentControl>());

    assert!(tbbar.is_some());

    assert!(tbbar.unwrap() != tbfoo);
    assert!(!tbfoo.is_attached_to_logical_tree());
    assert!(foo_detached.get());
}

#[test]
fn should_raise_detached_from_logical_tree_on_detached_standalone() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_content_template(Some(content_control_template()));

    // Deviation (DEVIATIONS.md, Tests and test support): upstream sets a `Mock<Control>` that also
    // implements `IContentPresenterHost`, `IPresentationSource` and `ILogicalRoot` as the parent; the
    // port uses `MockHostParent`, a control that is a logical root and a content presenter host.
    let parent_mock = MockHostParent::new();

    target.set_parent(&parent_mock);

    target.set_content(boxed_str("foo"));

    target.update_child();

    let foo = target.child().and_then(|x| x.cast::<ContentControl>());

    let foo_detached = Rc::new(Cell::new(false));

    assert!(foo.is_some());
    let foo = foo.unwrap();
    assert_eq!(foo.content().and_then(|x| string_of(&x)).as_deref(), Some("foo"));

    let detached = foo_detached.clone();
    foo.detached_from_logical_tree(move |_| detached.set(true));

    target.set_parent(None);

    assert!(!foo.is_attached_to_logical_tree());
    assert!(foo_detached.get());
}

#[test]
fn should_remove_old_child_from_logical_children_on_content_changed_standalone() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_content_template(Some(content_control_template()));

    target.set_content(boxed_str("foo"));

    target.update_child();

    let foo = target.child().and_then(|x| x.cast::<ContentControl>());

    assert!(foo.is_some());
    let foo = foo.unwrap();

    let logical_children = target.logical_children().to_vec();

    assert_eq!(logical_children.len(), 1);

    target.set_content(boxed_str("bar"));
    target.update_child();

    assert!(foo.parent().is_none());

    let logical_children = target.logical_children().to_vec();

    assert_eq!(logical_children.len(), 1);
    let logical_child = &logical_children[0];
    assert_ne!(logical_child, &foo.upcast::<StyledElement>());
}

#[test]
fn should_not_bind_old_child_to_new_data_context() {
    // Test for issue #1099.
    let _scope = test_scope();
    let text_block = TextBlock::new();
    text_block.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::empty());

    let target = ContentPresenter::new();
    let template_text_block = text_block.clone();
    target
        .data_templates()
        .add(FuncDataTemplate::for_type::<String>(move |_, _| Some(template_text_block.clone().upcast()), false));
    target.data_templates().add(FuncDataTemplate::for_type::<i32>(|_, _| Some(Canvas::new().upcast()), false));

    let _root = TestRoot::with_child(&target);
    target.set_content(boxed_str("foo"));
    assert_eq!(target.child(), Some(text_block.clone().upcast()));

    text_block.property_changed(|e| {
        let new_value = e.new_value();
        assert_ne!(new_value.downcast_ref::<Option<String>>().and_then(|x| x.as_deref()), Some("42"));
        assert_ne!(new_value.downcast_ref::<String>().map(String::as_str), Some("42"));
    });

    let content: BoxedValue = Rc::new(42_i32);
    target.set_content(Some(content));
}

#[test]
fn should_reset_inheritance_parent_when_child_removed() {
    let _scope = test_scope();
    let logical_parent = Canvas::new();
    let child = TextBlock::new();
    let target = ContentPresenter::new();
    let _root = TestRoot::with_child(&target);

    child.set_parent(&logical_parent);
    target.set_content(Some(Control::boxed(&child)));
    target.set_content(None);

    // The inheritance parent is exposed via the styling parent.
    let styling_parent = child.styling_parent().and_then(|parent| parent.as_element().cloned());
    assert_eq!(styling_parent.unwrap(), logical_parent);
}

#[test]
fn should_create_child_even_with_null_content_when_content_template_is_set() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_content_template(Some(FuncDataTemplate::new(|_| true, |_, _| Some(hello_world_text_block()), false)));
    target.set_content(None);

    target.update_child();

    let text_block = target.child().and_then(|x| x.cast::<TextBlock>());
    assert!(text_block.is_some());
    assert_eq!(text_block.unwrap().text().as_deref(), Some("Hello World"));
}

#[test]
fn should_not_create_child_even_with_null_content_and_data_templates_instead_of_content_template() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.data_templates().add(FuncDataTemplate::new(|_| true, |_, _| Some(hello_world_text_block()), false));
    target.set_content(None);

    target.update_child();

    assert!(target.child().is_none());
}

#[test]
fn should_not_create_child_when_content_and_template_are_null() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_content_template(None);
    target.set_content(None);

    target.update_child();

    assert!(target.child().is_none());
}

#[test]
fn should_not_create_when_child_content_is_null_but_expected_value_type_with_func_data_template() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_content_template(Some(FuncDataTemplate::for_type_with_match::<i32>(
        |_| true,
        |_, _| Some(hello_world_text_block()),
        false,
    )));
    target.set_content(None);

    target.update_child();

    assert!(target.child().is_none());
}

#[test]
fn should_create_child_when_content_is_null_and_expected_nullable_value_type_with_func_data_template() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    target.set_content_template(Some(FuncDataTemplate::for_type_with_match::<Option<i32>>(
        |_| true,
        |_, _| Some(hello_world_text_block()),
        false,
    )));
    target.set_content(None);

    target.update_child();

    assert!(target.child().is_some());
}

#[test]
fn content_presenter_letter_spacing_default_value_is_zero() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    assert_eq!(presenter.letter_spacing(), 0.0);
}

#[test]
fn content_presenter_letter_spacing_can_be_set_and_retrieved() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    presenter.set_letter_spacing(3.5);
    assert_eq!(presenter.letter_spacing(), 3.5);
}

#[test]
fn content_presenter_letter_spacing_can_be_negative() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    presenter.set_letter_spacing(-2.0);
    assert_eq!(presenter.letter_spacing(), -2.0);
}

#[test]
fn content_presenter_letter_spacing_propagates_to_text_block_child() {
    let _scope = test_scope();
    {
        let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());
        let presenter = ContentPresenter::new();
        presenter.set_content(boxed_str("Test Content"));
        presenter.set_letter_spacing(4.0);
        let _root = TestRoot::with_child(&presenter);

        presenter.update_child();

        let text_block = presenter.child().and_then(|x| x.cast::<TextBlock>());
        assert!(text_block.is_some());
        assert_eq!(text_block.unwrap().letter_spacing(), 4.0);
    }
}

#[test]
fn content_presenter_letter_spacing_updates_text_block_when_changed() {
    let _scope = test_scope();
    {
        let _app = UnitTestApplication::start(TestServices::mock_platform_render_interface());
        let presenter = ContentPresenter::new();
        presenter.set_content(boxed_str("Test Content"));
        presenter.set_letter_spacing(1.0);
        let _root = TestRoot::with_child(&presenter);

        presenter.update_child();
        let text_block = presenter.child().and_then(|x| x.cast::<TextBlock>());

        presenter.set_letter_spacing(6.0);

        assert!(text_block.is_some());
        assert_eq!(text_block.unwrap().letter_spacing(), 6.0);
    }
}

#[test]
fn content_presenter_letter_spacing_property_inherits_from_text_block() {
    let _scope = test_scope();
    // Verify that the letter spacing of the content presenter uses the text
    // element letter spacing definition.
    assert!(std::ptr::eq(
        TextElement::letter_spacing_property().as_property(),
        ContentPresenter::letter_spacing_property().as_property()
    ));
}

/// `new FuncDataTemplate<string>((t, _) => new ContentControl() { Content = t }, false)`.
fn content_control_template() -> Rc<FuncDataTemplate> {
    FuncDataTemplate::for_type::<String>(
        |t, _| {
            let control = ContentControl::new();
            control.set_content(boxed_str(t));
            Some(control.upcast())
        },
        false,
    )
}

/// `new TextBlock { Text = "Hello World" }`.
fn hello_world_text_block() -> Ref<Control> {
    let text_block = TextBlock::new();
    text_block.set_text(Some("Hello World"));
    text_block.upcast()
}

/// The mocked parent control of the upstream tests: a control that is the
/// root of a logical tree.
#[repr(C)]
struct MockParent {
    base: Control,
}

ferro_class!(MockParent: Control);
ferro_impl_classes!(
    MockParent: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl StyledElementImpl for MockParent {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl MockParent {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct() })
    }
}

/// The mocked parent control of the upstream tests that is also a content
/// presenter host: a control that is the root of a logical tree and accepts
/// no presenter, as the members of a mock return their default values.
#[repr(C)]
struct MockHostParent {
    base: Control,
}

ferro_class!(MockHostParent: Control);
ferro_impl_classes!(
    MockHostParent: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl StyledElementImpl for MockHostParent {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl IContentPresenterHost for MockHostParent {
    fn logical_children(&self) -> &FerroList<Ref<StyledElement>> {
        StyledElement::logical_children(self)
    }

    fn register_content_presenter(&self, _presenter: &ContentPresenter) -> bool {
        false
    }
}

impl MockHostParent {
    fn new() -> Ref<Self> {
        register_content_presenter_host::<MockHostParent>();
        instantiate(Self { base: Control::construct() })
    }
}
