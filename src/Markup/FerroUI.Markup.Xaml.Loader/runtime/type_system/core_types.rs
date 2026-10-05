//! The closed table of runtime library (`System.*`) types markup can name,
//! each mapped to the Rust types that hold its values, and the property
//! definition types of the framework.

use std::rc::Rc;

use ferroui_base::animation::TimeSpan;
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupDelegate, MarkupInvokeError, MarkupValue};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{CultureInfo, Uri};
use ferroui_base::{BoxedValue, FerroProperty, TypeInfo};
use xamlx::type_system::{IXamlType, IXamlTypeSystem};

use super::runtime_type::{RuntimeInvoker, RuntimeTypeKind};
use super::runtime_type_system::{MemberBuilder, RuntimeTypeSystem, PROPERTY_NAMESPACE};
use super::values::{DeferredContentFactory, ITypeDescriptorContext, RuntimeTypeValue};

use RuntimeTypeKind::{Class, Interface, Struct};

const SYSTEM: &str = "System";
const COLLECTIONS: &str = "System.Collections";
const GENERIC: &str = "System.Collections.Generic";
const COMPONENT_MODEL: &str = "System.ComponentModel";

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

/// Defines the runtime library types.
pub(crate) fn define_core_types(system: &Rc<RuntimeTypeSystem>) {
    let core = system.core_assembly();
    let define = |namespace: &str,
                  name: &str,
                  kind: RuntimeTypeKind,
                  parameters: &[&str],
                  handles: Vec<ValueType>,
                  init: Box<dyn FnOnce(&mut MemberBuilder)>| {
        system.define_synthetic(&core, namespace, name, kind, parameters, handles, init)
    };
    let plain = |namespace: &str, name: &str, kind: RuntimeTypeKind, base: Option<&'static str>| {
        define(
            namespace,
            name,
            kind,
            &[],
            Vec::new(),
            Box::new(move |b| {
                if let Some(base) = base {
                    b.base(base);
                }
            }),
        )
    };

    define(SYSTEM, "Object", Class, &[], handles![Option<BoxedValue>, BoxedValue], Box::new(|_| {}));
    plain(SYSTEM, "ValueType", Class, None);
    plain(SYSTEM, "Enum", Class, Some("System.ValueType"));
    plain(SYSTEM, "Attribute", Class, None);
    define(SYSTEM, "Void", Struct, &[], handles![()], Box::new(|b| b.base("System.ValueType")));
    plain(SYSTEM, "IntPtr", Struct, Some("System.ValueType"));

    let nullable = define(SYSTEM, "Nullable`1", Struct, &["T"], Vec::new(), Box::new(|b| b.base("System.ValueType")));

    macro_rules! primitive {
        ($name:literal, $type_:ty) => {{
            let type_: Rc<dyn IXamlType> =
                define(SYSTEM, $name, Struct, &[], handles![$type_], Box::new(|b| b.base("System.ValueType")));
            if let Ok(nullable) = nullable.make_generic_type(std::slice::from_ref(&type_)) {
                system.map_handle(ValueType::of::<Option<$type_>>(), &nullable);
            }
        }};
    }
    primitive!("Boolean", bool);
    primitive!("Char", char);
    primitive!("SByte", i8);
    primitive!("Byte", u8);
    primitive!("Int16", i16);
    primitive!("UInt16", u16);
    primitive!("Int32", i32);
    primitive!("UInt32", u32);
    primitive!("Int64", i64);
    primitive!("UInt64", u64);
    primitive!("Single", f32);
    primitive!("Double", f64);

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

    let time_span: Rc<dyn IXamlType> = define(
        SYSTEM,
        "TimeSpan",
        Struct,
        &[],
        handles![TimeSpan],
        Box::new(|b| {
            b.base("System.ValueType");
            // One source: the metadata the type declares; the member below only
            // stands in while no metadata is registered.
            if b.project_metadata(SYSTEM, "TimeSpan") {
                return;
            }
            b.method(
                "Parse",
                true,
                b.t("System.TimeSpan"),
                vec![b.t("System.String")],
                dynamic(|arguments| {
                    let text = argument::<String>(arguments, 0)?;
                    TimeSpan::parse(&text).map(boxed).map_err(|e| MarkupInvokeError::Failed(e.to_string()))
                }),
            );
        }),
    );
    if let Ok(nullable) = nullable.make_generic_type(std::slice::from_ref(&time_span)) {
        system.map_handle(ValueType::of::<Option<TimeSpan>>(), &nullable);
    }

    define(
        SYSTEM,
        "String",
        Class,
        &[],
        handles![String, Option<String>],
        Box::new(|b| {
            // `string.Length`: the number of UTF-16 code units.
            b.property(
                "Length",
                b.t("System.Int32"),
                false,
                dynamic(|arguments| {
                    let text = argument::<String>(arguments, 0)?;
                    Ok(boxed(text.encode_utf16().count() as i32))
                }),
            );
        }),
    );
    define(
        SYSTEM,
        "Type",
        Class,
        &[],
        handles![
            RuntimeTypeValue,
            Option<RuntimeTypeValue>,
            &'static TypeInfo,
            Option<&'static TypeInfo>,
            ValueType,
            Option<ValueType>
        ],
        Box::new(|_| {}),
    );
    define(
        SYSTEM,
        "Array",
        Class,
        &[],
        Vec::new(),
        Box::new(|b| {
            b.property(
                "Length",
                b.t("System.Int32"),
                false,
                dynamic(|arguments| {
                    let array = argument::<super::values::RuntimeArray>(arguments, 0)?;
                    Ok(boxed(array.items().len() as i32))
                }),
            );
        }),
    );
    define(
        SYSTEM,
        "Uri",
        Class,
        &[],
        handles![Uri, Option<Uri>],
        Box::new(|b| {
            if b.project_metadata(SYSTEM, "Uri") {
                return;
            }
            b.constructor(
                vec![b.t("System.String")],
                dynamic(|arguments| {
                    let text = argument::<String>(arguments, 0)?;
                    Uri::absolute(&text).map(boxed).map_err(|e| MarkupInvokeError::Failed(e.to_string()))
                }),
            );
        }),
    );
    define(SYSTEM, "Delegate", Class, &[], handles![MarkupDelegate, Option<MarkupDelegate>], Box::new(|_| {}));
    plain(SYSTEM, "MulticastDelegate", Class, Some("System.Delegate"));
    define(
        SYSTEM,
        "Exception",
        Class,
        &[],
        handles![ferroui_base::data::BindingError, Option<ferroui_base::data::BindingError>],
        Box::new(|b| {
            // One source: the metadata the base crate declares for its error type.
            let _ = b.project_metadata(SYSTEM, "Exception");
        }),
    );
    for name in ["InvalidCastException", "NotSupportedException", "NullReferenceException"] {
        define(
            SYSTEM,
            name,
            Class,
            &[],
            Vec::new(),
            Box::new(|b| {
                b.base("System.Exception");
                b.constructor(Vec::new(), RuntimeInvoker::None);
            }),
        );
    }
    for (namespace, name) in [
        (SYSTEM, "ObsoleteAttribute"),
        (SYSTEM, "FlagsAttribute"),
        (SYSTEM, "AttributeUsageAttribute"),
        ("System.Diagnostics.CodeAnalysis", "ExperimentalAttribute"),
        (COMPONENT_MODEL, "TypeConverterAttribute"),
    ] {
        plain(namespace, name, Class, Some("System.Attribute"));
    }

    define(
        SYSTEM,
        "IDisposable",
        Interface,
        &[],
        handles![Rc<dyn IDisposable>, Option<Rc<dyn IDisposable>>],
        Box::new(|b| {
            b.method("Dispose", false, b.t("System.Void"), Vec::new(), RuntimeInvoker::Virtual);
        }),
    );
    plain(SYSTEM, "IFormatProvider", Interface, None);
    define(
        "System.Globalization",
        "CultureInfo",
        Class,
        &[],
        handles![CultureInfo, Option<CultureInfo>],
        Box::new(|b| {
            b.interface(b.t("System.IFormatProvider"));
            b.property(
                "InvariantCulture",
                b.t("System.Globalization.CultureInfo"),
                true,
                dynamic(|_| Ok(boxed(CultureInfo::invariant_culture()))),
            );
        }),
    );
    plain("System.Reflection", "MethodInfo", Class, None);

    define(
        SYSTEM,
        "IServiceProvider",
        Interface,
        &[],
        handles![Rc<dyn IServiceProvider>, Option<Rc<dyn IServiceProvider>>],
        Box::new(|b| {
            // Services are identified by Rust handle types and are not
            // untyped values: the method exists for the transformers.
            b.method("GetService", false, b.t("System.Object"), vec![b.t("System.Type")], RuntimeInvoker::None);
        }),
    );
    define(
        COMPONENT_MODEL,
        "ITypeDescriptorContext",
        Interface,
        &[],
        handles![Rc<dyn ITypeDescriptorContext>, Option<Rc<dyn ITypeDescriptorContext>>],
        Box::new(|b| b.interface(b.t("System.IServiceProvider"))),
    );
    define(
        COMPONENT_MODEL,
        "ISupportInitialize",
        Interface,
        &[],
        Vec::new(),
        Box::new(|b| {
            b.method("BeginInit", false, b.t("System.Void"), Vec::new(), RuntimeInvoker::Virtual);
            b.method("EndInit", false, b.t("System.Void"), Vec::new(), RuntimeInvoker::Virtual);
        }),
    );
    define(
        COMPONENT_MODEL,
        "TypeConverter",
        Class,
        &[],
        Vec::new(),
        Box::new(|b| {
            b.constructor(Vec::new(), RuntimeInvoker::None);
            b.method(
                "ConvertFrom",
                false,
                b.t("System.Object"),
                vec![
                    b.t("System.ComponentModel.ITypeDescriptorContext"),
                    b.t("System.Globalization.CultureInfo"),
                    b.t("System.Object"),
                ],
                RuntimeInvoker::Virtual,
            );
        }),
    );

    // Collections.
    // The handle lets a list type declared in metadata state the contract
    // (`interfaces: [Rc<dyn INotifyCollectionChanged>]`): an indexer of such a type in a
    // compiled binding path is observed through the collection changes.
    define(
        "System.Collections.Specialized",
        "INotifyCollectionChanged",
        Interface,
        &[],
        handles![
            Rc<dyn ferroui_base::data::model::INotifyCollectionChanged>,
            Option<Rc<dyn ferroui_base::data::model::INotifyCollectionChanged>>
        ],
        Box::new(|_| {}),
    );
    plain(COLLECTIONS, "IEnumerator", Interface, None);
    define(
        COLLECTIONS,
        "IEnumerable",
        Interface,
        &[],
        Vec::new(),
        Box::new(|b| {
            b.method("GetEnumerator", false, b.t("System.Collections.IEnumerator"), Vec::new(), RuntimeInvoker::Virtual);
        }),
    );
    define(
        COLLECTIONS,
        "IList",
        Interface,
        &[],
        Vec::new(),
        Box::new(|b| {
            b.interface(b.t("System.Collections.IEnumerable"));
            b.method("Add", false, b.t("System.Int32"), vec![b.t("System.Object")], RuntimeInvoker::Virtual);
        }),
    );
    define(
        GENERIC,
        "IEnumerator`1",
        Interface,
        &["T"],
        Vec::new(),
        Box::new(|b| b.interface(b.t("System.Collections.IEnumerator"))),
    );
    define(
        GENERIC,
        "IEnumerable`1",
        Interface,
        &["T"],
        Vec::new(),
        Box::new(|b| b.interface(b.t("System.Collections.IEnumerable"))),
    );
    define(
        GENERIC,
        "ICollection`1",
        Interface,
        &["T"],
        Vec::new(),
        Box::new(|b| {
            let item = b.parameter(0);
            b.interface(b.generic("System.Collections.Generic.IEnumerable`1", std::slice::from_ref(&item)));
            b.method("Add", false, b.t("System.Void"), vec![item], RuntimeInvoker::Virtual);
            b.property("Count", b.t("System.Int32"), false, RuntimeInvoker::Virtual);
        }),
    );
    define(
        GENERIC,
        "IList`1",
        Interface,
        &["T"],
        Vec::new(),
        Box::new(|b| {
            let item = b.parameter(0);
            b.interface(b.generic("System.Collections.Generic.ICollection`1", std::slice::from_ref(&item)));
            b.indexer(vec![b.t("System.Int32")], item, true);
        }),
    );
    define(
        GENERIC,
        "IReadOnlyList`1",
        Interface,
        &["T"],
        Vec::new(),
        Box::new(|b| {
            let item = b.parameter(0);
            b.interface(b.generic("System.Collections.Generic.IEnumerable`1", std::slice::from_ref(&item)));
            b.property("Count", b.t("System.Int32"), false, RuntimeInvoker::Virtual);
            b.indexer(vec![b.t("System.Int32")], item, false);
        }),
    );
    define(
        GENERIC,
        "List`1",
        Class,
        &["T"],
        Vec::new(),
        Box::new(|b| {
            let item = b.parameter(0);
            b.constructor(Vec::new(), RuntimeInvoker::None);
            b.interface(b.generic("System.Collections.Generic.IList`1", std::slice::from_ref(&item)));
            b.interface(b.generic("System.Collections.Generic.IReadOnlyList`1", std::slice::from_ref(&item)));
            b.interface(b.t("System.Collections.IList"));
            b.method("Add", false, b.t("System.Void"), vec![item.clone()], RuntimeInvoker::Virtual);
            b.property("Count", b.t("System.Int32"), false, RuntimeInvoker::Virtual);
            b.indexer(vec![b.t("System.Int32")], item, true);
        }),
    );
    define(
        GENERIC,
        "Dictionary`2",
        Class,
        &["TKey", "TValue"],
        Vec::new(),
        Box::new(|b| {
            b.constructor(Vec::new(), RuntimeInvoker::None);
            b.method("Add", false, b.t("System.Void"), vec![b.parameter(0), b.parameter(1)], RuntimeInvoker::Virtual);
            b.property("Count", b.t("System.Int32"), false, RuntimeInvoker::Virtual);
            b.indexer(vec![b.parameter(0)], b.parameter(1), true);
        }),
    );

    define(
        GENERIC,
        "IDictionary`2",
        Interface,
        &["TKey", "TValue"],
        Vec::new(),
        Box::new(|b| {
            b.method("Add", false, b.t("System.Void"), vec![b.parameter(0), b.parameter(1)], RuntimeInvoker::Virtual);
            b.indexer(vec![b.parameter(0)], b.parameter(1), true);
        }),
    );

    // What the compiled binding paths of the framework language look up by
    // name: observables, tasks and weak references are recognised by their
    // generic definition; instantiations come from registered metadata.
    define(SYSTEM, "IObservable`1", Interface, &["T"], Vec::new(), Box::new(|_| {}));
    define("System.Threading.Tasks", "Task", Class, &[], Vec::new(), Box::new(|_| {}));
    define(
        "System.Threading.Tasks",
        "Task`1",
        Class,
        &["TResult"],
        Vec::new(),
        Box::new(|b| b.base("System.Threading.Tasks.Task")),
    );
    define(SYSTEM, "WeakReference`1", Class, &["T"], Vec::new(), Box::new(|_| {}));
    define(
        COMPONENT_MODEL,
        "CultureInfoConverter",
        Class,
        &[],
        Vec::new(),
        Box::new(|b| {
            b.base("System.ComponentModel.TypeConverter");
            b.constructor(Vec::new(), RuntimeInvoker::None);
        }),
    );

    // Delegates.
    let delegate = |name: String, parameters: Vec<String>, has_result: bool| {
        let names: Vec<&str> = parameters.iter().map(String::as_str).collect();
        let count = parameters.len();
        define(
            SYSTEM,
            &name,
            Class,
            &names,
            Vec::new(),
            Box::new(move |b| {
                b.base("System.MulticastDelegate");
                b.constructor(vec![b.t("System.Object"), b.t("System.IntPtr")], RuntimeInvoker::None);
                let (arguments, result) = if has_result {
                    ((0..count - 1).map(|i| b.parameter(i)).collect(), b.parameter(count - 1))
                } else {
                    ((0..count).map(|i| b.parameter(i)).collect(), b.t("System.Void"))
                };
                b.method("Invoke", false, result, arguments, RuntimeInvoker::Virtual);
            }),
        )
    };
    delegate("Action".to_string(), Vec::new(), false);
    for count in 1..=16usize {
        delegate(format!("Action`{count}"), (1..=count).map(|i| format!("T{i}")).collect(), false);
    }
    for count in 1..=17usize {
        let mut names: Vec<String> = (1..count).map(|i| format!("T{i}")).collect();
        names.push("TResult".to_string());
        delegate(format!("Func`{count}"), names, true);
    }

    define(
        SYSTEM,
        "EventHandler`1",
        Class,
        &["TEventArgs"],
        Vec::new(),
        Box::new(|b| {
            b.base("System.MulticastDelegate");
            b.constructor(vec![b.t("System.Object"), b.t("System.IntPtr")], RuntimeInvoker::None);
            b.method("Invoke", false, b.t("System.Void"), vec![b.t("System.Object"), b.parameter(0)], RuntimeInvoker::Virtual);
        }),
    );

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
    let name = |name: &str| format!("{PROPERTY_NAMESPACE}.{name}");
    if system.find_type(&name("StyledProperty`1")).is_some() {
        return;
    }
    let assembly = system.framework_assembly_for_types();
    if system.find_type(&name("FerroProperty")).is_none() {
        system.define_synthetic(
            &assembly,
            PROPERTY_NAMESPACE,
            "FerroProperty",
            Class,
            &[],
            handles![&'static FerroProperty, Option<&'static FerroProperty>],
            |_| {},
        );
    }
    let derived = |type_name: &'static str, parameters: &[&str], base: &'static str| {
        if system.find_type(&name(type_name)).is_some() {
            return;
        }
        let base = name(base);
        system.define_synthetic(&assembly, PROPERTY_NAMESPACE, type_name, Class, parameters, Vec::new(), move |b| {
            let value = b.parameter(b.type_.generic_parameter_types().len() - 1);
            let base_type = b.t(&base);
            if base_type.generic_parameters().is_empty() {
                b.base_type(base_type);
            } else {
                b.base_type(b.generic(&base, &[value]));
            }
        });
    };
    derived("FerroProperty`1", &["TValue"], "FerroProperty");
    derived("StyledProperty`1", &["TValue"], "FerroProperty`1");
    derived("AttachedProperty`1", &["TValue"], "StyledProperty`1");
    derived("DirectPropertyBase`1", &["TValue"], "FerroProperty`1");
    derived("DirectProperty`2", &["TOwner", "TValue"], "DirectPropertyBase`1");
}
