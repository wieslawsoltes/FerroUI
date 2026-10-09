//! The closed table of runtime library (`System.*`) types markup can name,
//! each mapped to the Rust types that hold its values, and the property
//! definition types of the framework.
//!
//! The shape of the types (kind, type parameters, base, interfaces, members)
//! is the shared table of the compiler ([`crate::core_table`]), which the
//! build-time type system defines the same types from. What is stated here is
//! what only the run-time type system has: the Rust types that hold the
//! values of a type ([`handles_of`]) and the invokers of the members the
//! table marks as implemented ([`invoker_of`], [`list_members`]).

use std::rc::Rc;

use ferroui_base::animation::TimeSpan;
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupDelegate, MarkupInvokeError, MarkupValue};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{CultureInfo, Uri};
use ferroui_base::{BoxedValue, FerroProperty, TypeInfo};
use xamlx::type_system::{IXamlType, IXamlTypeSystem, XamlPseudoType};

use crate::core_table::{self, CoreBody, CoreKind, CoreMember, CoreRef, CoreType};

use super::runtime_type::{RuntimeInvoker, RuntimeTypeKind};
use super::runtime_type_system::{MemberBuilder, RuntimeTypeSystem};
use super::values::{DeferredContentFactory, ITypeDescriptorContext, RuntimeList, RuntimeTypeValue};

use RuntimeTypeKind::{Class, Interface, Struct};

/// The generic definition of the list of the runtime library.
pub(crate) const LIST_DEFINITION: &str = core_table::LIST_DEFINITION;
/// The untyped list of the runtime library.
pub(crate) const ARRAY_LIST: &str = core_table::ARRAY_LIST;

fn dynamic(
    invoke: impl Fn(&[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> + 'static,
) -> RuntimeInvoker {
    RuntimeInvoker::Dynamic(Rc::new(invoke))
}

fn argument<T: Clone + 'static>(arguments: &[MarkupValue], index: usize) -> Result<T, MarkupInvokeError> {
    let value = arguments
        .get(index)
        .ok_or(MarkupInvokeError::ArgumentCount { expected: index + 1, actual: arguments.len() })?;
    from_markup_value::<T>(value).ok_or_else(|| MarkupInvokeError::Argument {
        index,
        expected: std::any::type_name::<T>(),
        actual: match value {
            Some(value) => value.type_name().to_string(),
            None => "null".to_string(),
        },
    })
}

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

macro_rules! handles {
    ($($type_:ty),* $(,)?) => {
        vec![$(ValueType::of::<$type_>()),*]
    };
}

/// The type a reference of the table names, for the type `b` builds. `element` is the
/// element type of a list markup creates.
fn type_of(b: &MemberBuilder, reference: &CoreRef, element: Option<&Rc<dyn IXamlType>>) -> Rc<dyn IXamlType> {
    match reference {
        CoreRef::Type(full_name) => b.t(full_name),
        CoreRef::Parameter(index) => b.parameter(*index),
        CoreRef::Element => match element {
            Some(element) => element.clone(),
            None => b.t("System.Object"),
        },
        CoreRef::Generic(definition, arguments) => {
            let arguments: Vec<Rc<dyn IXamlType>> = arguments.iter().map(|argument| type_of(b, argument, element)).collect();
            b.generic(definition, &arguments)
        }
        CoreRef::Array(element_type) => {
            type_of(b, element_type, element).make_array_type(1).unwrap_or_else(|_| XamlPseudoType::unknown())
        }
    }
}

fn types_of(b: &MemberBuilder, references: &[CoreRef], element: Option<&Rc<dyn IXamlType>>) -> Vec<Rc<dyn IXamlType>> {
    references.iter().map(|reference| type_of(b, reference, element)).collect()
}

/// Defines what the description of a type of the table states on the type `b` builds:
/// its base, its interfaces and its members. `invoker` gives the invoker of a member the
/// table marks as implemented.
fn apply(
    b: &mut MemberBuilder,
    description: &CoreType,
    element: Option<&Rc<dyn IXamlType>>,
    invoker: &dyn Fn(&CoreMember) -> RuntimeInvoker,
) {
    match &description.base {
        Some(CoreRef::Type(full_name)) => b.base(full_name),
        Some(base) => {
            let base = type_of(b, base, element);
            b.base_type(base);
        }
        None => {}
    }
    for interface in &description.interfaces {
        let interface = type_of(b, interface, element);
        b.interface(interface);
    }
    // One source: the metadata the type declares; the members of the table only stand in
    // while no metadata is registered.
    if description.stands_in && b.project_metadata(&description.namespace, &description.name) {
        return;
    }
    for member in &description.members {
        let invoker_for = |body: CoreBody| match body {
            CoreBody::Implemented => invoker(member),
            CoreBody::Virtual => RuntimeInvoker::Virtual,
            CoreBody::None => RuntimeInvoker::None,
        };
        match member {
            CoreMember::Constructor { parameters, body } => {
                let parameters = types_of(b, parameters, element);
                b.constructor(parameters, invoker_for(*body));
            }
            CoreMember::Method { name, is_static, return_type, parameters, body } => {
                let return_type = type_of(b, return_type, element);
                let parameters = types_of(b, parameters, element);
                b.method(name, *is_static, return_type, parameters, invoker_for(*body));
            }
            CoreMember::Property { name, type_, is_static, body } => {
                let type_ = type_of(b, type_, element);
                b.property(name, type_, *is_static, invoker_for(*body));
            }
            CoreMember::Indexer { parameters, type_, writable } => {
                let parameters = types_of(b, parameters, element);
                let type_ = type_of(b, type_, element);
                b.indexer(parameters, type_, *writable);
            }
        }
    }
}

/// The members of a list of the runtime library that markup creates: `List<T>` for the
/// element type `T`, `ArrayList` without one. Its values are run-time lists
/// ([`RuntimeList`]); the members are the ones markup and binding paths use (the
/// constructor, `Add`, `Count` and the indexer), as the table describes them
/// ([`core_table::list_members`]).
pub(crate) fn list_members(b: &mut MemberBuilder, element_type: Option<Rc<dyn IXamlType>>) {
    fn instance(arguments: &[MarkupValue]) -> Result<RuntimeList, MarkupInvokeError> {
        argument::<RuntimeList>(arguments, 0)
    }
    // The `ArgumentOutOfRangeException` of the indexer.
    fn index(list: &RuntimeList, arguments: &[MarkupValue]) -> Result<usize, MarkupInvokeError> {
        let index = argument::<i32>(arguments, 1)?;
        usize::try_from(index).ok().filter(|index| *index < list.count()).ok_or_else(|| {
            MarkupInvokeError::Failed(
                "Index was out of range. Must be non-negative and less than the size of the collection.".to_string(),
            )
        })
    }

    let description = core_table::list_members(element_type.is_some());
    // `List<T>.Add(T)` returns nothing, `ArrayList.Add(object)` the index of the item.
    let returns_index = element_type.is_none();
    let invoker = |member: &CoreMember| match member {
        CoreMember::Constructor { .. } => {
            let element_type = element_type.clone();
            dynamic(move |_| Ok(boxed(RuntimeList::new(element_type.clone()))))
        }
        CoreMember::Method { name, .. } if name == "Add" => dynamic(move |arguments| {
            let list = instance(arguments)?;
            let value =
                arguments.get(1).ok_or(MarkupInvokeError::ArgumentCount { expected: 2, actual: arguments.len() })?;
            let added = list.add(value.clone());
            Ok(if returns_index { boxed(added as i32) } else { None })
        }),
        CoreMember::Method { name, .. } if name == "get_Item" => dynamic(|arguments| {
            let list = instance(arguments)?;
            Ok(list.get(index(&list, arguments)?))
        }),
        CoreMember::Method { name, .. } if name == "set_Item" => dynamic(|arguments| {
            let list = instance(arguments)?;
            let value =
                arguments.get(2).ok_or(MarkupInvokeError::ArgumentCount { expected: 3, actual: arguments.len() })?;
            list.set(index(&list, arguments)?, value.clone());
            Ok(None)
        }),
        CoreMember::Property { .. } => dynamic(|arguments| Ok(boxed(instance(arguments)?.count() as i32))),
        _ => RuntimeInvoker::None,
    };
    apply(b, &description, element_type.as_ref(), &invoker);
}

/// The Rust types that hold the values of the type of the table named `full_name`, the
/// untyped (canonical) form first.
fn handles_of(full_name: &str) -> Vec<ValueType> {
    match full_name {
        "System.Object" => handles![Option<BoxedValue>, BoxedValue],
        "System.Void" => handles![()],
        "System.Boolean" => handles![bool],
        "System.Char" => handles![char],
        "System.SByte" => handles![i8],
        "System.Byte" => handles![u8],
        "System.Int16" => handles![i16],
        "System.UInt16" => handles![u16],
        "System.Int32" => handles![i32],
        "System.UInt32" => handles![u32],
        "System.Int64" => handles![i64],
        "System.UInt64" => handles![u64],
        "System.Single" => handles![f32],
        "System.Double" => handles![f64],
        "System.TimeSpan" => handles![TimeSpan],
        "System.String" => handles![String, Option<String>],
        "System.Type" => handles![
            RuntimeTypeValue,
            Option<RuntimeTypeValue>,
            &'static TypeInfo,
            Option<&'static TypeInfo>,
            ValueType,
            Option<ValueType>
        ],
        "System.Uri" => handles![Uri, Option<Uri>],
        "System.Delegate" => handles![MarkupDelegate, Option<MarkupDelegate>],
        "System.Exception" => handles![ferroui_base::data::BindingError, Option<ferroui_base::data::BindingError>],
        "System.IDisposable" => handles![Rc<dyn IDisposable>, Option<Rc<dyn IDisposable>>],
        "System.Globalization.CultureInfo" => handles![CultureInfo, Option<CultureInfo>],
        "System.IServiceProvider" => handles![Rc<dyn IServiceProvider>, Option<Rc<dyn IServiceProvider>>],
        "System.ComponentModel.ITypeDescriptorContext" => {
            handles![Rc<dyn ITypeDescriptorContext>, Option<Rc<dyn ITypeDescriptorContext>>]
        }
        // The handle lets a list type declared in metadata state the contract
        // (`interfaces: [Rc<dyn INotifyCollectionChanged>]`): an indexer of such a type in a
        // compiled binding path is observed through the collection changes.
        "System.Collections.Specialized.INotifyCollectionChanged" => handles![
            Rc<dyn ferroui_base::data::model::INotifyCollectionChanged>,
            Option<Rc<dyn ferroui_base::data::model::INotifyCollectionChanged>>
        ],
        _ => Vec::new(),
    }
}

/// The invoker of a member of the type of the table named `full_name` that the table
/// marks as implemented.
fn invoker_of(full_name: &str, member: &CoreMember) -> RuntimeInvoker {
    match (full_name, member) {
        ("System.TimeSpan", CoreMember::Method { name, .. }) if name == "Parse" => dynamic(|arguments| {
            let text = argument::<String>(arguments, 0)?;
            TimeSpan::parse(&text).map(boxed).map_err(|e| MarkupInvokeError::Failed(e.to_string()))
        }),
        // `string.Length`: the number of UTF-16 code units.
        ("System.String", CoreMember::Property { name, .. }) if name == "Length" => dynamic(|arguments| {
            let text = argument::<String>(arguments, 0)?;
            Ok(boxed(text.encode_utf16().count() as i32))
        }),
        ("System.Array", CoreMember::Property { name, .. }) if name == "Length" => dynamic(|arguments| {
            let array = argument::<super::values::RuntimeArray>(arguments, 0)?;
            Ok(boxed(array.items().len() as i32))
        }),
        ("System.Uri", CoreMember::Constructor { .. }) => dynamic(|arguments| {
            let text = argument::<String>(arguments, 0)?;
            Uri::absolute(&text).map(boxed).map_err(|e| MarkupInvokeError::Failed(e.to_string()))
        }),
        ("System.Globalization.CultureInfo", CoreMember::Property { name, .. }) if name == "InvariantCulture" => {
            dynamic(|_| Ok(boxed(CultureInfo::invariant_culture())))
        }
        _ => RuntimeInvoker::None,
    }
}

/// Defines the runtime library types.
pub(crate) fn define_core_types(system: &Rc<RuntimeTypeSystem>) {
    let core = system.core_assembly();
    for description in core_table::core_types() {
        let kind = match description.kind {
            CoreKind::Class => Class,
            CoreKind::Interface => Interface,
            CoreKind::Struct => Struct,
        };
        let full_name = description.full_name();
        let namespace = description.namespace.clone();
        let name = description.name.clone();
        let parameter_names = description.parameters.clone();
        let parameters: Vec<&str> = parameter_names.iter().map(String::as_str).collect();
        system.define_synthetic(&core, &namespace, &name, kind, &parameters, handles_of(&full_name), move |b| {
            // The untyped list: any children, null included.
            if full_name == ARRAY_LIST {
                list_members(b, None);
            } else {
                apply(b, &description, None, &|member: &CoreMember| invoker_of(&full_name, member));
            }
        });
    }

    // The nullable form of a value type of the table is `System.Nullable<T>`.
    let map_nullable = |full_name: &str, handle: ValueType| {
        let (Some(nullable), Some(type_)) = (system.find_type("System.Nullable`1"), system.find_type(full_name)) else {
            return;
        };
        if let Ok(nullable) = nullable.make_generic_type(std::slice::from_ref(&type_)) {
            system.map_handle(handle, &nullable);
        }
    };
    map_nullable("System.Boolean", ValueType::of::<Option<bool>>());
    map_nullable("System.Char", ValueType::of::<Option<char>>());
    map_nullable("System.SByte", ValueType::of::<Option<i8>>());
    map_nullable("System.Byte", ValueType::of::<Option<u8>>());
    map_nullable("System.Int16", ValueType::of::<Option<i16>>());
    map_nullable("System.UInt16", ValueType::of::<Option<u16>>());
    map_nullable("System.Int32", ValueType::of::<Option<i32>>());
    map_nullable("System.UInt32", ValueType::of::<Option<u32>>());
    map_nullable("System.Int64", ValueType::of::<Option<i64>>());
    map_nullable("System.UInt64", ValueType::of::<Option<u64>>());
    map_nullable("System.Single", ValueType::of::<Option<f32>>());
    map_nullable("System.Double", ValueType::of::<Option<f64>>());
    map_nullable("System.TimeSpan", ValueType::of::<Option<TimeSpan>>());

    // Arrays of the element types the compiler produces constants of: a member
    // that declares `Vec<T>` takes an array of `T` (see `values`).
    macro_rules! array_element {
        ($name:expr, $type_:ty) => {{
            super::values::RuntimeArray::register_element::<$type_>();
            if let Some(Ok(array)) = system.find_type($name).map(|t| t.make_array_type(1)) {
                system.map_handle(ValueType::of::<Vec<$type_>>(), &array);
                system.map_handle(ValueType::of::<Option<Vec<$type_>>>(), &array);
            }
        }};
    }
    array_element!("System.Boolean", bool);
    array_element!("System.Byte", u8);
    array_element!("System.Int32", i32);
    array_element!("System.Int64", i64);
    array_element!("System.Single", f32);
    array_element!("System.Double", f64);
    super::values::RuntimeArray::register_element::<String>();
    super::values::RuntimeArray::register_element::<ferroui_base::Point>();
    super::values::RuntimeArray::register_element::<BoxedValue>();

    // The deferred content delegate: `Func<IServiceProvider, object>`.
    let arguments = [system.get("System.IServiceProvider"), system.get("System.Object")];
    if let Some(Ok(func)) = system.find_type("System.Func`2").map(|f| f.make_generic_type(&arguments)) {
        system.map_handle(ValueType::of::<DeferredContentFactory>(), &func);
    }
}

/// Defines the types of property definitions (`FerroUI.FerroProperty` and
/// the generic `StyledProperty<T>`, `AttachedProperty<T>`,
/// `DirectProperty<TOwner, T>`, deriving from each other as the classes of
/// the managed original), unless types with these names are registered.
pub(crate) fn define_property_types(system: &RuntimeTypeSystem) {
    let descriptions = core_table::property_types();
    if descriptions.iter().any(|description| description.name == "StyledProperty`1" && system.find_type(&description.full_name()).is_some())
    {
        return;
    }
    let assembly = system.framework_assembly_for_types();
    for description in descriptions {
        if system.find_type(&description.full_name()).is_some() {
            continue;
        }
        let handles = if description.parameters.is_empty() {
            handles![&'static FerroProperty, Option<&'static FerroProperty>]
        } else {
            Vec::new()
        };
        let namespace = description.namespace.clone();
        let name = description.name.clone();
        let parameter_names = description.parameters.clone();
        let parameters: Vec<&str> = parameter_names.iter().map(String::as_str).collect();
        system.define_synthetic(&assembly, &namespace, &name, Class, &parameters, handles, move |b| {
            // The base of a definition with a value type is the base definition of that
            // value type.
            if let Some(base) = &description.base {
                let base = match base {
                    CoreRef::Type(full_name) => b.t(full_name),
                    other => type_of(b, other, None),
                };
                b.base_type(base);
            }
        });
    }
}
