//! The tests of the build-time type system: over the scan of the fixture
//! source tree of the scanner (`tests/fixtures/scanner`), and over models
//! written here for what the fixture does not declare (the root classes of the
//! object model, metadata of a runtime library type, a crate built on
//! another).
//!
//! None of the tests is from upstream: upstream's compiler reads the types of
//! an assembly from its metadata. The comparison with the run-time type system
//! over the real crates is the drift test of the XAML test crate.

use std::path::Path;
use std::rc::Rc;

use xamlx::type_system::{
    IXamlAssembly, IXamlConstructor, IXamlCustomAttribute, IXamlEventInfo, IXamlField, IXamlMember, IXamlMethod, IXamlParameterInfo, IXamlProperty,
    IXamlType, IXamlTypeSystem, XamlValue,
};

use super::*;
use crate::model::{
    AccessorModel, AssemblyModel, CallableModel, EnumMemberModel, ExportModel, GenericModel, MemberModel, ParameterModel, PropertyModel,
    RegisteredKind, RegisteredModel, RegistrationModel, RustType, TypeKind, TypeModel,
};
use crate::scanner::{scan_crate, ScanOptions};

const CONTROL: &str = "Option<::ferroui_base::Ref<::ferroui_base::Control>>";
const BRUSHES: &str = "Vec<::ferroui_base::Ref<::fixture::media::brush::Brush>>";

/// The type system over the scan of the fixture crate.
fn fixture() -> Rc<ModelTypeSystem> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("FerroUI.Build.Scan").join("tests").join("fixtures").join("scanner").join("lib.rs");
    ModelTypeSystem::new(vec![scan_crate(&ScanOptions::new("fixture", root)).model])
}

fn the(system: &ModelTypeSystem, full_name: &str) -> Rc<dyn IXamlType> {
    system.find_type(full_name).unwrap_or_else(|| panic!("the type system has no type `{full_name}`"))
}

fn full_names(types: &[Rc<dyn IXamlType>]) -> Vec<String> {
    types.iter().map(|type_| type_.full_name()).collect()
}

/// `static Name(A, B) -> R`.
fn signature(method: &Rc<dyn IXamlMethod>) -> String {
    format!(
        "{}{}({}) -> {}",
        if method.is_static() { "static " } else { "" },
        method.name(),
        full_names(&method.parameters()).join(", "),
        method.return_type().full_name()
    )
}

fn signatures(type_: &Rc<dyn IXamlType>) -> Vec<String> {
    type_.methods().iter().map(signature).collect()
}

fn attribute_names(attributes: &[Rc<dyn IXamlCustomAttribute>]) -> Vec<String> {
    attributes.iter().map(|attribute| attribute.type_().name()).collect()
}

fn property(type_: &Rc<dyn IXamlType>, name: &str) -> Rc<dyn IXamlProperty> {
    type_.properties().into_iter().find(|property| property.name() == name).unwrap_or_else(|| panic!("{} has no property `{name}`", type_.full_name()))
}

fn field(type_: &Rc<dyn IXamlType>, name: &str) -> Rc<dyn IXamlField> {
    type_.fields().into_iter().find(|field| field.name() == name).unwrap_or_else(|| panic!("{} has no field `{name}`", type_.full_name()))
}

fn method(type_: &Rc<dyn IXamlType>, name: &str) -> Rc<dyn IXamlMethod> {
    type_.methods().into_iter().find(|method| method.name() == name).unwrap_or_else(|| panic!("{} has no method `{name}`", type_.full_name()))
}

/// The runtime library types are the ones of the shared table, with their members, and the
/// Rust types the table lists are values of them.
#[test]
fn runtime_library_types_are_defined_from_the_table() {
    let system = fixture();
    let well_known = system.well_known_types();
    assert_eq!(well_known.string.full_name(), "System.String");
    assert_eq!(well_known.get_func_of_t(2).full_name(), "System.Func`2");

    let int32 = the(&system, "System.Int32");
    assert!(int32.is_value_type() && !int32.is_enum());
    assert_eq!(int32.base_type().map(|base| base.full_name()), Some("System.ValueType".to_string()));
    assert!(system.resolve("i32").equals(&*int32));
    let nullable = system.resolve("Option<i32>");
    assert_eq!(nullable.full_name(), "System.Nullable`1[System.Int32]");
    assert!(nullable.is_nullable() && nullable.is_nullable_of(&*int32) && nullable.is_assignable_from(&*int32));
    assert_eq!(system.resolve("String").full_name(), "System.String");
    assert_eq!(system.resolve("Option<String>").full_name(), "System.String");
    assert_eq!(system.resolve("Option<::ferroui_base::BoxedValue>").full_name(), "System.Object");
    assert_eq!(system.resolve("()").full_name(), "System.Void");
    assert_eq!(system.resolve("&'static ::ferroui_base::TypeInfo").full_name(), "System.Type");
    let array = system.resolve("Vec<f64>");
    assert!(array.is_array());
    assert_eq!(array.full_name(), "System.Double[]");
    assert_eq!(array.base_type().map(|base| base.full_name()), Some("System.Array".to_string()));
    assert_eq!(system.resolve("::ferroui_base::data::core::plugins::ObservableValue").full_name(), "System.IObservable`1[System.Object]");

    // A member of the table that only stands in is there while no metadata is declared.
    assert_eq!(signatures(&the(&system, "System.TimeSpan")), ["static Parse(System.String) -> System.TimeSpan"]);
    assert_eq!(signatures(&the(&system, "System.String")), ["get_Length() -> System.Int32"]);

    // The list markup creates for an element type, and the untyped one.
    let list = the(&system, "System.Collections.Generic.List`1").make_generic_type(std::slice::from_ref(&int32)).expect("List<int>");
    assert_eq!(list.full_name(), "System.Collections.Generic.List`1[System.Int32]");
    assert_eq!(
        signatures(&list),
        [
            "Add(System.Int32) -> System.Void",
            "get_Count() -> System.Int32",
            "get_Item(System.Int32) -> System.Int32",
            "set_Item(System.Int32, System.Int32) -> System.Void"
        ]
    );
    assert!(full_names(&list.interfaces()).contains(&"System.Collections.Generic.IList`1[System.Int32]".to_string()), "{:?}", full_names(&list.interfaces()));
    assert_eq!(attribute_names(&list.custom_attributes()), ["DefaultMemberAttribute"]);
    assert_eq!(property(&list, "Item").indexer_parameters().len(), 1);
    assert_eq!(signature(&method(&the(&system, "System.Collections.ArrayList"), "Add")), "Add(System.Object) -> System.Int32");
    let enumerable = the(&system, "System.Collections.IEnumerable");
    assert!(enumerable.is_interface() && enumerable.is_assignable_from(&*list));

    // An instantiation of a definition of the table has the substituted members of it.
    let arguments = [the(&system, "System.IServiceProvider"), the(&system, "System.Object")];
    let func = the(&system, "System.Func`2").make_generic_type(&arguments).expect("Func<IServiceProvider, object>");
    assert_eq!(signatures(&func), ["Invoke(System.IServiceProvider) -> System.Object"]);
    assert_eq!(func.base_type().map(|base| base.full_name()), Some("System.MulticastDelegate".to_string()));
    assert!(the(&system, "System.Nullable`1").make_generic_type(&arguments).is_err());

    // The property definition types, which no model of the fixture declares.
    let double = the(&system, "System.Double");
    let styled = the(&system, "FerroUI.StyledProperty`1").make_generic_type(std::slice::from_ref(&double)).expect("StyledProperty<double>");
    let base = styled.base_type().expect("the base of a styled property");
    assert_eq!(base.full_name(), "FerroUI.FerroProperty`1[System.Double]");
    assert_eq!(base.base_type().map(|base| base.full_name()), Some("FerroUI.FerroProperty".to_string()));
    assert_eq!(system.resolve("&'static ::ferroui_base::FerroProperty").full_name(), "FerroUI.FerroProperty");
    assert!(the(&system, "FerroUI.FerroProperty").is_assignable_from(&*styled));

    // A Rust type no model declares is opaque: itself and `object`, no members.
    let opaque = system.resolve("::unknown::Thing<f64>");
    assert_eq!(opaque.full_name(), "::unknown::Thing<f64>");
    assert!(opaque.as_any().downcast_ref::<ModelType>().is_some_and(ModelType::is_opaque));
    assert!(opaque.methods().is_empty() && opaque.properties().is_empty());
    assert!(the(&system, "System.Object").is_assignable_from(&*opaque) && !well_known.string.is_assignable_from(&*opaque));
    assert!(system.resolve("::unknown::Thing<f64>").equals(&*opaque));

    // Attribute types are found by name when they are known, and assemblies by any name.
    let content = the(&system, "FerroUI.Metadata.ContentAttribute");
    assert_eq!(content.base_type().map(|base| base.full_name()), Some("System.Attribute".to_string()));
    assert!(system.find_type("FerroUI.Metadata.MysteryAttribute").is_none());
    assert_eq!(system.assemblies().iter().map(|assembly| assembly.name()).collect::<Vec<_>>(), ["System.Runtime", "Fixture"]);
    assert!(system.find_assembly("system.runtime").is_some() && system.find_assembly("System").is_none());
    assert!(system.find_type_in_assembly("System.String", "mscorlib").is_some());
    assert!(system.find_type_in_assembly("System.String", "Fixture").is_none());
    assert!(system.find_type_in_assembly("Fixture.Controls.Border", "fixture").is_some());
}

/// A class of the object model with every kind of member: its registered properties (a
/// styled one, a direct one without a setter, an owner added to a property of its base, an
/// alias) and the members of its markup metadata, in the order the run-time type system
/// projects them.
#[test]
fn class_of_the_fixture_is_projected_with_its_members() {
    let system = fixture();
    let border = the(&system, "Fixture.Controls.Border");
    let decorator = the(&system, "Fixture.Controls.Decorator");
    assert!(!border.is_value_type() && !border.is_interface());
    assert!(border.base_type().is_some_and(|base| base.equals(&*decorator)));
    // The base of the decorator is a class of a crate without a model here.
    assert_eq!(decorator.base_type().map(|base| base.full_name()), Some("System.Object".to_string()));
    assert_eq!(border.assembly().map(|assembly| assembly.name()), Some("Fixture".to_string()));
    assert_eq!(full_names(&border.interfaces()), ["Fixture.Media.IBrush", "::std::rc::Rc<dyn ::ferroui_base::IOther>"]);
    assert!(the(&system, "Fixture.Media.IBrush").is_assignable_from(&*border));
    assert!(decorator.is_assignable_from(&*border) && !border.is_assignable_from(&*decorator));
    assert!(system.resolve("Option<::ferroui_base::Ref<::fixture::controls::border::Border>>").equals(&*border));

    assert_eq!(
        border.properties().iter().map(|property| property.name()).collect::<Vec<_>>(),
        ["Background", "Thickness", "Child", "Brush", "Tag", "Brushes", "Item", "Default"]
    );
    assert_eq!(
        signatures(&border),
        [
            "get_Background() -> Fixture.Media.IBrush".to_string(),
            "set_Background(Fixture.Media.IBrush) -> System.Void".to_string(),
            "get_Thickness() -> System.Double".to_string(),
            format!("get_Child() -> {CONTROL}"),
            format!("set_Child({CONTROL}) -> System.Void"),
            "get_Brush() -> Fixture.Media.Brush".to_string(),
            "set_Brush(Fixture.Media.Brush) -> System.Void".to_string(),
            "get_Tag() -> System.Object".to_string(),
            "set_Tag(System.Object) -> System.Void".to_string(),
            format!("get_Brushes() -> {BRUSHES}"),
            "get_Item(System.Int32) -> Fixture.Media.Brush".to_string(),
            "Add(Fixture.Media.Brush) -> System.Void".to_string(),
            "static Parse(System.String) -> Fixture.Controls.Border".to_string(),
            "Count() -> System.Int32".to_string(),
            "static get_Default() -> Fixture.Media.Thickness".to_string(),
            "static set_Default(Fixture.Media.Thickness) -> System.Void".to_string(),
            "add_Closed(System.EventHandler`1[::ferroui_base::EventArgs]) -> System.Void".to_string(),
            "add_Opened(System.Action) -> System.Void".to_string(),
        ]
    );

    // A direct property registered without a setter is read-only.
    let thickness = property(&border, "Thickness");
    assert!(thickness.getter().is_some() && thickness.setter().is_none());
    // The owner added to the property of the base: the attributes the metadata states for
    // it, and the content marker.
    let child = property(&border, "Child");
    assert_eq!(attribute_names(&child.custom_attributes()), ["ContentAttribute", "DependsOnAttribute", "ContentAttribute"]);
    assert_eq!(child.custom_attributes()[1].parameters(), vec![XamlValue::String("Tag".to_string())]);
    let registered = child.as_any().downcast_ref::<ModelProperty>().and_then(ModelProperty::registered);
    assert_eq!(registered, Some(("::fixture::Border", "child_property")));
    let tag = property(&border, "Tag");
    assert_eq!(attribute_names(&tag.custom_attributes()), ["DependsOnAttribute", "AssignBindingAttribute"]);
    assert_eq!(tag.custom_attributes()[1].type_().full_name(), "FerroUI.Data.AssignBindingAttribute");
    let item = property(&border, "Item");
    assert_eq!(full_names(&item.indexer_parameters()), ["System.Int32"]);
    assert_eq!(attribute_names(&item.custom_attributes()), ["IndexedAttribute"]);
    assert!(property(&border, "Default").getter().is_some_and(|getter| getter.is_static()));

    assert_eq!(
        border.fields().iter().map(|field| (field.name(), field.field_type().full_name())).collect::<Vec<_>>(),
        [
            ("BackgroundProperty".to_string(), "FerroUI.StyledProperty`1[Fixture.Media.IBrush]".to_string()),
            ("ThicknessProperty".to_string(), "FerroUI.DirectProperty`2[Fixture.Controls.Border,System.Double]".to_string()),
            ("ChildProperty".to_string(), format!("FerroUI.StyledProperty`1[{CONTROL}]")),
            ("BrushProperty".to_string(), "FerroUI.StyledProperty`1[Fixture.Media.Brush]".to_string()),
            ("PressedEvent".to_string(), "::ferroui_base::RoutedEvent<::ferroui_base::RoutedEventArgs>".to_string()),
            ("ReleasedEvent".to_string(), "::ferroui_base::RoutedEvent<::ferroui_base::RoutedEventArgs>".to_string()),
            ("NewEvent".to_string(), "Fixture.Controls.Border".to_string()),
        ]
    );
    assert_eq!(attribute_names(&field(&border, "ChildProperty").custom_attributes()), ["ContentAttribute", "DependsOnAttribute"]);
    assert!(field(&border, "BackgroundProperty").is_static() && !field(&border, "BackgroundProperty").is_literal());

    let constructors = border.constructors();
    assert_eq!(
        constructors.iter().map(|constructor| (constructor.is_public(), full_names(&constructor.parameters()).join(", "))).collect::<Vec<_>>(),
        [(true, String::new()), (true, "System.String".to_string()), (true, "&'static FerroProperty, Fixture.Media.Dock".to_string())]
    );
    let parameter = constructors[2].get_parameter_info(0).expect("the first parameter");
    assert_eq!(attribute_names(&parameter.custom_attributes()), ["InheritDataTypeFromAttribute"]);
    assert_eq!(parameter.custom_attributes()[0].parameters(), vec![XamlValue::Int32(2)]);
    assert!(constructors[1].get_parameter_info(0).expect("a positional parameter").custom_attributes().is_empty());
    let source = constructors[0].as_any().downcast_ref::<ModelConstructor>().map(|constructor| constructor.source().clone());
    assert!(matches!(source, Some(MemberSource::DefaultConstructor { ref type_path, .. }) if type_path == "::fixture::Border"), "{source:?}");

    assert_eq!(border.events().iter().map(|event| event.name()).collect::<Vec<_>>(), ["Closed", "Opened"]);
    assert_eq!(border.events()[0].add().map(|add| add.name()), Some("add_Closed".to_string()));
    let parse = method(&border, "Parse");
    assert_eq!(attribute_names(&parse.custom_attributes()), ["BrowsableAttribute"]);
    assert_eq!(parse.custom_attributes()[0].parameters(), vec![XamlValue::Boolean(false)]);
    let source = parse.as_any().downcast_ref::<ModelMethod>().map(|method| method.source().clone());
    assert!(
        matches!(source, Some(MemberSource::Declared { ref typed_function, fallible: true, .. }) if typed_function.as_deref() == Some("__markup_Parse_1")),
        "{source:?}"
    );

    // The attributes of the type: the default member of its indexer, then the declared ones.
    let attributes = border.custom_attributes();
    assert_eq!(
        attribute_names(&attributes),
        ["DefaultMemberAttribute", "UsableDuringInitializationAttribute", "TemplatePartAttribute", "SeparatorsAttribute"]
    );
    assert_eq!(attributes[2].type_().full_name(), "FerroUI.Controls.Metadata.TemplatePartAttribute");
    assert_eq!(attributes[2].parameters(), vec![XamlValue::String("PART_Bar".to_string()), XamlValue::Type(decorator.clone())]);
    let named = attributes[2].properties();
    assert_eq!(named.get("IsRequired"), Some(&XamlValue::Boolean(true)));
    assert_eq!(named.get("Weight"), Some(&XamlValue::Double(-1.5)));
    assert_eq!(named.get("Initial"), Some(&XamlValue::Int32('x' as i32)));
    assert_eq!(
        attributes[3].parameters(),
        vec![XamlValue::Array(vec![XamlValue::String(",".to_string()), XamlValue::String(" ".to_string())]), XamlValue::Null]
    );
}

/// Attached properties, static owner types, value types, enumerations, contracts, a
/// declared instantiation of a generic definition and the assembly of the fixture.
#[test]
fn other_types_of_the_fixture_are_projected() {
    let system = fixture();
    assert_eq!(system.types_of_model(0).len(), 19);

    // An attached property is its static accessors and the field of its definition.
    let grid = the(&system, "Fixture.Controls.Grid");
    assert_eq!(grid.methods().iter().map(|method| (method.name(), method.is_static())).collect::<Vec<_>>(), [("GetRow".to_string(), true), ("SetRow".to_string(), true)]);
    assert!(grid.properties().is_empty());
    assert_eq!(field(&grid, "RowProperty").field_type().full_name(), "FerroUI.AttachedProperty`1[System.Int32]");
    // A class without a constructor has one that is not public.
    assert_eq!(grid.constructors().iter().map(|constructor| constructor.is_public()).collect::<Vec<_>>(), [false]);

    // A static owner type: the accessors take the host type and carry what the definition
    // of the property states; a registration that is not read is no property.
    let layout = the(&system, "Fixture.Controls.Layout");
    assert_eq!(
        signatures(&layout),
        [
            "static GetSpacing(Fixture.Controls.Grid) -> System.Double",
            "static SetSpacing(Fixture.Controls.Grid, System.Double) -> System.Void",
            "static Reset() -> System.Void"
        ]
    );
    assert_eq!(attribute_names(&method(&layout, "GetSpacing").custom_attributes()), ["ResolveByNameAttribute", "AssignBindingAttribute"]);
    assert_eq!(method(&layout, "GetSpacing").custom_attributes()[0].type_().full_name(), "FerroUI.Controls.ResolveByNameAttribute");
    assert_eq!(layout.fields().iter().map(|field| field.name()).collect::<Vec<_>>(), ["SpacingProperty"]);
    assert_eq!(layout.base_type().map(|base| base.full_name()), Some("System.Object".to_string()));

    // A value type: its constructors, `Parse`, and its nullable form.
    let thickness = the(&system, "Fixture.Media.Thickness");
    assert!(thickness.is_value_type());
    assert_eq!(thickness.base_type().map(|base| base.full_name()), Some("System.ValueType".to_string()));
    assert_eq!(signatures(&thickness), ["get_Left() -> System.Double", "static Parse(System.String) -> Fixture.Media.Thickness"]);
    assert_eq!(
        thickness.constructors().iter().map(|constructor| full_names(&constructor.parameters()).join(", ")).collect::<Vec<_>>(),
        ["System.Double", "System.Double, System.Double"]
    );
    assert!(system.resolve("::fixture::media::Thickness").equals(&*thickness));
    let nullable = system.resolve("Option<::fixture::media::Thickness>");
    assert_eq!(nullable.full_name(), "System.Nullable`1[Fixture.Media.Thickness]");
    assert!(nullable.is_nullable_of(&*thickness));

    // Enumerations: literal fields, the underlying type, `[Flags]`.
    let dock = the(&system, "Fixture.Media.Dock");
    assert!(dock.is_enum() && dock.is_value_type());
    assert_eq!(dock.base_type().map(|base| base.full_name()), Some("System.Enum".to_string()));
    assert_eq!(dock.get_enum_underlying_type().map(|type_| type_.full_name()).ok(), Some("System.Int32".to_string()));
    let literals = |type_: &Rc<dyn IXamlType>| -> Vec<(String, Option<XamlValue>)> {
        type_.fields().iter().map(|field| (field.name(), field.get_literal_value().ok())).collect()
    };
    let literal = |name: &str, value: i32| (name.to_string(), Some(XamlValue::Int32(value)));
    assert_eq!(literals(&dock), [literal("Left", 0), literal("Bottom", 4), literal("Right", 5), literal("Top", -1)]);
    assert!(dock.fields().iter().all(|field| field.field_type().equals(&*dock)));
    assert!(system.resolve("::fixture::media::Dock").equals(&*dock));
    assert_eq!(system.resolve("Option<::fixture::media::Dock>").full_name(), "System.Nullable`1[Fixture.Media.Dock]");
    let routes = the(&system, "Fixture.Media.Routes");
    assert_eq!(attribute_names(&routes.custom_attributes()), ["FlagsAttribute"]);
    // The value of a member the scanner could not evaluate is no literal.
    assert_eq!(literals(&routes), [literal("Direct", 1), literal("Tunnel", 2), literal("Both", 6), ("Odd".to_string(), None)]);
    let mode = the(&system, "Fixture.Data.Mode");
    assert_eq!(attribute_names(&mode.custom_attributes()), ["FlaggedAttribute"]);
    assert_eq!(mode.custom_attributes()[0].type_().full_name(), "FerroUI.Metadata.FlaggedAttribute");

    // A contract, by its handles.
    let brush = the(&system, "Fixture.Media.IBrush");
    assert!(brush.is_interface() && brush.base_type().is_none());
    assert!(system.resolve("::std::rc::Rc<dyn ::fixture::media::brush::IBrush>").equals(&*brush));
    assert!(system.resolve("Option<::std::rc::Rc<dyn ::fixture::media::brush::IBrush>>").equals(&*brush));

    // A declared instantiation is named by its definition; the content property no own
    // property carries is stated on the type.
    let setter = system.resolve("::std::rc::Rc<::fixture::markup_types::plain::Setter>");
    assert_eq!(setter.full_name(), "Fixture.Styling.FerroList`1[Fixture.Controls.Border]");
    assert!(system.resolve("Option<::std::rc::Rc<::fixture::markup_types::plain::Setter>>").equals(&*setter));
    let definition = the(&system, "Fixture.Styling.FerroList`1");
    assert_eq!(definition.generic_parameters().len(), 1);
    assert!(setter.generic_type_definition().is_some_and(|found| found.equals(&*definition)));
    assert!(definition.make_generic_type(&[the(&system, "Fixture.Controls.Border")]).is_ok_and(|found| found.equals(&*setter)));
    assert!(definition.make_generic_type(&[the(&system, "System.Int32")]).is_err());
    assert!(system.find_type("Fixture.Styling.Setter").is_none());
    assert_eq!(
        setter.base_type().map(|base| base.full_name()),
        Some("::ferroui_base::collections::FerroList<::ferroui_base::Ref<::fixture::controls::border::Border>>".to_string())
    );
    assert_eq!(full_names(&setter.interfaces()), ["Fixture.Media.IBrush"]);
    assert_eq!(setter.constructors().len(), 1);
    let attributes = setter.custom_attributes();
    assert_eq!(attribute_names(&attributes), ["ContentAttribute"]);
    assert_eq!(attributes[0].properties().get("Name"), Some(&XamlValue::String("Value".to_string())));

    // A static type without a runtime type of its own: static properties.
    let colors = the(&system, "Fixture.Media.Colors");
    let red = property(&colors, "Red");
    assert_eq!(red.property_type().full_name(), "System.UInt32");
    assert!(red.getter().is_some_and(|getter| getter.is_static()) && red.setter().is_none());
    assert!(colors.constructors().is_empty());

    // The assembly: its xmlns definitions.
    let assembly = system.find_assembly("fixture").expect("the assembly of the fixture");
    let definitions = assembly.custom_attributes();
    assert_eq!(definitions.len(), 2);
    assert_eq!(definitions[0].type_().full_name(), "FerroUI.Metadata.XmlnsDefinitionAttribute");
    assert_eq!(
        definitions[0].parameters(),
        vec![XamlValue::String(ferroui_base::metadata::FERRO_XML_NAMESPACE.to_string()), XamlValue::String("Fixture.Controls".to_string())]
    );
    assert!(assembly.find_type("Fixture.Media.Dock").is_some() && assembly.find_type("System.String").is_none());
}

fn object_model(name: &str, namespace: &str, rust_path: &str, module: &str) -> TypeModel {
    let mut type_ = TypeModel::new(name, TypeKind::Class, RustType::resolved(rust_path), module);
    type_.namespace = namespace.to_string();
    type_.object_model = true;
    type_
}

/// A type with markup metadata whose values are held as `handle`.
fn markup(name: &str, kind: TypeKind, namespace: &str, rust_path: &str, handle: &str) -> TypeModel {
    let mut type_ = TypeModel::new(name, kind, RustType::resolved(rust_path), "ferroui_base::markup_types");
    type_.namespace = namespace.to_string();
    type_.handles = vec![RustType::resolved(handle)];
    type_
}

fn enumeration(name: &str, namespace: &str, rust_path: &str) -> TypeModel {
    let mut type_ = TypeModel::new(name, TypeKind::Enum, RustType::resolved(rust_path), "ferroui_base::markup_types");
    type_.namespace = namespace.to_string();
    type_.enum_members = vec![EnumMemberModel { name: "First".to_string(), rust_variant: Some("First".to_string()), rust_value: None, value: Some(0) }];
    type_
}

fn registered(name: Option<&str>, kind: RegisteredKind, value_type: &str, accessor: &str, registration: RegistrationModel) -> RegisteredModel {
    RegisteredModel {
        name: name.map(str::to_string),
        kind,
        value_type: RustType::resolved(value_type),
        owner: None,
        host: None,
        accessor: accessor.to_string(),
        function_of: None,
        visibility: "pub".to_string(),
        registration,
        source: None,
        assign_binding: false,
        inherits: false,
        read_only: false,
        added_owners: Vec::new(),
    }
}

fn parameter(type_: &str) -> ParameterModel {
    ParameterModel { name: None, type_: RustType::resolved(type_), attributes: Vec::new() }
}

const FERRO_OBJECT: &str = "::ferroui_base::ferro_object::FerroObject";
const INTERACTIVE: &str = "::ferroui_base::interactivity::interactive::Interactive";
const DESIGN: &str = "::ferroui_base::design::Design";

/// The models of two crates written by hand: a base crate with the root classes of the
/// object model, the types their synthesised members name, metadata of a runtime library
/// type and a static owner type with metadata declared next to it; and a crate built on it
/// that names the types of the first through its exports.
fn two_crates() -> Rc<ModelTypeSystem> {
    let export = |path: &str, declared: &str| ExportModel { path: path.to_string(), declared: declared.to_string() };
    let mut base = AssemblyModel::new("FerroUI.Base", "ferroui_base");
    base.exports = vec![
        export("::ferroui_base::Ref", "::ferroui_base::type_system::Ref"),
        export("::ferroui_base::BoxedValue", "::ferroui_base::ferro_property::BoxedValue"),
        export("::ferroui_base::FerroObject", FERRO_OBJECT),
        export("::ferroui_base::Interactive", INTERACTIVE),
        export("::ferroui_base::Design", DESIGN),
        export("::ferroui_base::animation::TimeSpan", "::ferroui_base::animation::time_span::TimeSpan"),
    ];
    let mut interactive = object_model("Interactive", "FerroUI.Interactivity", INTERACTIVE, "ferroui_base::interactivity::interactive");
    interactive.base = Some(RustType::resolved(FERRO_OBJECT));

    let routed_event = "::ferroui_base::interactivity::RoutedEvent";
    let arguments = "::ferroui_base::interactivity::RoutedEventArgs";
    let typed_event = format!("{routed_event}<{arguments}>");
    let mut typed = markup("RoutedEvent`1", TypeKind::Class, "FerroUI.Interactivity", &typed_event, &typed_event);
    typed.generic = Some(GenericModel { definition: "RoutedEvent`1".to_string(), arguments: vec![RustType::resolved(arguments)] });
    typed.base = Some(RustType::resolved(routed_event));

    // Metadata of a runtime library type: the type of the table has its members.
    let time_span = "::ferroui_base::animation::time_span::TimeSpan";
    let mut span = markup("TimeSpan", TypeKind::Struct, "System", time_span, time_span);
    span.explicit_namespace = Some("System".to_string());
    span.properties = vec![PropertyModel {
        name: "Days".to_string(),
        value_type: RustType::resolved("i32"),
        getter: Some(AccessorModel { typed_function: Some("__markup_get_Days".to_string()), ..AccessorModel::default() }),
        ..PropertyModel::default()
    }];

    // A static owner type of an attached property, and metadata of its name declared next
    // to it, which states the setter of the property itself.
    let mut design = object_model("Design", "FerroUI.Controls", DESIGN, "ferroui_base::design");
    design.kind = TypeKind::Static;
    let mut is_design_mode = registered(Some("IsDesignMode"), RegisteredKind::Attached, "bool", "is_design_mode_property", RegistrationModel::Declared);
    is_design_mode.owner = Some(RustType::resolved(DESIGN));
    is_design_mode.host = Some(RustType::resolved(FERRO_OBJECT));
    design.registered = vec![is_design_mode];
    let mut design_metadata = TypeModel::new("Design", TypeKind::Static, RustType::resolved(DESIGN), "ferroui_base::design");
    design_metadata.namespace = "FerroUI.Controls".to_string();
    design_metadata.methods = vec![MemberModel {
        name: "SetIsDesignMode".to_string(),
        parameters: vec![parameter("::ferroui_base::Ref<::ferroui_base::FerroObject>"), parameter("bool")],
        is_static: true,
        callable: CallableModel { path: Some("Design::set_is_design_mode".to_string()), resolved: Some(format!("{DESIGN}::set_is_design_mode")), dereferenced: None },
        typed_function: Some("__markup_SetIsDesignMode_0".to_string()),
        ..MemberModel::default()
    }];

    base.types = vec![
        object_model("FerroObject", "FerroUI", FERRO_OBJECT, "ferroui_base::ferro_object"),
        interactive,
        enumeration("BindingPriority", "FerroUI.Data", "::ferroui_base::data::BindingPriority"),
        markup("BindingBase", TypeKind::Interface, "FerroUI.Data", "dyn ::ferroui_base::data::BindingBase", "::std::rc::Rc<dyn ::ferroui_base::data::BindingBase>"),
        markup("BindingExpressionBase", TypeKind::Class, "FerroUI.Data", "::ferroui_base::data::BindingExpressionBase", "::ferroui_base::data::BindingExpressionBase"),
        markup("RoutedEvent", TypeKind::Class, "FerroUI.Interactivity", routed_event, routed_event),
        markup("RoutedEventArgs", TypeKind::Class, "FerroUI.Interactivity", arguments, arguments),
        typed,
        enumeration("RoutingStrategies", "FerroUI.Interactivity", "::ferroui_base::interactivity::RoutingStrategies"),
        span,
        design,
        design_metadata,
    ];

    // The crate built on it names the types of the base crate as its sources spell them.
    let mut controls = AssemblyModel::new("FerroUI.Controls", "ferroui_controls");
    let mut button = object_model("Button", "FerroUI.Controls", "::ferroui_controls::button::Button", "ferroui_controls::button");
    button.base = Some(RustType::resolved("::ferroui_base::Interactive"));
    button.default_constructor = Some(CallableModel { path: Some("Button::new".to_string()), resolved: Some("::ferroui_controls::button::Button::new".to_string()), dereferenced: None });
    let mut owner = registered(None, RegisteredKind::Attached, "bool", "is_design_mode_property", RegistrationModel::AddedOwner);
    owner.source = Some(CallableModel {
        path: Some("Design::is_design_mode_property".to_string()),
        resolved: Some("::ferroui_base::Design::is_design_mode_property".to_string()),
        dereferenced: None,
    });
    let mut tag = registered(Some("Tag"), RegisteredKind::Styled, "Option<::ferroui_base::BoxedValue>", "tag_property", RegistrationModel::Declared);
    tag.assign_binding = true;
    button.registered = vec![owner, tag];
    button.content_property = Some("Tag".to_string());
    controls.types = vec![button];
    ModelTypeSystem::new(vec![base, controls])
}

/// The members of the root classes the compiler resolves and no declaration states, with
/// the signatures of the managed original.
#[test]
fn object_model_members_are_synthesised_on_the_root_classes() {
    let system = two_crates();
    let object = the(&system, "FerroUI.FerroObject");
    assert_eq!(
        signatures(&object),
        [
            "SetValue(FerroUI.StyledProperty`1[T], T, FerroUI.Data.BindingPriority) -> System.IDisposable",
            "SetValue(FerroUI.FerroProperty, System.Object, FerroUI.Data.BindingPriority) -> System.IDisposable",
            "GetValue(FerroUI.FerroProperty) -> System.Object",
            "Bind(FerroUI.FerroProperty, FerroUI.Data.BindingBase) -> FerroUI.Data.BindingExpressionBase"
        ]
    );
    let set_value = object.methods().remove(0);
    assert!(set_value.is_generic_method_definition() && set_value.generic_parameters().len() == 1);
    let typed = set_value.make_generic_method(&[the(&system, "System.Double")]).expect("SetValue<double>");
    assert_eq!(full_names(&typed.parameters())[..2], ["FerroUI.StyledProperty`1[System.Double]".to_string(), "System.Double".to_string()]);
    assert!(!typed.is_generic_method_definition() && typed.is_generic_method());
    assert_eq!(object.constructors().iter().map(|constructor| constructor.is_public()).collect::<Vec<_>>(), [false]);

    let interactive = the(&system, "FerroUI.Interactivity.Interactive");
    assert!(interactive.base_type().is_some_and(|base| base.equals(&*object)));
    assert_eq!(
        signatures(&interactive),
        [
            "AddHandler(FerroUI.Interactivity.RoutedEvent, System.Delegate, FerroUI.Interactivity.RoutingStrategies, System.Boolean) -> System.Void",
            "AddHandler(FerroUI.Interactivity.RoutedEvent`1[TEventArgs], System.EventHandler`1[TEventArgs], FerroUI.Interactivity.RoutingStrategies, System.Boolean) -> System.Void"
        ]
    );
    // The open instantiation of the typed event derives from the untyped one, as the
    // declared instantiation does.
    let routed_event = the(&system, "FerroUI.Interactivity.RoutedEvent");
    let open = interactive.methods()[1].parameters().remove(0);
    assert!(routed_event.is_assignable_from(&*open));
    let declared = system.resolve("::ferroui_base::interactivity::RoutedEvent<::ferroui_base::interactivity::RoutedEventArgs>");
    assert_eq!(declared.full_name(), "FerroUI.Interactivity.RoutedEvent`1[FerroUI.Interactivity.RoutedEventArgs]");
    assert!(routed_event.is_assignable_from(&*declared) && !declared.is_assignable_from(&*routed_event));
    assert_eq!(the(&system, "FerroUI.Metadata.ContentAttribute").assembly().map(|assembly| assembly.name()), Some("FerroUI.Base".to_string()));
}

/// Metadata of a runtime library type is that type of the table; metadata declared next
/// to a type of the object model is part of it; and a type of another crate is the same
/// type by every path it is named by.
#[test]
fn declarations_are_merged_and_found_across_crates() {
    let system = two_crates();

    // The runtime library type has the members of the metadata, and not the member of the
    // table that only stands in for it.
    let time_span = the(&system, "System.TimeSpan");
    assert_eq!(signatures(&time_span), ["get_Days() -> System.Int32"]);
    assert_eq!(time_span.assembly().map(|assembly| assembly.name()), Some("System.Runtime".to_string()));
    assert!(system.type_of_model((0, 9)).equals(&*time_span));
    assert!(system.resolve("::ferroui_base::animation::TimeSpan").equals(&*time_span));
    assert_eq!(system.resolve("Option<::ferroui_base::animation::TimeSpan>").full_name(), "System.Nullable`1[System.TimeSpan]");

    // The handles of the table and of the classes are found by the path a crate exports
    // them by and by the path of the declaring module.
    assert_eq!(system.resolve("Option<::ferroui_base::BoxedValue>").full_name(), "System.Object");
    assert_eq!(system.resolve("::ferroui_base::ferro_property::BoxedValue").full_name(), "System.Object");
    let object = the(&system, "FerroUI.FerroObject");
    assert!(system.resolve("Option<::ferroui_base::Ref<::ferroui_base::FerroObject>>").equals(&*object));
    assert!(system.resolve("::ferroui_base::type_system::Ref<::ferroui_base::ferro_object::FerroObject>").equals(&*object));

    // The static owner type and the metadata declared next to it are one type; the
    // declared setter stands for the plain one, the getter is the plain one.
    let design = the(&system, "FerroUI.Controls.Design");
    assert!(system.type_of_model((0, 10)).equals(&*design) && system.type_of_model((0, 11)).equals(&*design));
    assert_eq!(
        signatures(&design),
        ["static GetIsDesignMode(FerroUI.FerroObject) -> System.Boolean", "static SetIsDesignMode(FerroUI.FerroObject, System.Boolean) -> System.Void"]
    );
    let sources: Vec<MemberSource> =
        design.methods().iter().filter_map(|method| method.as_any().downcast_ref::<ModelMethod>().map(|method| method.source().clone())).collect();
    assert_eq!(sources[0], MemberSource::Registered { type_path: DESIGN.to_string(), accessor: "is_design_mode_property".to_string() });
    assert!(matches!(&sources[1], MemberSource::Declared { typed_function: Some(function), .. } if function == "__markup_SetIsDesignMode_0"), "{sources:?}");
    assert_eq!(field(&design, "IsDesignModeProperty").field_type().full_name(), "FerroUI.AttachedProperty`1[System.Boolean]");

    // The class of the second crate: its base is the class of the first, named through an
    // export; the owner it adds to the attached property of the first is an instance
    // property with the name the first crate declares.
    let button = the(&system, "FerroUI.Controls.Button");
    let interactive = the(&system, "FerroUI.Interactivity.Interactive");
    assert!(button.base_type().is_some_and(|base| base.equals(&*interactive)));
    assert!(object.is_assignable_from(&*button));
    assert_eq!(button.assembly().map(|assembly| assembly.name()), Some("FerroUI.Controls".to_string()));
    assert_eq!(button.properties().iter().map(|property| property.name()).collect::<Vec<_>>(), ["IsDesignMode", "Tag"]);
    assert_eq!(field(&button, "IsDesignModeProperty").field_type().full_name(), "FerroUI.StyledProperty`1[System.Boolean]");
    assert_eq!(attribute_names(&property(&button, "Tag").custom_attributes()), ["AssignBindingAttribute", "ContentAttribute"]);
    assert_eq!(button.constructors().iter().map(|constructor| constructor.is_public()).collect::<Vec<_>>(), [true]);
    // The members of the base class are found through the class, as the transformers look.
    assert!(button.find_method(|method| method.name() == "AddHandler").is_some());
    assert!(button.get_all_properties().iter().any(|property| property.name() == "Tag"));
    assert_eq!(system.find_type_in_assembly("FerroUI.Controls.Button", "FerroUI.Controls").map(|type_| type_.full_name()), Some("FerroUI.Controls.Button".to_string()));
    assert!(system.find_type_in_assembly("FerroUI.Controls.Button", "FerroUI.Base").is_none());
}

/// The type system over the scan of the third fixture crate (`tests/fixtures/registration`).
fn registration() -> Rc<ModelTypeSystem> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("FerroUI.Build.Scan").join("tests").join("fixtures").join("registration").join("lib.rs");
    ModelTypeSystem::new(vec![scan_crate(&ScanOptions::new("registration", root)).model])
}

fn is_opaque(type_: &Rc<dyn IXamlType>) -> bool {
    type_.as_any().downcast_ref::<ModelType>().is_some_and(ModelType::is_opaque)
}

/// What a crate registers decides what its declarations are: a class its list of
/// registered classes leaves out is not known by its name or by its handle (it is the base
/// of the classes that derive from it all the same); a property is a member of the owner
/// its registration names, whichever type has the accessor; an accessor that is a
/// function of another type carries the path of that type.
#[test]
fn registration_decides_the_classes_and_the_owners_of_properties() {
    let system = registration();
    let panel = the(&system, "Registration.Panel");

    assert!(system.find_type("Registration.Hidden").is_none());
    assert!(is_opaque(&system.resolve("::ferroui_base::Ref<::registration::panel::Hidden>")));
    assert!(is_opaque(&property(&the(&system, "Registration.PanelCollection"), "Owner").property_type()));
    let hidden = the(&system, "Registration.Deep").base_type().expect("the base of Deep");
    assert_eq!(hidden.full_name(), "Registration.Hidden");
    assert!(hidden.base_type().is_some_and(|base| base.equals(&*panel)));
    assert!(system.resolve("::ferroui_base::Ref<::registration::panel::Deep>").equals(&*the(&system, "Registration.Deep")));

    // `Saved` is registered by the properties of `Slot` with `Panel` as its owner.
    assert_eq!(panel.properties().iter().map(|property| property.name()).collect::<Vec<_>>(), ["Items", "Mark", "Spacing", "Gap"]);
    assert_eq!(panel.fields().iter().map(|field| field.name()).collect::<Vec<_>>(), ["ItemsProperty", "MarkProperty", "SpacingProperty", "GapProperty", "SavedProperty"]);
    let statics: Vec<String> = panel.methods().iter().filter(|method| method.is_static()).map(signature).collect();
    assert_eq!(statics, ["static GetSaved(Registration.Panel) -> System.Int32", "static SetSaved(Registration.Panel, System.Int32) -> System.Void"]);
    let slot = the(&system, "Registration.Slot");
    assert_eq!(slot.properties().iter().map(|property| property.name()).collect::<Vec<_>>(), ["Mark"]);
    assert_eq!(slot.fields().iter().map(|field| field.name()).collect::<Vec<_>>(), ["MarkProperty"]);
    assert_eq!(field(&slot, "MarkProperty").field_type().full_name(), "FerroUI.StyledProperty`1[System.Boolean]");
    assert_eq!(field(&panel, "SavedProperty").field_type().full_name(), "FerroUI.AttachedProperty`1[System.Int32]");

    // The accessors: of the type the property is listed under, of the type whose function
    // the accessor is, and of the type whose properties register the property.
    let source_of = |type_: &Rc<dyn IXamlType>, name: &str| field(type_, name).as_any().downcast_ref::<ModelField>().map(|field| field.source().clone());
    let accessor = |type_path: &str, accessor: &str| Some(MemberSource::Registered { type_path: type_path.to_string(), accessor: accessor.to_string() });
    assert_eq!(source_of(&panel, "ItemsProperty"), accessor("::registration::Panel", "items_property"));
    assert_eq!(source_of(&panel, "MarkProperty"), accessor("::registration::panel::Marker", "mark_property"));
    assert_eq!(source_of(&panel, "SavedProperty"), accessor("::registration::Slot", "saved_property"));
    assert_eq!(source_of(&slot, "MarkProperty"), accessor("::registration::Slot", "mark_property"));
    // A direct property registered without a setter, by the accessor a macro writes.
    let spacing = property(&panel, "Spacing");
    assert!(spacing.getter().is_some() && spacing.setter().is_none());
    assert_eq!(field(&panel, "SpacingProperty").field_type().full_name(), "FerroUI.DirectProperty`2[Registration.Panel,System.Double]");
}

/// A Rust type is one type by every spelling: through a type alias, through the handles a
/// crate registers next to the declaration, and a collection is the list it declares as
/// its base only when the crate registers the cast to it.
#[test]
fn aliases_registered_handles_and_casts_decide_the_type_of_a_text() {
    let system = registration();
    const LIST: &str = "FerroUI.Collections.FerroList`1[Registration.Panel]";
    let list = system.resolve("::ferroui_base::collections::FerroList<::ferroui_base::Ref<::registration::panel::Panel>>");
    assert_eq!(list.full_name(), LIST);
    assert!(system.resolve("::registration::panel::PanelList").equals(&*list));
    assert!(system.resolve("::registration::PanelList").equals(&*list));
    // The optional form of the handle of a class is the class, through two aliases.
    assert!(property(&the(&system, "Registration.Panel"), "Items").property_type().equals(&*list));
    assert!(is_opaque(&system.resolve("::registration::panel::Lost")));

    // The handles the registration function adds.
    let contract = the(&system, "Registration.IPanel");
    let collection = the(&system, "Registration.PanelCollection");
    assert!(system.resolve("::registration::panel::Wrapper").equals(&*contract));
    assert!(system.resolve("Option<::registration::panel::Wrapper>").equals(&*collection));
    assert!(system.resolve("::std::rc::Rc<dyn ::registration::panel::IPanel>").equals(&*contract));

    // The registered cast, by either spelling of the list.
    assert!(system.is_cast("::registration::panel::PanelCollection", "::registration::panel::PanelList"));
    assert!(system.is_cast("::registration::panel::PanelCollection", "::ferroui_base::collections::FerroList<::ferroui_base::Ref<::registration::Panel>>"));
    assert!(system.is_cast("::registration::panel::PanelList", "::ferroui_base::collections::FerroList<::ferroui_base::Ref<::registration::Panel>>"));
    assert!(!system.is_cast("::registration::panel::PanelStack", "::registration::panel::PanelList"));
    assert!(collection.base_type().is_some_and(|base| base.equals(&*list)));
    assert!(list.is_assignable_from(&*collection));
    let stack = the(&system, "Registration.PanelStack");
    assert_eq!(stack.base_type().map(|base| base.full_name()), Some("System.Object".to_string()));
    assert!(!list.is_assignable_from(&*stack));
}
