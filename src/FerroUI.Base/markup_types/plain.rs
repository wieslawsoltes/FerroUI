//! Markup metadata of the plain (non object-model) classes and of the
//! collections of this crate that markup can name, construct or add to.
//!
//! A collection is a shared handle (the type itself); a plain class is held
//! through `Rc`.

use crate::utilities::CultureInfo;
use crate::animation::{IAnimation, IAnimationSetter, ITransition, KeyFrame, KeyFrames, Transitions};
use crate::collections::{FerroDictionary, FerroList};
use crate::data::core::ValueTypes;
use crate::data::{
    BindingBase, BindingMode, BindingOperations, BindingPriority, CompiledBinding, CompiledBindingPath, MultiBinding, ReflectionBinding,
    RelativeSource, RelativeSourceMode, TemplateBinding, TreeType, UpdateSourceTrigger,
};
use crate::ferro_markup_type;
use crate::controls::{IResourceProvider, IThemeVariantProvider, ResourceKey};
use crate::data::converters::{
    BoolConverters, DefaultValueConverter, IMultiValueConverter, IValueConverter, ObjectConverters, StringConverters,
    StringFormatMultiValueConverter, StringFormatValueConverter,
};
use crate::input::KeyBinding;
use crate::input::gesture_recognizers::{GestureRecognizer, GestureRecognizerCollection};
use crate::interactivity::{RoutedEvent, RoutingStrategies};
use crate::media::{
    Drawing, DrawingCollection, Geometry, GeometryCollection, GradientStop, GradientStops, MediaCollection,
    PathFigure, PathFigures, PathSegment, PathSegments, Points, Transform, Transforms,
};
use crate::media::transformation::TransformOperations;
use crate::metadata::{from_markup_value, MarkupType, MarkupTyped};
use crate::styling::{
    IStyle, ITemplate, Selector, Setter, SetterBase, SetterValue, StyleChildren, StyleQuery, ThemeVariant,
};
use crate::utilities::NumberFormatInfo;
use crate::{BoxedValue, FerroProperty, Point, Ref, TypeInfo};
use std::rc::Rc;

/// The key of a resource from the untyped key markup gives: the managed
/// original takes any object; the port has text keys, type keys and keys that
/// already are a [`ResourceKey`] (a theme variant is used through its key).
pub(super) fn resource_key(key: Option<BoxedValue>) -> Result<ResourceKey, String> {
    let Some(key) = key else {
        return Err("The key of a resource cannot be null.".to_string());
    };
    if let Some(text) = key.downcast_ref::<String>() {
        Ok(ResourceKey::from(text.as_str()))
    } else if let Some(type_) = key.downcast_ref::<&'static TypeInfo>() {
        Ok(ResourceKey::from(*type_))
    } else if let Some(key) = key.downcast_ref::<ResourceKey>() {
        Ok(key.clone())
    } else if let Some(variant) = key.downcast_ref::<crate::styling::ThemeVariant>() {
        Ok(variant.key().clone())
    } else {
        Err(format!("A value of type '{}' cannot be the key of a resource.", key.type_name()))
    }
}

/// The key of a resource as the object it is in the managed original: the text of a
/// string key, the type of a type key, the key itself otherwise.
pub(super) fn resource_key_value(key: &ResourceKey) -> Option<BoxedValue> {
    Some(match key {
        ResourceKey::String(text) => Rc::new(text.to_string()),
        ResourceKey::Type(type_) => Rc::new(*type_),
        ResourceKey::Object(_) => Rc::new(key.clone()),
    })
}

/// An untyped value as a value of exactly the type of `property`, as the
/// property system takes it. Only assignable values are accepted: converting
/// (parsing, numeric conversion) is the business of the caller.
pub(super) fn property_value(property: &'static FerroProperty, value: Option<BoxedValue>) -> Result<BoxedValue, String> {
    let type_ = crate::data::core::ValueType::new(property.property_type(), property.property_type_name());
    // The unset value is a value of every property.
    if let Some(unset) = value.as_ref().filter(|value| value.is::<crate::UnsetValueType>()) {
        return Ok(unset.clone());
    }
    let exact = match &value {
        Some(value) => ValueTypes::try_cast(value, type_),
        None => ValueTypes::null_value(type_),
    };
    exact.ok_or_else(|| format!("The value is not assignable to the property '{}' ({type_}).", property.name()))
}

/// The key for adding a resource to `dictionary`: adding a key twice is an
/// error, as in the managed original.
pub(super) fn new_resource_key(
    dictionary: &Ref<crate::controls::ResourceDictionary>,
    key: Option<BoxedValue>,
) -> Result<ResourceKey, String> {
    let key = resource_key(key)?;
    if dictionary.contains_key(&key) {
        return Err(format!("An item with the same key has already been added. Key: {key}"));
    }
    Ok(key)
}

/// `FerroList<T>.Capacity`.
fn list_capacity<T: Clone>(list: &FerroList<T>) -> i32 {
    i32::try_from(list.capacity()).unwrap_or(i32::MAX)
}

/// The setter of `FerroList<T>.Capacity`: a capacity below the number of
/// items is an error, as in the managed original.
fn set_list_capacity<T: Clone>(list: &FerroList<T>, capacity: i32) -> Result<(), &'static str> {
    match usize::try_from(capacity) {
        Ok(capacity) if capacity >= list.count() => Ok(list.set_capacity(capacity)),
        _ => Err("capacity was less than the current size."),
    }
}

// System

// A runtime type: the `Type` of the managed original.
ferro_markup_type!(class TypeInfo as "Type" {
    this: &'static TypeInfo,
    namespace: "System",
    handles: [&'static TypeInfo, Option<&'static TypeInfo>],
    properties: [
        Name: String { get: |type_: &&'static TypeInfo| type_.name().to_string() },
        Namespace: String { get: |type_: &&'static TypeInfo| type_.namespace().to_string() },
        FullName: String { get: |type_: &&'static TypeInfo| type_.full_name() },
        BaseType: Option<&'static TypeInfo> { get: |type_: &&'static TypeInfo| type_.base_type() },
    ],
});

// System.Globalization

// The mirror of the runtime library's number format, which markup creates as a resource for the
// `NumberFormat` of numeric inputs (the colour view of the colour picker). Its numeric
// properties are settable; the text properties are not declared.
ferro_markup_type!(class NumberFormatInfo {
    namespace: "System.Globalization",
    handles: [Rc<NumberFormatInfo>, Option<Rc<NumberFormatInfo>>],
    this: Rc<NumberFormatInfo>,
    constructors: [() => || Rc::new(NumberFormatInfo::new())],
    properties: [
        CurrencyDecimalDigits: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.currency_decimal_digits(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_currency_decimal_digits(value)
        },
        CurrencyNegativePattern: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.currency_negative_pattern(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_currency_negative_pattern(value)
        },
        CurrencyPositivePattern: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.currency_positive_pattern(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_currency_positive_pattern(value)
        },
        NumberDecimalDigits: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.number_decimal_digits(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_number_decimal_digits(value)
        },
        NumberNegativePattern: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.number_negative_pattern(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_number_negative_pattern(value)
        },
        PercentDecimalDigits: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.percent_decimal_digits(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_percent_decimal_digits(value)
        },
        PercentNegativePattern: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.percent_negative_pattern(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_percent_negative_pattern(value)
        },
        PercentPositivePattern: i32 {
            get: |info: &Rc<NumberFormatInfo>| info.percent_positive_pattern(),
            set: |info: &Rc<NumberFormatInfo>, value: i32| info.set_percent_positive_pattern(value)
        },
    ],
});

// FerroUI

ferro_markup_type!(class FerroProperty {
    this: &'static FerroProperty,
    namespace: "FerroUI",
    handles: [&'static FerroProperty, Option<&'static FerroProperty>],
    properties: [
        Name: String { get: |property: &&'static FerroProperty| property.name().to_string() },
        OwnerType: &'static TypeInfo { get: |property: &&'static FerroProperty| property.owner_type() },
    ],
    // The marker of an unset property value.
    fields: [UnsetValue: crate::UnsetValueType => || crate::UnsetValueType],
});

// FerroUI.Data

// The marker that tells a binding target to do nothing is any object in the managed original.
ferro_markup_type!(static BindingOperations {
    namespace: "FerroUI.Data",
    fields: [DoNothing: Option<BoxedValue> => || Some(BindingOperations::do_nothing())],
});

// FerroUI.Interactivity

// A routed event without the type of its arguments: what the typed handles
// (`RoutedEvent<TappedEventArgs>`, the static values of the classes) denote.
ferro_markup_type!(class RoutedEvent as "RoutedEvent" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent, Option<RoutedEvent>],
    properties: [
        Name: String { get: |event: &RoutedEvent| event.name().to_string() },
        OwnerType: &'static TypeInfo { get: RoutedEvent::owner_type },
        RoutingStrategies: RoutingStrategies { get: RoutedEvent::routing_strategies },
    ],
});

// FerroUI.Styling

/// Whether a template is a value of `property` itself (`ITemplate` is
/// assignable from the type of the property), as opposed to what builds
/// its value.
fn is_template_property(property: &'static FerroProperty) -> bool {
    let type_ = crate::data::core::ValueType::new(property.property_type(), property.property_type_name());
    ValueTypes::is_assignable(type_, crate::data::core::ValueType::of::<Option<Rc<dyn ITemplate>>>())
}

/// `Setter.Value` of the managed original holds any object; what the setter
/// does with it follows from its type, as `Setter.Instance` decides there: a
/// binding is bound, a template builds the value unless the property itself
/// holds templates, anything else is the value. The object may arrive in
/// any of its forms (its concrete handle, the handle of a contract, an "any
/// value" box holding either).
fn set_setter_value(setter: &Rc<Setter>, value: Option<BoxedValue>) {
    let property = setter.property();
    setter.set_value(value.map(|value| {
        let object = match value.downcast_ref::<Option<BoxedValue>>() {
            Some(contents) => contents.clone(),
            None => Some(value.clone()),
        };
        if let Some(binding) = from_markup_value::<Rc<dyn BindingBase>>(&object) {
            SetterValue::BindingBase(binding)
        } else if let Some(template) =
            from_markup_value::<Rc<dyn ITemplate>>(&object).filter(|_| !property.is_some_and(is_template_property))
        {
            SetterValue::Template(template)
        } else {
            // The value of the property: what is assignable to its type is held as a value
            // of exactly that type (the setter requires it); anything else is kept as it
            // is, and reported by the setter when the style is applied.
            let exact = property.and_then(|property| {
                property_value(property, Some(value.clone())).or_else(|_| property_value(property, object.clone())).ok()
            });
            SetterValue::Value(exact.unwrap_or(value))
        }
    }));
}

/// The value of a setter as the object it was set to. (A setter bound to a
/// plain source of values, which markup cannot create, reads as null.)
fn setter_value(setter: &Rc<Setter>) -> Option<BoxedValue> {
    match setter.value()? {
        SetterValue::Value(value) => Some(value),
        SetterValue::BindingBase(binding) => Some(Rc::new(binding)),
        SetterValue::Template(template) => Some(Rc::new(template)),
        SetterValue::Binding(_) => None,
    }
}

fn new_setter(property: &'static FerroProperty, value: Option<BoxedValue>) -> Rc<Setter> {
    let setter = Setter::empty();
    setter.set_property(Some(property));
    set_setter_value(&setter, value);
    setter
}

ferro_markup_type!(class Setter {
    namespace: "FerroUI.Styling",
    handles: [Rc<Setter>, Option<Rc<Setter>>],
    this: Rc<Setter>,
    base: Rc<dyn SetterBase>,
    interfaces: [Rc<dyn IAnimationSetter>],
    content: Value,
    constructors: [
        () => Setter::empty,
        (&'static FerroProperty, Option<BoxedValue>) => new_setter,
    ],
    properties: [
        Property: Option<&'static FerroProperty> { get: Setter::property, set: Setter::set_property },
        Value: Option<BoxedValue> { get: setter_value, set: set_setter_value } [AssignBinding, DependsOn("Property")],
    ],
});

ferro_markup_type!(class Selector {
    namespace: "FerroUI.Styling",
    handles: [Selector, Option<Selector>],
});

ferro_markup_type!(class StyleQuery {
    namespace: "FerroUI.Styling",
    handles: [StyleQuery, Option<StyleQuery>],
});

// The children of a style.
ferro_markup_type!(class StyleChildren {
    namespace: "FerroUI.Styling",
    handles: [StyleChildren, Option<StyleChildren>],
    properties: [Count: i32 { get: |children: &StyleChildren| children.count() as i32 }],
    methods: [fn Add(Rc<dyn IStyle>) => |children: &StyleChildren, style: Rc<dyn IStyle>| children.add(style)],
});

// FerroUI.Data

ferro_markup_type!(class RelativeSource {
    namespace: "FerroUI.Data",
    handles: [Rc<RelativeSource>, Option<Rc<RelativeSource>>],
    this: Rc<RelativeSource>,
    constructors: [
        () => RelativeSource::empty,
        (RelativeSourceMode) => RelativeSource::new,
    ],
    properties: [
        AncestorLevel: i32 { get: RelativeSource::ancestor_level, set: RelativeSource::set_ancestor_level },
        AncestorType: Option<&'static TypeInfo> {
            get: RelativeSource::ancestor_type,
            set: RelativeSource::set_ancestor_type
        },
        Mode: RelativeSourceMode { get: RelativeSource::mode, set: RelativeSource::set_mode },
        Tree: TreeType { get: RelativeSource::tree, set: RelativeSource::set_tree },
    ],
});

ferro_markup_type!(class CompiledBinding {
    namespace: "FerroUI.Data",
    handles: [Rc<CompiledBinding>, Option<Rc<CompiledBinding>>],
    this: Rc<CompiledBinding>,
    base: Rc<dyn BindingBase>,
    constructors: [
        () => CompiledBinding::empty,
        (CompiledBindingPath) => CompiledBinding::new,
    ],
    properties: [
        Delay: i32 { get: CompiledBinding::delay, set: CompiledBinding::set_delay },
        Converter: Option<Rc<dyn IValueConverter>> { get: CompiledBinding::converter, set: CompiledBinding::set_converter },
        ConverterCulture: Option<CultureInfo> {
            get: CompiledBinding::converter_culture,
            set: CompiledBinding::set_converter_culture
        },
        ConverterParameter: Option<BoxedValue> {
            get: CompiledBinding::converter_parameter,
            set: CompiledBinding::set_converter_parameter
        },
        FallbackValue: Option<BoxedValue> {
            get: CompiledBinding::fallback_value,
            set: CompiledBinding::set_fallback_value
        },
        Mode: BindingMode { get: CompiledBinding::mode, set: CompiledBinding::set_mode },
        Path: Option<CompiledBindingPath> { get: CompiledBinding::path, set: CompiledBinding::set_path }
            [ConstructorArgument("path")],
        Priority: BindingPriority { get: CompiledBinding::priority, set: CompiledBinding::set_priority },
        Source: Option<BoxedValue> { get: CompiledBinding::source, set: CompiledBinding::set_source },
        StringFormat: Option<String> { get: CompiledBinding::string_format, set: CompiledBinding::set_string_format },
        TargetNullValue: Option<BoxedValue> {
            get: CompiledBinding::target_null_value,
            set: CompiledBinding::set_target_null_value
        },
        UpdateSourceTrigger: UpdateSourceTrigger {
            get: CompiledBinding::update_source_trigger,
            set: CompiledBinding::set_update_source_trigger
        },
    ],
});

ferro_markup_type!(class ReflectionBinding {
    namespace: "FerroUI.Data",
    handles: [Rc<ReflectionBinding>, Option<Rc<ReflectionBinding>>],
    this: Rc<ReflectionBinding>,
    base: Rc<dyn BindingBase>,
    constructors: [
        () => ReflectionBinding::empty,
        (String) => |path: String| ReflectionBinding::new(&path),
    ],
    properties: [
        Delay: i32 { get: ReflectionBinding::delay, set: ReflectionBinding::set_delay },
        Converter: Option<Rc<dyn IValueConverter>> {
            get: ReflectionBinding::converter,
            set: ReflectionBinding::set_converter
        },
        ConverterCulture: Option<CultureInfo> {
            get: ReflectionBinding::converter_culture,
            set: ReflectionBinding::set_converter_culture
        },
        ConverterParameter: Option<BoxedValue> {
            get: ReflectionBinding::converter_parameter,
            set: ReflectionBinding::set_converter_parameter
        },
        ElementName: Option<String> { get: ReflectionBinding::element_name, set: ReflectionBinding::set_element_name },
        FallbackValue: Option<BoxedValue> {
            get: ReflectionBinding::fallback_value,
            set: ReflectionBinding::set_fallback_value
        },
        Mode: BindingMode { get: ReflectionBinding::mode, set: ReflectionBinding::set_mode },
        Path: String { get: ReflectionBinding::path, set: ReflectionBinding::set_path } [ConstructorArgument("path")],
        Priority: BindingPriority { get: ReflectionBinding::priority, set: ReflectionBinding::set_priority },
        RelativeSource: Option<Rc<RelativeSource>> {
            get: ReflectionBinding::relative_source,
            set: ReflectionBinding::set_relative_source
        },
        Source: Option<BoxedValue> { get: ReflectionBinding::source, set: ReflectionBinding::set_source },
        StringFormat: Option<String> {
            get: ReflectionBinding::string_format,
            set: ReflectionBinding::set_string_format
        },
        TargetNullValue: Option<BoxedValue> {
            get: ReflectionBinding::target_null_value,
            set: ReflectionBinding::set_target_null_value
        },
        UpdateSourceTrigger: UpdateSourceTrigger {
            get: ReflectionBinding::update_source_trigger,
            set: ReflectionBinding::set_update_source_trigger
        },
    ],
});

ferro_markup_type!(class MultiBinding {
    namespace: "FerroUI.Data",
    handles: [Rc<MultiBinding>, Option<Rc<MultiBinding>>],
    this: Rc<MultiBinding>,
    base: Rc<dyn BindingBase>,
    content: Bindings,
    constructors: [() => MultiBinding::new],
    properties: [
        Bindings: FerroList<Rc<dyn BindingBase>> { get: MultiBinding::bindings, set: MultiBinding::set_bindings }
            [AssignBinding],
        Converter: Option<Rc<dyn IMultiValueConverter>> { get: MultiBinding::converter, set: MultiBinding::set_converter },
        ConverterCulture: Option<CultureInfo> {
            get: MultiBinding::converter_culture,
            set: MultiBinding::set_converter_culture
        },
        ConverterParameter: Option<BoxedValue> {
            get: MultiBinding::converter_parameter,
            set: MultiBinding::set_converter_parameter
        },
        FallbackValue: Option<BoxedValue> { get: MultiBinding::fallback_value, set: MultiBinding::set_fallback_value },
        TargetNullValue: Option<BoxedValue> {
            get: MultiBinding::target_null_value,
            set: MultiBinding::set_target_null_value
        },
        Mode: BindingMode { get: MultiBinding::mode, set: MultiBinding::set_mode },
        Priority: BindingPriority { get: MultiBinding::priority, set: MultiBinding::set_priority },
        RelativeSource: Option<Rc<RelativeSource>> {
            get: MultiBinding::relative_source,
            set: MultiBinding::set_relative_source
        },
        StringFormat: Option<String> { get: MultiBinding::string_format, set: MultiBinding::set_string_format },
    ],
});

ferro_markup_type!(class TemplateBinding {
    namespace: "FerroUI.Data",
    handles: [Rc<TemplateBinding>, Option<Rc<TemplateBinding>>],
    this: Rc<TemplateBinding>,
    base: Rc<dyn BindingBase>,
    constructors: [
        () => TemplateBinding::empty,
        (property: &'static FerroProperty [InheritDataTypeFrom(2)]) => TemplateBinding::new,
    ],
    properties: [
        Converter: Option<Rc<dyn IValueConverter>> { get: TemplateBinding::converter, set: TemplateBinding::set_converter },
        ConverterCulture: Option<CultureInfo> {
            get: TemplateBinding::converter_culture,
            set: TemplateBinding::set_converter_culture
        },
        ConverterParameter: Option<BoxedValue> {
            get: TemplateBinding::converter_parameter,
            set: TemplateBinding::set_converter_parameter
        },
        Mode: BindingMode { get: TemplateBinding::mode, set: TemplateBinding::set_mode },
        // The argument of `InheritDataTypeFrom` is `InheritDataTypeFromScopeKind.ControlTemplate` (2).
        Property: Option<&'static FerroProperty> { get: TemplateBinding::property, set: TemplateBinding::set_property }
            [ConstructorArgument("property"), InheritDataTypeFrom(2)],
    ],
    // The binding is its own markup extension: it provides itself, as a binding.
    methods: [
        fn ProvideValue() -> Rc<dyn BindingBase> =>
            |this: &Rc<TemplateBinding>| -> Rc<dyn BindingBase> { this.clone() },
    ],
});

// FerroUI.Data.Converters

ferro_markup_type!(static ObjectConverters {
    namespace: "FerroUI.Data.Converters",
    fields: [
        IsNull: Rc<dyn IValueConverter> => ObjectConverters::is_null,
        IsNotNull: Rc<dyn IValueConverter> => ObjectConverters::is_not_null,
        Equal: Rc<dyn IValueConverter> => ObjectConverters::equal,
        NotEqual: Rc<dyn IValueConverter> => ObjectConverters::not_equal,
        AreAllNull: Rc<dyn IMultiValueConverter> => ObjectConverters::are_all_null,
        AreAnyNull: Rc<dyn IMultiValueConverter> => ObjectConverters::are_any_null,
        AreAllEqual: Rc<dyn IMultiValueConverter> => ObjectConverters::are_all_equal,
    ],
});

ferro_markup_type!(static BoolConverters {
    namespace: "FerroUI.Data.Converters",
    fields: [
        And: Rc<dyn IMultiValueConverter> => BoolConverters::and,
        Or: Rc<dyn IMultiValueConverter> => BoolConverters::or,
        Not: Rc<dyn IValueConverter> => BoolConverters::not,
    ],
});

ferro_markup_type!(static StringConverters {
    namespace: "FerroUI.Data.Converters",
    fields: [
        IsNullOrEmpty: Rc<dyn IValueConverter> => StringConverters::is_null_or_empty,
        IsNotNullOrEmpty: Rc<dyn IValueConverter> => StringConverters::is_not_null_or_empty,
    ],
});

// The converter classes are reference types: an instance equals itself.
impl PartialEq for DefaultValueConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl PartialEq for StringFormatValueConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl PartialEq for StringFormatMultiValueConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

// The static instance is held through the converter contract in this port.
ferro_markup_type!(class DefaultValueConverter {
    namespace: "FerroUI.Data.Converters",
    handles: [Rc<DefaultValueConverter>, Option<Rc<DefaultValueConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    fields: [Instance: Rc<dyn IValueConverter> => DefaultValueConverter::instance],
});

ferro_markup_type!(class StringFormatValueConverter {
    namespace: "FerroUI.Data.Converters",
    handles: [Rc<StringFormatValueConverter>, Option<Rc<StringFormatValueConverter>>],
    this: Rc<StringFormatValueConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [
        (String, Option<Rc<dyn IValueConverter>>) => |format: String, inner: Option<Rc<dyn IValueConverter>>| {
            Rc::new(StringFormatValueConverter::new(format, inner))
        },
    ],
    properties: [
        Inner: Option<Rc<dyn IValueConverter>> { get: |c: &Rc<StringFormatValueConverter>| c.inner().cloned() },
        Format: String { get: |c: &Rc<StringFormatValueConverter>| c.format().to_string() },
    ],
});

ferro_markup_type!(class StringFormatMultiValueConverter {
    namespace: "FerroUI.Data.Converters",
    handles: [Rc<StringFormatMultiValueConverter>, Option<Rc<StringFormatMultiValueConverter>>],
    this: Rc<StringFormatMultiValueConverter>,
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [
        (String, Option<Rc<dyn IMultiValueConverter>>) =>
            |format: String, inner: Option<Rc<dyn IMultiValueConverter>>| {
                Rc::new(StringFormatMultiValueConverter::new(format, inner))
            },
    ],
    properties: [
        Inner: Option<Rc<dyn IMultiValueConverter>> {
            get: |c: &Rc<StringFormatMultiValueConverter>| c.inner().cloned()
        },
        Format: String { get: |c: &Rc<StringFormatMultiValueConverter>| c.format().to_string() },
    ],
});

// FerroUI.Animation

ferro_markup_type!(class KeyFrames {
    namespace: "FerroUI.Animation",
    handles: [KeyFrames, Option<KeyFrames>],
    base: FerroList<Ref<KeyFrame>>,
    constructors: [() => KeyFrames::new],
    methods: [fn Add(Ref<KeyFrame>) => |frames: &KeyFrames, frame: Ref<KeyFrame>| frames.add(frame)],
});

ferro_markup_type!(class Transitions {
    namespace: "FerroUI.Animation",
    handles: [Transitions, Option<Transitions>],
    base: FerroList<Rc<dyn ITransition>>,
    constructors: [() => Transitions::new],
    methods: [
        fn Add(Rc<dyn ITransition>) => |transitions: &Transitions, item: Rc<dyn ITransition>| transitions.add(item),
    ],
});

// FerroUI.Input.GestureRecognizers

ferro_markup_type!(class GestureRecognizerCollection {
    namespace: "FerroUI.Input.GestureRecognizers",
    handles: [GestureRecognizerCollection, Option<GestureRecognizerCollection>],
    methods: [
        fn Add(Ref<GestureRecognizer>) =>
            |recognizers: &GestureRecognizerCollection, item: Ref<GestureRecognizer>| recognizers.add(item),
    ],
});

// FerroUI.Media

ferro_markup_type!(class GradientStops {
    namespace: "FerroUI.Media",
    handles: [GradientStops, Option<GradientStops>],
    base: FerroList<Ref<GradientStop>>,
    constructors: [() => GradientStops::new],
    methods: [fn Add(Ref<GradientStop>) => |stops: &GradientStops, item: Ref<GradientStop>| stops.add(item)],
});

ferro_markup_type!(class PathFigures {
    namespace: "FerroUI.Media",
    handles: [PathFigures, Option<PathFigures>],
    base: FerroList<Ref<PathFigure>>,
    parse: PathFigures::parse,
    constructors: [() => PathFigures::new],
    methods: [fn Add(Ref<PathFigure>) => |figures: &PathFigures, item: Ref<PathFigure>| figures.add(item)],
});

ferro_markup_type!(class PathSegments {
    namespace: "FerroUI.Media",
    handles: [PathSegments, Option<PathSegments>],
    base: FerroList<Ref<PathSegment>>,
    constructors: [() => PathSegments::new],
    methods: [fn Add(Ref<PathSegment>) => |segments: &PathSegments, item: Ref<PathSegment>| segments.add(item)],
});

ferro_markup_type!(class Points {
    namespace: "FerroUI",
    handles: [Points, Option<Points>],
    base: FerroList<Point>,
    constructors: [() => Points::new],
    methods: [fn Add(Point) => |points: &Points, item: Point| points.add(item)],
});

ferro_markup_type!(class GeometryCollection {
    namespace: "FerroUI.Media",
    handles: [GeometryCollection, Option<GeometryCollection>],
    base: FerroList<Ref<Geometry>>,
    constructors: [() => GeometryCollection::new],
    methods: [
        fn Add(Ref<Geometry>) => |geometries: &GeometryCollection, item: Ref<Geometry>| geometries.add(item),
    ],
});

ferro_markup_type!(class Transforms {
    namespace: "FerroUI.Media",
    handles: [Transforms, Option<Transforms>],
    base: FerroList<Ref<Transform>>,
    constructors: [() => Transforms::new],
    methods: [fn Add(Ref<Transform>) => |transforms: &Transforms, item: Ref<Transform>| transforms.add(item)],
});

ferro_markup_type!(class DrawingCollection {
    namespace: "FerroUI.Media",
    handles: [DrawingCollection, Option<DrawingCollection>],
    base: FerroList<Ref<Drawing>>,
    constructors: [() => DrawingCollection::new],
    methods: [fn Add(Ref<Drawing>) => |drawings: &DrawingCollection, item: Ref<Drawing>| drawings.add(item)],
});

// The font manager: markup names the current one (`{x:Static FontManager.Current}`) and binds
// to the fonts of the system.
ferro_markup_type!(class crate::media::FontManager as "FontManager" {
    namespace: "FerroUI.Media",
    handles: [Rc<crate::media::FontManager>, Option<Rc<crate::media::FontManager>>],
    this: Rc<crate::media::FontManager>,
    properties: [
        DefaultFontFamily: crate::media::FontFamily {
            get: |font_manager: &Rc<crate::media::FontManager>| font_manager.default_font_family().clone()
        },
        SystemFonts: Rc<dyn crate::media::fonts::IFontCollection> {
            get: |font_manager: &Rc<crate::media::FontManager>| font_manager.system_fonts()
        },
    ],
    static_properties: [Current: Rc<crate::media::FontManager> { get: crate::media::FontManager::current }],
});

// FerroUI.Media.Transformation

// A list of transform operations: the text content of an element is its text form.
ferro_markup_type!(class TransformOperations {
    namespace: "FerroUI.Media.Transformation",
    handles: [Rc<TransformOperations>, Option<Rc<TransformOperations>>],
    this: Rc<TransformOperations>,
    interfaces: [Rc<dyn crate::media::ITransform>],
    parse: TransformOperations::parse,
    properties: [
        IsIdentity: bool { get: |operations: &Rc<TransformOperations>| operations.is_identity() },
        Value: crate::Matrix { get: |operations: &Rc<TransformOperations>| operations.value() },
    ],
    static_properties: [Identity: Rc<TransformOperations> { get: TransformOperations::identity }],
});

// FerroUI.Collections

// The list of numbers of dash arrays and tick lists.
ferro_markup_type!(class MediaCollection<f64> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [MediaCollection<f64>, Option<MediaCollection<f64>>],
    generic: "FerroList`1" [f64],
    constructors: [() => MediaCollection::<f64>::new],
    properties: [
        Capacity: i32 {
            get: |list: &MediaCollection<f64>| list_capacity(list.list()),
            try_set: |list: &MediaCollection<f64>, capacity: i32| set_list_capacity(list.list(), capacity)
        },
        Count: i32 { get: |list: &MediaCollection<f64>| list.len() as i32 },
    ],
    methods: [fn Add(f64) => |list: &MediaCollection<f64>, item: f64| list.add(item)],
});

// The instantiations of the notifying list that are types of properties.

ferro_markup_type!(class FerroList<Rc<dyn IAnimationSetter>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn IAnimationSetter>>, Option<FerroList<Rc<dyn IAnimationSetter>>>],
    generic: "FerroList`1" [Rc<dyn IAnimationSetter>],
    constructors: [() => FerroList::<Rc<dyn IAnimationSetter>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn IAnimationSetter>>| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn IAnimationSetter>) => |list: &FerroList<Rc<dyn IAnimationSetter>>, item: Rc<dyn IAnimationSetter>| list.add(item)],
});

ferro_markup_type!(class FerroList<Rc<dyn IResourceProvider>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn IResourceProvider>>, Option<FerroList<Rc<dyn IResourceProvider>>>],
    generic: "FerroList`1" [Rc<dyn IResourceProvider>],
    constructors: [() => FerroList::<Rc<dyn IResourceProvider>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn IResourceProvider>>| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn IResourceProvider>) => |list: &FerroList<Rc<dyn IResourceProvider>>, item: Rc<dyn IResourceProvider>| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<KeyBinding>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<KeyBinding>>, Option<FerroList<Ref<KeyBinding>>>],
    generic: "FerroList`1" [Ref<KeyBinding>],
    constructors: [() => FerroList::<Ref<KeyBinding>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<KeyBinding>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<KeyBinding>) => |list: &FerroList<Ref<KeyBinding>>, item: Ref<KeyBinding>| list.add(item)],
});

ferro_markup_type!(class FerroList<Rc<dyn SetterBase>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn SetterBase>>, Option<FerroList<Rc<dyn SetterBase>>>],
    generic: "FerroList`1" [Rc<dyn SetterBase>],
    constructors: [() => FerroList::<Rc<dyn SetterBase>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn SetterBase>>| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn SetterBase>) => |list: &FerroList<Rc<dyn SetterBase>>, item: Rc<dyn SetterBase>| list.add(item)],
});

ferro_markup_type!(class FerroList<Rc<dyn IAnimation>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn IAnimation>>, Option<FerroList<Rc<dyn IAnimation>>>],
    generic: "FerroList`1" [Rc<dyn IAnimation>],
    constructors: [() => FerroList::<Rc<dyn IAnimation>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn IAnimation>>| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn IAnimation>) => |list: &FerroList<Rc<dyn IAnimation>>, item: Rc<dyn IAnimation>| list.add(item)],
});

ferro_markup_type!(class FerroList<Rc<dyn BindingBase>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn BindingBase>>, Option<FerroList<Rc<dyn BindingBase>>>],
    generic: "FerroList`1" [Rc<dyn BindingBase>],
    constructors: [() => FerroList::<Rc<dyn BindingBase>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn BindingBase>>| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn BindingBase>) => |list: &FerroList<Rc<dyn BindingBase>>, item: Rc<dyn BindingBase>| list.add(item)],
});

// The lists the named collections derive from.

ferro_markup_type!(class FerroList<Point> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Point>, Option<FerroList<Point>>],
    generic: "FerroList`1" [Point],
    constructors: [() => FerroList::<Point>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Point>| list.count() as i32 },
    ],
    methods: [fn Add(Point) => |list: &FerroList<Point>, item: Point| list.add(item)],
});

ferro_markup_type!(class FerroList<String> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<String>, Option<FerroList<String>>],
    generic: "FerroList`1" [String],
    constructors: [() => FerroList::<String>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<String>| list.count() as i32 },
    ],
    methods: [fn Add(String) => |list: &FerroList<String>, item: String| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<GradientStop>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<GradientStop>>, Option<FerroList<Ref<GradientStop>>>],
    generic: "FerroList`1" [Ref<GradientStop>],
    constructors: [() => FerroList::<Ref<GradientStop>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<GradientStop>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<GradientStop>) => |list: &FerroList<Ref<GradientStop>>, item: Ref<GradientStop>| list.add(item)],
});

ferro_markup_type!(class FerroList<Rc<dyn ITransition>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn ITransition>>, Option<FerroList<Rc<dyn ITransition>>>],
    generic: "FerroList`1" [Rc<dyn ITransition>],
    constructors: [() => FerroList::<Rc<dyn ITransition>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn ITransition>>| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn ITransition>) => |list: &FerroList<Rc<dyn ITransition>>, item: Rc<dyn ITransition>| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<KeyFrame>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<KeyFrame>>, Option<FerroList<Ref<KeyFrame>>>],
    generic: "FerroList`1" [Ref<KeyFrame>],
    constructors: [() => FerroList::<Ref<KeyFrame>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<KeyFrame>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<KeyFrame>) => |list: &FerroList<Ref<KeyFrame>>, item: Ref<KeyFrame>| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<PathFigure>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<PathFigure>>, Option<FerroList<Ref<PathFigure>>>],
    generic: "FerroList`1" [Ref<PathFigure>],
    constructors: [() => FerroList::<Ref<PathFigure>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<PathFigure>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<PathFigure>) => |list: &FerroList<Ref<PathFigure>>, item: Ref<PathFigure>| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<PathSegment>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<PathSegment>>, Option<FerroList<Ref<PathSegment>>>],
    generic: "FerroList`1" [Ref<PathSegment>],
    constructors: [() => FerroList::<Ref<PathSegment>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<PathSegment>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<PathSegment>) => |list: &FerroList<Ref<PathSegment>>, item: Ref<PathSegment>| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<Drawing>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<Drawing>>, Option<FerroList<Ref<Drawing>>>],
    generic: "FerroList`1" [Ref<Drawing>],
    constructors: [() => FerroList::<Ref<Drawing>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<Drawing>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<Drawing>) => |list: &FerroList<Ref<Drawing>>, item: Ref<Drawing>| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<Transform>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<Transform>>, Option<FerroList<Ref<Transform>>>],
    generic: "FerroList`1" [Ref<Transform>],
    constructors: [() => FerroList::<Ref<Transform>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<Transform>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<Transform>) => |list: &FerroList<Ref<Transform>>, item: Ref<Transform>| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<Geometry>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<Geometry>>, Option<FerroList<Ref<Geometry>>>],
    generic: "FerroList`1" [Ref<Geometry>],
    constructors: [() => FerroList::<Ref<Geometry>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<Geometry>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<Geometry>) => |list: &FerroList<Ref<Geometry>>, item: Ref<Geometry>| list.add(item)],
});

ferro_markup_type!(class FerroList<crate::media::FontFeature> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<crate::media::FontFeature>, Option<FerroList<crate::media::FontFeature>>],
    generic: "FerroList`1" [crate::media::FontFeature],
    constructors: [() => FerroList::<crate::media::FontFeature>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<crate::media::FontFeature>| list.count() as i32 },
    ],
    methods: [fn Add(crate::media::FontFeature) => |list: &FerroList<crate::media::FontFeature>, item: crate::media::FontFeature| list.add(item)],
});

ferro_markup_type!(class FerroList<Ref<crate::media::TextDecoration>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<crate::media::TextDecoration>>, Option<FerroList<Ref<crate::media::TextDecoration>>>],
    generic: "FerroList`1" [Ref<crate::media::TextDecoration>],
    constructors: [() => FerroList::<Ref<crate::media::TextDecoration>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<crate::media::TextDecoration>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<crate::media::TextDecoration>) => |list: &FerroList<Ref<crate::media::TextDecoration>>, item: Ref<crate::media::TextDecoration>| list.add(item)],
});

// The theme dictionaries of a resource dictionary.
ferro_markup_type!(class FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>> as "FerroDictionary`2" {
    namespace: "FerroUI.Collections",
    handles: [
        FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>>,
        Option<FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>>>,
    ],
    generic: "FerroDictionary`2" [ThemeVariant, Rc<dyn IThemeVariantProvider>],
    properties: [
        Count: i32 { get: |d: &FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>>| d.count() as i32 },
    ],
    methods: [
        // Adding a key twice is an error, as in the managed original.
        try fn Add(ThemeVariant, Rc<dyn IThemeVariantProvider>) =>
            |d: &FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>>,
             key: ThemeVariant,
             value: Rc<dyn IThemeVariantProvider>| {
                if d.contains_key(&key) {
                    return Err(format!("An item with the same key has already been added. Key: {key}"));
                }
                Ok(d.add(key, value))
            },
    ],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <TypeInfo as MarkupTyped>::MARKUP,
    <FerroProperty as MarkupTyped>::MARKUP,
    <NumberFormatInfo as MarkupTyped>::MARKUP,
    <BindingOperations as MarkupTyped>::MARKUP,
    <RoutedEvent as MarkupTyped>::MARKUP,
    <Setter as MarkupTyped>::MARKUP,
    <Selector as MarkupTyped>::MARKUP,
    <StyleQuery as MarkupTyped>::MARKUP,
    <StyleChildren as MarkupTyped>::MARKUP,
    <RelativeSource as MarkupTyped>::MARKUP,
    <CompiledBinding as MarkupTyped>::MARKUP,
    <ReflectionBinding as MarkupTyped>::MARKUP,
    <MultiBinding as MarkupTyped>::MARKUP,
    <TemplateBinding as MarkupTyped>::MARKUP,
    <ObjectConverters as MarkupTyped>::MARKUP,
    <BoolConverters as MarkupTyped>::MARKUP,
    <StringConverters as MarkupTyped>::MARKUP,
    <DefaultValueConverter as MarkupTyped>::MARKUP,
    <StringFormatValueConverter as MarkupTyped>::MARKUP,
    <StringFormatMultiValueConverter as MarkupTyped>::MARKUP,
    <KeyFrames as MarkupTyped>::MARKUP,
    <Transitions as MarkupTyped>::MARKUP,
    <GestureRecognizerCollection as MarkupTyped>::MARKUP,
    <GradientStops as MarkupTyped>::MARKUP,
    <PathFigures as MarkupTyped>::MARKUP,
    <PathSegments as MarkupTyped>::MARKUP,
    <Points as MarkupTyped>::MARKUP,
    <GeometryCollection as MarkupTyped>::MARKUP,
    <Transforms as MarkupTyped>::MARKUP,
    <DrawingCollection as MarkupTyped>::MARKUP,
    <TransformOperations as MarkupTyped>::MARKUP,
    <crate::media::FontManager as MarkupTyped>::MARKUP,
    <MediaCollection<f64> as MarkupTyped>::MARKUP,
    <FerroList<Rc<dyn IAnimationSetter>> as MarkupTyped>::MARKUP,
    <FerroList<Rc<dyn IResourceProvider>> as MarkupTyped>::MARKUP,
    <FerroList<Ref<KeyBinding>> as MarkupTyped>::MARKUP,
    <FerroList<Rc<dyn SetterBase>> as MarkupTyped>::MARKUP,
    <FerroList<Rc<dyn IAnimation>> as MarkupTyped>::MARKUP,
    <FerroList<Rc<dyn BindingBase>> as MarkupTyped>::MARKUP,
    <FerroList<Point> as MarkupTyped>::MARKUP,
    <FerroList<String> as MarkupTyped>::MARKUP,
    <FerroList<Ref<GradientStop>> as MarkupTyped>::MARKUP,
    <FerroList<Rc<dyn ITransition>> as MarkupTyped>::MARKUP,
    <FerroList<Ref<KeyFrame>> as MarkupTyped>::MARKUP,
    <FerroList<Ref<PathFigure>> as MarkupTyped>::MARKUP,
    <FerroList<Ref<PathSegment>> as MarkupTyped>::MARKUP,
    <FerroList<Ref<Drawing>> as MarkupTyped>::MARKUP,
    <FerroList<Ref<Transform>> as MarkupTyped>::MARKUP,
    <FerroList<Ref<Geometry>> as MarkupTyped>::MARKUP,
    <FerroList<crate::media::FontFeature> as MarkupTyped>::MARKUP,
    <FerroList<Ref<crate::media::TextDecoration>> as MarkupTyped>::MARKUP,
    <FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>> as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the types that can be held in untyped
/// values with the untyped value conversions of the current thread.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<&'static TypeInfo>();
    ValueTypes::register_nullable::<&'static FerroProperty>();
    // The vectors of the element types markup has arrays of (the elements of the runtime
    // library and the values the markup compiler parses a list of): a property that holds
    // one is null without it (`Option<Vec<T>>`). The run-time XAML loader states the same
    // nine when its type system is created; a process that loads compiled markup alone
    // never creates one, and reads such a property all the same.
    ValueTypes::register_nullable::<Vec<bool>>();
    ValueTypes::register_nullable::<Vec<u8>>();
    ValueTypes::register_nullable::<Vec<i32>>();
    ValueTypes::register_nullable::<Vec<i64>>();
    ValueTypes::register_nullable::<Vec<f32>>();
    ValueTypes::register_nullable::<Vec<f64>>();
    ValueTypes::register_nullable::<Vec<String>>();
    ValueTypes::register_nullable::<Vec<Point>>();
    ValueTypes::register_nullable::<Vec<BoxedValue>>();
    ValueTypes::register_nullable::<Rc<NumberFormatInfo>>();
    ValueTypes::register_nullable::<Transitions>();
    ValueTypes::register_nullable::<GradientStops>();
    ValueTypes::register_nullable::<PathFigures>();
    ValueTypes::register_nullable::<PathSegments>();
    ValueTypes::register_nullable::<Points>();
    ValueTypes::register_nullable::<GeometryCollection>();
    ValueTypes::register_nullable::<Transforms>();
    ValueTypes::register_nullable::<DrawingCollection>();
    ValueTypes::register_nullable::<MediaCollection<f64>>();
    ValueTypes::register_nullable::<RoutedEvent>();
    ValueTypes::register_nullable::<FerroList<Point>>();
    ValueTypes::register_nullable::<FerroList<String>>();
    ValueTypes::register_nullable::<FerroList<Ref<GradientStop>>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn ITransition>>>();
    ValueTypes::register_nullable::<FerroList<Ref<KeyFrame>>>();
    ValueTypes::register_nullable::<FerroList<Ref<PathFigure>>>();
    ValueTypes::register_nullable::<FerroList<Ref<PathSegment>>>();
    ValueTypes::register_nullable::<FerroList<Ref<Drawing>>>();
    ValueTypes::register_nullable::<FerroList<Ref<Transform>>>();
    ValueTypes::register_nullable::<FerroList<Ref<Geometry>>>();
    ValueTypes::register_nullable::<Selector>();
    ValueTypes::register_nullable::<StyleQuery>();
    ValueTypes::register_nullable::<StyleChildren>();
    ValueTypes::register_nullable::<KeyFrames>();
    ValueTypes::register_nullable::<GestureRecognizerCollection>();
    ValueTypes::register_nullable::<Rc<Setter>>();
    ValueTypes::register_nullable::<Rc<RelativeSource>>();
    ValueTypes::register_nullable::<Rc<CompiledBinding>>();
    ValueTypes::register_nullable::<Rc<ReflectionBinding>>();
    ValueTypes::register_nullable::<Rc<MultiBinding>>();
    ValueTypes::register_nullable::<Rc<TemplateBinding>>();
    ValueTypes::register_nullable::<FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn IAnimationSetter>>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn IResourceProvider>>>();
    ValueTypes::register_nullable::<FerroList<Ref<KeyBinding>>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn SetterBase>>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn IAnimation>>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn BindingBase>>>();
    // A named collection is the list it derives from: the same list, so that the members
    // of the list (`Capacity`) are reached through the collection.
    ValueTypes::register_cast::<KeyFrames, FerroList<Ref<KeyFrame>>>(|c| (**c).clone());
    ValueTypes::register_cast::<Transitions, FerroList<Rc<dyn ITransition>>>(|c| (**c).clone());
    ValueTypes::register_cast::<GradientStops, FerroList<Ref<GradientStop>>>(|c| (**c.list()).clone());
    ValueTypes::register_cast::<PathFigures, FerroList<Ref<PathFigure>>>(|c| (**c.list()).clone());
    ValueTypes::register_cast::<PathSegments, FerroList<Ref<PathSegment>>>(|c| (**c.list()).clone());
    ValueTypes::register_cast::<Points, FerroList<Point>>(|c| (**c.list()).clone());
    ValueTypes::register_cast::<GeometryCollection, FerroList<Ref<Geometry>>>(|c| (**c.list()).clone());
    ValueTypes::register_cast::<Transforms, FerroList<Ref<Transform>>>(|c| (**c.list()).clone());
    ValueTypes::register_cast::<DrawingCollection, FerroList<Ref<Drawing>>>(|c| (**c.list()).clone());
    ValueTypes::register_cast::<crate::media::FontFeatureCollection, FerroList<crate::media::FontFeature>>(|c| {
        (**c.list()).clone()
    });
    ValueTypes::register_cast::<crate::media::TextDecorationCollection, FerroList<Ref<crate::media::TextDecoration>>>(
        |c| (**c.list()).clone(),
    );
    // The named collections whose Rust type dereferences to the list: generated code
    // passes a reference to such a collection where a member of the list takes the list.
    ValueTypes::register_deref::<KeyFrames, FerroList<Ref<KeyFrame>>>(|c| c);
    ValueTypes::register_deref::<Transitions, FerroList<Rc<dyn ITransition>>>(|c| c);
    ValueTypes::register_nullable::<FerroList<crate::media::FontFeature>>();
    ValueTypes::register_nullable::<FerroList<Ref<crate::media::TextDecoration>>>();
    // A list of transform operations is a transform.
    ValueTypes::register_nullable::<Rc<TransformOperations>>();
    ValueTypes::register_nullable::<Rc<crate::media::FontManager>>();
    ValueTypes::register_cast::<Rc<TransformOperations>, Rc<dyn crate::media::ITransform>>(|operations| operations.clone());
    // A converter is a converter contract.
    ValueTypes::register_nullable::<Rc<DefaultValueConverter>>();
    ValueTypes::register_nullable::<Rc<StringFormatValueConverter>>();
    ValueTypes::register_nullable::<Rc<StringFormatMultiValueConverter>>();
    ValueTypes::register_cast::<Rc<DefaultValueConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_cast::<Rc<StringFormatValueConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_cast::<Rc<StringFormatMultiValueConverter>, Rc<dyn IMultiValueConverter>>(|c| c.clone());
    // A setter is a setter base; a binding is a binding base.
    ValueTypes::register_cast::<Rc<Setter>, Rc<dyn SetterBase>>(|setter| setter.clone());
    // A setter is also the setter of a key frame.
    ValueTypes::register_cast::<Rc<Setter>, Rc<dyn IAnimationSetter>>(|setter| setter.clone());
    ValueTypes::register_cast::<Rc<CompiledBinding>, Rc<dyn BindingBase>>(|binding| binding.clone());
    ValueTypes::register_cast::<Rc<ReflectionBinding>, Rc<dyn BindingBase>>(|binding| binding.clone());
    ValueTypes::register_cast::<Rc<MultiBinding>, Rc<dyn BindingBase>>(|binding| binding.clone());
    ValueTypes::register_cast::<Rc<TemplateBinding>, Rc<dyn BindingBase>>(|binding| binding.clone());
}
