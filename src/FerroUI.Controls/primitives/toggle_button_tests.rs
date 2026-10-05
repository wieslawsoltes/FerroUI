use super::ToggleButton;
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::ferro_model;
use std::cell::Cell;
use std::rc::Rc;

const UNCHECKED_CLASS: &str = ":unchecked";
const CHECKED_CLASS: &str = ":checked";
const INDETERMINATE_CLASS: &str = ":indeterminate";

struct Class1 {
    foo: Cell<bool>,
    nullable_foo: Cell<Option<bool>>,
    property_changed: Event<str>,
}

impl Class1 {
    fn new() -> Rc<Self> {
        Model::new_model(Self { foo: Cell::new(false), nullable_foo: Cell::new(None), property_changed: Event::new() })
    }

    fn set_foo(&self, value: bool) {
        self.foo.set(value);
        self.property_changed.raise("Foo");
    }

    fn set_nullable_foo(&self, value: Option<bool>) {
        self.nullable_foo.set(value);
        self.property_changed.raise("NullableFoo");
    }
}

impl INotifyPropertyChanged for Class1 {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Class1, |b| b
    .notify_property_changed()
    .property::<Value<bool>>("Foo", |vm| vm.foo.get(), |vm, v| vm.set_foo(v))
    .property::<Maybe<bool>>("NullableFoo", |vm| vm.nullable_foo.get(), |vm, v| vm.set_nullable_foo(v)));

#[test]
fn toggle_button_has_correct_class_according_to_is_checked() {
    let data = [
        (Some(false), UNCHECKED_CLASS, false),
        (Some(false), UNCHECKED_CLASS, true),
        (Some(true), CHECKED_CLASS, false),
        (Some(true), CHECKED_CLASS, true),
        (None, INDETERMINATE_CLASS, false),
        (None, INDETERMINATE_CLASS, true),
    ];

    for (is_checked, expected_class, is_three_state) in data {
        let toggle_button = ToggleButton::new();
        toggle_button.set_is_three_state(is_three_state);
        toggle_button.set_is_checked(is_checked);

        assert!(toggle_button.classes().contains(expected_class), "{is_checked:?} {is_three_state}");
    }
}

#[test]
fn toggle_button_is_checked_binds_to_bool() {
    let toggle_button = ToggleButton::new();
    let source = Class1::new();

    toggle_button.set_data_context(Some(source.clone()));
    toggle_button.bind_binding(ToggleButton::is_checked_property(), &ReflectionBinding::new("Foo"));

    source.set_foo(true);
    assert_eq!(toggle_button.is_checked(), Some(true));

    source.set_foo(false);
    assert_eq!(toggle_button.is_checked(), Some(false));
}

#[test]
fn toggle_button_three_state_checked_binds_to_nullable_bool() {
    let three_state_button = ToggleButton::new();
    let source = Class1::new();

    three_state_button.set_data_context(Some(source.clone()));
    three_state_button.bind_binding(ToggleButton::is_checked_property(), &ReflectionBinding::new("NullableFoo"));

    source.set_nullable_foo(Some(true));
    assert_eq!(three_state_button.is_checked(), Some(true));

    source.set_nullable_foo(Some(false));
    assert_eq!(three_state_button.is_checked(), Some(false));

    source.set_nullable_foo(None);
    assert_eq!(three_state_button.is_checked(), None);
}

#[test]
fn toggle_button_is_checked_changed_is_raised_on_is_checked_changes() {
    let three_state_button = ToggleButton::new();
    assert_eq!(three_state_button.is_checked(), Some(false));

    let change_count = Rc::new(Cell::new(0));
    let counter = change_count.clone();
    three_state_button.is_checked_changed(move |_, _| counter.set(counter.get() + 1));

    three_state_button.set_is_checked(Some(true));
    assert_eq!(1, change_count.get());
    assert_eq!(three_state_button.is_checked(), Some(true));

    three_state_button.set_is_checked(Some(false));
    assert_eq!(2, change_count.get());
    assert_eq!(three_state_button.is_checked(), Some(false));

    three_state_button.set_is_checked(None);
    assert_eq!(3, change_count.get());
    assert_eq!(three_state_button.is_checked(), None);
}

#[test]
fn toggle_button_is_checked_changed_is_raised_when_toggling() {
    let three_state_button = ToggleButton::new();
    three_state_button.set_is_three_state(true);
    assert_eq!(three_state_button.is_checked(), Some(false));

    let change_count = Rc::new(Cell::new(0));
    let counter = change_count.clone();
    three_state_button.is_checked_changed(move |_, _| counter.set(counter.get() + 1));

    three_state_button.toggle();
    assert_eq!(1, change_count.get());
    assert_eq!(three_state_button.is_checked(), Some(true));

    three_state_button.toggle();
    assert_eq!(2, change_count.get());
    assert_eq!(three_state_button.is_checked(), None);

    three_state_button.toggle();
    assert_eq!(3, change_count.get());
    assert_eq!(three_state_button.is_checked(), Some(false));
}
