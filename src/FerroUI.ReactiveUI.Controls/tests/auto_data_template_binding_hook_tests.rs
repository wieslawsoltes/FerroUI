//! `AutoDataTemplateBindingHookTests` (`AutoDataTemplateBindingHookTests.cs`): the default item
//! template the hook gives an items control.
//!
//! The reference builds the observed changes from member expressions (`x => x.Items`); here an
//! observed change names its property, and its sender is the untyped handle of the control that
//! the property paths of the integration give ([`FerroObjectHandles::handle_of`]).

use super::view_support::start;
use crate::{AutoDataTemplateBindingHook, FerroObjectHandles, ViewModelViewHost};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{Control, ListBox, TextBlock};
use ferroui_reactiveui::{BindingDirection, BindingHookChanges, IPropertyBindingHook, ObservedChange};
use std::any::Any;
use std::rc::Rc;

/// An observed change of the property `name` of `control`, with the value `value`.
fn observed_change<T: ObjectType>(control: &Ref<T>, name: &str, value: Rc<dyn Any>) -> ObservedChange<Rc<dyn Any>, Rc<dyn Any>> {
    ObservedChange::new(FerroObjectHandles::handle_of(control), name, value)
}

/// An observed change of the `Items` property of an items control (`ItemsObservedChange`).
fn items_observed_change(items: &Ref<ListBox>) -> ObservedChange<Rc<dyn Any>, Rc<dyn Any>> {
    observed_change(items, "Items", Rc::new(items.items()))
}

/// An observed change of the `ItemsSource` property of an items control
/// (`ItemsSourceObservedChange`).
fn items_source_observed_change(items: &Ref<ListBox>) -> ObservedChange<Rc<dyn Any>, Rc<dyn Any>> {
    observed_change(items, "ItemsSource", Rc::new(items.items_source()))
}

/// An observed change of the `Tag` property of a control (`TagObservedChange`).
fn tag_observed_change(control: &Ref<ListBox>) -> ObservedChange<Rc<dyn Any>, Rc<dyn Any>> {
    observed_change(control, "Tag", Rc::new(control.tag()))
}

/// An observed change of the `Text` property of a text block (`TextObservedChange`).
fn text_observed_change(text: &Ref<TextBlock>) -> ObservedChange<Rc<dyn Any>, Rc<dyn Any>> {
    observed_change(text, "Text", Rc::new(text.text()))
}

/// The target of a hook call: the control, untyped.
fn target<T: ObjectType>(control: &Ref<T>) -> Rc<dyn Any> {
    FerroObjectHandles::handle_of(control)
}

fn no_changes() -> BindingHookChanges {
    Vec::new()
}

/// `new FuncDataTemplate<object>((_, _) => new TextBlock(), true)`.
fn text_block_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|data| data.is_some(), |_, _| Some(TextBlock::new().upcast()), true)
}

#[test]
fn execute_hook_items_control_without_template_sets_default_template() {
    let _app = start();
    let hook = AutoDataTemplateBindingHook::new();
    let items = ListBox::new();

    let result = hook.execute_hook(None, &target(&items), &no_changes, &|| vec![items_observed_change(&items)], BindingDirection::TwoWay);

    assert!(result);
    assert!(items.item_template().is_some());
}

#[test]
fn execute_hook_default_template_builds_view_model_view_host() {
    let _app = start();
    let hook = AutoDataTemplateBindingHook::new();
    let items = ListBox::new();

    let _ = hook.execute_hook(None, &target(&items), &no_changes, &|| vec![items_source_observed_change(&items)], BindingDirection::OneWay);

    let template = items.item_template().expect("the hook set the item template");
    let control: Option<Ref<Control>> = template.build(&Some(Rc::new(()) as ferroui_base::BoxedValue));

    let host = control.and_then(|control| control.cast::<ViewModelViewHost>()).expect("the template builds a view model view host");
    assert_eq!(host.horizontal_content_alignment(), HorizontalAlignment::Stretch);
    assert_eq!(host.vertical_content_alignment(), VerticalAlignment::Stretch);
}

// Not ported: ExecuteHook_NullGetCurrentViewProperties_Throws - argument validation (the function
// of the view properties is a reference, never null).

#[test]
fn execute_hook_no_view_properties_returns_true() {
    let _app = start();
    let hook = AutoDataTemplateBindingHook::new();
    let items = ListBox::new();

    let result = hook.execute_hook(None, &target(&items), &no_changes, &no_changes, BindingDirection::OneWay);

    assert!(result);
    assert!(items.item_template().is_none());
}

#[test]
fn execute_hook_non_items_control_sender_returns_true() {
    let _app = start();
    let hook = AutoDataTemplateBindingHook::new();
    let text = TextBlock::new();

    let result = hook.execute_hook(None, &target(&text), &no_changes, &|| vec![text_observed_change(&text)], BindingDirection::OneWay);

    assert!(result);
}

#[test]
fn execute_hook_non_items_property_returns_true() {
    let _app = start();
    let hook = AutoDataTemplateBindingHook::new();
    let items = ListBox::new();

    let result = hook.execute_hook(None, &target(&items), &no_changes, &|| vec![tag_observed_change(&items)], BindingDirection::OneWay);

    assert!(result);
    assert!(items.item_template().is_none());
}

#[test]
fn execute_hook_when_item_template_already_set_does_not_override() {
    let _app = start();
    let hook = AutoDataTemplateBindingHook::new();
    let items = ListBox::new();
    let template = text_block_template();
    items.set_item_template(Some(template.clone()));

    let result = hook.execute_hook(None, &target(&items), &no_changes, &|| vec![items_observed_change(&items)], BindingDirection::OneWay);

    assert!(result);
    assert!(items.item_template().is_some_and(|current| Rc::ptr_eq(&current, &template)));
}

#[test]
fn execute_hook_when_data_templates_present_does_not_override() {
    let _app = start();
    let hook = AutoDataTemplateBindingHook::new();
    let items = ListBox::new();
    items.data_templates().add(text_block_template());

    let result = hook.execute_hook(None, &target(&items), &no_changes, &|| vec![items_observed_change(&items)], BindingDirection::OneWay);

    assert!(result);
    assert!(items.item_template().is_none());
}
