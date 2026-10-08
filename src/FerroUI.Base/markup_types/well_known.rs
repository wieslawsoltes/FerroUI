//! Markup metadata of the types the markup compiler looks up by name (its
//! "well-known types") that are not declared with the other value types,
//! contracts and collections of this module.
//!
//! Several of them cannot be held in an untyped value yet (their handles do
//! not compare) or have a different shape in this port than in the managed
//! original (builders that are consumed, static classes that are traits or
//! do not exist). Such a type is published by name, so that a lookup
//! succeeds, with the members that can be expressed today; the types
//! declared on a marker of this file have no counterpart in the port at all.

use crate::controls::{Classes, NameScopeRef};
use crate::data::core::expression_nodes::CastTarget;
use crate::data::core::plugins::IPropertyAccessor;
use crate::data::core::{ClrPropertyInfo, IPropertyInfo, ValueTypes};
use crate::data::{BindingBase, BindingExpressionBase, CompiledBindingPath, CompiledBindingPathBuilder};
use crate::ferro_markup_type;
use crate::interactivity::{IRoutedEventArgs, RoutedEvent, RoutedEventArgs, RoutingStrategies};
use crate::media::imaging::Bitmap;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::IBrush;
use crate::metadata::{IAddChild, MarkupType, MarkupTyped};
use crate::platform::{FormFactorType, IRuntimePlatform, RuntimePlatformInfo};
use crate::reactive::IDisposable;
use crate::styling::{Selector, Selectors, StyleQueries, StyleQuery, StyleQueryComparisonOperator};
use crate::utilities::{CancelEventArgs, EventArgs, FormatError};
use crate::{
    BoxedValue, ClassBindingManager, FerroObject, FerroProperty, ISupportInitialize, Ref, StyledElement, TypeInfo,
    UnsetValueType,
};
use std::marker::PhantomData;
use std::rc::Rc;

// Markers for types of the managed original that the port does not have.

/// `FerroUI.FerroObjectExtensions`: a static class in the managed original,
/// a trait implemented by the root class in this port.
pub(crate) struct FerroObjectExtensionsClass;

/// `FerroUI.Utilities.TypeUtilities`: not ported; conversions are made by
/// the untyped value conversions.
pub(crate) struct TypeUtilities;

/// ``FerroUI.Collections.FerroListConverter`1``: not ported; lists are
/// parsed by the `parse` of their type.
pub(crate) struct FerroListConverter<T>(PhantomData<T>);

/// ``FerroUI.Data.Core.IPropertyInfo`2``: the port describes a property with
/// the untyped [`IPropertyInfo`] only.
pub(crate) struct PropertyInfoOf<TOwner, TValue>(PhantomData<(TOwner, TValue)>);

/// ``FerroUI.Data.Core.ClrPropertyInfo`2``: the port describes a property
/// with the untyped [`ClrPropertyInfo`] only.
pub(crate) struct ClrPropertyInfoOf<TOwner, TValue>(PhantomData<(TOwner, TValue)>);

// FerroUI

ferro_markup_type!(struct UnsetValueType {
    namespace: "FerroUI",
    handles: [UnsetValueType],
});

/// The anchor of a binding: an object of the object model, or none.
fn anchor(anchor: &Option<BoxedValue>) -> Option<Ref<FerroObject>> {
    anchor.as_ref().and_then(|anchor| ValueTypes::as_object(&**anchor))
}

ferro_markup_type!(static FerroObjectExtensionsClass as "FerroObjectExtensions" {
    namespace: "FerroUI",
    methods: [
        static fn Bind(Ref<FerroObject>, &'static FerroProperty, Rc<dyn BindingBase>, Option<BoxedValue>)
            -> Rc<dyn IDisposable> =>
            |target: Ref<FerroObject>,
             property: &'static FerroProperty,
             binding: Rc<dyn BindingBase>,
             anchor_object: Option<BoxedValue>|
             -> Rc<dyn IDisposable> {
                target.bind_binding_with_anchor(property, &*binding, anchor(&anchor_object).as_ref())
            },
    ],
});

// The static class `StyledElementExtensions` of the managed original forwards
// to the class binding manager.
ferro_markup_type!(static ClassBindingManager as "StyledElementExtensions" {
    namespace: "FerroUI",
    methods: [
        static fn BindClass(Ref<StyledElement>, String, Rc<dyn BindingBase>, Option<BoxedValue>) -> Rc<dyn IDisposable> =>
            |target: Ref<StyledElement>,
             class_name: String,
             binding: Rc<dyn BindingBase>,
             anchor_object: Option<BoxedValue>|
             -> Rc<dyn IDisposable> {
                ClassBindingManager::bind(&target, &class_name, &*binding, anchor(&anchor_object).as_ref())
            },
        static fn GetClassProperty(String) -> &'static FerroProperty =>
            |class_name: String| ClassBindingManager::get_class_property(&class_name),
    ],
});

// FerroUI.Controls

ferro_markup_type!(class Classes {
    namespace: "FerroUI.Controls",
    handles: [Classes, Option<Classes>],
    base: crate::collections::FerroList<String>,
    parse: |s: &str| Ok::<_, FormatError>(Classes::parse(s)),
    constructors: [() => Classes::new],
    properties: [
        // The list of names is not reached through the collection (names are added through
        // the collection only), so the capacity of the list is declared here.
        Capacity: i32 {
            get: |classes: &Classes| i32::try_from(classes.capacity()).unwrap_or(i32::MAX),
            try_set: |classes: &Classes, capacity: i32| match usize::try_from(capacity) {
                Ok(capacity) if capacity >= classes.count() => Ok(classes.set_capacity(capacity)),
                _ => Err("capacity was less than the current size."),
            }
        },
        Count: i32 { get: |classes: &Classes| classes.count() as i32 },
    ],
    methods: [
        fn Add(String) => |classes: &Classes, name: String| classes.add(&name),
        fn Remove(String) -> bool => |classes: &Classes, name: String| classes.remove(&name),
        fn Contains(String) -> bool => |classes: &Classes, name: String| classes.contains(&name),
        fn Set(String, bool) => |classes: &Classes, name: String, value: bool| classes.set(&name, value),
        fn Clear() => |classes: &Classes| classes.clear(),
    ],
});

// FerroUI.Data

ferro_markup_type!(class dyn BindingExpressionBase as "BindingExpressionBase" {
    namespace: "FerroUI.Data",
    handles: [Rc<dyn BindingExpressionBase>, Option<Rc<dyn BindingExpressionBase>>],
});

ferro_markup_type!(class CompiledBindingPath {
    namespace: "FerroUI.Data",
    handles: [CompiledBindingPath, Option<CompiledBindingPath>],
    constructors: [() => CompiledBindingPath::new],
});

// The builder is a shared handle: each method adds an element and returns
// the builder. The methods of the managed original that take delegates
// (`Property`, `Command`, `Method`) are left out: the port takes closures
// over values that cannot be held in untyped values.
ferro_markup_type!(class CompiledBindingPathBuilder {
    namespace: "FerroUI.Data",
    handles: [CompiledBindingPathBuilder, Option<CompiledBindingPathBuilder>],
    constructors: [() => CompiledBindingPathBuilder::new],
    methods: [
        fn Not() -> CompiledBindingPathBuilder => CompiledBindingPathBuilder::not,
        fn Self() -> CompiledBindingPathBuilder => CompiledBindingPathBuilder::self_,
        fn TemplatedParent() -> CompiledBindingPathBuilder => CompiledBindingPathBuilder::templated_parent,
        fn StreamTask() -> CompiledBindingPathBuilder => CompiledBindingPathBuilder::stream_task,
        fn StreamObservable() -> CompiledBindingPathBuilder => CompiledBindingPathBuilder::stream_observable,
        fn Ancestor(Option<&'static TypeInfo>, i32) -> CompiledBindingPathBuilder =>
            |builder: &CompiledBindingPathBuilder, ancestor_type: Option<&'static TypeInfo>, level: i32| {
                builder.ancestor(ancestor_type, level.max(0) as usize)
            },
        fn VisualAncestor(Option<&'static TypeInfo>, i32) -> CompiledBindingPathBuilder =>
            |builder: &CompiledBindingPathBuilder, ancestor_type: Option<&'static TypeInfo>, level: i32| {
                builder.visual_ancestor(ancestor_type, level.max(0) as usize)
            },
        fn ElementName(NameScopeRef, String) -> CompiledBindingPathBuilder =>
            |builder: &CompiledBindingPathBuilder, name_scope: NameScopeRef, name: String| {
                builder.element_name(name_scope, &name)
            },
        fn TypeCast(&'static TypeInfo) -> CompiledBindingPathBuilder =>
            |builder: &CompiledBindingPathBuilder, type_: &'static TypeInfo| {
                builder.type_cast_value(CastTarget::Class(type_))
            },
        fn Build() -> CompiledBindingPath => CompiledBindingPathBuilder::build,
    ],
});

// FerroUI.Data.Core

ferro_markup_type!(interface dyn IPropertyInfo as "IPropertyInfo" {
    namespace: "FerroUI.Data.Core",
    handles: [Rc<dyn IPropertyInfo>, Option<Rc<dyn IPropertyInfo>>],
    this: Rc<dyn IPropertyInfo>,
    properties: [
        Name: String { get: |info: &Rc<dyn IPropertyInfo>| info.name().to_string() },
        CanGet: bool { get: |info: &Rc<dyn IPropertyInfo>| info.can_get() },
        CanSet: bool { get: |info: &Rc<dyn IPropertyInfo>| info.can_set() },
    ],
    methods: [
        try fn Get(BoxedValue) -> Option<BoxedValue> =>
            |info: &Rc<dyn IPropertyInfo>, target: BoxedValue| info.try_get_boxed(&target),
        try fn Set(BoxedValue, Option<BoxedValue>) =>
            |info: &Rc<dyn IPropertyInfo>, target: BoxedValue, value: Option<BoxedValue>| {
                info.set_boxed(&target, value.as_ref())
            },
    ],
});

ferro_markup_type!(class ClrPropertyInfo {
    namespace: "FerroUI.Data.Core",
    handles: [Rc<ClrPropertyInfo>, Option<Rc<ClrPropertyInfo>>],
    interfaces: [Rc<dyn IPropertyInfo>],
});

ferro_markup_type!(interface PropertyInfoOf<Option<BoxedValue>, Option<BoxedValue>> as "IPropertyInfo`2" {
    namespace: "FerroUI.Data.Core",
    handles: [PropertyInfoOf<Option<BoxedValue>, Option<BoxedValue>>],
    generic: "IPropertyInfo`2" [Option<BoxedValue>, Option<BoxedValue>],
});

ferro_markup_type!(class ClrPropertyInfoOf<Option<BoxedValue>, Option<BoxedValue>> as "ClrPropertyInfo`2" {
    namespace: "FerroUI.Data.Core",
    handles: [ClrPropertyInfoOf<Option<BoxedValue>, Option<BoxedValue>>],
    generic: "ClrPropertyInfo`2" [Option<BoxedValue>, Option<BoxedValue>],
});

// FerroUI.Data.Core.Plugins

ferro_markup_type!(interface dyn IPropertyAccessor as "IPropertyAccessor" {
    namespace: "FerroUI.Data.Core.Plugins",
    handles: [Rc<dyn IPropertyAccessor>, Option<Rc<dyn IPropertyAccessor>>],
});

// FerroUI.Interactivity

// The arguments of a routed event are a live handle: a handler that sets
// `Handled` is seen by the route. The typed arguments (`TappedEventArgs`,
// ...) are values of this type; their own members are not declared.
ferro_markup_type!(class dyn IRoutedEventArgs as "RoutedEventArgs" {
    namespace: "FerroUI.Interactivity",
    handles: [Rc<dyn IRoutedEventArgs>, Option<Rc<dyn IRoutedEventArgs>>, RoutedEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    properties: [
        Handled: bool {
            get: |e: &Rc<dyn IRoutedEventArgs>| e.as_routed_event_args().handled(),
            set: |e: &Rc<dyn IRoutedEventArgs>, value: bool| e.as_routed_event_args().set_handled(value)
        },
        RoutedEvent: Option<RoutedEvent> { get: |e: &Rc<dyn IRoutedEventArgs>| e.as_routed_event_args().routed_event() },
        Route: RoutingStrategies { get: |e: &Rc<dyn IRoutedEventArgs>| e.as_routed_event_args().route() },
        Source: Option<Ref<FerroObject>> {
            get: |e: &Rc<dyn IRoutedEventArgs>| e.as_routed_event_args().source(),
            set: |e: &Rc<dyn IRoutedEventArgs>, value: Option<Ref<FerroObject>>| {
                e.as_routed_event_args().set_source(value)
            }
        },
    ],
});

// System: the errors of bindings and data validation stand for exceptions.

ferro_markup_type!(class crate::data::BindingError as "Exception" {
    namespace: "System",
    handles: [crate::data::BindingError, Option<crate::data::BindingError>],
    constructors: [
        () => || crate::data::BindingError::message("Exception of type 'System.Exception' was thrown."),
        (String) => |message: String| crate::data::BindingError::message(message),
    ],
    properties: [
        Message: String { get: |error: &crate::data::BindingError| error.to_string() },
    ],
});

// FerroUI.Data

// The exception that wraps the data of a validation error (`DataValidationException :
// Exception`): markup creates one with the error data as the argument of its constructor.
ferro_markup_type!(class crate::data::DataValidationException as "DataValidationException" {
    namespace: "FerroUI.Data",
    handles: [crate::data::DataValidationException, Option<crate::data::DataValidationException>],
    base: crate::data::BindingError,
    constructors: [(Option<BoxedValue>) => crate::data::DataValidationException::new],
    properties: [
        ErrorData: Option<BoxedValue> {
            get: |error: &crate::data::DataValidationException| error.error_data().cloned()
        },
    ],
});

// FerroUI.Media.Imaging

ferro_markup_type!(class Bitmap {
    namespace: "FerroUI.Media.Imaging",
    handles: [Rc<Bitmap>, Option<Rc<Bitmap>>],
    interfaces: [
        Rc<dyn crate::media::IImage>,
        Rc<dyn crate::media::IImageBrushSource>,
        Rc<dyn crate::media::imaging::IBitmap>,
    ],
});

// FerroUI.Media.Immutable

ferro_markup_type!(class ImmutableSolidColorBrush {
    namespace: "FerroUI.Media.Immutable",
    handles: [Rc<ImmutableSolidColorBrush>, Option<Rc<ImmutableSolidColorBrush>>],
    interfaces: [
        Rc<dyn crate::media::IImmutableSolidColorBrush>,
        Rc<dyn crate::media::ISolidColorBrush>,
        Rc<dyn crate::media::IImmutableBrush>,
        Rc<dyn IBrush>,
    ],
    constructors: [
        (u32) => |color: u32| Rc::new(ImmutableSolidColorBrush::from_uint32(color)),
        (crate::media::Color) => |color: crate::media::Color| Rc::new(ImmutableSolidColorBrush::new(color)),
        (crate::media::Color, f64) =>
            |color: crate::media::Color, opacity: f64| Rc::new(ImmutableSolidColorBrush::with_opacity(color, opacity)),
    ],
});

// FerroUI.Metadata

// The untyped `IAddChild` is the contract with any object as the child.
ferro_markup_type!(interface dyn IAddChild<BoxedValue> as "IAddChild" {
    namespace: "FerroUI.Metadata",
    handles: [Rc<dyn IAddChild<BoxedValue>>, Option<Rc<dyn IAddChild<BoxedValue>>>],
    this: Rc<dyn IAddChild<BoxedValue>>,
    methods: [
        fn AddChild(BoxedValue) => |this: &Rc<dyn IAddChild<BoxedValue>>, child: BoxedValue| this.add_child(child),
    ],
});

ferro_markup_type!(interface dyn IAddChild<String> as "IAddChild`1" {
    namespace: "FerroUI.Metadata",
    handles: [Rc<dyn IAddChild<String>>, Option<Rc<dyn IAddChild<String>>>],
    this: Rc<dyn IAddChild<String>>,
    generic: "IAddChild`1" [String],
    methods: [fn AddChild(String) => |this: &Rc<dyn IAddChild<String>>, child: String| this.add_child(child)],
});

// FerroUI.Styling

// The static builders of selectors, with the signatures of the managed
// original: the previous selector comes first and may be null.
ferro_markup_type!(static Selectors {
    namespace: "FerroUI.Styling",
    methods: [
        static fn Child(Option<Selector>) -> Selector => Selectors::child,
        static fn Class(Option<Selector>, String) -> Selector =>
            |previous: Option<Selector>, name: String| Selectors::class(previous, &name),
        static fn Descendant(Option<Selector>) -> Selector => Selectors::descendant,
        static fn Is(Option<Selector>, &'static TypeInfo) -> Selector => Selectors::is_type_info,
        static fn Name(Option<Selector>, String) -> Selector =>
            |previous: Option<Selector>, name: String| Selectors::name(previous, &name),
        static fn Nesting(Option<Selector>) -> Selector => Selectors::nesting,
        static fn Not(Option<Selector>, Selector) -> Selector => Selectors::not,
        static fn NthChild(Option<Selector>, i32, i32) -> Selector => Selectors::nth_child,
        static fn NthLastChild(Option<Selector>, i32, i32) -> Selector => Selectors::nth_last_child,
        static fn OfType(Option<Selector>, &'static TypeInfo) -> Selector => Selectors::of_type_info,
        static fn Or(Vec<Selector>) -> Selector => |selectors: Vec<Selector>| Selectors::or(selectors),
        static try fn PropertyEquals(Option<Selector>, &'static FerroProperty, Option<BoxedValue>) -> Selector =>
            |previous: Option<Selector>, property: &'static FerroProperty, value: Option<BoxedValue>| {
                // The value is compared as a value of the type of the property.
                super::plain::property_value(property, value)
                    .map(|value| Selectors::property_equals_untyped(previous, property, value))
            },
        static fn Template(Option<Selector>) -> Selector => Selectors::template,
    ],
});

ferro_markup_type!(static StyleQueries {
    namespace: "FerroUI.Styling",
    methods: [
        static fn Width(Option<StyleQuery>, StyleQueryComparisonOperator, f64) -> StyleQuery => StyleQueries::width,
        static fn Height(Option<StyleQuery>, StyleQueryComparisonOperator, f64) -> StyleQuery => StyleQueries::height,
        static fn Or(Vec<StyleQuery>) -> StyleQuery => |queries: Vec<StyleQuery>| StyleQueries::or(queries),
        static fn And(Vec<StyleQuery>) -> StyleQuery => |queries: Vec<StyleQuery>| StyleQueries::and(queries),
    ],
});

// FerroUI.Platform

ferro_markup_type!(struct RuntimePlatformInfo {
    namespace: "FerroUI.Platform",
    handles: [RuntimePlatformInfo],
    properties: [
        FormFactor: FormFactorType { get: RuntimePlatformInfo::form_factor },
        IsDesktop: bool { get: |info: &RuntimePlatformInfo| info.is_desktop },
        IsMobile: bool { get: |info: &RuntimePlatformInfo| info.is_mobile },
        IsTV: bool { get: |info: &RuntimePlatformInfo| info.is_tv },
    ],
    constructors: [() => RuntimePlatformInfo::default],
});

ferro_markup_type!(interface dyn IRuntimePlatform as "IRuntimePlatform" {
    namespace: "FerroUI.Platform",
    handles: [Rc<dyn IRuntimePlatform>, Option<Rc<dyn IRuntimePlatform>>],
    this: Rc<dyn IRuntimePlatform>,
    methods: [
        fn GetRuntimeInfo() -> RuntimePlatformInfo => |platform: &Rc<dyn IRuntimePlatform>| platform.get_runtime_info(),
    ],
});

// System

// The arguments of an event without data.
ferro_markup_type!(class EventArgs {
    namespace: "System",
    handles: [EventArgs, Option<EventArgs>],
    constructors: [() => || EventArgs],
    fields: [Empty: EventArgs => || EventArgs::EMPTY],
});

ferro_markup_type!(class CancelEventArgs {
    namespace: "System.ComponentModel",
    handles: [CancelEventArgs, Option<CancelEventArgs>],
    base: EventArgs,
    constructors: [() => CancelEventArgs::new],
    properties: [
        Cancel: bool { get: CancelEventArgs::cancel, set: CancelEventArgs::set_cancel },
    ],
});

// System.ComponentModel

ferro_markup_type!(interface dyn ISupportInitialize as "ISupportInitialize" {
    namespace: "System.ComponentModel",
    handles: [Rc<dyn ISupportInitialize>, Option<Rc<dyn ISupportInitialize>>],
    this: Rc<dyn ISupportInitialize>,
    methods: [
        fn BeginInit() => |this: &Rc<dyn ISupportInitialize>| this.begin_init(),
        try fn EndInit() => |this: &Rc<dyn ISupportInitialize>| this.try_end_init(),
    ],
});

// FerroUI.Utilities

ferro_markup_type!(static TypeUtilities {
    namespace: "FerroUI.Utilities",
});

// FerroUI.Collections

ferro_markup_type!(class FerroListConverter<Option<BoxedValue>> as "FerroListConverter`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroListConverter<Option<BoxedValue>>],
    generic: "FerroListConverter`1" [Option<BoxedValue>],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <crate::data::BindingError as MarkupTyped>::MARKUP,
    <crate::data::DataValidationException as MarkupTyped>::MARKUP,
    <UnsetValueType as MarkupTyped>::MARKUP,
    <FerroObjectExtensionsClass as MarkupTyped>::MARKUP,
    <ClassBindingManager as MarkupTyped>::MARKUP,
    <Classes as MarkupTyped>::MARKUP,
    <dyn BindingExpressionBase as MarkupTyped>::MARKUP,
    <CompiledBindingPath as MarkupTyped>::MARKUP,
    <CompiledBindingPathBuilder as MarkupTyped>::MARKUP,
    <dyn IPropertyInfo as MarkupTyped>::MARKUP,
    <ClrPropertyInfo as MarkupTyped>::MARKUP,
    <PropertyInfoOf<Option<BoxedValue>, Option<BoxedValue>> as MarkupTyped>::MARKUP,
    <ClrPropertyInfoOf<Option<BoxedValue>, Option<BoxedValue>> as MarkupTyped>::MARKUP,
    <dyn IPropertyAccessor as MarkupTyped>::MARKUP,
    <dyn IRoutedEventArgs as MarkupTyped>::MARKUP,
    <RuntimePlatformInfo as MarkupTyped>::MARKUP,
    <dyn IRuntimePlatform as MarkupTyped>::MARKUP,
    <dyn ISupportInitialize as MarkupTyped>::MARKUP,
    <EventArgs as MarkupTyped>::MARKUP,
    <CancelEventArgs as MarkupTyped>::MARKUP,
    <Bitmap as MarkupTyped>::MARKUP,
    <ImmutableSolidColorBrush as MarkupTyped>::MARKUP,
    <dyn IAddChild<BoxedValue> as MarkupTyped>::MARKUP,
    <dyn IAddChild<String> as MarkupTyped>::MARKUP,
    <Selectors as MarkupTyped>::MARKUP,
    <StyleQueries as MarkupTyped>::MARKUP,
    <TypeUtilities as MarkupTyped>::MARKUP,
    <FerroListConverter<Option<BoxedValue>> as MarkupTyped>::MARKUP,
];

/// Registers what the untyped value conversions need to know about the
/// types declared in this file.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<crate::data::BindingError>();
    // A data validation exception is an exception: the error that wraps it.
    ValueTypes::register_nullable::<crate::data::DataValidationException>();
    ValueTypes::register_cast::<crate::data::DataValidationException, crate::data::BindingError>(|error| {
        crate::data::BindingError::new(error.clone())
    });
    // A bitmap is an image and a source of image brushes.
    ValueTypes::register_nullable::<Rc<Bitmap>>();
    ValueTypes::register_cast::<Rc<Bitmap>, Rc<dyn crate::media::IImage>>(|bitmap| bitmap.clone());
    ValueTypes::register_cast::<Rc<Bitmap>, Rc<dyn crate::media::IImageBrushSource>>(|bitmap| bitmap.clone());
    ValueTypes::register_cast::<Rc<Bitmap>, Rc<dyn crate::media::imaging::IBitmap>>(|bitmap| bitmap.clone());
    ValueTypes::register_nullable::<UnsetValueType>();
    ValueTypes::register_nullable::<EventArgs>();
    ValueTypes::register_nullable::<CancelEventArgs>();
    ValueTypes::register_nullable::<RuntimePlatformInfo>();
    ValueTypes::register_nullable::<CompiledBindingPath>();
    ValueTypes::register_nullable::<CompiledBindingPathBuilder>();
    ValueTypes::register_nullable::<Rc<dyn IPropertyInfo>>();
    ValueTypes::register_nullable::<Rc<dyn IPropertyAccessor>>();
    ValueTypes::register_nullable::<Rc<dyn IRoutedEventArgs>>();
    ValueTypes::register_nullable::<Rc<dyn IRuntimePlatform>>();
    ValueTypes::register_nullable::<Rc<dyn ISupportInitialize>>();
    ValueTypes::register_nullable::<Rc<dyn BindingExpressionBase>>();
    ValueTypes::register_nullable::<Rc<dyn IDisposable>>();
    // A binding expression is disposable.
    ValueTypes::register_cast::<Rc<dyn BindingExpressionBase>, Rc<dyn IDisposable>>(|expression| expression.clone());
    ValueTypes::register_nullable::<Classes>();
    ValueTypes::register_nullable::<Rc<ImmutableSolidColorBrush>>();
    ValueTypes::register_nullable::<Rc<dyn IAddChild<BoxedValue>>>();
    ValueTypes::register_nullable::<Rc<dyn IAddChild<String>>>();
    // An immutable brush is a brush.
    ValueTypes::register_cast::<Rc<ImmutableSolidColorBrush>, Rc<dyn IBrush>>(|brush| brush.clone());
    ValueTypes::register_cast::<Rc<ImmutableSolidColorBrush>, Rc<dyn crate::media::IImmutableSolidColorBrush>>(
        |brush| brush.clone(),
    );
    ValueTypes::register_cast::<Rc<ImmutableSolidColorBrush>, Rc<dyn crate::media::ISolidColorBrush>>(|brush| brush.clone());
    ValueTypes::register_cast::<Rc<ImmutableSolidColorBrush>, Rc<dyn crate::media::IImmutableBrush>>(|brush| brush.clone());
}
