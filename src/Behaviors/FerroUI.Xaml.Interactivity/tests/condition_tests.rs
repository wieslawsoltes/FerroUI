//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/ConditionTests.cs` of Xaml.Behaviors
//! (`ConditionTests` and `ConditionCollectionTests`).

use super::support::start_app;
use crate::{Condition, ConditionCollection};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingBase;
use ferroui_base::{ferro_class, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty};
use ferroui_controls::TextBlock;
use ferroui_markup::data::Binding;
use std::rc::Rc;

/// `BindingSource`: an object with a `Value` property.
#[repr(C)]
struct BindingSource {
    base: FerroObject,
}

ferro_class!(BindingSource: FerroObject);
ferroui_base::ferro_impl_classes!(BindingSource: FerroObjectImpl);

ferro_properties! {
    impl BindingSource {
        fn value_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<BindingSource, _>("Value", None)
        }
    }
}

impl BindingSource {
    fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }

    fn set_source_value(&self, value: Option<String>) {
        self.set_value(Self::value_property(), value)
    }
}

/// The text an untyped value holds (the value of a binding to a text property).
fn text_of(value: Option<BoxedValue>) -> Option<String> {
    let value = value.and_then(ValueTypes::normalize)?;
    value.downcast_ref::<String>().cloned().or_else(|| value.downcast_ref::<Option<String>>().cloned().flatten())
}

fn binding(path: &str, source: Option<&Ref<BindingSource>>) -> Rc<dyn BindingBase> {
    let binding = Binding::with_path(path);
    if let Some(source) = source {
        binding.set_source(Some(Rc::new(source.clone()) as BoxedValue));
    }
    binding
}

#[test]
fn binding_updates_binding_value() {
    let _app = start_app();
    let source = BindingSource::new();
    source.set_source_value(Some("Initial".to_string()));
    let condition = Condition::new();
    condition.set_binding(Some(binding("Value", Some(&source))));

    assert_eq!(Some("Initial".to_string()), text_of(condition.binding_value()));

    source.set_source_value(Some("Updated".to_string()));

    assert_eq!(Some("Updated".to_string()), text_of(condition.binding_value()));
}

#[test]
fn setting_property_after_binding_throws() {
    let _app = start_app();
    let source = BindingSource::new();
    let condition = Condition::new();
    condition.set_binding(Some(binding("Value", Some(&source))));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        condition.set_property(Some(TextBlock::text_property().as_property()))
    }));

    assert!(result.is_err());
}

#[test]
fn setting_binding_after_property_throws() {
    let _app = start_app();
    let condition = Condition::new();
    condition.set_property(Some(TextBlock::text_property().as_property()));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        condition.set_binding(Some(binding("Value", None)))
    }));

    assert!(result.is_err());
}

#[test]
fn can_add_conditions() {
    let _app = start_app();
    let collection = ConditionCollection::new();
    let condition = Condition::new();

    collection.add(condition.clone());

    assert_eq!(1, collection.count());
    assert_eq!(condition, collection.get(0));
}

#[test]
fn clear_removes_all_items() {
    let _app = start_app();
    let collection = ConditionCollection::new();
    collection.add(Condition::new());
    collection.add(Condition::new());

    collection.clear();

    assert!(collection.is_empty());
}
