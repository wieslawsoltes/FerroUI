//! Port of the upstream `CompiledBindingTests_Create`.
//!
//! The upstream factory builds the binding path from an expression tree;
//! here the same paths are built with the path builder, which is what the
//! factory produces.

use crate::utilities::CultureInfo;
use super::*;
use crate::data::converters::IValueConverter;
use crate::data::core::{Maybe, ModelRef, Untyped, ValueType, ValueTypes};
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{BindingError, BindingMode, CompiledBinding, CompiledBindingPath, CompiledBindingPathBuilder};
use crate::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElement, StyledElementImpl, StyledProperty,
};

struct TestViewModel {
    string_property: RefCell<Option<String>>,
    child: RefCell<Option<Rc<TestViewModel>>>,
    property_changed: Event<str>,
}

impl TestViewModel {
    fn new(string_property: Option<&str>) -> Rc<Self> {
        Model::new_model(Self {
            string_property: RefCell::new(string_property.map(s)),
            child: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    fn string_property(&self) -> Option<String> {
        self.string_property.borrow().clone()
    }

    fn set_string_property(&self, value: Option<String>) {
        self.string_property.replace(value);
        self.property_changed.raise("StringProperty");
    }

    fn child(&self) -> Option<Rc<TestViewModel>> {
        self.child.borrow().clone()
    }

    fn set_child(&self, value: Option<Rc<TestViewModel>>) {
        self.child.replace(value);
        self.property_changed.raise("Child");
    }
}

impl INotifyPropertyChanged for TestViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TestViewModel, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("StringProperty", |o| o.string_property(), |o, v| o.set_string_property(v))
    .property::<ModelRef<TestViewModel>>("Child", |o| o.child(), |o, v| o.set_child(v)));

struct TestConverter;

impl IValueConverter for TestConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.map(|v| boxed(ValueTypes::to_display_string(Some(v)).to_uppercase())))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.map(|v| boxed(ValueTypes::to_display_string(Some(v)).to_lowercase())))
    }
}

/// The equivalent of the upstream text block: an element with a text
/// property.
#[repr(C)]
pub struct TextBlock {
    base: StyledElement,
}

ferro_class!(TextBlock: StyledElement);
ferro_impl_classes!(TextBlock: FerroObjectImpl, StyledElementImpl);

impl TextBlock {
    ferro_property!(pub fn text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<TextBlock, _>("Text", None)
    });

    fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElement::construct() })
    }

    fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }
}

fn string_property(builder: CompiledBindingPathBuilder) -> CompiledBindingPathBuilder {
    builder.notifying_property::<TestViewModel, Maybe<String>>(
        "StringProperty",
        |vm| vm.string_property(),
        |vm, v| vm.set_string_property(v),
    )
}

/// `vm => vm.StringProperty`.
fn string_property_path() -> CompiledBindingPath {
    string_property(CompiledBindingPathBuilder::new()).build()
}

#[test]
fn create_should_create_binding_with_simple_property() {
    let binding = CompiledBinding::new(string_property_path());

    let path = binding.path().expect("a path");
    assert_eq!(path.to_string(), "StringProperty");
    assert!(binding.source().is_some_and(|source| source.is::<crate::UnsetValueType>()));
    assert_eq!(binding.mode(), BindingMode::Default);
}

#[test]
fn create_should_create_binding_with_source() {
    let source = TestViewModel::new(Some("Test"));
    let binding = CompiledBinding::new(string_property_path()).with_source(Some(source.clone()));

    let path = binding.path().expect("a path");
    assert_eq!(path.to_string(), "StringProperty");
    let bound = binding.source().expect("a source");
    let source: BoxedValue = source;
    assert!(Rc::ptr_eq(&bound, &source));
}

#[test]
fn create_should_apply_converter() {
    let converter: Rc<dyn IValueConverter> = Rc::new(TestConverter);
    let binding = CompiledBinding::new(string_property_path()).with_converter(converter.clone());

    assert!(Rc::ptr_eq(binding.converter().as_ref().expect("a converter"), &converter));
}

#[test]
fn create_should_apply_mode() {
    let binding = CompiledBinding::new(string_property_path()).with_mode(BindingMode::TwoWay);

    assert_eq!(binding.mode(), BindingMode::TwoWay);
}

#[test]
fn create_should_work_with_nested_properties() {
    let path = string_property(CompiledBindingPathBuilder::new().notifying_property::<TestViewModel, ModelRef<TestViewModel>>(
        "Child",
        |vm| vm.child(),
        |vm, v| vm.set_child(v),
    ))
    .build();
    let binding = CompiledBinding::new(path);

    assert_eq!(binding.path().as_ref().expect("a path").to_string(), "Child.StringProperty");
}

#[test]
fn create_should_work_with_indexer() {
    let path = CompiledBindingPathBuilder::new()
        .notifying_read_only_property::<TestViewModel, Untyped>("Items", |_| None)
        .array_element(&[0])
        .build();
    let binding = CompiledBinding::new(path);

    assert_eq!(binding.path().as_ref().expect("a path").to_string(), "Items[0]");
}

#[test]
fn binding_should_work_when_applied_to_control() {
    let target = TextBlock::new();
    let view_model = TestViewModel::new(Some("Hello"));
    let binding = CompiledBinding::new(string_property_path()).with_source(Some(view_model.clone()));

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text(), Some(s("Hello")));
}

#[test]
fn binding_should_update_when_source_property_changes() {
    let target = TextBlock::new();
    let view_model = TestViewModel::new(Some("Initial"));
    let binding = CompiledBinding::new(string_property_path()).with_source(Some(view_model.clone()));

    target.bind_binding(TextBlock::text_property(), &binding);
    assert_eq!(target.text(), Some(s("Initial")));

    view_model.set_string_property(Some(s("Updated")));
    assert_eq!(target.text(), Some(s("Updated")));
}

#[test]
fn binding_should_use_data_context_when_no_source_specified() {
    let target = TextBlock::new();
    let view_model = TestViewModel::new(Some("FromDataContext"));
    let binding = CompiledBinding::new(string_property_path());

    target.set_data_context(Some(view_model.clone()));
    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text(), Some(s("FromDataContext")));
}
