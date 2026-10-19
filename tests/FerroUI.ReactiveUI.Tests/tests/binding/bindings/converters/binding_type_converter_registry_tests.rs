//! `BindingTypeConverterRegistryTests` (`ReactiveUI.Binding.Tests/Bindings/Converters/BindingTypeConverterRegistryTests.cs`).

use ferroui_reactiveui::*;
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

type Registry = HashMap<ConverterTypePair, Rc<Vec<Rc<dyn IBindingTypeConverter>>>>;

fn string_to_int() -> ConverterTypePair {
    ConverterTypePair::new(ConverterType::of::<String>(), ConverterType::of::<i32>())
}

#[test]
fn clone_registry_shallow_empty_dictionary_returns_empty() {
    let source = Registry::new();
    let clone = BindingTypeConverterRegistry::clone_registry_shallow(&source);
    assert_eq!(clone.len(), 0);
}

#[test]
fn clone_registry_shallow_with_entries_returns_shallow_copy() {
    let converter: Rc<dyn IBindingTypeConverter> = Rc::new(TestConverter::new(string_to_int()));
    let list = Rc::new(vec![converter]);
    let mut source = Registry::new();
    source.insert(string_to_int(), list.clone());

    let clone = BindingTypeConverterRegistry::clone_registry_shallow(&source);

    assert_eq!(clone.len(), 1);
    assert!(clone.contains_key(&string_to_int()));
    let cloned_list = &clone[&string_to_int()];
    assert_eq!(cloned_list.len(), 1);
    assert!(Rc::ptr_eq(cloned_list, &list));
}

#[test]
fn clone_registry_shallow_modifying_clone_does_not_affect_original() {
    let converter: Rc<dyn IBindingTypeConverter> = Rc::new(TestConverter::new(string_to_int()));
    let mut source = Registry::new();
    source.insert(string_to_int(), Rc::new(vec![converter]));

    let mut clone = BindingTypeConverterRegistry::clone_registry_shallow(&source);
    clone.remove(&string_to_int());

    assert_eq!(source.len(), 1);
    assert_eq!(clone.len(), 0);
}

// Not ported: CloneRegistryShallow_NullInput_ThrowsArgumentNullException - argument validation

/// A converter of the given pair that never converts (`TestConverter`).
struct TestConverter {
    pair: ConverterTypePair,
}

impl TestConverter {
    fn new(pair: ConverterTypePair) -> Self {
        Self { pair }
    }
}

impl IBindingTypeConverter for TestConverter {
    fn from_type(&self) -> ConverterType {
        self.pair.from_type
    }

    fn to_type(&self) -> ConverterType {
        self.pair.to_type
    }

    fn get_affinity_for_objects(&self) -> i32 {
        2
    }

    fn try_convert_typed(&self, _from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        None
    }
}
