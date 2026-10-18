//! `Models/TextCellTests.cs`: a text cell reads its value from its binding, writes changed values
//! and texts back, and commits or discards an edit; with a string format, the text is the
//! formatted value.

use super::support::BehaviorSubject;
use crate::models::{ICellBinding, IEditableObject, ITextCellCultureOptions, TextCell, TextColumnOptions};
use ferroui_base::utilities::CultureInfo;
use std::cell::RefCell;
use std::rc::Rc;

fn record<T: Clone + 'static>(binding: &Rc<BehaviorSubject<T>>) -> Rc<RefCell<Vec<T>>> {
    let result = Rc::new(RefCell::new(Vec::new()));
    let sink = result.clone();
    binding.subscribe(Rc::new(move |value: &T| sink.borrow_mut().push(value.clone())));
    result
}

#[test]
fn value_is_initially_read_from_string() {
    let binding = BehaviorSubject::new("initial".to_string());
    let target = TextCell::<String>::new(binding, true, None);

    assert_eq!(Some("initial".to_string()), target.text());
    assert_eq!(Some("initial".to_string()), target.value());
}

#[test]
fn modified_value_is_written_to_binding() {
    let binding = BehaviorSubject::new("initial".to_string());
    let target = TextCell::<String>::new(binding.clone(), false, None);
    let result = record(&binding);

    target.set_value(Some("new".to_string()));

    assert_eq!(vec!["initial".to_string(), "new".to_string()], *result.borrow());
}

#[test]
fn modified_text_is_written_to_binding() {
    let binding = BehaviorSubject::new("initial".to_string());
    let target = TextCell::<String>::new(binding.clone(), false, None);
    let result = record(&binding);

    target.set_text(Some("new".to_string()));

    assert_eq!(vec!["initial".to_string(), "new".to_string()], *result.borrow());
}

#[test]
fn modified_value_is_written_to_binding_on_end_edit() {
    let binding = BehaviorSubject::new("initial".to_string());
    let target = TextCell::<String>::new(binding.clone(), false, None);
    let result = record(&binding);

    target.begin_edit();
    target.set_text(Some("new".to_string()));

    assert_eq!(Some("new".to_string()), target.text());
    assert_eq!(Some("initial".to_string()), target.value());
    assert_eq!(vec!["initial".to_string()], *result.borrow());

    target.end_edit();

    assert_eq!(Some("new".to_string()), target.text());
    assert_eq!(Some("new".to_string()), target.value());
    assert_eq!(vec!["initial".to_string(), "new".to_string()], *result.borrow());
}

#[test]
fn modified_value_is_not_written_to_binding_on_cancel_edit() {
    let binding = BehaviorSubject::new("initial".to_string());
    let target = TextCell::<String>::new(binding.clone(), false, None);
    let result = record(&binding);

    target.begin_edit();
    target.set_text(Some("new".to_string()));

    assert_eq!(Some("new".to_string()), target.text());
    assert_eq!(Some("initial".to_string()), target.value());
    assert_eq!(vec!["initial".to_string()], *result.borrow());

    target.cancel_edit();

    assert_eq!(Some("initial".to_string()), target.text());
    assert_eq!(Some("initial".to_string()), target.value());
    assert_eq!(vec!["initial".to_string()], *result.borrow());
}

/// `TextCellTests.StringFormat`.
mod string_format {
    use super::*;

    fn get_options(format: &str) -> Option<Rc<dyn ITextCellCultureOptions>> {
        let options = TextColumnOptions::<i32>::default();
        options.set_string_format(format);
        options.set_culture(CultureInfo::invariant_culture());
        Some(Rc::new(options))
    }

    #[test]
    fn initial_int_value_is_formatted() {
        let binding = BehaviorSubject::new(42);
        let target = TextCell::<i32>::new(binding, true, get_options("{0:n2}"));

        assert_eq!(Some("42.00".to_string()), target.text());
        assert_eq!(Some(42), target.value());
    }

    #[test]
    fn int_value_is_formatted_after_editing() {
        let binding = BehaviorSubject::new(42);
        let target = TextCell::<i32>::new(binding.clone(), false, get_options("{0:n2}"));
        let result = record(&binding);

        target.begin_edit();
        target.set_text(Some("43".to_string()));

        assert_eq!(Some("43".to_string()), target.text());
        assert_eq!(Some(42), target.value());
        assert_eq!(vec![42], *result.borrow());

        target.end_edit();

        assert_eq!(Some("43.00".to_string()), target.text());
        assert_eq!(Some(43), target.value());
        assert_eq!(vec![42, 43], *result.borrow());
    }
}
