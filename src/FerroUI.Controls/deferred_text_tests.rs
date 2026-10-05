//! Tests of the reference that earlier ports left out because they need a
//! real text block (a text block with a bound text, or the type of the
//! text block itself).

use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::{Canvas, ContentControl, Control, TextBlock};
use ferroui_base::controls::NameScope;
use ferroui_base::data::core::Untyped;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{ReflectionBinding, TemplateBinding};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{ferro_model, BoxedValue, FerroObject, FerroObjectExtensions, Ref, StyledElement};
use std::cell::RefCell;
use std::rc::Rc;

/// A text block whose text is bound to its data context.
fn bound_text_block(path: &str) -> Ref<TextBlock> {
    let text_block = TextBlock::new();
    text_block.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::new(path));
    text_block
}

/// Fails when the text of `text_block` becomes "42".
fn assert_never_becomes_42(text_block: &TextBlock) {
    text_block.property_changed(|e| {
        let new_value = e.new_value();
        assert_ne!(new_value.downcast_ref::<Option<String>>().and_then(|x| x.as_deref()), Some("42"));
        assert_ne!(new_value.downcast_ref::<String>().map(String::as_str), Some("42"));
    });
}

fn add_text_block_and_canvas_templates(target: &Control, text_block: &Ref<TextBlock>) {
    let text_block = text_block.clone();
    target.data_templates().add(FuncDataTemplate::for_type::<String>(move |_, _| Some(text_block.clone().upcast()), false));
    target.data_templates().add(FuncDataTemplate::for_type::<i32>(|_, _| Some(Canvas::new().upcast()), false));
}

// --- Presenters/ContentPresenterTests_InTemplate.cs --------------------------

/// A presenter in the template of a rooted content control.
fn create_target() -> (TestScope, Ref<TestRoot>, Ref<ContentPresenter>, Ref<ContentControl>) {
    let scope = test_scope();
    let templated_parent = ContentControl::new();
    templated_parent.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|_, s| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**s).upcast()
    })));
    let root = TestRoot::with_child(&templated_parent);
    templated_parent.apply_template();
    let presenter = templated_parent.presenter().unwrap();
    (scope, root, presenter, templated_parent)
}

#[test]
fn control_content_should_not_be_name_scope() {
    let (_scope, _root, target, _) = create_target();

    target.set_content(Some(Control::boxed(TextBlock::new())));

    let child = target.child().unwrap();
    assert!(child.is::<TextBlock>());
    assert!(NameScope::get_name_scope(&child).is_none());
}

#[test]
fn should_not_bind_old_child_to_new_data_context_in_template() {
    // Test for issue #1099.
    let (_scope, _root, target, host) = create_target();
    let text_block = bound_text_block("");
    add_text_block_and_canvas_templates(&host, &text_block);

    target.set_content(boxed_str("foo"));
    assert_eq!(Some(text_block.clone().upcast()), target.child());

    assert_never_becomes_42(&text_block);

    let content: BoxedValue = Rc::new(42_i32);
    target.set_content(Some(content));
}

struct TestViewModel {
    content: RefCell<Option<BoxedValue>>,
    property_changed: Event<str>,
}

impl TestViewModel {
    fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    fn set_content(&self, value: Option<BoxedValue>) {
        self.content.replace(value);
        self.property_changed.raise("Content");
    }
}

impl INotifyPropertyChanged for TestViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TestViewModel, |b| b
    .notify_property_changed()
    .property::<Untyped>("Content", |vm| vm.content(), |vm, v| vm.set_content(v)));

#[test]
fn should_not_bind_child_to_wrong_data_context_when_removing() {
    // Test for issue #2823
    let canvas = Canvas::new();
    let (_scope, _root, target, host) = create_target();
    let view_model = Model::new_model(TestViewModel { content: RefCell::new(boxed_str("foo")), property_changed: Event::new() });
    let data_contexts: Rc<RefCell<Vec<Option<String>>>> = Rc::new(RefCell::new(Vec::new()));

    target.bind_binding(
        ContentPresenter::content_property().as_property(),
        &TemplateBinding::new(ContentControl::content_property().as_property()),
    );
    let seen = data_contexts.clone();
    let object: &FerroObject = &canvas;
    FerroObjectExtensions::get_observable(object, StyledElement::data_context_property())
        .subscribe_fn(move |x: Option<BoxedValue>| seen.borrow_mut().push(x.map(|x| string_of(&x).unwrap())));

    let template_canvas = canvas.clone();
    host.data_templates().add(FuncDataTemplate::for_type::<String>(move |_, _| Some(template_canvas.clone().upcast()), false));
    host.bind_binding(ContentControl::content_property().as_property(), &ReflectionBinding::new("Content"));
    host.set_data_context(Some(view_model.clone()));

    assert_eq!(Some(canvas.clone().upcast()), target.child());

    let content: BoxedValue = Rc::new(42_i32);
    view_model.set_content(Some(content));

    assert_eq!(vec![None, Some("foo".to_string()), None], *data_contexts.borrow());
}

#[test]
fn content_should_become_data_context_when_control_template_is_not_null() {
    let (_scope, _root, target, _) = create_target();

    let text_block = bound_text_block("Name");

    let canvas = Canvas::new();
    canvas.set_name(Some("Canvas".to_string()));

    let template_text_block = text_block.clone();
    target.set_content_template(Some(FuncDataTemplate::for_type_with_match::<Ref<Control>>(
        |control| control.is::<Canvas>(),
        move |_, _| Some(template_text_block.clone().upcast()),
        false,
    )));
    target.set_content(Some(Control::boxed(&canvas)));

    let data_context = target.data_context().unwrap();
    assert_eq!(Control::from_boxed(&data_context), Some(canvas.clone().upcast()));
    assert_eq!(text_block.text().as_deref(), Some("Canvas"));
}

// --- Presenters/ContentPresenterTests_Standalone.cs --------------------------

#[test]
fn should_not_bind_old_child_to_new_data_context_standalone() {
    // Test for issue #1099.
    let _scope = test_scope();
    let text_block = bound_text_block("");

    let target = ContentPresenter::new();
    add_text_block_and_canvas_templates(&target, &text_block);

    let _root = TestRoot::with_child(&target);
    target.set_content(boxed_str("foo"));
    assert_eq!(Some(text_block.clone().upcast()), target.child());

    assert_never_becomes_42(&text_block);

    let content: BoxedValue = Rc::new(42_i32);
    target.set_content(Some(content));
}
