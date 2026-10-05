//! Shared fixture of the binding expression tests: the view models, the
//! target and source classes, and the creation of binding expressions in the
//! two flavours the upstream fixture is parameterised over (a compiled path
//! of typed accessors, and a string path resolved at run time).

use super::*;
use crate::data::converters::IValueConverter;
use crate::data::core::expression_nodes::{CastTarget, DataContextNode, ExpressionNode};
use crate::data::core::parsers::{BindingExpressionGrammar, ExpressionNodeFactory, TypeResolver};
use crate::data::core::plugins::{ObservableValue, PropertyAccessorFactory, PropertyInfoAccessorFactory};
use crate::data::core::{
    BindingExpression, BindingExpressionOptions, ClrPropertyInfo, IPropertyInfo, Maybe, ModelRef, PropertyKind,
    TargetTypeConverter, Untyped, Value, ValueType, ValueTypes,
};
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{
    BindingError, BindingErrorType, BindingMode, BindingValueType, CompiledBindingPathBuilder, UpdateSourceTrigger,
};
use crate::input::{FocusManager, IInputRoot, InputElement, InputElementImpl, KeyboardDevice, IKeyboardDevice};
use crate::interactivity::InteractiveImpl;
use crate::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl};
use crate::rendering::{IHitTester, IPresentationSource, IRenderer, ManagedHitTester};
use crate::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, AttachedProperty, DirectProperty,
    FerroLocator, FerroObject, FerroObjectImpl, FerroProperty, Rect, Ref, Size, StaticType, StyledElement, StyledElementImpl,
    StyledProperty, TypeInfo, Visual, VisualImpl,
};
use std::collections::HashMap;
use std::rc::Weak;

/// The two kinds of binding path the tests run against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    /// A compiled path: typed accessors, no lookup by name.
    Compiled,
    /// A string path, parsed and resolved at run time.
    Reflection,
}

/// Declares each test once and runs it for both flavours.
macro_rules! binding_tests {
    ($(fn $name:ident($f:ident) $body:block)*) => {
        $(fn $name($f: $crate::tests::binding_test_support::Flavor) $body)*

        mod compiled {
            $(#[test] fn $name() { super::$name($crate::tests::binding_test_support::Flavor::Compiled) })*
        }

        mod reflection {
            $(#[test] fn $name() { super::$name($crate::tests::binding_test_support::Flavor::Reflection) })*
        }
    };
}
pub(crate) use binding_tests;

// --- view models ------------------------------------------------------------

/// The notifying view model of the fixture.
pub struct ViewModel {
    bool_value: Cell<bool>,
    double_value: Cell<f64>,
    int_value: Cell<i32>,
    object_value: RefCell<Option<BoxedValue>>,
    string_value: RefCell<Option<String>>,
    next: RefCell<Option<Rc<ViewModel>>>,
    next_observable: RefCell<Option<ObservableValue>>,
    pub property_changed: Event<str>,
}

impl ViewModel {
    pub fn new() -> Rc<Self> {
        Model::new_model(Self {
            bool_value: Cell::new(false),
            double_value: Cell::new(0.0),
            int_value: Cell::new(0),
            object_value: RefCell::new(None),
            string_value: RefCell::new(None),
            next: RefCell::new(None),
            next_observable: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    pub fn with_string(value: &str) -> Rc<Self> {
        let result = Self::new();
        result.set_string_value(Some(s(value)));
        result
    }

    pub fn with_next(next: Rc<ViewModel>) -> Rc<Self> {
        let result = Self::new();
        result.set_next(Some(next));
        result
    }

    pub fn with_bool(value: bool) -> Rc<Self> {
        let result = Self::new();
        result.set_bool_value(value);
        result
    }

    pub fn with_int(value: i32) -> Rc<Self> {
        let result = Self::new();
        result.set_int_value(value);
        result
    }

    pub fn with_double(value: f64) -> Rc<Self> {
        let result = Self::new();
        result.set_double_value(value);
        result
    }

    pub fn bool_value(&self) -> bool {
        self.bool_value.get()
    }

    pub fn set_bool_value(&self, value: bool) {
        self.bool_value.set(value);
        self.raise_property_changed("BoolValue");
    }

    pub fn int_value(&self) -> i32 {
        self.int_value.get()
    }

    pub fn set_int_value(&self, value: i32) {
        self.int_value.set(value);
        self.raise_property_changed("IntValue");
    }

    pub fn double_value(&self) -> f64 {
        self.double_value.get()
    }

    pub fn set_double_value(&self, value: f64) {
        self.double_value.set(value);
        self.raise_property_changed("DoubleValue");
    }

    pub fn object_value(&self) -> Option<BoxedValue> {
        self.object_value.borrow().clone()
    }

    pub fn set_object_value(&self, value: Option<BoxedValue>) {
        self.object_value.replace(value);
        self.raise_property_changed("ObjectValue");
    }

    pub fn string_value(&self) -> Option<String> {
        self.string_value.borrow().clone()
    }

    pub fn set_string_value(&self, value: Option<String>) {
        self.string_value.replace(value);
        self.raise_property_changed("StringValue");
    }

    pub fn next(&self) -> Option<Rc<ViewModel>> {
        self.next.borrow().clone()
    }

    pub fn set_next(&self, value: Option<Rc<ViewModel>>) {
        self.next.replace(value);
        self.raise_property_changed("Next");
    }

    pub fn next_observable(&self) -> Option<ObservableValue> {
        self.next_observable.borrow().clone()
    }

    pub fn set_next_observable(&self, value: Option<ObservableValue>) {
        self.next_observable.replace(value);
        self.raise_property_changed("NextObservable");
    }

    pub fn set_string_value_without_raising(&self, value: &str) {
        self.string_value.replace(Some(s(value)));
    }

    /// Raises the property change notification; an empty name means that all
    /// properties changed.
    pub fn raise_property_changed(&self, name: &str) {
        self.property_changed.raise(name);
    }

    // The steps of binding paths over the view model.

    pub fn bool_step() -> Step {
        inpc_prop::<ViewModel, Value<bool>>("BoolValue", Out::Bool, |o| o.bool_value(), |o, v| o.set_bool_value(v))
    }

    pub fn int_step() -> Step {
        inpc_prop::<ViewModel, Value<i32>>("IntValue", Out::Int, |o| o.int_value(), |o, v| o.set_int_value(v))
    }

    pub fn double_step() -> Step {
        inpc_prop::<ViewModel, Value<f64>>("DoubleValue", Out::Double, |o| o.double_value(), |o, v| {
            o.set_double_value(v)
        })
    }

    pub fn object_step() -> Step {
        inpc_prop::<ViewModel, Untyped>("ObjectValue", Out::Object, |o| o.object_value(), |o, v| {
            o.set_object_value(v)
        })
    }

    pub fn string_step() -> Step {
        inpc_prop::<ViewModel, Maybe<String>>("StringValue", Out::String, |o| o.string_value(), |o, v| {
            o.set_string_value(v)
        })
    }

    pub fn next_step() -> Step {
        inpc_prop::<ViewModel, ModelRef<ViewModel>>("Next", Out::Object, |o| o.next(), |o, v| o.set_next(v))
    }

    pub fn next_observable_step() -> Step {
        inpc_prop::<ViewModel, Maybe<ObservableValue>>(
            "NextObservable",
            Out::Object,
            |o| o.next_observable(),
            |o, v| o.set_next_observable(v),
        )
    }
}

impl INotifyPropertyChanged for ViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ViewModel, |b| b
    .notify_property_changed()
    .property::<Value<bool>>("BoolValue", |o| o.bool_value(), |o, v| o.set_bool_value(v))
    .property::<Value<i32>>("IntValue", |o| o.int_value(), |o, v| o.set_int_value(v))
    .property::<Value<f64>>("DoubleValue", |o| o.double_value(), |o, v| o.set_double_value(v))
    .property::<Untyped>("ObjectValue", |o| o.object_value(), |o, v| o.set_object_value(v))
    .property::<Maybe<String>>("StringValue", |o| o.string_value(), |o, v| o.set_string_value(v))
    .property::<ModelRef<ViewModel>>("Next", |o| o.next(), |o, v| o.set_next(v))
    .property::<Maybe<ObservableValue>>("NextObservable", |o| o.next_observable(), |o, v| o
        .set_next_observable(v)));

/// A view model that does not raise change notifications.
pub struct PodViewModel {
    string_value: RefCell<Option<String>>,
}

impl PodViewModel {
    pub fn new(value: Option<&str>) -> Rc<Self> {
        Model::new_model(Self { string_value: RefCell::new(value.map(s)) })
    }

    pub fn string_value(&self) -> Option<String> {
        self.string_value.borrow().clone()
    }

    pub fn set_string_value(&self, value: Option<String>) {
        self.string_value.replace(value);
    }

    pub fn string_step() -> Step {
        plain_prop::<PodViewModel, Maybe<String>>("StringValue", Out::String, |o| o.string_value(), |o, v| {
            o.set_string_value(v)
        })
    }
}

ferro_model!(PodViewModel, |b| b.property::<Maybe<String>>("StringValue", |o| o.string_value(), |o, v| o
    .set_string_value(v)));

// --- classes ----------------------------------------------------------------

static_type!(AttachedProperties);

impl AttachedProperties {
    ferro_property!(pub fn attached_string_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<AttachedProperties, FerroObject, _>("AttachedString", None)
    });

    pub fn attached_string_step() -> Step {
        Step::Attached { owner: AttachedProperties::TYPE, property: Self::attached_string_property() }
    }
}

/// A source object of the class hierarchy.
#[repr(C)]
pub struct SourceControl {
    base: StyledElement,
    clr_property: RefCell<Option<String>>,
}

ferro_class!(SourceControl: StyledElement);
ferro_impl_classes!(SourceControl: FerroObjectImpl, StyledElementImpl);

impl SourceControl {
    ferro_property!(pub fn next_property() -> StyledProperty<Option<Ref<SourceControl>>> {
        FerroProperty::register::<SourceControl, _>("Next", None)
    });
    ferro_property!(pub fn string_value_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<SourceControl, _>("StringValue", None)
    });

    pub fn new() -> Ref<Self> {
        once_per_thread!({
            ValueTypes::register_object::<SourceControl>();
            crate::data::model::ModelTypes::register::<Ref<SourceControl>>(|b| {
                b.property::<Maybe<String>>("ClrProperty", |o| o.clr_property(), |o, v| o.set_clr_property(v))
            });
            // The properties are looked up by name by string paths.
            Self::next_property();
            Self::string_value_property();
            AttachedProperties::attached_string_property();
        });
        instantiate(Self { base: StyledElement::construct(), clr_property: RefCell::new(None) })
    }

    pub fn next(&self) -> Option<Ref<SourceControl>> {
        self.get_value(Self::next_property())
    }

    pub fn set_next(&self, value: Option<Ref<SourceControl>>) {
        self.set_value(Self::next_property(), value)
    }

    pub fn string_value(&self) -> Option<String> {
        self.get_value(Self::string_value_property())
    }

    pub fn set_string_value(&self, value: Option<String>) {
        self.set_value(Self::string_value_property(), value)
    }

    pub fn clr_property(&self) -> Option<String> {
        self.clr_property.borrow().clone()
    }

    pub fn set_clr_property(&self, value: Option<String>) {
        self.clr_property.replace(value);
    }

    pub fn next_step() -> Step {
        Step::Ferro(Self::next_property())
    }

    pub fn string_step() -> Step {
        Step::Ferro(Self::string_value_property()).with_out(Out::String)
    }

    pub fn clr_step() -> Step {
        plain_prop::<Ref<SourceControl>, Maybe<String>>("ClrProperty", Out::String, |o| o.clr_property(), |o, v| {
            o.set_clr_property(v)
        })
    }
}

/// The target of the bindings.
#[repr(C)]
pub struct TargetClass {
    base: InputElement,
    read_only_string: RefCell<Option<String>>,
    binding_notifications: RefCell<HashMap<u32, (BindingErrorType, BindingError)>>,
}

ferro_class!(TargetClass: InputElement);
ferro_impl_classes!(TargetClass: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl FerroObjectImpl for TargetClass {
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        Self::parent_update_data_validation(this, property, state, error);

        let type_ = match state {
            BindingValueType::BINDING_ERROR | BindingValueType::BINDING_ERROR_WITH_FALLBACK => BindingErrorType::Error,
            BindingValueType::DATA_VALIDATION_ERROR | BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK => {
                BindingErrorType::DataValidationError
            }
            _ => BindingErrorType::None,
        };

        match error {
            Some(error) if type_ != BindingErrorType::None => {
                this.binding_notifications.borrow_mut().insert(property.id(), (type_, error.clone()));
            }
            _ => {
                this.binding_notifications.borrow_mut().remove(&property.id());
            }
        }
    }
}

use crate::FerroObjectImplExt;

impl TargetClass {
    ferro_property!(pub fn bool_property() -> StyledProperty<bool> {
        FerroProperty::register::<TargetClass, _>("Bool", false)
    });
    ferro_property!(pub fn double_property() -> StyledProperty<f64> {
        FerroProperty::register::<TargetClass, _>("Double", 0.0)
    });
    ferro_property!(pub fn int_property() -> StyledProperty<i32> {
        FerroProperty::register::<TargetClass, _>("Int", 0)
    });
    ferro_property!(pub fn object_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<TargetClass, _>("Object", None)
    });
    ferro_property!(pub fn string_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<TargetClass, _>("String", None)
    });
    // The equivalent of the `Tag` property of the upstream control class.
    ferro_property!(pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<TargetClass, _>("Tag", None)
    });
    ferro_property!(pub fn read_only_string_property() -> DirectProperty<TargetClass, Option<String>> {
        FerroProperty::register_direct::<TargetClass, _>("ReadOnlyString", |o| o.read_only_string(), None, None)
    });

    pub fn new() -> Ref<Self> {
        once_per_thread!({
            InputElement::focusable_property().override_default_value::<TargetClass>(true);
        });
        instantiate(Self {
            base: InputElement::construct(),
            read_only_string: RefCell::new(Some(s("readonly"))),
            binding_notifications: RefCell::new(HashMap::new()),
        })
    }

    pub fn bool(&self) -> bool {
        self.get_value(Self::bool_property())
    }

    pub fn set_bool(&self, value: bool) {
        self.set_value(Self::bool_property(), value)
    }

    pub fn double(&self) -> f64 {
        self.get_value(Self::double_property())
    }

    pub fn set_double(&self, value: f64) {
        self.set_value(Self::double_property(), value)
    }

    pub fn int(&self) -> i32 {
        self.get_value(Self::int_property())
    }

    pub fn set_int(&self, value: i32) {
        self.set_value(Self::int_property(), value)
    }

    pub fn object(&self) -> Option<BoxedValue> {
        self.get_value(Self::object_property())
    }

    pub fn set_object(&self, value: Option<BoxedValue>) {
        self.set_value(Self::object_property(), value)
    }

    pub fn string(&self) -> Option<String> {
        self.get_value(Self::string_property())
    }

    pub fn set_string(&self, value: Option<&str>) {
        self.set_value(Self::string_property(), value.map(s))
    }

    pub fn tag(&self) -> Option<BoxedValue> {
        self.get_value(Self::tag_property())
    }

    pub fn set_tag(&self, value: Option<BoxedValue>) {
        self.set_value(Self::tag_property(), value)
    }

    pub fn read_only_string(&self) -> Option<String> {
        self.read_only_string.borrow().clone()
    }

    pub fn set_read_only_string(&self, value: Option<&str>) {
        self.set_and_raise(Self::read_only_string_property(), &self.read_only_string, value.map(s));
    }

    /// The error reported for a property through data validation, if any.
    pub fn binding_notification(&self, property: &'static FerroProperty) -> Option<(BindingErrorType, BindingError)> {
        self.binding_notifications.borrow().get(&property.id()).cloned()
    }
}

/// The text form of a value as the converters of the fixture see it: an
/// object of the class hierarchy reads as the name of its class.
pub fn to_text(value: Option<&BoxedValue>) -> Option<String> {
    let value = value?;
    if let Some(o) = ValueTypes::as_object(&**value) {
        return Some(o.get_type().name().to_string());
    }
    Some(ValueTypes::to_display_string(Some(value)))
}

/// Prepends a prefix (the converter parameter, or its own) to text.
pub struct PrefixConverter {
    pub prefix: RefCell<Option<String>>,
}

impl PrefixConverter {
    pub fn new(prefix: Option<&str>) -> Rc<Self> {
        Rc::new(Self { prefix: RefCell::new(prefix.map(s)) })
    }
}

fn is_string_type(target_type: ValueType) -> bool {
    target_type.is::<String>() || target_type.is::<Option<String>>()
}

impl IValueConverter for PrefixConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if !is_string_type(target_type) {
            return Ok(value.cloned());
        }
        let mut result = to_text(value).unwrap_or_default();
        let prefix = to_text(parameter).or_else(|| self.prefix.borrow().clone());
        if let Some(prefix) = prefix {
            result = prefix + &result;
        }
        Ok(Some(boxed(result)))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(prefix) = to_text(parameter).filter(|_| is_string_type(target_type)) else {
            return Ok(value.cloned());
        };
        let text = to_text(value).unwrap_or_default();
        match text.strip_prefix(&prefix) {
            Some(rest) => Ok(Some(boxed(rest.to_string()))),
            None => Ok(value.cloned()),
        }
    }
}

// --- binding paths ----------------------------------------------------------

/// The target property a path binds to when the test names none: chosen by
/// the type of the value the path produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    Bool,
    Double,
    Int,
    String,
    Object,
}

/// One element of a binding path, in a form that can produce both flavours.
#[derive(Clone)]
pub enum Step {
    /// A plain property.
    Prop { info: Rc<dyn IPropertyInfo>, factory: PropertyAccessorFactory, out: Out },
    /// A registered property.
    Ferro(&'static FerroProperty),
    /// A registered property with the type of the value it produces.
    FerroOut(&'static FerroProperty, Out),
    /// An attached property; string paths name it `(Owner.Name)`.
    Attached { owner: &'static TypeInfo, property: &'static FerroProperty },
    /// The stream operator.
    Stream(Out),
    /// A cast; string paths leave it out, as they resolve members at run
    /// time.
    Cast(CastTarget),
}

impl Step {
    pub fn with_out(self, out: Out) -> Step {
        match self {
            Step::Prop { info, factory, .. } => Step::Prop { info, factory, out },
            Step::Ferro(p) | Step::FerroOut(p, _) => Step::FerroOut(p, out),
            Step::Stream(_) => Step::Stream(out),
            other => other,
        }
    }

    fn out(&self) -> Out {
        match self {
            Step::Prop { out, .. } | Step::FerroOut(_, out) | Step::Stream(out) => *out,
            Step::Ferro(_) | Step::Attached { .. } | Step::Cast(_) => Out::Object,
        }
    }
}

/// A step over a property of a model that raises change notifications.
pub fn inpc_prop<O: INotifyPropertyChanged + PartialEq + 'static, K: PropertyKind>(
    name: &str,
    out: Out,
    get: impl Fn(&O) -> K::Typed + 'static,
    set: impl Fn(&O, K::Typed) + 'static,
) -> Step {
    ValueTypes::register_reference::<O>();
    Step::Prop {
        info: Rc::new(ClrPropertyInfo::read_write::<O, K>(name, get, set)),
        factory: PropertyInfoAccessorFactory::create_inpc_property_accessor::<O>(),
        out,
    }
}

/// A step over a property of a model whose setter can reject values.
pub fn inpc_validated_prop<O: INotifyPropertyChanged + PartialEq + 'static, K: PropertyKind>(
    name: &str,
    out: Out,
    get: impl Fn(&O) -> K::Typed + 'static,
    set: impl Fn(&O, K::Typed) -> Result<(), BindingError> + 'static,
) -> Step {
    ValueTypes::register_reference::<O>();
    Step::Prop {
        info: Rc::new(ClrPropertyInfo::read_write_validated::<O, K>(name, get, set)),
        factory: PropertyInfoAccessorFactory::create_inpc_property_accessor::<O>(),
        out,
    }
}

/// A step over a property of an object that does not notify.
pub fn plain_prop<O: 'static, K: PropertyKind>(
    name: &str,
    out: Out,
    get: impl Fn(&O) -> K::Typed + 'static,
    set: impl Fn(&O, K::Typed) + 'static,
) -> Step {
    Step::Prop {
        info: Rc::new(ClrPropertyInfo::read_write::<O, K>(name, get, set)),
        factory: PropertyInfoAccessorFactory::create_plain_property_accessor(),
        out,
    }
}

/// A read-only step over a property of an object that does not notify.
pub fn plain_read_only_prop<O: 'static, K: PropertyKind>(
    name: &str,
    out: Out,
    get: impl Fn(&O) -> K::Typed + 'static,
) -> Step {
    Step::Prop {
        info: Rc::new(ClrPropertyInfo::read_only::<O, K>(name, get)),
        factory: PropertyInfoAccessorFactory::create_plain_property_accessor(),
        out,
    }
}

/// A binding path: what the upstream tests write as a lambda.
#[derive(Clone)]
pub struct Path {
    negations: usize,
    steps: Vec<(Step, bool)>,
    out: Out,
}

impl Path {
    /// The empty path (`o => o`), producing a value for the given target.
    pub fn empty(out: Out) -> Self {
        Self { negations: 0, steps: Vec::new(), out }
    }

    /// A path of one step.
    pub fn of(step: Step) -> Self {
        Self::empty(Out::Object).then(step)
    }

    pub fn then(mut self, step: Step) -> Self {
        self.out = step.out();
        self.steps.push((step, false));
        self
    }

    /// Adds a step reached through the null-conditional operator (`?.`).
    pub fn then_null_conditional(mut self, step: Step) -> Self {
        self.out = step.out();
        self.steps.push((step, true));
        self
    }

    /// Negates the result of the path.
    pub fn not(mut self) -> Self {
        self.negations += 1;
        self.out = Out::Bool;
        self
    }

    pub fn out(&self) -> Out {
        self.out
    }

    /// The path as text.
    pub fn text(&self) -> String {
        let mut result = "!".repeat(self.negations);
        let mut first = true;
        for (step, accepts_null) in &self.steps {
            let separator = if *accepts_null { "?." } else { "." };
            match step {
                Step::Prop { info, .. } => {
                    if !first {
                        result.push_str(separator);
                    }
                    result.push_str(info.name());
                }
                Step::Ferro(p) | Step::FerroOut(p, _) => {
                    if !first {
                        result.push_str(separator);
                    }
                    result.push_str(p.name());
                }
                Step::Attached { owner, property } => {
                    if !first {
                        result.push_str(separator);
                    }
                    result.push_str(&format!("({}.{})", owner.name(), property.name()));
                }
                Step::Stream(_) => result.push('^'),
                Step::Cast(_) => continue,
            }
            first = false;
        }
        result
    }

    /// Resolves the owners of the attached properties of the path.
    pub fn type_resolver(&self) -> TypeResolver {
        let owners: Vec<&'static TypeInfo> = self
            .steps
            .iter()
            .filter_map(|(step, _)| match step {
                Step::Attached { owner, .. } => Some(*owner),
                _ => None,
            })
            .collect();
        Rc::new(move |_, name| owners.iter().find(|t| t.name() == name).map(|t| CastTarget::Class(t)))
    }

    /// The path as a compiled binding path.
    pub fn compiled(&self) -> crate::data::CompiledBindingPath {
        let mut builder = CompiledBindingPathBuilder::new();
        for _ in 0..self.negations {
            builder = builder.not();
        }
        for (step, accepts_null) in &self.steps {
            builder = match step {
                Step::Prop { info, factory, .. } => builder.property_with(info.clone(), factory.clone(), *accepts_null),
                Step::Ferro(p) | Step::FerroOut(p, _) => builder.ferro_property_with(p, *accepts_null),
                Step::Attached { property, .. } => builder.ferro_property_with(property, *accepts_null),
                Step::Stream(_) => builder.stream_observable(),
                Step::Cast(target) => builder.type_cast_value(*target),
            };
        }
        builder.build()
    }

    /// The expression nodes of the path.
    pub fn nodes(&self, flavor: Flavor) -> Vec<Rc<dyn ExpressionNode>> {
        let mut nodes: Vec<Rc<dyn ExpressionNode>> = Vec::new();
        match flavor {
            Flavor::Compiled => {
                self.compiled().build_expression(&mut nodes);
            }
            Flavor::Reflection => {
                let path = self.text();
                if !path.is_empty() {
                    let (ast, _) = BindingExpressionGrammar::parse(&path).expect("the path parses");
                    ExpressionNodeFactory::create_from_ast(&ast, Some(&self.type_resolver()), None, &mut nodes)
                        .expect("the path resolves");
                }
            }
        }
        nodes
    }
}

// --- creating targets -------------------------------------------------------

/// The optional arguments of the creation functions.
pub struct Opts {
    pub target_property: Option<&'static FerroProperty>,
    pub converter: Option<Rc<dyn IValueConverter>>,
    pub converter_parameter: Option<BoxedValue>,
    pub data_context: Option<BoxedValue>,
    pub enable_data_validation: bool,
    /// The fallback value; the unset marker for none.
    pub fallback_value: Option<BoxedValue>,
    pub mode: BindingMode,
    /// Binds relative to the target itself.
    pub relative_source_self: bool,
    /// The binding source; `None` binds to the data context.
    pub source: Option<Option<BoxedValue>>,
    pub target_null_value: Option<BoxedValue>,
    pub string_format: Option<String>,
    pub update_source_trigger: UpdateSourceTrigger,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            target_property: None,
            converter: None,
            converter_parameter: None,
            data_context: None,
            enable_data_validation: false,
            fallback_value: Some(FerroProperty::unset_value()),
            mode: BindingMode::OneWay,
            relative_source_self: false,
            source: None,
            target_null_value: None,
            string_format: None,
            update_source_trigger: UpdateSourceTrigger::PropertyChanged,
        }
    }
}

impl Opts {
    pub fn mode(mode: BindingMode) -> Self {
        Self { mode, ..Self::default() }
    }

    pub fn property(target_property: &'static FerroProperty) -> Self {
        Self { target_property: Some(target_property), ..Self::default() }
    }

    pub fn with_mode(mut self, mode: BindingMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_property(mut self, target_property: &'static FerroProperty) -> Self {
        self.target_property = Some(target_property);
        self
    }

    pub fn with_data_context(mut self, data_context: Option<BoxedValue>) -> Self {
        self.data_context = data_context;
        self
    }

    pub fn with_data_validation(mut self, enable: bool) -> Self {
        self.enable_data_validation = enable;
        self
    }

    pub fn with_converter(mut self, converter: Rc<dyn IValueConverter>) -> Self {
        self.converter = Some(converter);
        self
    }

    pub fn with_converter_parameter(mut self, parameter: &str) -> Self {
        self.converter_parameter = Some(boxed(s(parameter)));
        self
    }

    pub fn with_fallback_value(mut self, value: BoxedValue) -> Self {
        self.fallback_value = Some(value);
        self
    }

    pub fn with_target_null_value(mut self, value: BoxedValue) -> Self {
        self.target_null_value = Some(value);
        self
    }

    pub fn with_string_format(mut self, format: &str) -> Self {
        self.string_format = Some(s(format));
        self
    }

    pub fn with_update_source_trigger(mut self, trigger: UpdateSourceTrigger) -> Self {
        self.update_source_trigger = trigger;
        self
    }

    pub fn with_source(mut self, source: Option<BoxedValue>) -> Self {
        self.source = Some(source);
        self
    }
}

/// A shared model object as a binding source or data context.
pub fn src<T: PartialEq + 'static>(value: &Rc<T>) -> Option<BoxedValue> {
    Some(value.clone())
}

/// Creates a target bound to the data context (or the source of `opts`).
pub fn create_target(flavor: Flavor, path: &Path, opts: Opts) -> Ref<TargetClass> {
    create_target_and_expression(flavor, path, opts).0
}

/// Creates a target bound to `source`.
pub fn create_target_with_source(
    flavor: Flavor,
    source: Option<BoxedValue>,
    path: &Path,
    opts: Opts,
) -> Ref<TargetClass> {
    create_target_and_expression(flavor, path, opts.with_source(source)).0
}

/// Creates a target and the binding expression bound to it, the way the
/// upstream fixture does: the expression is built from the nodes of the path
/// and added to the value store of the target.
pub fn create_target_and_expression(flavor: Flavor, path: &Path, opts: Opts) -> (Ref<TargetClass>, Rc<BindingExpression>) {
    let target_property = opts.target_property.unwrap_or_else(|| match path.out() {
        Out::Bool => TargetClass::bool_property().as_property(),
        Out::Double => TargetClass::double_property().as_property(),
        Out::Int => TargetClass::int_property().as_property(),
        Out::String => TargetClass::string_property().as_property(),
        Out::Object => TargetClass::object_property().as_property(),
    });

    let target = TargetClass::new();
    target.set_data_context(opts.data_context.clone());

    let mut nodes = path.nodes(flavor);
    if opts.source.is_none() && !opts.relative_source_self {
        nodes.insert(0, DataContextNode::new());
    }

    let source = match &opts.source {
        Some(source) => source.clone(),
        None => Some(Rc::new(target.clone().upcast::<FerroObject>()) as BoxedValue),
    };

    let expression = BindingExpression::new(
        source,
        nodes,
        BindingExpressionOptions {
            fallback_value: opts.fallback_value,
            converter: opts.converter,
            converter_parameter: opts.converter_parameter,
            enable_data_validation: opts.enable_data_validation,
            mode: opts.mode,
            target_null_value: opts.target_null_value,
            target_type_converter: Some(TargetTypeConverter::get_reflection_converter()),
            string_format: opts.string_format,
            update_source_trigger: opts.update_source_trigger,
            ..BindingExpressionOptions::default()
        },
    );

    target.values().add_binding_expression(&target, target_property, expression.clone());
    (target, expression)
}

// --- data validation assertions --------------------------------------------

/// Asserts that no error is reported for `property`.
#[track_caller]
pub fn assert_no_error(target: &TargetClass, property: &'static FerroProperty) {
    let notification = target.binding_notification(property);
    assert!(notification.is_none(), "unexpected error: {:?}", notification.map(|n| n.1.to_string()));
}

/// Asserts that an error with the given message is reported for `property`.
#[track_caller]
pub fn assert_binding_error(
    target: &TargetClass,
    property: &'static FerroProperty,
    expected_message: &str,
    error_type: BindingErrorType,
) -> BindingError {
    let (type_, error) = target.binding_notification(property).expect("an error is reported");
    assert_eq!(type_, error_type);
    assert_eq!(error.to_string(), expected_message);
    error
}

/// Asserts that a broken binding chain is reported for `property`.
#[track_caller]
pub fn assert_binding_chain_error(
    target: &TargetClass,
    property: &'static FerroProperty,
    message: &str,
    expression: &str,
    error_point: &str,
) {
    let expected = crate::data::BindingChainException::with_expression(message, expression, error_point);
    let error = assert_binding_error(target, property, &expected.message(), BindingErrorType::Error);
    let chain = error
        .inner()
        .downcast_ref::<crate::data::BindingChainException>()
        .expect("the error is a binding chain error");
    assert_eq!(chain.expression(), Some(expression));
    assert_eq!(chain.expression_error_point(), Some(error_point));
}

/// Asserts that a data validation error reported by a model is reported for
/// `property`.
#[track_caller]
pub fn assert_data_validation_error(target: &TargetClass, property: &'static FerroProperty, message: &str) {
    let error = assert_binding_error(target, property, message, BindingErrorType::DataValidationError);
    assert!(error.inner().downcast_ref::<crate::data::DataValidationException>().is_some());
}

// --- docking ----------------------------------------------------------------

/// The equivalent of the upstream docking enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dock {
    Left = 0,
    Bottom = 1,
    Right = 2,
    Top = 3,
}

static_type!(DockPanel);

impl DockPanel {
    ferro_property!(pub fn dock_property() -> AttachedProperty<Dock> {
        ValueTypes::register_conversion::<i32, Dock>(|v| match v {
            0 => Some(Dock::Left),
            1 => Some(Dock::Bottom),
            2 => Some(Dock::Right),
            3 => Some(Dock::Top),
            _ => None,
        });
        ValueTypes::register_conversion::<Dock, i32>(|v| Some(*v as i32));
        FerroProperty::register_attached::<DockPanel, FerroObject, _>("Dock", Dock::Left)
    });

    pub fn dock_step() -> Step {
        Step::Attached { owner: DockPanel::TYPE, property: Self::dock_property() }
    }
}

// --- focus ------------------------------------------------------------------

#[derive(Default)]
struct TestRenderer;

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<crate::rendering::RendererDiagnostics> {
        crate::rendering::RendererDiagnostics::new()
    }
    fn scene_invalidated(
        &self,
        _handler: Rc<dyn Fn(&crate::rendering::SceneInvalidatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        None
    }
    fn add_dirty(&self, _visual: &Visual) {}
    fn recalculate_children(&self, _visual: &Visual) {}
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

/// The root element of a focus test: a logical root and focus scope.
#[repr(C)]
pub struct RootControl {
    base: InputElement,
}

ferro_class!(RootControl: InputElement);
ferro_impl_classes!(RootControl: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl StyledElementImpl for RootControl {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl InputElementImpl for RootControl {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }
}

struct TestHost {
    this: Weak<TestHost>,
    root: Ref<RootControl>,
    renderer: Rc<TestRenderer>,
    layout_manager: RefCell<Option<Rc<LayoutManager>>>,
    focus_manager: Rc<FocusManager>,
    hit_tester: Rc<ManagedHitTester>,
}

impl IPresentationSource for TestHost {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        Some(self.root.clone().upcast())
    }
    fn render_scaling(&self) -> f64 {
        1.0
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().unwrap()
    }
    fn client_size(&self) -> Size {
        self.root.bounds().size()
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        self.hit_tester.clone()
    }
    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }
}

impl ILayoutRoot for TestHost {
    fn layout_scaling(&self) -> f64 {
        1.0
    }
    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.borrow().clone().unwrap()
    }
    fn root_visual(&self) -> Ref<Layoutable> {
        self.root.clone().upcast()
    }
}

impl IInputRoot for TestHost {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        Some(self.focus_manager.clone())
    }
    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}
    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}
    fn root_element(&self) -> Ref<InputElement> {
        self.root.clone().upcast()
    }
    fn focus_root(&self) -> Ref<InputElement> {
        self.root.clone().upcast()
    }
    fn pointer_over_invalidated(&self) {}
}

/// A focusable root hosting one child, with real focus support.
pub struct TestRoot {
    pub root: Ref<RootControl>,
    _host: Rc<TestHost>,
    _keyboard: Rc<KeyboardDevice>,
}

impl TestRoot {
    /// Installs a keyboard device for the current test thread and hosts
    /// `child` in a focusable root.
    pub fn new(child: &Ref<TargetClass>) -> Self {
        let keyboard = KeyboardDevice::new();
        FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(keyboard.clone());

        let root = instantiate(RootControl { base: InputElement::construct() });
        root.set_focusable(true);
        let host = Rc::new_cyclic(|this: &Weak<TestHost>| TestHost {
            this: this.clone(),
            root: root.clone(),
            renderer: Rc::new(TestRenderer),
            layout_manager: RefCell::new(None),
            focus_manager: FocusManager::new(),
            hit_tester: Rc::new(ManagedHitTester::new()),
        });
        let as_layout_root: Rc<dyn ILayoutRoot> = host.clone();
        *host.layout_manager.borrow_mut() = Some(LayoutManager::new(Rc::downgrade(&as_layout_root)));
        root.set_bounds(Rect::new(0.0, 0.0, 200.0, 200.0));
        root.set_presentation_source_for_root_visual(Some(host.clone()));

        root.logical_children().add(child.clone().upcast());
        root.visual_children().add(child.clone().upcast());
        Self { root, _host: host, _keyboard: keyboard }
    }
}
