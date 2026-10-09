//! The closed table of runtime library (`System.*`) types markup can name,
//! as data: what both type systems of the compiler define from it
//! (docs/porting/xaml.md, 9.5.1 and 9.5.5).
//!
//! The run-time type system (`runtime::type_system`) defines the types of the
//! table over the registries of the process and gives their members the
//! invokers the interpreter calls; the build-time type system (`ferroui-build`,
//! `type_system`) defines the same types over the type model of the crates,
//! without linking them. Neither states a type or a member of its own: the
//! shape of a type (its kind, type parameters, base, interfaces and members)
//! is stated here once.
//!
//! What the table does not hold is what only one of the two has: the Rust
//! types whose values are values of a type (the run-time type system has the
//! types, the build-time one their text, [`core_handles`]), and the invokers.
//!
//! Next to the table: the names the attributes of markup metadata are
//! projected to ([`attribute_type_name`]), the property definition types
//! ([`property_types`]) and the lists markup creates ([`list_members`]).

use ferroui_base::metadata::attributes;

/// The namespace of the attribute types the metadata attributes are
/// projected to.
pub const METADATA_NAMESPACE: &str = "FerroUI.Metadata";
/// The namespace of the attribute types of control metadata.
pub const CONTROLS_METADATA_NAMESPACE: &str = "FerroUI.Controls.Metadata";
/// The namespace of the property definition types.
pub const PROPERTY_NAMESPACE: &str = "FerroUI";

/// The generic definition of the list of the runtime library.
pub const LIST_DEFINITION: &str = "System.Collections.Generic.List`1";
/// The untyped list of the runtime library.
pub const ARRAY_LIST: &str = "System.Collections.ArrayList";
/// The generic definition of the notifying list of the framework.
pub const FERRO_LIST_DEFINITION: &str = "FerroUI.Collections.FerroList`1";
/// The generic definition of the converter that creates a notifying list from its text.
pub const FERRO_LIST_CONVERTER_DEFINITION: &str = "FerroUI.Collections.FerroListConverter`1";
/// The generic definition of the dictionary of the framework.
pub const FERRO_DICTIONARY_DEFINITION: &str = "FerroUI.Collections.FerroDictionary`2";

/// The attributes whose types the type systems define when no type of the
/// name is declared: the type of an attribute of this list is found by its
/// full name.
pub const KNOWN_ATTRIBUTES: &[&str] = &[
    "XmlnsDefinition",
    "XmlnsPrefix",
    attributes::CONTENT,
    attributes::TEMPLATE_CONTENT,
    attributes::DEPENDS_ON,
    attributes::ASSIGN_BINDING,
    attributes::USABLE_DURING_INITIALIZATION,
    attributes::WHITESPACE_SIGNIFICANT_COLLECTION,
    attributes::TRIM_SURROUNDING_WHITESPACE,
    attributes::CONTROL_TEMPLATE_SCOPE,
    attributes::DATA_TYPE,
    attributes::INHERIT_DATA_TYPE_FROM,
    attributes::INHERIT_DATA_TYPE_FROM_ITEMS,
    attributes::RESOLVE_BY_NAME,
    attributes::MARKUP_EXTENSION_OPTION,
    attributes::MARKUP_EXTENSION_DEFAULT_OPTION,
    attributes::FERRO_LIST,
    attributes::TEMPLATE_PART,
    attributes::PSEUDO_CLASSES,
    attributes::UNSTABLE,
    attributes::PRIVATE_API,
    attributes::NOT_CLIENT_IMPLEMENTABLE,
    attributes::CONSTRUCTOR_ARGUMENT,
];

/// The full name of the attribute type a metadata attribute named `name`
/// is projected to.
pub fn attribute_type_name(name: &str) -> String {
    match name {
        "Obsolete" => "System.ObsoleteAttribute".to_string(),
        "TypeConverter" => "System.ComponentModel.TypeConverterAttribute".to_string(),
        "DefaultMember" => "System.Reflection.DefaultMemberAttribute".to_string(),
        "TemplatePart" | "PseudoClasses" => format!("{CONTROLS_METADATA_NAMESPACE}.{name}Attribute"),
        // Declared next to the bindings, where the compiler looks it up.
        "AssignBinding" => format!("FerroUI.Data.{name}Attribute"),
        // Declared with the controls (`FerroUI.Controls.ResolveByNameAttribute`), where the
        // resolve-by-name replacer looks it up.
        "ResolveByName" => format!("FerroUI.Controls.{name}Attribute"),
        _ => format!("{METADATA_NAMESPACE}.{name}Attribute"),
    }
}

/// What kind of type a type of the table is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreKind {
    Class,
    Interface,
    Struct,
}

/// A type as a member of the table names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreRef {
    /// A type by its full name (`System.Int32`).
    Type(String),
    /// A type parameter of the type being described, by position.
    Parameter(usize),
    /// The element type of a list markup creates ([`list_members`]); `System.Object` for
    /// the untyped list.
    Element,
    /// An instantiation of a generic definition.
    Generic(String, Vec<CoreRef>),
    /// The one-dimensional array of a type.
    Array(Box<CoreRef>),
}

/// How a member of the table is implemented at run time. The build-time type system does
/// not call members and reads this only to describe them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreBody {
    /// The run-time type system implements the member.
    Implemented,
    /// An abstract member: a call is dispatched by name to the member of the run-time type
    /// of the instance.
    Virtual,
    /// The member exists for the transformers and cannot be invoked.
    None,
}

/// A member of a type of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreMember {
    /// A public constructor.
    Constructor { parameters: Vec<CoreRef>, body: CoreBody },
    Method { name: String, is_static: bool, return_type: CoreRef, parameters: Vec<CoreRef>, body: CoreBody },
    /// A read-only property with the getter `get_<name>`.
    Property { name: String, type_: CoreRef, is_static: bool, body: CoreBody },
    /// The indexer of the type: the property `Item` with the accessors `get_Item` and
    /// (when `writable`) `set_Item`, and `[DefaultMember("Item")]` on the type. An accessor
    /// a method of the same name and arity declares before the indexer is that method;
    /// otherwise it is abstract.
    Indexer { parameters: Vec<CoreRef>, type_: CoreRef, writable: bool },
}

/// A type of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreType {
    pub namespace: String,
    pub name: String,
    pub kind: CoreKind,
    /// The names of the type parameters of a generic definition.
    pub parameters: Vec<String>,
    /// The base type the description states. A class that states none derives from
    /// `System.Object`.
    pub base: Option<CoreRef>,
    /// The interfaces the type declares (without the ones they inherit).
    pub interfaces: Vec<CoreRef>,
    /// The members only stand in while no markup metadata of the type is declared: with
    /// such metadata the type has the members of the metadata and none of these.
    pub stands_in: bool,
    pub members: Vec<CoreMember>,
}

impl CoreType {
    fn new(namespace: &str, name: &str, kind: CoreKind) -> Self {
        Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            kind,
            parameters: Vec::new(),
            base: None,
            interfaces: Vec::new(),
            stands_in: false,
            members: Vec::new(),
        }
    }

    /// The namespace-qualified name (`System.Collections.Generic.List`1`).
    pub fn full_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.namespace, self.name)
        }
    }

    fn parameters(mut self, names: &[&str]) -> Self {
        self.parameters = names.iter().map(|name| name.to_string()).collect();
        self
    }

    fn base(mut self, full_name: &str) -> Self {
        self.base = Some(t(full_name));
        self
    }

    fn base_type(mut self, base: CoreRef) -> Self {
        self.base = Some(base);
        self
    }

    fn interface(mut self, interface: CoreRef) -> Self {
        self.interfaces.push(interface);
        self
    }

    fn stands_in(mut self) -> Self {
        self.stands_in = true;
        self
    }

    fn constructor(mut self, parameters: Vec<CoreRef>, body: CoreBody) -> Self {
        self.members.push(CoreMember::Constructor { parameters, body });
        self
    }

    fn method(mut self, name: &str, is_static: bool, return_type: CoreRef, parameters: Vec<CoreRef>, body: CoreBody) -> Self {
        self.members.push(CoreMember::Method { name: name.to_string(), is_static, return_type, parameters, body });
        self
    }

    fn property(mut self, name: &str, type_: CoreRef, is_static: bool, body: CoreBody) -> Self {
        self.members.push(CoreMember::Property { name: name.to_string(), type_, is_static, body });
        self
    }

    fn indexer(mut self, parameters: Vec<CoreRef>, type_: CoreRef, writable: bool) -> Self {
        self.members.push(CoreMember::Indexer { parameters, type_, writable });
        self
    }
}

fn t(full_name: &str) -> CoreRef {
    CoreRef::Type(full_name.to_string())
}

fn p(index: usize) -> CoreRef {
    CoreRef::Parameter(index)
}

fn g(definition: &str, arguments: Vec<CoreRef>) -> CoreRef {
    CoreRef::Generic(definition.to_string(), arguments)
}

const SYSTEM: &str = "System";
const COLLECTIONS: &str = "System.Collections";
const GENERIC: &str = "System.Collections.Generic";
const COMPONENT_MODEL: &str = "System.ComponentModel";

/// The primitive value types of the runtime library, by name, with the Rust type that
/// holds their values. The nullable form of each (`Option<T>`) is `System.Nullable<T>`.
pub const PRIMITIVES: &[(&str, &str)] = &[
    ("Boolean", "bool"),
    ("Char", "char"),
    ("SByte", "i8"),
    ("Byte", "u8"),
    ("Int16", "i16"),
    ("UInt16", "u16"),
    ("Int32", "i32"),
    ("UInt32", "u32"),
    ("Int64", "i64"),
    ("UInt64", "u64"),
    ("Single", "f32"),
    ("Double", "f64"),
];

/// The element types of the arrays the compiler produces constants of, by full name, with
/// the Rust type of an element: a member that declares `Vec<T>` (or `Option<Vec<T>>`)
/// takes an array of `T`.
pub const ARRAY_ELEMENTS: &[(&str, &str)] = &[
    ("System.Boolean", "bool"),
    ("System.Byte", "u8"),
    ("System.Int32", "i32"),
    ("System.Int64", "i64"),
    ("System.Single", "f32"),
    ("System.Double", "f64"),
];

/// The members of a list of the runtime library that markup creates: `List<T>` for the
/// element type `T` (`typed`), `ArrayList` without one. The members are the ones markup
/// and binding paths use (the constructor, `Add`, `Count` and the indexer); the element
/// type is [`CoreRef::Element`]. The namespace and the name of the result are the ones of
/// the untyped list.
pub fn list_members(typed: bool) -> CoreType {
    use CoreBody::Implemented;
    let mut list = CoreType::new(COLLECTIONS, "ArrayList", CoreKind::Class).base("System.Object");
    if typed {
        list = list
            .interface(g("System.Collections.Generic.IList`1", vec![CoreRef::Element]))
            .interface(g("System.Collections.Generic.IReadOnlyList`1", vec![CoreRef::Element]));
    }
    // `List<T>.Add(T)` returns nothing, `ArrayList.Add(object)` the index of the item.
    let add_result = if typed { t("System.Void") } else { t("System.Int32") };
    list.interface(t("System.Collections.IList"))
        .constructor(Vec::new(), Implemented)
        .method("Add", false, add_result, vec![CoreRef::Element], Implemented)
        .property("Count", t("System.Int32"), false, Implemented)
        // The accessors of the indexer: the indexer takes the ones declared here.
        .method("get_Item", false, CoreRef::Element, vec![t("System.Int32")], Implemented)
        .method("set_Item", false, t("System.Void"), vec![t("System.Int32"), CoreRef::Element], Implemented)
        .indexer(vec![t("System.Int32")], CoreRef::Element, true)
}

/// The members of an instantiation of the converter that creates a notifying list from
/// its text ([`FERRO_LIST_CONVERTER_DEFINITION`]), which the type systems synthesize for
/// every element type.
pub fn list_converter_members() -> CoreType {
    CoreType::new("FerroUI.Collections", "FerroListConverter`1", CoreKind::Class)
        .base("System.ComponentModel.TypeConverter")
        .constructor(Vec::new(), CoreBody::Implemented)
        .method(
            "ConvertFrom",
            false,
            t("System.Object"),
            vec![t("System.ComponentModel.ITypeDescriptorContext"), t("System.Globalization.CultureInfo"), t("System.Object")],
            CoreBody::Implemented,
        )
}

/// The runtime library types, in the order they are defined.
pub fn core_types() -> Vec<CoreType> {
    use CoreBody::{Implemented, Virtual};
    use CoreKind::{Class, Interface, Struct};
    let none = CoreBody::None;
    let define = CoreType::new;
    let mut types = vec![
        define(SYSTEM, "Object", Class),
        define(SYSTEM, "ValueType", Class),
        define(SYSTEM, "Enum", Class).base("System.ValueType"),
        define(SYSTEM, "Attribute", Class),
        define(SYSTEM, "Void", Struct).base("System.ValueType"),
        define(SYSTEM, "IntPtr", Struct).base("System.ValueType"),
        define(SYSTEM, "Nullable`1", Struct).parameters(&["T"]).base("System.ValueType"),
    ];
    for (name, _) in PRIMITIVES {
        types.push(define(SYSTEM, name, Struct).base("System.ValueType"));
    }
    types.extend([
        // One source: the metadata the type declares; the member only stands in while no
        // metadata is declared.
        define(SYSTEM, "TimeSpan", Struct).base("System.ValueType").stands_in().method(
            "Parse",
            true,
            t("System.TimeSpan"),
            vec![t("System.String")],
            Implemented,
        ),
        // `string.Length`: the number of UTF-16 code units.
        define(SYSTEM, "String", Class).property("Length", t("System.Int32"), false, Implemented),
        define(SYSTEM, "Type", Class),
        define(SYSTEM, "Array", Class).property("Length", t("System.Int32"), false, Implemented),
        define(SYSTEM, "Uri", Class).stands_in().constructor(vec![t("System.String")], Implemented),
        define(SYSTEM, "Delegate", Class),
        define(SYSTEM, "MulticastDelegate", Class).base("System.Delegate"),
        // One source: the metadata the base crate declares for its error type.
        define(SYSTEM, "Exception", Class).stands_in(),
    ]);
    for name in ["InvalidCastException", "NotSupportedException", "NullReferenceException"] {
        types.push(define(SYSTEM, name, Class).base("System.Exception").constructor(Vec::new(), none));
    }
    for (namespace, name) in [
        (SYSTEM, "ObsoleteAttribute"),
        (SYSTEM, "FlagsAttribute"),
        (SYSTEM, "AttributeUsageAttribute"),
        ("System.Diagnostics.CodeAnalysis", "ExperimentalAttribute"),
        (COMPONENT_MODEL, "TypeConverterAttribute"),
    ] {
        types.push(define(namespace, name, Class).base("System.Attribute"));
    }
    types.extend([
        define(SYSTEM, "IDisposable", Interface).method("Dispose", false, t("System.Void"), Vec::new(), Virtual),
        define(SYSTEM, "IFormatProvider", Interface),
        define("System.Globalization", "CultureInfo", Class).interface(t("System.IFormatProvider")).stands_in().property(
            "InvariantCulture",
            t("System.Globalization.CultureInfo"),
            true,
            Implemented,
        ),
        define("System.Reflection", "MethodInfo", Class),
        // Services are identified by Rust handle types and are not untyped values: the
        // method exists for the transformers.
        define(SYSTEM, "IServiceProvider", Interface).method("GetService", false, t("System.Object"), vec![t("System.Type")], none),
        define(COMPONENT_MODEL, "ITypeDescriptorContext", Interface).interface(t("System.IServiceProvider")),
        define(COMPONENT_MODEL, "ISupportInitialize", Interface)
            .method("BeginInit", false, t("System.Void"), Vec::new(), Virtual)
            .method("EndInit", false, t("System.Void"), Vec::new(), Virtual),
        define(COMPONENT_MODEL, "TypeConverter", Class).constructor(Vec::new(), none).method(
            "ConvertFrom",
            false,
            t("System.Object"),
            vec![t("System.ComponentModel.ITypeDescriptorContext"), t("System.Globalization.CultureInfo"), t("System.Object")],
            Virtual,
        ),
        // Collections.
        define("System.Collections.Specialized", "INotifyCollectionChanged", Interface),
        define(COLLECTIONS, "IEnumerator", Interface),
        define(COLLECTIONS, "IEnumerable", Interface).method("GetEnumerator", false, t("System.Collections.IEnumerator"), Vec::new(), Virtual),
        define(COLLECTIONS, "IList", Interface).interface(t("System.Collections.IEnumerable")).method(
            "Add",
            false,
            t("System.Int32"),
            vec![t("System.Object")],
            Virtual,
        ),
        define(GENERIC, "IEnumerator`1", Interface).parameters(&["T"]).interface(t("System.Collections.IEnumerator")),
        define(GENERIC, "IEnumerable`1", Interface).parameters(&["T"]).interface(t("System.Collections.IEnumerable")),
        define(GENERIC, "ICollection`1", Interface)
            .parameters(&["T"])
            .interface(g("System.Collections.Generic.IEnumerable`1", vec![p(0)]))
            .method("Add", false, t("System.Void"), vec![p(0)], Virtual)
            .property("Count", t("System.Int32"), false, Virtual),
        define(GENERIC, "IList`1", Interface)
            .parameters(&["T"])
            .interface(g("System.Collections.Generic.ICollection`1", vec![p(0)]))
            .indexer(vec![t("System.Int32")], p(0), true),
        define(GENERIC, "IReadOnlyList`1", Interface)
            .parameters(&["T"])
            .interface(g("System.Collections.Generic.IEnumerable`1", vec![p(0)]))
            .property("Count", t("System.Int32"), false, Virtual)
            .indexer(vec![t("System.Int32")], p(0), false),
        // The untyped list: any children, null included.
        list_members(false),
        // The definition of the generic list: an instantiation metadata declares has the
        // members of its metadata, every other instantiation the ones of `list_members`.
        define(GENERIC, "List`1", Class)
            .parameters(&["T"])
            .constructor(Vec::new(), none)
            .interface(g("System.Collections.Generic.IList`1", vec![p(0)]))
            .interface(g("System.Collections.Generic.IReadOnlyList`1", vec![p(0)]))
            .interface(t("System.Collections.IList"))
            .method("Add", false, t("System.Void"), vec![p(0)], Virtual)
            .property("Count", t("System.Int32"), false, Virtual)
            .indexer(vec![t("System.Int32")], p(0), true),
        define(GENERIC, "Dictionary`2", Class)
            .parameters(&["TKey", "TValue"])
            .constructor(Vec::new(), none)
            .method("Add", false, t("System.Void"), vec![p(0), p(1)], Virtual)
            .property("Count", t("System.Int32"), false, Virtual)
            .indexer(vec![p(0)], p(1), true),
        define(GENERIC, "IDictionary`2", Interface)
            .parameters(&["TKey", "TValue"])
            .method("Add", false, t("System.Void"), vec![p(0), p(1)], Virtual)
            .indexer(vec![p(0)], p(1), true),
        // What the compiled binding paths of the framework language look up by name:
        // observables, tasks and weak references are recognised by their generic
        // definition; instantiations come from declared metadata.
        define(SYSTEM, "IObservable`1", Interface).parameters(&["T"]),
        define("System.Threading.Tasks", "Task", Class),
        define("System.Threading.Tasks", "Task`1", Class).parameters(&["TResult"]).base("System.Threading.Tasks.Task"),
        define(SYSTEM, "WeakReference`1", Class).parameters(&["T"]),
        define(COMPONENT_MODEL, "CultureInfoConverter", Class).base("System.ComponentModel.TypeConverter").constructor(Vec::new(), none),
    ]);

    // Delegates.
    let delegate = |name: String, parameters: Vec<String>, has_result: bool| {
        let count = parameters.len();
        let (arguments, result): (Vec<CoreRef>, CoreRef) = if has_result {
            ((0..count - 1).map(p).collect(), p(count - 1))
        } else {
            ((0..count).map(p).collect(), t("System.Void"))
        };
        let mut type_ = define(SYSTEM, &name, Class).base("System.MulticastDelegate");
        type_.parameters = parameters;
        type_.constructor(vec![t("System.Object"), t("System.IntPtr")], none).method("Invoke", false, result, arguments, Virtual)
    };
    types.push(delegate("Action".to_string(), Vec::new(), false));
    for count in 1..=16usize {
        types.push(delegate(format!("Action`{count}"), (1..=count).map(|index| format!("T{index}")).collect(), false));
    }
    for count in 1..=17usize {
        let mut names: Vec<String> = (1..count).map(|index| format!("T{index}")).collect();
        names.push("TResult".to_string());
        types.push(delegate(format!("Func`{count}"), names, true));
    }
    types.push(
        define(SYSTEM, "EventHandler`1", Class)
            .parameters(&["TEventArgs"])
            .base("System.MulticastDelegate")
            .constructor(vec![t("System.Object"), t("System.IntPtr")], none)
            .method("Invoke", false, t("System.Void"), vec![t("System.Object"), p(0)], Virtual),
    );
    types
}

/// The types of property definitions (`FerroUI.FerroProperty` and the generic
/// `StyledProperty<T>`, `AttachedProperty<T>`, `DirectProperty<TOwner, T>`, deriving from
/// each other as the classes of the managed original), in the order they are defined. A
/// type system defines the ones no declared type has the name of.
pub fn property_types() -> Vec<CoreType> {
    let name = |name: &str| format!("{PROPERTY_NAMESPACE}.{name}");
    let define = |type_name: &str, parameters: &[&str]| CoreType::new(PROPERTY_NAMESPACE, type_name, CoreKind::Class).parameters(parameters);
    vec![
        define("FerroProperty", &[]),
        define("FerroProperty`1", &["TValue"]).base(&name("FerroProperty")),
        define("StyledProperty`1", &["TValue"]).base_type(g(&name("FerroProperty`1"), vec![p(0)])),
        define("AttachedProperty`1", &["TValue"]).base_type(g(&name("StyledProperty`1"), vec![p(0)])),
        define("DirectPropertyBase`1", &["TValue"]).base_type(g(&name("FerroProperty`1"), vec![p(0)])),
        define("DirectProperty`2", &["TOwner", "TValue"]).base_type(g(&name("DirectPropertyBase`1"), vec![p(1)])),
    ]
}

/// The Rust types that hold the values of the types of the table, as normalised type text
/// (docs/porting/xaml.md, 9.5.2: every path absolute, by a path the crate that declares
/// the type exports), each with the type its values are values of. What the run-time type
/// system states with the types themselves.
///
/// Not listed: the types only the run-time back end has (its `System.Type` value, its
/// arrays and lists, the deferred content factory), which no declaration can name.
pub fn core_handles() -> Vec<(String, CoreRef)> {
    let mut handles: Vec<(String, CoreRef)> = Vec::new();
    let mut add = |text: &str, type_: CoreRef| handles.push((text.to_string(), type_));
    let nullable = |full_name: &str| g("System.Nullable`1", vec![t(full_name)]);
    // A reference type is held by its handle and by the optional handle.
    let reference = |add: &mut dyn FnMut(&str, CoreRef), text: &str, full_name: &str| {
        add(text, t(full_name));
        add(&format!("Option<{text}>"), t(full_name));
    };

    add("Option<::ferroui_base::BoxedValue>", t("System.Object"));
    add("::ferroui_base::BoxedValue", t("System.Object"));
    add("()", t("System.Void"));
    for (name, rust) in PRIMITIVES {
        let full_name = format!("System.{name}");
        add(*rust, t(&full_name));
        add(&format!("Option<{rust}>"), nullable(&full_name));
    }
    for (full_name, rust) in ARRAY_ELEMENTS {
        add(&format!("Vec<{rust}>"), CoreRef::Array(Box::new(t(full_name))));
        add(&format!("Option<Vec<{rust}>>"), CoreRef::Array(Box::new(t(full_name))));
    }
    add("::ferroui_base::animation::TimeSpan", t("System.TimeSpan"));
    add("Option<::ferroui_base::animation::TimeSpan>", nullable("System.TimeSpan"));
    reference(&mut add, "String", "System.String");
    reference(&mut add, "&'static ::ferroui_base::TypeInfo", "System.Type");
    reference(&mut add, "::ferroui_base::data::core::ValueType", "System.Type");
    reference(&mut add, "::ferroui_base::utilities::Uri", "System.Uri");
    reference(&mut add, "::ferroui_base::metadata::MarkupDelegate", "System.Delegate");
    reference(&mut add, "::ferroui_base::data::BindingError", "System.Exception");
    reference(&mut add, "::std::rc::Rc<dyn ::ferroui_base::reactive::IDisposable>", "System.IDisposable");
    reference(&mut add, "::ferroui_base::utilities::CultureInfo", "System.Globalization.CultureInfo");
    reference(&mut add, "::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>", "System.IServiceProvider");
    reference(
        &mut add,
        "::std::rc::Rc<dyn ::ferroui_base::data::model::INotifyCollectionChanged>",
        "System.Collections.Specialized.INotifyCollectionChanged",
    );
    // An observable or a task held as a value (what a binding path streams with `^`): the
    // values are held untyped, so it is `IObservable<object>` or `Task<object>`.
    for (text, definition) in [
        ("::ferroui_base::data::core::plugins::ObservableValue", "System.IObservable`1"),
        ("::ferroui_base::data::core::plugins::TaskValue", "System.Threading.Tasks.Task`1"),
    ] {
        let stream = g(definition, vec![t("System.Object")]);
        add(text, stream.clone());
        add(&format!("Option<{text}>"), stream);
    }
    handles
}

/// The handles of the property definition type `FerroUI.FerroProperty`, for a type system
/// that defines the type itself ([`property_types`]).
pub const PROPERTY_HANDLES: &[&str] = &["&'static ::ferroui_base::FerroProperty", "Option<&'static ::ferroui_base::FerroProperty>"];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The names the references of a description use.
    fn names_of(reference: &CoreRef, names: &mut Vec<String>) {
        match reference {
            CoreRef::Type(name) => names.push(name.clone()),
            CoreRef::Parameter(_) | CoreRef::Element => {}
            CoreRef::Generic(definition, arguments) => {
                names.push(definition.clone());
                arguments.iter().for_each(|argument| names_of(argument, names));
            }
            CoreRef::Array(element) => names_of(element, names),
        }
    }

    fn references_of(type_: &CoreType) -> Vec<String> {
        let mut names = Vec::new();
        type_.base.iter().chain(&type_.interfaces).for_each(|reference| names_of(reference, &mut names));
        for member in &type_.members {
            match member {
                CoreMember::Constructor { parameters, .. } => parameters.iter().for_each(|parameter| names_of(parameter, &mut names)),
                CoreMember::Method { return_type, parameters, .. } => {
                    names_of(return_type, &mut names);
                    parameters.iter().for_each(|parameter| names_of(parameter, &mut names));
                }
                CoreMember::Property { type_, .. } => names_of(type_, &mut names),
                CoreMember::Indexer { parameters, type_, .. } => {
                    names_of(type_, &mut names);
                    parameters.iter().for_each(|parameter| names_of(parameter, &mut names));
                }
            }
        }
        names
    }

    /// Not from upstream: the table is closed. Every type a description names is a type of
    /// the table (or, for the property definition types, of that list), no name is defined
    /// twice, and a type parameter a member names is one of its type.
    #[test]
    fn table_names_only_its_own_types() {
        let types = core_types();
        let mut defined: HashSet<String> = HashSet::new();
        for type_ in &types {
            assert!(defined.insert(type_.full_name()), "{} is defined twice", type_.full_name());
        }
        let checked: Vec<CoreType> = types.into_iter().chain([list_members(true), list_converter_members()]).collect();
        for type_ in &checked {
            for name in references_of(type_) {
                assert!(defined.contains(&name), "{} names {name}, which the table does not define", type_.full_name());
            }
        }
        let mut with_properties = defined.clone();
        for type_ in property_types() {
            assert!(with_properties.insert(type_.full_name()), "{} is defined twice", type_.full_name());
            for name in references_of(&type_) {
                assert!(with_properties.contains(&name), "{} names {name} before it is defined", type_.full_name());
            }
        }
        for (text, type_) in core_handles() {
            let mut names = Vec::new();
            names_of(&type_, &mut names);
            for name in names {
                assert!(defined.contains(&name), "the handle {text} names {name}, which the table does not define");
            }
        }
    }

    /// Not from upstream: the types the well-known types of the compiler are read from are
    /// in the table, with the delegates of every arity.
    #[test]
    fn table_has_the_well_known_types() {
        let defined: HashSet<String> = core_types().iter().map(CoreType::full_name).collect();
        for name in [
            "System.Object",
            "System.Void",
            "System.String",
            "System.Nullable`1",
            "System.Collections.Generic.List`1",
            "System.Collections.Generic.Dictionary`2",
            "System.Collections.IList",
            "System.Reflection.MethodInfo",
            "System.Diagnostics.CodeAnalysis.ExperimentalAttribute",
            "System.Action",
            "System.Action`16",
            "System.Func`17",
            "System.EventHandler`1",
            ARRAY_LIST,
            LIST_DEFINITION,
        ] {
            assert!(defined.contains(name), "{name}");
        }
        assert_eq!(defined.len(), 7 + 12 + 8 + 3 + 5 + 26 + 1 + 16 + 17 + 1);
        assert_eq!(attribute_type_name("Content"), "FerroUI.Metadata.ContentAttribute");
        assert!(KNOWN_ATTRIBUTES.contains(&"Content"));
    }
}
