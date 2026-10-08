//! The reference has no tests of its own for this class: it is exercised
//! by the data validation tests of the controls (see the data validation
//! tests of the text box). These tests cover the members and the edge cases
//! that those tests do not reach.

use crate::test_support::{string_of, test_scope};
use crate::{Control, DataValidationErrors, ErrorConverter, ItemsControl};
use ferroui_base::data::{
    AggregateError, AggregateException, BindingChainException, BindingError, DataValidationException,
};
use ferroui_base::{AnyValue, BoxedValue};
use std::rc::Rc;

fn boxed(value: &str) -> Option<BoxedValue> {
    let value: BoxedValue = Rc::new(value.to_string());
    Some(value)
}

fn strs<const N: usize>(values: [&str; N]) -> Option<Vec<BoxedValue>> {
    Some(values.into_iter().filter_map(boxed).collect())
}

fn errors_of(control: &Control) -> Vec<String> {
    DataValidationErrors::get_errors(control)
        .map(|errors| errors.iter().map(|error| string_of(error).unwrap()).collect())
        .unwrap_or_default()
}

#[test]
fn setting_errors_sets_has_errors_and_pseudo_class() {
    let _scope = test_scope();
    let target = Control::new();

    DataValidationErrors::set_errors(&target, strs(["foo"]));

    assert!(DataValidationErrors::get_has_errors(&target));
    assert!(target.classes().contains(":error"));
    assert_eq!(vec!["foo"], errors_of(&target));

    DataValidationErrors::clear_errors(&target);

    assert!(!DataValidationErrors::get_has_errors(&target));
    assert!(!target.classes().contains(":error"));
    assert!(DataValidationErrors::get_errors(&target).is_none());
}

// The template of the errors shows them with an items control whose items are the errors
// themselves (`ItemsSource="{Binding}"`, the errors being the data context).
#[test]
fn errors_are_the_items_of_an_items_control_bound_to_them() {
    let _scope = test_scope();
    crate::register_types();
    let target = Control::new();
    DataValidationErrors::set_errors(&target, strs(["foo", "bar"]));

    let errors: BoxedValue = Rc::new(DataValidationErrors::get_errors(&target).expect("the errors"));
    let items = ItemsControl::new();
    items.set_data_context(Some(errors));
    let _expression = items.bind_binding(
        ItemsControl::items_source_property().as_property(),
        &ferroui_base::data::ReflectionBinding::new("."),
    );

    let source = items.items_source().expect("the items source");
    assert_eq!(2, source.count());
    assert_eq!(Some("foo".to_string()), source.get_at(0).and_then(|error| string_of(&error)));
    assert_eq!(Some("bar".to_string()), source.get_at(1).and_then(|error| string_of(&error)));
    assert_eq!(2, items.item_count());
}

#[test]
fn empty_errors_do_not_set_has_errors() {
    let _scope = test_scope();
    let target = Control::new();

    DataValidationErrors::set_errors(&target, Some(Vec::new()));

    assert!(!DataValidationErrors::get_has_errors(&target));
    assert_eq!(Some(0), DataValidationErrors::get_errors(&target).map(|errors| errors.len()));
}

#[test]
fn set_error_keeps_the_error_itself() {
    let _scope = test_scope();
    let target = Control::new();
    let exception = BindingError::message("failed validation");

    DataValidationErrors::set_error(&target, Some(&exception));

    let errors = DataValidationErrors::get_errors(&target).unwrap();
    assert_eq!(1, errors.len());
    let error: &dyn AnyValue = &*errors[0];
    assert!(error.downcast_ref::<BindingError>() == Some(&exception));

    DataValidationErrors::set_error(&target, None);

    assert!(DataValidationErrors::get_errors(&target).is_none());
    assert!(!DataValidationErrors::get_has_errors(&target));
}

#[test]
fn set_error_unpacks_data_validation_errors() {
    let _scope = test_scope();
    let target = Control::new();

    DataValidationErrors::set_error(&target, Some(&BindingError::new(DataValidationException::new(boxed("foo")))));
    assert_eq!(vec!["foo"], errors_of(&target));

    DataValidationErrors::set_error(
        &target,
        Some(&BindingError::new(AggregateException::new(vec![
            DataValidationException::new(boxed("bar")),
            DataValidationException::new(None),
            DataValidationException::new(boxed("baz")),
        ]))),
    );
    assert_eq!(vec!["bar", "baz"], errors_of(&target));

    DataValidationErrors::set_error(
        &target,
        Some(&BindingError::new(AggregateError::new(vec![
            BindingError::new(DataValidationException::new(boxed("foo"))),
            BindingError::new(BindingChainException::with_message("broken")),
            BindingError::new(DataValidationException::new(boxed("bar"))),
        ]))),
    );
    assert_eq!(vec!["foo", "bar"], errors_of(&target));
}

#[test]
fn set_error_ignores_binding_chain_errors() {
    let _scope = test_scope();
    let target = Control::new();

    DataValidationErrors::set_error(&target, Some(&BindingError::new(BindingChainException::with_message("foo"))));

    assert_eq!(Some(0), DataValidationErrors::get_errors(&target).map(|errors| errors.len()));
    assert!(!DataValidationErrors::get_has_errors(&target));
}

#[test]
fn error_converter_converts_and_removes_errors() {
    let _scope = test_scope();
    let target = Control::new();

    DataValidationErrors::set_errors(&target, strs(["foo", "skip", "bar"]));
    DataValidationErrors::set_error_converter(
        &target,
        Some(ErrorConverter::new(|error| match string_of(error).as_deref() {
            Some("skip") => None,
            Some(text) => boxed(&format!("{text}!")),
            None => None,
        })),
    );

    assert_eq!(vec!["foo!", "bar!"], errors_of(&target));
    assert!(DataValidationErrors::get_has_errors(&target));

    // The converter is applied to the original errors.
    DataValidationErrors::set_error_converter(&target, None);

    assert_eq!(vec!["foo", "skip", "bar"], errors_of(&target));

    DataValidationErrors::set_error_converter(&target, Some(ErrorConverter::new(|_| None)));

    assert_eq!(Some(0), DataValidationErrors::get_errors(&target).map(|errors| errors.len()));
    assert!(!DataValidationErrors::get_has_errors(&target));
}

#[test]
fn owner_is_the_templated_parent_unless_set() {
    let _scope = test_scope();
    let parent = Control::new();
    let other = Control::new();

    let target = DataValidationErrors::new();
    assert!(target.owner().is_none());
    target.set_templated_parent(&parent);
    assert!(target.owner() == Some(parent.clone()));

    let target = DataValidationErrors::new();
    target.set_owner(Some(other.clone()));
    target.set_templated_parent(&parent);
    assert!(target.owner() == Some(other));
}
