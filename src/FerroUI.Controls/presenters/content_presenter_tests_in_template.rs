//! Port of `ContentPresenterTests_InTemplate.cs`: tests for content controls
//! that are hosted in a control template.

use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{Border, Canvas, ContentControl, Control, TextBlock};
use ferroui_base::controls::NameScope;
use ferroui_base::data::core::Untyped;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{ReflectionBinding, TemplateBinding};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{ferro_model, BoxedValue, FerroObject, FerroObjectExtensions, Ref, StyledElement, Visual};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn should_register_with_host_when_templated_parent_set() {
    let _scope = test_scope();
    let host = ContentControl::new();
    let target = ContentPresenter::new();
    target.set_name(Some("PART_ContentPresenter".to_string()));

    assert!(host.presenter().is_none());

    target.set_templated_parent(host.clone().upcast::<FerroObject>());

    assert_eq!(host.presenter().unwrap(), target);
}

#[test]
fn setting_content_to_control_should_set_child() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(target.child().unwrap(), child);
}

#[test]
fn setting_content_to_control_should_update_logical_tree() {
    let _scope = test_scope();
    let (target, parent, _root) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(child.parent().unwrap(), parent);
    let expected: Vec<Ref<StyledElement>> = vec![child.upcast()];
    assert_eq!(StyledElement::logical_children(&parent).to_vec(), expected);
}

#[test]
fn setting_content_to_control_should_update_visual_tree() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(child.visual_parent().unwrap(), target);
    let expected: Vec<Ref<Visual>> = vec![child.upcast()];
    assert_eq!(target.visual_children().to_vec(), expected);
}

#[test]
fn setting_content_to_string_should_create_text_block() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    target.set_content(boxed_str("Foo"));

    assert!(target.child().unwrap().is::<TextBlock>());
    assert_eq!(target.child().unwrap().cast::<TextBlock>().unwrap().text().as_deref(), Some("Foo"));
}

#[test]
fn setting_content_to_string_should_update_logical_tree() {
    let _scope = test_scope();
    let (target, parent, _root) = create_target();

    target.set_content(boxed_str("Foo"));

    let child = target.child();
    assert!(child.is_some());
    let child = child.unwrap();
    assert_eq!(child.parent().unwrap(), parent);
    let expected: Vec<Ref<StyledElement>> = vec![child.upcast()];
    assert_eq!(StyledElement::logical_children(&parent).to_vec(), expected);
}

#[test]
fn setting_content_to_string_should_update_visual_tree() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    target.set_content(boxed_str("Foo"));

    let child = target.child();
    assert!(child.is_some());
    let child = child.unwrap();
    assert_eq!(child.visual_parent().unwrap(), target);
    let expected: Vec<Ref<Visual>> = vec![child.upcast()];
    assert_eq!(target.visual_children().to_vec(), expected);
}

#[test]
fn clearing_control_content_should_update_logical_tree() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));
    target.set_content(None);

    assert!(child.parent().is_none());
    assert!(target.logical_children().is_empty());
}

#[test]
fn clearing_control_content_should_update_visual_tree() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));
    target.set_content(None);

    assert!(child.visual_parent().is_none());
    assert!(target.visual_children().is_empty());
}

#[test]
fn control_content_should_not_be_name_scope() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    target.set_content(Some(Control::boxed(TextBlock::new())));

    assert!(target.child().unwrap().is::<TextBlock>());
    assert!(NameScope::get_name_scope(&target.child().unwrap()).is_none());
}

#[test]
fn assigning_control_to_content_should_not_set_data_context() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    target.set_content(Some(Control::boxed(Border::new())));

    assert!(!target.is_set(StyledElement::data_context_property().as_property()));
}

#[test]
fn assigning_non_control_to_content_should_set_data_context_on_update_child() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    target.set_content(boxed_str("foo"));

    assert_eq!(target.data_context().and_then(|x| string_of(&x)).as_deref(), Some("foo"));
}

#[test]
fn should_use_content_template_if_specified() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    target.set_content_template(Some(FuncDataTemplate::for_type::<String>(|_, _| Some(Canvas::new().upcast()), false)));
    target.set_content(boxed_str("Foo"));

    assert!(target.child().unwrap().is::<Canvas>());
}

#[test]
fn should_update_if_content_template_changed() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    target.set_content(boxed_str("Foo"));
    assert!(target.child().unwrap().is::<TextBlock>());

    target.set_content_template(Some(FuncDataTemplate::for_type::<String>(|_, _| Some(Canvas::new().upcast()), false)));
    assert!(target.child().unwrap().is::<Canvas>());

    target.set_content_template(None);
    assert!(target.child().unwrap().is::<TextBlock>());
}

#[test]
fn assigning_control_to_content_after_non_control_should_clear_data_context() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    target.set_content(boxed_str("foo"));

    assert!(target.is_set(StyledElement::data_context_property().as_property()));

    target.set_content(Some(Control::boxed(Border::new())));

    assert!(!target.is_set(StyledElement::data_context_property().as_property()));
}

#[test]
fn recycles_data_template() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    target.data_templates().add(FuncDataTemplate::for_type::<String>(|_, _| Some(Border::new().upcast()), true));

    target.set_content(boxed_str("foo"));

    let control = target.child();
    assert!(control.as_ref().unwrap().is::<Border>());

    target.set_content(boxed_str("bar"));
    assert_eq!(control, target.child());
}

#[test]
fn detects_data_template_doesnt_match_and_doesnt_recycle() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    target.data_templates().add(FuncDataTemplate::for_type_with_match::<String>(
        |x| x == "foo",
        |_, _| Some(Border::new().upcast()),
        true,
    ));

    target.set_content(boxed_str("foo"));

    let control = target.child();
    assert!(control.as_ref().unwrap().is::<Border>());

    target.set_content(boxed_str("bar"));
    assert!(target.child().unwrap().is::<TextBlock>());
}

#[test]
fn detects_data_template_doesnt_support_recycling() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();
    target.data_templates().add(FuncDataTemplate::for_type::<String>(|_, _| Some(Border::new().upcast()), false));

    target.set_content(boxed_str("foo"));

    let control = target.child();
    assert!(control.as_ref().unwrap().is::<Border>());

    target.set_content(boxed_str("bar"));
    assert_ne!(control, target.child());
}

#[test]
fn reevaluates_data_templates_when_recycling() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    target.data_templates().add(FuncDataTemplate::for_type_with_match::<String>(
        |x| x == "bar",
        |_, _| Some(Canvas::new().upcast()),
        true,
    ));
    target.data_templates().add(FuncDataTemplate::for_type::<String>(|_, _| Some(Border::new().upcast()), true));

    target.set_content(boxed_str("foo"));

    let control = target.child();
    assert!(control.as_ref().unwrap().is::<Border>());

    target.set_content(boxed_str("bar"));
    assert!(target.child().unwrap().is::<Canvas>());
}

#[test]
fn should_not_bind_old_child_to_new_data_context() {
    // Test for issue #1099.
    let _scope = test_scope();
    let text_block = TextBlock::new();
    text_block.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::empty());

    let (target, host, _root) = create_target();
    let template_text_block = text_block.clone();
    host.data_templates()
        .add(FuncDataTemplate::for_type::<String>(move |_, _| Some(template_text_block.clone().upcast()), false));
    host.data_templates().add(FuncDataTemplate::for_type::<i32>(|_, _| Some(Canvas::new().upcast()), false));

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
fn should_not_bind_child_to_wrong_data_context_when_removing() {
    // Test for issue #2823
    let _scope = test_scope();
    let canvas = Canvas::new();
    let (target, host, _root) = create_target();
    let view_model =
        Model::new_model(TestViewModel { content: RefCell::new(boxed_str("foo")), property_changed: Event::new() });
    let data_contexts: Rc<RefCell<Vec<Option<BoxedValue>>>> = Rc::new(RefCell::new(Vec::new()));

    target.bind_binding(
        ContentPresenter::content_property().as_property(),
        &TemplateBinding::new(ContentControl::content_property().as_property()),
    );
    let added = data_contexts.clone();
    let object: &FerroObject = &canvas;
    FerroObjectExtensions::get_observable(object, StyledElement::data_context_property())
        .subscribe_fn(move |x: Option<BoxedValue>| added.borrow_mut().push(x));

    let template_canvas = canvas.clone();
    host.data_templates()
        .add(FuncDataTemplate::for_type::<String>(move |_, _| Some(template_canvas.clone().upcast()), false));
    host.bind_binding(ContentControl::content_property().as_property(), &ReflectionBinding::new("Content"));
    host.set_data_context(Some(view_model.clone()));

    assert_eq!(target.child(), Some(canvas.clone().upcast()));

    let content: BoxedValue = Rc::new(42_i32);
    view_model.set_content(Some(content));

    assert_eq!(vec![None, boxed_str("foo"), None], *data_contexts.borrow());
}

#[test]
fn should_set_inheritance_parent_even_when_logical_parent_is_already_set() {
    let _scope = test_scope();
    let logical_parent = Canvas::new();
    let child = TextBlock::new();
    let (target, _host, _root) = create_target();

    child.set_parent(&logical_parent);
    target.set_content(Some(Control::boxed(&child)));

    assert_eq!(child.parent().unwrap(), logical_parent);

    // The inheritance parent is exposed via the styling parent.
    assert_eq!(styling_parent(&child).unwrap(), target);
}

#[test]
fn should_reset_inheritance_parent_when_child_removed() {
    let _scope = test_scope();
    let logical_parent = Canvas::new();
    let child = TextBlock::new();
    let (target, _, _root) = create_target();

    child.set_parent(&logical_parent);
    target.set_content(Some(Control::boxed(&child)));
    target.set_content(None);

    // The inheritance parent is exposed via the styling parent.
    assert_eq!(styling_parent(&child).unwrap(), logical_parent);
}

#[test]
fn should_clear_host_when_host_template_cleared() {
    let _scope = test_scope();
    let (target, host, _root) = create_target();

    assert_eq!(target.host().unwrap(), host);

    host.set_template(None);
    host.apply_template();

    assert!(target.host().is_none());
}

#[test]
fn content_should_become_data_context_when_control_template_is_not_null() {
    let _scope = test_scope();
    let (target, _, _root) = create_target();

    let text_block = TextBlock::new();
    text_block.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::new("Name"));

    let canvas = Canvas::new();
    canvas.set_name(Some("Canvas".to_string()));

    let template_text_block = text_block.clone();
    // A typed data template matches the exact type of the boxed data, and a
    // control is boxed as `Ref<Control>`: the template of `Canvas` matches a
    // control that is a canvas.
    target.set_content_template(Some(FuncDataTemplate::for_type_with_match::<Ref<Control>>(
        |x| x.is::<Canvas>(),
        move |_, _| Some(template_text_block.clone().upcast()),
        false,
    )));
    target.set_content(Some(Control::boxed(&canvas)));

    assert!(target.data_context().is_some());
    assert_eq!(target.data_context().and_then(|x| Control::from_boxed(&x)), Some(canvas.clone().upcast()));
    assert_eq!(text_block.text().as_deref(), Some("Canvas"));
}

/// Creates a presenter in the template of a content control in a test root.
///
/// The root is returned as well: a logical parent is held weakly, so the
/// test keeps the root alive.
fn create_target() -> (Ref<ContentPresenter>, Ref<ContentControl>, Ref<TestRoot>) {
    let templated_parent = ContentControl::new();
    templated_parent.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|_, s| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**s).upcast()
    })));
    let root = TestRoot::with_child(&templated_parent);

    templated_parent.apply_template();

    (templated_parent.presenter().unwrap(), templated_parent, root)
}

/// The styling parent of an element, when it is an element.
fn styling_parent(element: &StyledElement) -> Option<Ref<StyledElement>> {
    element.styling_parent().and_then(|parent| parent.as_element().cloned())
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
        let changed = match (&*self.content.borrow(), &value) {
            (Some(old), Some(new)) => !Rc::ptr_eq(old, new),
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.content.replace(value);
            self.property_changed.raise("Content");
        }
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
