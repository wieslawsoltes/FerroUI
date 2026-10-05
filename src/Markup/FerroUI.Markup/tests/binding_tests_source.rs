//! Ported from the upstream `BindingTests_Source`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::core::Maybe;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::ferro_model;
use std::cell::RefCell;
use std::rc::Rc;

struct Source {
    foo: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl Source {
    fn new(foo: &str) -> Rc<Self> {
        Model::new_model(Self { foo: RefCell::new(Some(s(foo))), property_changed: Event::new() })
    }

    fn set_foo(&self, value: Option<String>) {
        self.foo.replace(value);
        self.property_changed.raise("Foo");
    }
}

impl INotifyPropertyChanged for Source {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Source, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("Foo", |o| o.foo.borrow().clone(), |o, v| o.set_foo(v)));

#[test]
fn source_should_be_used() {
    let source = Source::new("foo");
    let binding = Binding::with_path("Foo");
    binding.set_source(Some(source.clone()));
    let target = TextBlock::new();

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
}
