//! End-to-end tests of bindings to model objects (specific to this port):
//! the model layer, compiled and string-path bindings, two-way updates,
//! converters and data validation.

use crate::utilities::CultureInfo;
use super::*;
use crate::data::converters::IValueConverter;
use crate::data::core::{Maybe, ModelRef, Value, ValueType};
use crate::data::model::{ICommand, Event, INotifyDataErrorInfo, INotifyPropertyChanged, Model};
use crate::data::{
    BindingError, BindingMode, BindingOperations, BindingValueType, CompiledBinding, CompiledBindingPathBuilder,
    ReflectionBinding,
};
use crate::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Ref, StyledElement, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};

pub struct PersonVm {
    name: RefCell<String>,
    age: Cell<i32>,
    nick: RefCell<Option<String>>,
    child: RefCell<Option<Rc<PersonVm>>>,
    saved: Cell<i32>,
    property_changed: Event<str>,
    errors_changed: Event<str>,
}

impl PersonVm {
    pub fn new(name: &str, age: i32) -> Rc<Self> {
        Model::new_model(Self {
            name: RefCell::new(name.to_string()),
            age: Cell::new(age),
            nick: RefCell::new(None),
            child: RefCell::new(None),
            saved: Cell::new(0),
            property_changed: Event::new(),
            errors_changed: Event::new(),
        })
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: String) {
        self.name.replace(value);
        self.property_changed.raise("Name");
        self.errors_changed.raise("Name");
    }

    pub fn age(&self) -> i32 {
        self.age.get()
    }

    pub fn set_age(&self, value: i32) -> Result<(), BindingError> {
        if value < 0 {
            return Err(BindingError::message("Age must not be negative."));
        }
        self.age.set(value);
        self.property_changed.raise("Age");
        Ok(())
    }

    pub fn nick(&self) -> Option<String> {
        self.nick.borrow().clone()
    }

    pub fn set_nick(&self, value: Option<String>) {
        self.nick.replace(value);
        self.property_changed.raise("Nick");
    }

    pub fn child(&self) -> Option<Rc<PersonVm>> {
        self.child.borrow().clone()
    }

    pub fn set_child(&self, value: Option<Rc<PersonVm>>) {
        self.child.replace(value);
        self.property_changed.raise("Child");
    }

    pub fn save(&self) {
        self.saved.set(self.saved.get() + 1);
    }

    pub fn can_save(&self) -> bool {
        !self.name.borrow().is_empty()
    }
}

impl INotifyPropertyChanged for PersonVm {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl INotifyDataErrorInfo for PersonVm {
    fn has_errors(&self) -> bool {
        self.name.borrow().is_empty()
    }

    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
        if property_name == Some("Name") && self.name.borrow().is_empty() {
            vec![boxed(s("Name is required."))]
        } else {
            Vec::new()
        }
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

ferro_model!(PersonVm, |b| b
    .notify_property_changed()
    .notify_data_error_info()
    .property::<Value<String>>("Name", |vm| vm.name(), |vm, v| vm.set_name(v))
    .validated_property::<Value<i32>>("Age", |vm| vm.age(), |vm, v| vm.set_age(v))
    .property::<Maybe<String>>("Nick", |vm| vm.nick(), |vm, v| vm.set_nick(v))
    .property::<ModelRef<PersonVm>>("Child", |vm| vm.child(), |vm, v| vm.set_child(v))
    .method_with_can_execute("Save", |vm, _| vm.save(), |vm, _| vm.can_save(), &["Name"]));

#[repr(C)]
pub struct Target {
    base: StyledElement,
    validation: RefCell<Vec<(String, BindingValueType, Option<String>)>>,
}

ferro_class!(Target: StyledElement);
ferro_impl_classes!(Target: StyledElementImpl);

impl FerroObjectImpl for Target {
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        this.validation.borrow_mut().push((property.name().to_string(), state, error.map(|e| e.to_string())));
    }
}

impl Target {
    ferro_property!(pub fn text_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Target, _>("Text",
            StyledPropertyOptions::new(s("default")).enable_data_validation(true))
    });
    ferro_property!(pub fn number_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Target, _>("Number",
            StyledPropertyOptions::new(0).enable_data_validation(true))
    });
    ferro_property!(pub fn maybe_text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<Target, _>("MaybeText", None)
    });
    ferro_property!(pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
        FerroProperty::register::<Target, _>("Command", None)
    });
    ferro_property!(pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<Target, _>("Tag", None)
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElement::construct(), validation: RefCell::new(Vec::new()) })
    }

    fn last_validation(&self) -> Option<(String, BindingValueType, Option<String>)> {
        self.validation.borrow().last().cloned()
    }
}

fn name_path() -> crate::data::CompiledBindingPath {
    CompiledBindingPathBuilder::new()
        .notifying_property::<PersonVm, Value<String>>("Name", |vm| vm.name(), |vm, v| vm.set_name(v))
        .build()
}

#[test]
fn compiled_binding_reads_and_tracks_data_context_property() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::text_property(), &CompiledBinding::new(name_path()));
    assert_eq!(target.get_value(Target::text_property()), "Ann");

    vm.set_name(s("Bob"));
    assert_eq!(target.get_value(Target::text_property()), "Bob");

    let other = PersonVm::new("Cid", 1);
    target.set_data_context(Some(other));
    assert_eq!(target.get_value(Target::text_property()), "Cid");

    target.set_data_context(None);
    assert_eq!(target.get_value(Target::text_property()), "default");
}

#[test]
fn string_path_binding_resolves_declared_properties_and_nested_paths() {
    let vm = PersonVm::new("Ann", 30);
    let child = PersonVm::new("Kid", 3);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::text_property(), &ReflectionBinding::new("Child.Name"));
    // A null link in the chain is a binding error: the target reverts to its default.
    assert_eq!(target.get_value(Target::text_property()), "default");

    vm.set_child(Some(child.clone()));
    assert_eq!(target.get_value(Target::text_property()), "Kid");

    child.set_name(s("Kiddo"));
    assert_eq!(target.get_value(Target::text_property()), "Kiddo");
}

#[test]
fn two_way_binding_writes_back_to_the_model() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::text_property(), &CompiledBinding::new(name_path()).with_mode(BindingMode::TwoWay));
    target.set_value(Target::text_property(), s("Zed"));
    assert_eq!(vm.name(), "Zed");
    assert_eq!(target.get_value(Target::text_property()), "Zed");

    vm.set_name(s("Ann"));
    assert_eq!(target.get_value(Target::text_property()), "Ann");
}

#[test]
fn string_path_two_way_binding_converts_between_text_and_number() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::text_property(), &ReflectionBinding::new("Age").with_mode(BindingMode::TwoWay));
    assert_eq!(target.get_value(Target::text_property()), "30");

    target.set_value(Target::text_property(), s("41"));
    assert_eq!(vm.age(), 41);
}

#[test]
fn rejected_value_surfaces_as_data_validation_error() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::number_property(), &ReflectionBinding::new("Age").with_mode(BindingMode::TwoWay));
    assert_eq!(target.get_value(Target::number_property()), 30);

    target.set_value(Target::number_property(), -1);
    assert_eq!(vm.age(), 30);
    let (property, state, error) = target.last_validation().expect("validation was reported");
    assert_eq!(property, "Number");
    assert_eq!(state, BindingValueType::DATA_VALIDATION_ERROR);
    assert_eq!(error.as_deref(), Some("Age must not be negative."));

    target.set_value(Target::number_property(), 5);
    assert_eq!(vm.age(), 5);
    assert_eq!(target.last_validation().map(|v| v.1), Some(BindingValueType::VALUE));
}

#[test]
fn notify_data_error_info_errors_surface_on_the_target() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::text_property(), &ReflectionBinding::new("Name").with_mode(BindingMode::TwoWay));
    target.set_value(Target::text_property(), s(""));

    assert_eq!(vm.name(), "");
    let (_, state, error) = target.last_validation().expect("validation was reported");
    assert_eq!(state, BindingValueType::DATA_VALIDATION_ERROR);
    assert_eq!(error.as_deref(), Some("Name is required."));

    target.set_value(Target::text_property(), s("Ann"));
    assert_eq!(target.last_validation().map(|v| v.1), Some(BindingValueType::VALUE));
}

struct Shout;

impl IValueConverter for Shout {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.and_then(|v| v.downcast_ref::<String>()).map(|v| boxed(v.to_uppercase())))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(value.and_then(|v| v.downcast_ref::<String>()).map(|v| boxed(v.to_lowercase())))
    }
}

#[test]
fn converter_is_applied_in_both_directions() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    let binding = CompiledBinding::new(name_path()).with_mode(BindingMode::TwoWay).with_converter(Rc::new(Shout));
    target.bind_binding(Target::text_property(), &binding);
    assert_eq!(target.get_value(Target::text_property()), "ANN");

    target.set_value(Target::text_property(), s("BOB"));
    assert_eq!(vm.name(), "bob");
}

#[test]
fn nullable_property_maps_to_null_and_target_null_value() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::maybe_text_property(), &ReflectionBinding::new("Nick"));
    assert_eq!(target.get_value(Target::maybe_text_property()), None);
    vm.set_nick(Some(s("Annie")));
    assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Annie")));

    let binding = ReflectionBinding::new("Nick");
    binding.set_target_null_value(Some(boxed(s("(none)"))));
    target.bind_binding(Target::text_property(), &binding);
    assert_eq!(target.get_value(Target::text_property()), "Annie");
    vm.set_nick(None);
    assert_eq!(target.get_value(Target::text_property()), "(none)");
}

#[test]
fn string_format_and_fallback_value_apply() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    let binding = ReflectionBinding::new("Age");
    binding.set_string_format(Some(s("Age: {0:D3}")));
    target.bind_binding(Target::text_property(), &binding);
    assert_eq!(target.get_value(Target::text_property()), "Age: 030");

    let binding = ReflectionBinding::new("Missing");
    binding.set_fallback_value(Some(boxed(s("fallback"))));
    target.bind_binding(Target::text_property(), &binding);
    assert_eq!(target.get_value(Target::text_property()), "fallback");
}

#[test]
fn method_binds_as_command() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    target.bind_binding(Target::command_property(), &ReflectionBinding::new("Save"));
    let command = target.get_value(Target::command_property()).expect("a command");
    assert!(command.can_execute(None));
    command.execute(None);
    assert_eq!(vm.saved.get(), 1);

    vm.set_name(s(""));
    assert!(!command.can_execute(None));
}

#[test]
fn binding_does_not_keep_target_or_source_alive() {
    let vm = PersonVm::new("Ann", 30);
    let weak_vm = Rc::downgrade(&vm);
    let target = Target::new();
    let weak_target = target.downgrade();
    target.set_data_context(Some(vm.clone()));
    target.bind_binding(Target::text_property(), &CompiledBinding::new(name_path()).with_mode(BindingMode::TwoWay));

    assert_eq!(vm.property_changed.handler_count(), 1);
    drop(target);
    assert!(weak_target.upgrade().is_none());
    assert_eq!(vm.property_changed.handler_count(), 0);

    drop(vm);
    assert!(weak_vm.upgrade().is_none());
}

#[test]
fn get_binding_expression_base_returns_the_active_expression() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm));

    assert!(BindingOperations::get_binding_expression_base(&target, Target::text_property()).is_none());
    let expression = target.bind_binding(Target::text_property(), &CompiledBinding::new(name_path()));
    let found = BindingOperations::get_binding_expression_base(&target, Target::text_property()).expect("found");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&expression), Rc::as_ptr(&found)));

    expression.dispose();
    assert!(BindingOperations::get_binding_expression_base(&target, Target::text_property()).is_none());
    assert_eq!(target.get_value(Target::text_property()), "default");
}

#[test]
fn untyped_target_receives_values_unconverted() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm));

    target.bind_binding(Target::tag_property(), &ReflectionBinding::new("Age"));
    let tag = target.get_value(Target::tag_property()).expect("a value");
    assert_eq!(tag.downcast_ref::<i32>(), Some(&30));
}


// --- template, multi, setter and class bindings ---------------------------

use crate::data::converters::FuncMultiValueConverter;
use crate::data::{BindingBase, MultiBinding, TemplateBinding};
use crate::styling::{Selectors, Setter, Style, Styles};
use crate::ClassBindingManager;

#[test]
fn template_binding_tracks_templated_parent_property() {
    let parent = Target::new();
    parent.set_value(Target::text_property(), s("parent"));
    let child = Target::new();

    child.bind_binding(Target::text_property(), &TemplateBinding::new(Target::text_property()));
    assert_eq!(child.get_value(Target::text_property()), "default");

    child.set_templated_parent(&parent);
    assert_eq!(child.get_value(Target::text_property()), "parent");

    parent.set_value(Target::text_property(), s("changed"));
    assert_eq!(child.get_value(Target::text_property()), "changed");
}

#[test]
fn two_way_template_binding_writes_to_templated_parent() {
    let parent = Target::new();
    parent.set_value(Target::text_property(), s("parent"));
    let child = Target::new();
    child.set_templated_parent(&parent);

    child.bind_binding(
        Target::text_property(),
        &TemplateBinding::new(Target::text_property()).with_mode(BindingMode::TwoWay),
    );
    child.set_value_with_priority(Target::text_property(), s("from child"), crate::data::BindingPriority::Animation);
    assert_eq!(parent.get_value(Target::text_property()), "from child");
}

#[test]
fn template_binding_has_template_priority() {
    let binding = TemplateBinding::new(Target::text_property());
    let target = Target::new();
    let expression = binding.create_instance(&target, Some(Target::text_property()), None);
    assert_eq!(expression.default_priority(), crate::data::BindingPriority::Template);
}

#[test]
fn multi_binding_combines_values_through_converter() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    let binding = MultiBinding::new().with_bindings(vec![ReflectionBinding::new("Name"), ReflectionBinding::new("Age")]).with_converter_value(Some(Rc::new(FuncMultiValueConverter::<Option<BoxedValue>, String>::new(|values| {
            values.iter().map(|v| crate::data::core::ValueTypes::to_display_string(v.as_ref())).collect::<Vec<_>>().join("/")
        }))));
    target.bind_binding(Target::text_property(), &binding);
    assert_eq!(target.get_value(Target::text_property()), "Ann/30");

    vm.set_name(s("Bob"));
    assert_eq!(target.get_value(Target::text_property()), "Bob/30");
}

#[test]
fn multi_binding_applies_string_format() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm));

    let binding = MultiBinding::new().with_bindings(vec![ReflectionBinding::new("Name"), ReflectionBinding::new("Age")]).with_string_format(Some(s("{0} is {1}")));
    target.bind_binding(Target::text_property(), &binding);
    assert_eq!(target.get_value(Target::text_property()), "Ann is 30");
}

#[test]
fn setter_with_binding_is_instanced_per_control_and_follows_activation() {
    let vm = PersonVm::new("Ann", 30);
    let styles = Styles::new();
    let style = Style::with_selector(Selectors::of_type::<Target>().class("bound"));
    style.add_setter(Setter::new_binding_base(Target::text_property(), CompiledBinding::new(name_path())));
    styles.add(style);

    let target = Target::new();
    target.set_data_context(Some(vm.clone()));
    styles.try_attach(&target, None);
    assert_eq!(target.get_value(Target::text_property()), "default");
    assert_eq!(vm.property_changed.handler_count(), 0);

    target.classes().add("bound");
    assert_eq!(target.get_value(Target::text_property()), "Ann");
    vm.set_name(s("Bob"));
    assert_eq!(target.get_value(Target::text_property()), "Bob");

    target.classes().remove("bound");
    assert_eq!(target.get_value(Target::text_property()), "default");
    assert_eq!(vm.property_changed.handler_count(), 0);
}

#[test]
fn two_way_setter_binding_writes_back_and_reports_validation() {
    let vm = PersonVm::new("Ann", 30);
    let styles = Styles::new();
    let style = Style::with_selector(Selectors::of_type::<Target>());
    style.add_setter(Setter::new_binding_base(
        Target::number_property(),
        ReflectionBinding::new("Age").with_mode(BindingMode::TwoWay),
    ));
    styles.add(style);

    let target = Target::new();
    target.set_data_context(Some(vm.clone()));
    styles.try_attach(&target, None);
    assert_eq!(target.get_value(Target::number_property()), 30);

    target.set_current_value(Target::number_property(), 31);
    assert_eq!(vm.age(), 31);

    target.set_current_value(Target::number_property(), -5);
    assert_eq!(vm.age(), 31);
    assert_eq!(target.last_validation().map(|v| v.1), Some(BindingValueType::DATA_VALIDATION_ERROR));
}

#[test]
fn setter_with_template_binding_reads_templated_parent() {
    let parent = Target::new();
    parent.set_value(Target::text_property(), s("parent"));
    let styles = Styles::new();
    let style = Style::with_selector(Selectors::of_type::<Target>());
    style.add_setter(Setter::new_binding_base(
        Target::text_property(),
        TemplateBinding::new(Target::text_property()),
    ));
    styles.add(style);

    let target = Target::new();
    target.set_templated_parent(&parent);
    styles.try_attach(&target, None);
    assert_eq!(target.get_value(Target::text_property()), "parent");
}

#[test]
fn class_binding_adds_and_removes_class() {
    let source = Target::new();
    let target = Target::new();
    let flag = FerroProperty::register::<Target, bool>("Flag", false);

    let binding = CompiledBinding::new(CompiledBindingPathBuilder::new().ferro_property(flag).build())
        .with_source(Some(boxed(source.clone().upcast::<crate::FerroObject>())));
    ClassBindingManager::bind(&target, "active", &binding, None);
    assert!(!target.classes().contains("active"));

    source.set_value(flag, true);
    assert!(target.classes().contains("active"));
    source.set_value(flag, false);
    assert!(!target.classes().contains("active"));
}

// Upstream `Bound_Validated_String_Property_Can_Be_Set_To_Null`.
#[test]
fn bound_validated_string_property_can_be_set_to_null() {
    let source = Target::new();
    source.set_value(Target::maybe_text_property(), Some(s("foo")));
    let target = Target::new();
    let validated = FerroProperty::register_with::<Target, Option<String>>(
        "ValidatedMaybeText",
        StyledPropertyOptions::new(None).enable_data_validation(true),
    );

    let binding = CompiledBinding::new(
        CompiledBindingPathBuilder::new().ferro_property(Target::maybe_text_property()).build(),
    )
    .with_source(Some(boxed(source.clone().upcast::<crate::FerroObject>())))
    .with_mode(BindingMode::TwoWay);
    target.bind_binding(validated, &binding);
    assert_eq!(target.get_value(validated), Some(s("foo")));

    target.set_value(validated, None);
    assert_eq!(source.get_value(Target::maybe_text_property()), None);
}

// --- typed binding expression -----------------------------------------------

use crate::data::core::{BindingExpression, TypedBindingExpression};

fn typed_name_path() -> crate::data::CompiledBindingPath {
    CompiledBindingPathBuilder::new()
        .typed_property::<PersonVm, String>("Name", |vm| vm.name(), Some(Rc::new(|vm, v| vm.set_name(v))))
        .build()
}

#[test]
fn compiled_binding_uses_typed_expression_when_types_match() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    // `MaybeText` has no data validation and `Name` is a `String`, which is
    // assignable to `Option<String>`: the typed expression is used one-way.
    // The reverse assignment is not valid, so two-way is untyped.
    let widened = target.bind_binding(Target::maybe_text_property(), &CompiledBinding::new(typed_name_path()));
    assert!(widened.as_any().is::<TypedBindingExpression<PersonVm, String>>());
    assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Ann")));
    let untyped = target.bind_binding(
        Target::maybe_text_property(),
        &CompiledBinding::new(typed_name_path()).with_mode(BindingMode::TwoWay),
    );
    assert!(untyped.as_any().is::<BindingExpression>());
    assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Ann")));

    let plain = FerroProperty::register::<Target, String>("Plain", s("plain-default"));
    let typed = target.bind_binding(plain, &CompiledBinding::new(typed_name_path()).with_mode(BindingMode::TwoWay));
    assert!(typed.as_any().is::<TypedBindingExpression<PersonVm, String>>());
    assert_eq!(target.get_value(plain), "Ann");

    vm.set_name(s("Bob"));
    assert_eq!(target.get_value(plain), "Bob");

    target.set_value(plain, s("Cid"));
    assert_eq!(vm.name(), "Cid");

    let other = PersonVm::new("Dan", 1);
    target.set_data_context(Some(other.clone()));
    assert_eq!(target.get_value(plain), "Dan");
    assert_eq!(vm.property_changed.handler_count(), 0);

    target.set_data_context(None);
    assert_eq!(target.get_value(plain), "plain-default");

    typed.dispose();
    assert_eq!(other.property_changed.handler_count(), 0);
}

#[test]
fn typed_expression_is_not_used_with_data_validation_converter_or_source() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    target.set_data_context(Some(vm.clone()));

    // Data validation is enabled on `Text`.
    let e = target.bind_binding(Target::text_property(), &CompiledBinding::new(typed_name_path()));
    assert!(e.as_any().is::<BindingExpression>());

    let plain = FerroProperty::register::<Target, String>("Plain", s(""));
    let e = target.bind_binding(plain, &CompiledBinding::new(typed_name_path()).with_converter(Rc::new(Shout)));
    assert!(e.as_any().is::<BindingExpression>());
    let e = target.bind_binding(plain, &CompiledBinding::new(typed_name_path()).with_source(Some(vm.clone())));
    assert!(e.as_any().is::<BindingExpression>());
    assert_eq!(target.get_value(plain), "Ann");
}

#[test]
fn typed_expression_does_not_keep_target_alive_and_unsubscribes_on_drop() {
    let vm = PersonVm::new("Ann", 30);
    let target = Target::new();
    let weak = target.downgrade();
    target.set_data_context(Some(vm.clone()));
    let plain = FerroProperty::register::<Target, String>("Plain", s(""));
    target.bind_binding(plain, &CompiledBinding::new(typed_name_path()));
    assert_eq!(vm.property_changed.handler_count(), 1);
    drop(target);
    assert!(weak.upgrade().is_none());
    assert_eq!(vm.property_changed.handler_count(), 0);
}

// --- the typed path hook of markup metadata ------------------------------------

mod metadata_typed_path {
    use super::*;
    use crate::ferro_markup_type;
    use crate::metadata::MarkupTyped;

    /// A view model declared in markup metadata, as the run-time loader sees it.
    pub struct HookVm {
        name: RefCell<Option<String>>,
        flag: Cell<bool>,
        count: Cell<i32>,
        property_changed: Event<str>,
    }

    impl PartialEq for HookVm {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    impl INotifyPropertyChanged for HookVm {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    impl HookVm {
        pub fn new(name: &str) -> Rc<Self> {
            Rc::new(Self {
                name: RefCell::new(Some(name.to_string())),
                flag: Cell::new(false),
                count: Cell::new(0),
                property_changed: Event::new(),
            })
        }

        fn name(&self) -> Option<String> {
            self.name.borrow().clone()
        }

        fn set_name(&self, value: Option<String>) {
            self.name.replace(value);
            self.property_changed.raise("Name");
        }

        fn set_flag(&self, value: bool) {
            self.flag.set(value);
            self.property_changed.raise("Flag");
        }

        fn try_count(&self) -> Result<i32, String> {
            match self.count.get() {
                count if count < 0 => Err("The count is negative.".to_string()),
                count => Ok(count),
            }
        }

        fn try_set_count(&self, value: i32) -> Result<(), String> {
            self.count.set(value);
            Ok(())
        }
    }

    ferro_markup_type!(class HookVm {
        handles: [HookVm, Rc<HookVm>, Option<Rc<HookVm>>],
        this: Rc<HookVm>,
        properties: [
            Name: Option<String> { get: HookVm::name, set: HookVm::set_name },
            Flag: bool { get: |vm: &Rc<HookVm>| vm.flag.get() },
            Count: i32 { try_get: HookVm::try_count },
            Guarded: i32 { get: |vm: &Rc<HookVm>| vm.count.get(), try_set: HookVm::try_set_count },
            WriteOnly: bool { set: HookVm::set_flag },
        ],
        notify_property_changed: HookVm,
    });

    /// A type whose instances are not shared notifying objects.
    #[derive(Clone, PartialEq)]
    pub struct HookPlain {
        value: i32,
    }

    pub struct HookSilent {
        value: Cell<i32>,
    }

    impl PartialEq for HookSilent {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    ferro_markup_type!(struct HookPlain {
        handles: [HookPlain],
        properties: [Value: i32 { get: |plain: &HookPlain| plain.value }],
    });

    ferro_markup_type!(class HookSilent {
        handles: [HookSilent, Rc<HookSilent>, Option<Rc<HookSilent>>],
        this: Rc<HookSilent>,
        properties: [
            Value: i32 {
                get: |silent: &Rc<HookSilent>| silent.value.get(),
                set: |silent: &Rc<HookSilent>, value: i32| silent.value.set(value)
            },
        ],
    });

    fn path_of(name: &str) -> crate::data::CompiledBindingPath {
        let property = <HookVm as MarkupTyped>::MARKUP.find_property(name).expect("the property");
        let hook = property.typed_path_element.expect("the hook");
        hook(&CompiledBindingPathBuilder::new(), false).expect("the typed element").build()
    }

    #[test]
    fn a_property_of_a_notifying_shared_type_has_a_typed_path_element() {
        let vm = HookVm::new("Ann");
        let target = Target::new();
        target.set_data_context(Some(vm.clone()));

        let binding = CompiledBinding::new(path_of("Name")).with_mode(BindingMode::TwoWay);
        let expression = target.bind_binding(Target::maybe_text_property(), &binding);
        assert!(expression.as_any().is::<TypedBindingExpression<HookVm, Option<String>>>());
        assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Ann")));

        // Change notifications of the source and writes of the target.
        vm.set_name(Some(s("Bob")));
        assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Bob")));
        target.set_value(Target::maybe_text_property(), Some(s("Cid")));
        assert_eq!(vm.name(), Some(s("Cid")));
        vm.set_name(None);
        assert_eq!(target.get_value(Target::maybe_text_property()), None);

        let other = HookVm::new("Dan");
        target.set_data_context(Some(other));
        assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Dan")));
        assert_eq!(vm.property_changed.handler_count(), 0);
    }

    #[test]
    fn the_typed_element_is_a_plain_property_where_the_typed_expression_does_not_apply() {
        // A binding with a source is not a data context binding: the untyped expression
        // reads and writes through the same description, by the handle of the owner.
        let vm = HookVm::new("Ann");
        let target = Target::new();
        let binding = CompiledBinding::new(path_of("Name"))
            .with_source(Some(vm.clone() as BoxedValue))
            .with_mode(BindingMode::TwoWay);
        let expression = target.bind_binding(Target::maybe_text_property(), &binding);
        assert!(expression.as_any().is::<BindingExpression>());
        assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Ann")));
        vm.set_name(Some(s("Bob")));
        assert_eq!(target.get_value(Target::maybe_text_property()), Some(s("Bob")));
        target.set_value(Target::maybe_text_property(), Some(s("Cid")));
        assert_eq!(vm.name(), Some(s("Cid")));
    }

    #[test]
    fn a_boolean_read_into_an_untyped_property_uses_the_cached_boxes() {
        let target = Target::new();
        let expression = target.bind_binding(Target::tag_property(), &CompiledBinding::new(path_of("Flag")));
        assert!(expression.as_any().is::<TypedBindingExpression<HookVm, bool>>());

        let with_flag = |value: bool| {
            let vm = HookVm::new("x");
            vm.flag.set(value);
            vm
        };
        target.set_data_context(Some(with_flag(true)));
        let boxed_true = target.get_value(Target::tag_property()).expect("a value");
        assert_eq!(boxed_true.downcast_ref::<bool>(), Some(&true));
        target.set_data_context(Some(with_flag(false)));
        let boxed_false = target.get_value(Target::tag_property()).expect("a value");
        assert_eq!(boxed_false.downcast_ref::<bool>(), Some(&false));

        target.set_data_context(Some(with_flag(true)));
        assert!(Rc::ptr_eq(&boxed_true, &target.get_value(Target::tag_property()).unwrap()));
        target.set_data_context(Some(with_flag(false)));
        assert!(Rc::ptr_eq(&boxed_false, &target.get_value(Target::tag_property()).unwrap()));
        assert!(Rc::ptr_eq(&boxed_true, &crate::utilities::BooleanBoxes::true_()));
    }

    #[test]
    fn a_failing_getter_has_no_value_and_a_fallible_setter_has_no_typed_form() {
        let vm = HookVm::new("Ann");
        vm.count.set(4);
        let target = Target::new();
        target.set_data_context(Some(vm.clone()));
        let plain = FerroProperty::register::<Target, i32>("HookCount", -7);
        let expression = target.bind_binding(plain, &CompiledBinding::new(path_of("Count")));
        assert!(expression.as_any().is::<TypedBindingExpression<HookVm, i32>>());
        assert_eq!(target.get_value(plain), 4);
        vm.count.set(-1);
        vm.property_changed.raise("Count");
        assert_eq!(target.get_value(plain), -7);

        let markup = <HookVm as MarkupTyped>::MARKUP;
        assert!(markup.find_property("Guarded").unwrap().typed_path_element.is_none());
        // A property without a getter still has the typed element (it can be written).
        let write_only = markup.find_property("WriteOnly").unwrap().typed_path_element.unwrap();
        assert!(write_only(&CompiledBindingPathBuilder::new(), false).is_some());
    }

    #[test]
    fn other_declarations_have_no_typed_path_element() {
        let builder = CompiledBindingPathBuilder::new();
        // A value type: no shared handle.
        let plain = <HookPlain as MarkupTyped>::MARKUP.find_property("Value").unwrap();
        assert!((plain.typed_path_element.expect("the hook"))(&builder, false).is_none());
        // Nothing was added to the path.
        assert_eq!(builder.build().elements().count(), 0);
    }

    #[test]
    fn a_property_of_a_shared_type_that_does_not_notify_has_a_typed_path_element() {
        let silent = <HookSilent as MarkupTyped>::MARKUP.find_property("Value").unwrap();
        let hook = silent.typed_path_element.expect("the hook");
        let path = hook(&CompiledBindingPathBuilder::new(), false).expect("the typed element").build();
        let source = Rc::new(HookSilent { value: Cell::new(1) });
        let target = Target::new();
        target.set_data_context(Some(source.clone()));
        let plain = FerroProperty::register::<Target, i32>("HookSilentValue", 0);
        let expression = target.bind_binding(plain, &CompiledBinding::new(path).with_mode(BindingMode::TwoWay));
        assert!(expression.as_any().is::<TypedBindingExpression<HookSilent, i32>>());
        assert_eq!(target.get_value(plain), 1);
        // The source does not notify: the target writes, a new source is read.
        target.set_value(plain, 5);
        assert_eq!(source.value.get(), 5);
        let other = Rc::new(HookSilent { value: Cell::new(9) });
        target.set_data_context(Some(other));
        assert_eq!(target.get_value(plain), 9);
    }

    /// Not from upstream: the owner of a plain property may come as a box of
    /// its handle, the form in which an items control gives an item to its
    /// container as data context, and the value is converted to the type of
    /// the target property (the untyped expression).
    #[test]
    fn a_plain_property_is_read_from_a_handle_of_its_owner_through_the_untyped_expression() {
        let silent = <HookSilent as MarkupTyped>::MARKUP.find_property("Value").unwrap();
        let hook = silent.typed_path_element.expect("the hook");
        let converted = FerroProperty::register::<Target, f64>("HookSilentConverted", -1.0);

        // The box is the object.
        let path = hook(&CompiledBindingPathBuilder::new(), false).expect("the typed element").build();
        let target = Target::new();
        let expression = target.bind_binding(converted, &CompiledBinding::new(path));
        assert!(expression.as_any().is::<BindingExpression>());
        target.set_data_context(Some(Rc::new(HookSilent { value: Cell::new(3) })));
        assert_eq!(target.get_value(converted), 3.0);

        // The box holds a handle of the object.
        let path = hook(&CompiledBindingPathBuilder::new(), false).expect("the typed element").build();
        let target = Target::new();
        let source = Rc::new(HookSilent { value: Cell::new(5) });
        target.bind_binding(converted, &CompiledBinding::new(path).with_mode(BindingMode::TwoWay));
        let handle: BoxedValue = Rc::new(source.clone());
        target.set_data_context(Some(handle));
        assert_eq!(target.get_value(converted), 5.0);
        // And is written through the same handle.
        target.set_value(converted, 7.0);
        assert_eq!(source.value.get(), 7);
    }
}
