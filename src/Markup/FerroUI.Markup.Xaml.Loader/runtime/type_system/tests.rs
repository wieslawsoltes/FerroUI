//! Tests of the run-time type system. Classes of the object model are the
//! ones of the base crate; types with markup metadata are the test classes
//! of the interpreter tests.

use std::rc::Rc;

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupValue};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, FerroObject, Ref, TypeInfo};
use xamlx::type_system::{
    IXamlMember, IXamlMethod, IXamlType, IXamlTypeSystem, XamlPseudoType,
    XamlValue,
};

use crate::runtime::interpreter::tests::classes::{self, *};

use super::*;

fn system() -> Rc<RuntimeTypeSystem> {
    classes::register();
    RuntimeTypeSystem::new()
}

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

fn runtime(type_: &Rc<dyn IXamlType>) -> &RuntimeType {
    type_.as_any().downcast_ref::<RuntimeType>().expect("a run-time type")
}

fn method(type_: &Rc<dyn IXamlType>, name: &str) -> Rc<RuntimeMethod> {
    runtime(type_).runtime_methods().into_iter().find(|m| m.name() == name).unwrap_or_else(|| panic!("method {name}"))
}

#[test]
fn runtime_library_types_are_complete_and_mapped_to_rust_types() {
    let ts = system();
    let known = ts.well_known_types();
    assert_eq!(known.string.full_name(), "System.String");
    assert_eq!(known.get_func_of_t(2).full_name(), "System.Func`2");

    let resolve = |handle: ValueType| ts.resolve(handle).full_name();
    assert_eq!(resolve(ValueType::of::<String>()), "System.String");
    assert_eq!(resolve(ValueType::of::<Option<String>>()), "System.String");
    assert_eq!(resolve(ValueType::of::<i32>()), "System.Int32");
    assert_eq!(resolve(ValueType::of::<Option<i32>>()), "System.Nullable`1[System.Int32]");
    assert_eq!(resolve(ValueType::of::<f64>()), "System.Double");
    assert_eq!(resolve(ValueType::of::<Option<BoxedValue>>()), "System.Object");
    assert_eq!(resolve(ValueType::of::<BoxedValue>()), "System.Object");
    assert_eq!(resolve(ValueType::of::<()>()), "System.Void");
    assert_eq!(resolve(ValueType::of::<Rc<dyn IServiceProvider>>()), "System.IServiceProvider");
    assert_eq!(resolve(ValueType::of::<&'static TypeInfo>()), "System.Type");
    assert_eq!(resolve(ValueType::of::<RuntimeTypeValue>()), "System.Type");
    assert_eq!(resolve(ValueType::of::<CultureInfo>()), "System.Globalization.CultureInfo");
    assert_eq!(
        resolve(ValueType::of::<DeferredContentFactory>()),
        "System.Func`2[System.IServiceProvider,System.Object]"
    );

    let nullable = ts.resolve(ValueType::of::<Option<i32>>());
    assert!(nullable.is_value_type() && nullable.is_nullable());
    assert!(nullable.is_assignable_from(&*known.int32));
    assert!(nullable.is_assignable_from(&*XamlPseudoType::null()));
    assert!(!known.int32.is_assignable_from(&*XamlPseudoType::null()));
    assert!(known.object.is_assignable_from(&*known.int32));
    assert!(ts.find_type("System.Collections.Generic.List`1").is_some());
    assert!(ts.find_type("System.DoesNotExist").is_none());

    // The core assembly answers to the names of the runtime library assemblies.
    assert!(ts.find_type_in_assembly("System.String", "netstandard").is_some());
    assert!(ts.find_type_in_assembly("System.String", "RtXamlParserTests").is_none());
    let invariant = method(&known.culture_info, "get_InvariantCulture").invoke(&[]).expect("invoke");
    assert!(from_markup_value::<CultureInfo>(&invariant) == Some(CultureInfo::invariant_culture()));
}

#[test]
fn classes_project_their_hierarchy_constructor_and_registered_properties() {
    let ts = system();
    let brush = ts.find_type("FerroUI.Media.SolidColorBrush").expect("the class");
    assert_eq!(brush.name(), "SolidColorBrush");
    assert_eq!(brush.namespace().as_deref(), Some("FerroUI.Media"));
    assert_eq!(brush.assembly().map(|a| a.name()).as_deref(), Some("FerroUI.Base"));
    assert!(brush.equals(&*ts.type_of_class(SolidColorBrush::TYPE)));
    assert!(brush.equals(&*ts.resolve(ValueType::of::<Ref<SolidColorBrush>>())));
    assert!(brush.equals(&*ts.resolve(ValueType::of::<Option<Ref<SolidColorBrush>>>())));
    assert!(!brush.is_value_type() && !brush.is_interface());

    let base = brush.base_type().expect("a base type");
    assert_eq!(base.full_name(), "FerroUI.Media.Brush");
    let mut root = base.clone();
    while let Some(next) = root.base_type() {
        root = next;
    }
    assert_eq!(root.full_name(), "System.Object");
    assert!(base.is_assignable_from(&*brush));
    assert!(!brush.is_assignable_from(&*base));
    assert!(ts.type_of_class(FerroObject::TYPE).is_assignable_from(&*brush));
    assert!(ts.well_known_types().object.is_assignable_from(&*brush));
    // The interface the base class declares.
    assert!(!base.interfaces().is_empty());
    assert!(base.interfaces().iter().all(|i| i.is_assignable_from(&*brush)));

    // The default constructor creates an instance held in the handle of its class.
    let constructor = brush.find_constructor(None).expect("the default constructor");
    let constructor = constructor.as_any().downcast_ref::<RuntimeConstructor>().expect("a run-time constructor");
    let instance = constructor.invoke(&[]).expect("an instance");
    let object = from_markup_value::<Ref<SolidColorBrush>>(&instance).expect("the handle of the class");
    assert!(ts.runtime_type_of(instance.as_ref().unwrap()).equals(&*brush));
    assert!(ts.is_instance(&instance, &*base));

    // A registered property is a field with the definition and a property with accessors.
    let field = brush.fields().into_iter().find(|f| f.name() == "ColorProperty").expect("the field");
    assert!(field.is_static() && !field.is_literal());
    let field_type = field.field_type();
    assert_eq!(field_type.generic_type_definition().map(|d| d.full_name()).as_deref(), Some("FerroUI.StyledProperty`1"));
    assert_eq!(field_type.generic_arguments().len(), 1);
    let definition_base = field_type.base_type().expect("the base of the definition type");
    assert_eq!(definition_base.generic_type_definition().map(|d| d.full_name()).as_deref(), Some("FerroUI.FerroProperty`1"));
    assert_eq!(definition_base.base_type().map(|b| b.full_name()).as_deref(), Some("FerroUI.FerroProperty"));
    let runtime_field = field.as_any().downcast_ref::<RuntimeField>().expect("a run-time field");
    assert!(std::ptr::eq(
        runtime_field.ferro_property().unwrap(),
        SolidColorBrush::color_property().as_property()
    ));
    assert!(from_markup_value::<&'static ferroui_base::FerroProperty>(&runtime_field.get().unwrap()).is_some());

    let color = brush.properties().into_iter().find(|p| p.name() == "Color").expect("the property");
    assert!(color.property_type().equals(&*field_type.generic_arguments()[0]));
    let red = Color::from_rgb(255, 0, 0);
    method(&brush, "set_Color").invoke(&[instance.clone(), boxed(red)]).expect("set");
    assert!(object.color() == red);
    let read = method(&brush, "get_Color").invoke(&[instance.clone()]).expect("get");
    assert!(from_markup_value::<Color>(&read) == Some(red));
    // A value of another type is rejected, not converted.
    assert!(method(&brush, "set_Color").invoke(&[instance.clone(), boxed("Red".to_string())]).is_err());

    // The properties of the base class are found through the hierarchy and take the derived handle.
    assert!(brush.get_all_properties().iter().any(|p| p.name() == "Opacity"));
    assert!(!brush.properties().iter().any(|p| p.name() == "Opacity"));
    method(&base, "set_Opacity").invoke(&[instance.clone(), boxed(0.5f64)]).expect("set");
    assert_eq!(object.opacity(), 0.5);

    // A class reference converts to the representations members take types in.
    let type_value = RuntimeTypeValue::new(brush.clone());
    assert!(std::ptr::eq(type_value.type_info().unwrap(), SolidColorBrush::TYPE));
    let as_type_info = type_value.to_handle(ValueType::of::<Option<&'static TypeInfo>>());
    assert!(from_markup_value::<Option<&'static TypeInfo>>(&as_type_info) == Some(Some(SolidColorBrush::TYPE)));
}

#[test]
fn direct_and_attached_properties_are_projected() {
    let ts = system();
    let element = ts.find_type("FerroUI.StyledElement").expect("the class");
    let name = element.fields().into_iter().find(|f| f.name() == "NameProperty").expect("the field");
    assert_eq!(name.field_type().full_name(), "FerroUI.DirectProperty`2[FerroUI.StyledElement,System.String]");
    let direct_base = name.field_type().base_type().expect("the base");
    assert_eq!(direct_base.full_name(), "FerroUI.DirectPropertyBase`1[System.String]");
    let parent = element.properties().into_iter().find(|p| p.name() == "Parent").expect("the property");
    assert!(parent.getter().is_some());
    assert!(parent.setter().is_none(), "a read-only property has no setter");
    assert!(element.properties().iter().find(|p| p.name() == "Name").unwrap().setter().is_some());

    let navigation = ts.find_type("FerroUI.Input.KeyboardNavigation").expect("the static type");
    let field = navigation.fields().into_iter().find(|f| f.name() == "TabIndexProperty").expect("the field");
    assert_eq!(field.field_type().full_name(), "FerroUI.AttachedProperty`1[System.Int32]");
    assert_eq!(
        field.field_type().base_type().map(|b| b.full_name()).as_deref(),
        Some("FerroUI.StyledProperty`1[System.Int32]")
    );
    // An attached property has static accessors and no property.
    assert!(!navigation.properties().iter().any(|p| p.name() == "TabIndex"));
    let get = method(&navigation, "GetTabIndex");
    let set = method(&navigation, "SetTabIndex");
    assert!(get.is_static() && set.is_static());
    assert_eq!(get.return_type().full_name(), "System.Int32");
    assert_eq!(set.parameters().len(), 2);
    assert_eq!(set.parameters()[1].full_name(), "System.Int32");
    if let Some(host) = ferroui_base::StyledElement::TYPE.create_instance() {
        let host = boxed(host);
        set.invoke(&[host.clone(), boxed(7i32)]).expect("set");
        assert_eq!(from_markup_value::<i32>(&get.invoke(&[host]).expect("get")), Some(7));
    }
    assert!(set.invoke(&[None, boxed(7i32)]).is_err());
}

#[test]
fn markup_types_project_members_with_invokers() {
    let ts = system();
    let simple = ts.find_type("RtXamlParserTests.SimpleClass").expect("the type");
    assert!(simple.equals(&*ts.resolve(ValueType::of::<Rc<SimpleClass>>())));
    assert!(simple.equals(&*ts.resolve(ValueType::of::<Option<Rc<SimpleClass>>>())));
    assert_eq!(simple.base_type().map(|b| b.full_name()).as_deref(), Some("System.Object"));
    assert_eq!(simple.assembly().map(|a| a.name()).as_deref(), Some("RtXamlParserTests"));

    let constructor = simple.find_constructor(None).expect("the constructor");
    let instance = constructor.as_any().downcast_ref::<RuntimeConstructor>().unwrap().invoke(&[]).expect("an instance");
    let object = from_markup_value::<Rc<SimpleClass>>(&instance).expect("the handle");
    method(&simple, "set_Test").invoke(&[instance.clone(), boxed("x".to_string())]).expect("set");
    assert_eq!(object.test.borrow().as_deref(), Some("x"));
    // Null is assignable to a property of a reference type.
    method(&simple, "set_Test").invoke(&[instance.clone(), None]).expect("set");
    assert!(object.test.borrow().is_none());

    // The content property carries the content attribute; a collection is a read-only property.
    let children = simple.properties().into_iter().find(|p| p.name() == "Children").expect("the property");
    assert!(children.setter().is_none());
    let content = ts.attribute_type("Content");
    assert_eq!(content.full_name(), "FerroUI.Metadata.ContentAttribute");
    assert!(children.custom_attributes().iter().any(|a| a.type_().equals(&*content)));
    assert!(ts.find_type("FerroUI.Metadata.ContentAttribute").unwrap().equals(&*content));

    // Generic property types: a registered instantiation of a runtime library definition.
    let list = children.property_type();
    assert_eq!(list.full_name(), "System.Collections.Generic.List`1[RtXamlParserTests.SimpleSubClass]");
    let definition = ts.find_type("System.Collections.Generic.List`1").unwrap();
    assert!(list.generic_type_definition().unwrap().equals(&*definition));
    let sub = ts.find_type("RtXamlParserTests.SimpleSubClass").unwrap();
    assert!(definition.make_generic_type(&[sub.clone()]).unwrap().equals(&*list));
    let ilist = ts.find_type("System.Collections.Generic.IList`1").unwrap().make_generic_type(&[sub.clone()]).unwrap();
    assert!(ilist.is_assignable_from(&*list), "the instantiation has the interfaces of the definition");
    assert!(ts.well_known_types().i_enumerable.is_assignable_from(&*list));

    // An abstract interface method dispatches to the method of the run-time type.
    let collection = method(&simple, "get_Children").invoke(&[instance.clone()]).expect("get");
    let add = ilist.get_all_interfaces().iter().flat_map(|i| i.methods()).find(|m| m.name() == "Add").expect("Add");
    let add = add.as_any().downcast_ref::<RuntimeMethod>().unwrap();
    assert!(!add.is_invocable());
    let child = boxed(Rc::new(SimpleSubClass::default()));
    add.invoke(&[collection, child]).expect("dispatch");
    assert_eq!(object.children.items().len(), 1);

    // An instantiation that is not registered is an error, not an empty type.
    let user_definition = ts.find_type("RtXamlParserTests.GenericClass`1").expect("the definition");
    assert_eq!(user_definition.generic_parameters().len(), 1);
    assert!(user_definition.make_generic_type(&[sub]).is_err());
    let argument = ts.find_type("RtXamlParserTests.TypeArgument").unwrap();
    let instantiation = user_definition.make_generic_type(&[argument]).expect("the registered instantiation");
    assert!(instantiation.equals(&*ts.resolve(ValueType::of::<Rc<GenericClassOfTypeArgument>>())));

    // A static method with arguments.
    let value_type = ts.find_type("RtXamlParserTests.ConvertersTestValueType").unwrap();
    assert!(value_type.is_value_type());
    assert_eq!(value_type.base_type().map(|b| b.full_name()).as_deref(), Some("System.ValueType"));
    let parse = method(&value_type, "Parse");
    assert!(parse.is_static() && parse.return_type().equals(&*value_type));
    assert_eq!(parse.parameters()[1].full_name(), "System.Globalization.CultureInfo");
    let parsed = parse.invoke(&[boxed("text".to_string()), boxed(CultureInfo::invariant_culture())]).expect("parse");
    assert_eq!(from_markup_value::<ConvertersTestValueType>(&parsed).unwrap().value, "text");
    assert!(parse.invoke(&[boxed(1i32), None]).is_err());
    assert_eq!(
        ts.resolve(ValueType::of::<Option<ConvertersTestValueType>>()).full_name(),
        "System.Nullable`1[RtXamlParserTests.ConvertersTestValueType]"
    );

    // Static values are fields.
    let intrinsics = ts.find_type("RtXamlParserTests.IntrinsicsTestsClass").unwrap();
    let constant = intrinsics.fields().into_iter().find(|f| f.name() == "IntConstant").expect("the field");
    assert_eq!(constant.field_type().full_name(), "System.Int32");
    let constant = constant.as_any().downcast_ref::<RuntimeField>().unwrap().get().unwrap();
    assert_eq!(from_markup_value::<i32>(&constant), Some(100));
}

#[test]
fn hierarchies_interfaces_enums_and_attributes_of_markup_types() {
    let ts = system();
    let t = |name: &str| ts.find_type(&format!("RtXamlParserTests.{name}")).unwrap_or_else(|| panic!("type {name}"));
    let base = t("InitializationTestsClass");
    let support = t("InitializationTestsSupportInitializeClass");
    let top_down = t("InitializationTestsTopDownClass");
    let interface = t("ISupportInitialize");
    assert!(interface.is_interface());
    assert!(base.is_assignable_from(&*top_down) && support.is_assignable_from(&*top_down));
    assert!(!top_down.is_assignable_from(&*support));
    assert!(interface.is_assignable_from(&*support) && interface.is_assignable_from(&*top_down));
    assert!(!interface.is_assignable_from(&*base));
    assert!(ts.well_known_types().object.is_assignable_from(&*interface));

    // A generic interface instantiation and the interface it inherits.
    let add_child_of_string = t("IAddChild`1").make_generic_type(&[ts.well_known_types().string.clone()]).unwrap();
    assert!(add_child_of_string.interfaces().iter().any(|i| i.equals(&*t("IAddChild"))));
    assert!(t("IAddChild").is_assignable_from(&*t("ObjectWithGenericAddChild")));

    // Attributes: the type is resolved by name, arguments are projected.
    let usable = top_down.custom_attributes();
    assert_eq!(usable.len(), 1);
    assert_eq!(usable[0].type_().full_name(), "FerroUI.Metadata.UsableDuringInitializationAttribute");
    assert_eq!(usable[0].parameters(), [XamlValue::Boolean(true)]);
    assert_eq!(usable[0].type_().base_type().map(|b| b.full_name()).as_deref(), Some("System.Attribute"));
    let converter = t("ConvertersTestsClassWithConverter").custom_attributes();
    assert_eq!(converter[0].type_().full_name(), "System.ComponentModel.TypeConverterAttribute");
    assert!(converter[0].parameters()[0].as_type().unwrap().equals(&*t("TestConverter")));
    assert_eq!(attribute_type_name("Obsolete"), "System.ObsoleteAttribute");
    assert_eq!(attribute_type_name("TemplatePart"), "FerroUI.Controls.Metadata.TemplatePartAttribute");
    assert_eq!(attribute_type_name("DependsOn"), "FerroUI.Metadata.DependsOnAttribute");
    let deferred = t("DeferredContentTestsClass").properties().into_iter().find(|p| p.name() == "DeferredContent").unwrap();
    let names: Vec<String> = deferred.custom_attributes().iter().map(|a| a.type_().name()).collect();
    assert_eq!(names, ["DeferredContentAttribute", "ContentAttribute"]);

    // Enumerations: members are static literal fields with the member as their value.
    let enumeration = t("ConvertersTestsEnum");
    assert!(enumeration.is_enum() && enumeration.is_value_type());
    assert_eq!(enumeration.base_type().map(|b| b.full_name()).as_deref(), Some("System.Enum"));
    assert_eq!(enumeration.get_enum_underlying_type().unwrap().full_name(), "System.Int32");
    let fields = enumeration.fields();
    let names: Vec<String> = fields.iter().map(|f| f.name()).collect();
    assert_eq!(names, ["First", "Second", "Third"]);
    assert!(enumeration.custom_attributes().iter().any(|a| a.type_().full_name() == "System.FlagsAttribute"));
    assert!(fields.iter().all(|f| f.is_static() && f.is_literal() && f.field_type().equals(&*enumeration)));
    assert_eq!(fields[2].get_literal_value().unwrap(), XamlValue::Int32(4));
    let third = fields[2].as_any().downcast_ref::<RuntimeField>().unwrap().get().unwrap();
    assert_eq!(from_markup_value::<ConvertersTestsEnum>(&third), Some(ConvertersTestsEnum::THIRD));
    assert!(ts.is_instance(&third, &*enumeration));
    assert!(ts.is_instance(&third, &*ts.nullable_of(&enumeration).unwrap()));
    assert!(!ts.is_instance(&None, &*enumeration));
}

#[test]
fn assemblies_expose_their_xml_namespace_definitions() {
    let ts = system();
    let assembly = ts.find_assembly("RtXamlParserTests").expect("the assembly");
    // Assembly names are looked up ignoring case.
    assert!(ts.find_assembly("rtxamlparsertests").is_some_and(|a| a.equals(&*assembly)));
    assert!(ts.find_assembly("ferroui.base").is_some());
    assert!(ts.find_type_in_assembly("RtXamlParserTests.SimpleClass", "RTXAMLPARSERTESTS").is_some());
    assert!(ts.find_assembly("no.such.assembly").is_none());
    let attributes = assembly.custom_attributes();
    let definition = ts.attribute_type("XmlnsDefinition");
    assert_eq!(definition.full_name(), "FerroUI.Metadata.XmlnsDefinitionAttribute");
    let test = attributes
        .iter()
        .find(|a| a.parameters().first() == Some(&XamlValue::String("rt-test".to_string())))
        .expect("the definition");
    assert!(test.type_().equals(&*definition));
    assert_eq!(test.parameters()[1], XamlValue::String("RtXamlParserTests".to_string()));
    assert!(assembly.find_type("RtXamlParserTests.SimpleClass").is_some());
    assert!(assembly.find_type("System.String").is_none());
    assert!(assembly.find_type("FerroUI.Media.SolidColorBrush").is_none());

    let base = ts.assemblies().into_iter().find(|a| a.name() == "FerroUI.Base").expect("the base assembly");
    assert!(base.custom_attributes().iter().any(|a| {
        a.parameters()
            == [
                XamlValue::String(ferroui_base::metadata::FERRO_XML_NAMESPACE.to_string()),
                XamlValue::String("FerroUI.Media".to_string()),
            ]
    }));
    assert!(base.find_type("FerroUI.Media.SolidColorBrush").is_some());
    assert!(ts.find_type_in_assembly("RtXamlParserTests.SimpleClass", "RtXamlParserTests").is_some());
}

#[test]
fn rust_types_without_metadata_are_opaque() {
    let ts = system();
    let opaque = ts.resolve(ValueType::of::<std::time::Duration>());
    assert_eq!(opaque.name(), "core::time::Duration");
    assert!(opaque.equals(&*ts.resolve(ValueType::of::<std::time::Duration>())));
    assert!(opaque.is_assignable_from(&*opaque));
    assert!(ts.well_known_types().object.is_assignable_from(&*opaque));
    assert!(!opaque.is_assignable_from(&*ts.well_known_types().object));
    assert!(!opaque.is_assignable_from(&*ts.well_known_types().string));
    assert!(opaque.methods().is_empty() && opaque.constructors().is_empty());
    // A value of the type is an instance of it.
    let value = boxed(std::time::Duration::from_secs(1));
    assert!(ts.is_instance(&value, &*opaque));
    assert!(!ts.is_instance(&value, &*ts.well_known_types().string));
    let _ = ValueTypes::accepts_null(ValueType::of::<String>());
}

#[test]
fn static_types_merge_their_metadata_and_project_attached_property_attributes() {
    let ts = system();
    let owner = ts.find_type("RtXamlParserTests.RtAttachedOwner").expect("the static type");
    // The runtime type and its metadata are one type.
    assert!(owner.equals(&*ts.type_of_class(<RtAttachedOwner as ferroui_base::StaticType>::TYPE)));
    assert!(owner.equals(&*ts.type_of_markup(<RtAttachedOwner as ferroui_base::metadata::MarkupTyped>::MARKUP)));
    assert!(runtime(&owner).markup().is_some());
    let twice = method(&owner, "Twice");
    assert_eq!(from_markup_value::<i32>(&twice.invoke(&[boxed(4i32)]).unwrap()), Some(8));

    // The attributes declared for the registered property sit on the field and on the accessors.
    let names = |attributes: Vec<Rc<dyn xamlx::type_system::IXamlCustomAttribute>>| -> Vec<String> {
        attributes.iter().map(|a| a.type_().full_name()).collect()
    };
    let expected = ["FerroUI.Controls.ResolveByNameAttribute", "FerroUI.Metadata.DependsOnAttribute"];
    let field = owner.fields().into_iter().find(|f| f.name() == "TargetProperty").expect("the field");
    assert_eq!(names(field.custom_attributes()), expected);
    assert_eq!(field.custom_attributes()[1].parameters(), [XamlValue::String("Other".to_string())]);
    let get = method(&owner, "GetTarget");
    let set = method(&owner, "SetTarget");
    assert_eq!(names(get.custom_attributes()), expected);
    assert_eq!(names(set.custom_attributes()), expected);
    assert!(!owner.properties().iter().any(|p| p.name() == "Target"));
    // The accessors take the host type of the attached property.
    assert_eq!(get.parameters()[0].full_name(), "FerroUI.StyledElement");
    assert_eq!(set.parameters()[0].full_name(), "FerroUI.StyledElement");
    assert_eq!(method(&ts.find_type("FerroUI.Input.KeyboardNavigation").unwrap(), "SetTabIndex").parameters()[0].full_name(), "FerroUI.StyledElement");
}

#[test]
fn events_project_their_handler_type() {
    use ferroui_base::metadata::MarkupDelegate;
    let ts = system();
    let source = ts.find_type("RtXamlParserTests.RtEventSource").unwrap();
    let handler = |name: &str| {
        let event = source.events().into_iter().find(|e| e.name() == name).expect("the event");
        event.add().expect("the add method").parameters()[0].clone()
    };
    // `(object sender, TArgs)` is an `EventHandler<TArgs>`.
    let changed = handler("Changed");
    assert_eq!(changed.full_name(), "System.EventHandler`1[System.String]");
    assert!(ts.well_known_types().delegate.is_assignable_from(&*changed));
    let invoke = changed.get_method(|m| m.name() == "Invoke").unwrap();
    let parameters: Vec<String> = invoke.parameters().iter().map(|p| p.full_name()).collect();
    assert_eq!(parameters, ["System.Object", "System.String"]);
    // Zero-argument events and other shapes use the action family.
    assert_eq!(handler("Pinged").full_name(), "System.Action");
    assert_eq!(handler("Moved").full_name(), "System.Action`2[System.Int32,System.Int32]");

    let instance = Rc::new(RtEventSource::default());
    let called = Rc::new(std::cell::Cell::new(0usize));
    let counter = called.clone();
    let delegate = MarkupDelegate::new(move |arguments| {
        counter.set(counter.get() + arguments.len() + 1);
        None
    });
    method(&source, "add_Pinged").invoke(&[boxed(instance.clone()), boxed(delegate)]).expect("subscribe");
    instance.pinged.borrow()[0].invoke(&[]);
    assert_eq!(called.get(), 1);
}

#[test]
fn compile_time_values_are_parsed_by_the_parse_function_of_the_type() {
    use crate::compiler_extensions::ast_nodes::FerroXamlIlGridUnitType;
    use crate::compiler_extensions::IXamlCompileTimeValueParser;
    use crate::runtime::value_parser::grid_length;
    use crate::runtime::RuntimeCompileTimeValueParser;
    let ts = system();
    let parser = RuntimeCompileTimeValueParser;
    let length = ts.find_type("RtXamlParserTests.RtLength").unwrap();
    // Text the type rejects is an error; accepted text of a type that is not a compile-time
    // value of the language is left to the caller.
    assert!(matches!(parser.try_parse(&length, "abc"), Some(Err(_))));
    assert!(parser.try_parse(&length, "2*").is_none());
    // Types without a parse function, and types of another type system, are not handled.
    assert!(parser.try_parse(&ts.find_type("RtXamlParserTests.SimpleClass").unwrap(), "x").is_none());
    assert!(parser.try_parse(&XamlPseudoType::unknown(), "x").is_none());

    // A parsed value is taken apart through its `Value` and `GridUnitType` properties.
    let parse = runtime(&length).markup().unwrap().parse.unwrap();
    let parsed = parse(&[boxed("2*".to_string())]).unwrap();
    let taken_apart = grid_length(runtime(&length), &parsed).expect("a grid length");
    assert_eq!(taken_apart.value, 2.0);
    assert_eq!(taken_apart.grid_unit_type, FerroXamlIlGridUnitType::Star);
    let parsed = parse(&[boxed("12".to_string())]).unwrap();
    assert_eq!(grid_length(runtime(&length), &parsed).unwrap().grid_unit_type, FerroXamlIlGridUnitType::Pixel);
}

#[test]
fn system_types_with_registered_metadata_are_projections_of_it() {
    ferroui_base::register_types();
    let ts = RuntimeTypeSystem::new();
    let uri = ts.get("System.Uri");
    let kinds: Vec<Vec<String>> =
        uri.constructors().iter().map(|c| c.parameters().iter().map(|p| p.full_name()).collect()).collect();
    assert!(kinds.contains(&vec!["System.String".to_string(), "System.UriKind".to_string()]), "{kinds:?}");
    assert!(kinds.contains(&vec!["System.String".to_string()]));
    // The metadata and the runtime library name are one type.
    let markup = ferroui_base::metadata::MarkupType::find("System", "Uri").expect("metadata of Uri");
    assert!(ts.type_of_markup(markup).equals(&*uri));
    assert!(ts.resolve(ValueType::of::<ferroui_base::utilities::Uri>()).equals(&*uri));

    let time_span = ts.get("System.TimeSpan");
    assert!(time_span.is_value_type());
    let parse = method(&time_span, "Parse");
    let parsed = parse.invoke(&[boxed("0:0:1.5".to_string())]).ok().flatten().expect("parsed");
    assert_eq!(parsed.downcast_ref::<ferroui_base::animation::TimeSpan>().map(|t| t.ticks()), Some(15_000_000));
    // Members only the metadata declares.
    assert!(time_span.methods().iter().any(|m| m.name() == "FromSeconds"));
    assert!(time_span.properties().iter().any(|p| p.name() == "Ticks"));
}

#[test]
fn type_values_are_passed_in_the_declared_representation() {
    ferroui_base::register_types();
    let ts = RuntimeTypeSystem::new();
    let class = ts.type_of_class(<ferroui_base::StyledElement as ferroui_base::StaticType>::TYPE);
    let class_value = ts.type_value(&class);
    let class_value = class_value.downcast_ref::<RuntimeTypeValue>().unwrap().clone();
    let double = ts.get("System.Double");
    let double_value = RuntimeTypeValue::new(double);

    // A class: the class reference where one is declared, also for an untyped target.
    let declared = class_value.to_declared(ValueType::of::<&'static TypeInfo>()).unwrap();
    assert!(declared.downcast_ref::<&'static TypeInfo>().is_some());
    let optional = class_value.to_declared(ValueType::of::<Option<&'static TypeInfo>>()).unwrap();
    assert!(optional.downcast_ref::<Option<&'static TypeInfo>>().is_some_and(|t| t.is_some()));
    let untyped = class_value.to_declared(ValueType::object()).unwrap();
    assert!(untyped.downcast_ref::<&'static TypeInfo>().is_some());
    assert!(untyped.value_eq(&*declared), "type keys compare equal however they were produced");
    let as_value_type = class_value.to_declared(ValueType::of::<ValueType>()).unwrap();
    assert!(as_value_type.downcast_ref::<ValueType>().is_some());

    // Any other type: its value type; untyped targets receive the same.
    let declared = double_value.to_declared(ValueType::of::<ValueType>()).unwrap();
    assert_eq!(declared.downcast_ref::<ValueType>().copied(), Some(ValueType::of::<f64>()));
    let untyped = double_value.to_declared(ValueType::object()).unwrap();
    assert_eq!(untyped.downcast_ref::<ValueType>().copied(), Some(ValueType::of::<f64>()));
    // A type that is not a class where a class is declared is an error that names the type.
    let error = double_value.to_declared(ValueType::of::<&'static TypeInfo>()).unwrap_err();
    assert!(error.contains("System.Double") && error.contains("not a class"), "{error}");
    // A member that takes `System.Type` itself receives the value unchanged.
    assert!(double_value.to_declared(ValueType::of::<RuntimeTypeValue>()).unwrap().downcast_ref::<RuntimeTypeValue>().is_some());

    // Through an invoker: the error names the member.
    let converted = super::to_exact_value(&Some(Rc::new(double_value.clone())), ValueType::object()).unwrap();
    assert!(converted.downcast_ref::<ValueType>().is_some());
}

#[test]
fn the_list_converter_is_synthesized_for_every_element_type() {
    ferroui_base::register_types();
    let ts = RuntimeTypeSystem::new();
    let definition = ts.get("FerroUI.Collections.FerroListConverter`1");
    let double = ts.get("System.Double");
    let converter_type = definition.make_generic_type(std::slice::from_ref(&double)).ok().expect("converter of double");
    assert!(ts.get("System.ComponentModel.TypeConverter").is_assignable_from(&*converter_type));
    assert!(converter_type.generic_arguments()[0].equals(&*double));
    // The same instantiation every time.
    assert!(definition.make_generic_type(std::slice::from_ref(&double)).ok().unwrap().equals(&*converter_type));

    let runtime = converter_type.as_any().downcast_ref::<RuntimeType>().unwrap();
    let constructor = runtime.runtime_constructors().into_iter().next().expect("constructor");
    let converter = constructor.invoke(&[]).ok().expect("converter");
    assert!(ts.runtime_type_of(converter.as_ref().unwrap()).equals(&*converter_type));
    let convert_from = method(&converter_type, "ConvertFrom");
    assert_eq!(convert_from.parameters().len(), 3);

    let list = convert_from.invoke(&[converter.clone(), None, None, boxed("1, 2.5,-3".to_string())]).ok().flatten();
    let list = list.expect("a list");
    assert_eq!(ts.runtime_type_of(&list).full_name(), "FerroUI.Collections.FerroList`1[System.Double]");
    // A value that is not text converts to null; an entry that is no number is the invalid cast.
    assert!(convert_from.invoke(&[converter.clone(), None, None, boxed(5i32)]).ok().expect("null").is_none());
    let error = convert_from.invoke(&[converter.clone(), None, None, boxed("1,x".to_string())]).unwrap_err();
    assert!(error.to_string().contains("Could not convert 'x' to System.Double."), "{error}");
    // Entries are not dropped: an empty entry fails for a number.
    assert!(convert_from.invoke(&[converter, None, None, boxed("1,,2".to_string())]).is_err());

    // The conversions of an entry.
    use super::list_converter::convert_text;
    let text = convert_text(&ts.get("System.String"), " a ").flatten();
    assert_eq!(text.and_then(|t| t.downcast_ref::<String>().cloned()).as_deref(), Some(" a "));
    assert!(convert_text(&ts.get("System.Boolean"), "True").is_some());
    assert!(convert_text(&ts.get("System.Int32"), "1.5").is_none());
    let priority = ts.get("FerroUI.Data.BindingPriority");
    assert!(convert_text(&priority, "Style").is_some());
    assert!(convert_text(&priority, "style").is_none(), "enumeration members are matched exactly");
    assert!(convert_text(&ts.get("System.Uri"), "http://x/").is_none());
}

// --- projections added for the framework language ------------------------------

mod projection {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use ferroui_base::collections::FerroList;
    use ferroui_base::ferro_markup_type;
    use ferroui_base::metadata::{MarkupType, MarkupTyped};

    /// A type with an indexer, a static value and a constructor argument.
    #[derive(Default)]
    pub(super) struct Probe {
        pub items: RefCell<HashMap<String, i32>>,
        pub name: RefCell<Option<String>>,
    }

    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    fn with_name(name: Option<String>) -> Rc<Probe> {
        Rc::new(Probe { items: RefCell::default(), name: RefCell::new(name) })
    }

    ferro_markup_type!(class Probe {
        handles: [Rc<Probe>, Option<Rc<Probe>>],
        this: Rc<Probe>,
        namespace: "RtProjection",
        constructors: [
            () => || Rc::new(Probe::default()),
            (Option<String>) => with_name,
        ],
        properties: [
            Name: Option<String> {
                get: |probe: &Rc<Probe>| probe.name.borrow().clone(),
                set: |probe: &Rc<Probe>, value: Option<String>| *probe.name.borrow_mut() = value
            } [ConstructorArgument("name"), InheritDataTypeFrom(2)],
        ],
        indexers: [
            (String) -> i32 {
                try_get: |probe: &Rc<Probe>, key: String| {
                    probe.items.borrow().get(&key).copied().ok_or_else(|| format!("The key '{key}' is missing."))
                },
                set: |probe: &Rc<Probe>, key: String, value: i32| {
                    probe.items.borrow_mut().insert(key, value);
                }
            },
        ],
        fields: [Shared: String => || "shared".to_string()],
    });

    /// A collection that names the notifying list as its base, without a cast of its
    /// values to the list.
    #[derive(Clone, PartialEq, Default)]
    pub(super) struct Detached(pub FerroList<String>);

    ferro_markup_type!(class Detached {
        handles: [Detached, Option<Detached>],
        namespace: "RtProjection",
        base: FerroList<String>,
        constructors: [() => Detached::default],
    });

    /// A type whose declaration states its constructor parameters, its static
    /// properties and an array-valued attribute argument.
    #[derive(Clone, Debug, PartialEq, Default)]
    pub(super) struct Declared {
        pub name: Option<String>,
    }

    thread_local! {
        static CURRENT: RefCell<String> = RefCell::new("initial".to_string());
    }

    ferro_markup_type!(class Declared {
        handles: [Declared, Option<Declared>],
        namespace: "RtProjection",
        constructors: [
            () => Declared::default,
            (name: Option<String> [InheritDataTypeFrom(1)], count: i32) =>
                |name: Option<String>, _count: i32| Declared { name },
            (Option<String>) => |name: Option<String>| Declared { name },
        ],
        properties: [
            Name: Option<String> { get: |declared: &Declared| declared.name.clone() }
                [ConstructorArgument("name"), InheritDataTypeFrom(2)],
        ],
        fields: [Constant: String => || "constant".to_string()],
        static_properties: [
            ReadOnly: String { get: || "read only".to_string() } [Unstable],
            Current: String {
                get: || CURRENT.with(|current| current.borrow().clone()),
                set: |value: String| CURRENT.with(|current| *current.borrow_mut() = value)
            },
        ],
        attributes: [FerroList(Separators = [",", " "]), Sample([1, ["a", null]], Types = [type(Declared)])],
    });

    pub(super) fn register() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            MarkupType::register_all(&[
                <Probe as MarkupTyped>::MARKUP,
                <Detached as MarkupTyped>::MARKUP,
                <Declared as MarkupTyped>::MARKUP,
            ]);
        });
    }
}

fn projection_system() -> Rc<RuntimeTypeSystem> {
    projection::register();
    system()
}

fn attribute_names(attributes: Vec<Rc<dyn xamlx::type_system::IXamlCustomAttribute>>) -> Vec<String> {
    attributes.iter().map(|a| a.type_().full_name()).collect()
}

#[test]
fn an_indexer_is_the_default_member_item() {
    let ts = projection_system();
    let probe = ts.find_type("RtProjection.Probe").expect("the type");
    let attribute = probe.custom_attributes().into_iter().find(|a| a.type_().name() == "DefaultMemberAttribute");
    let attribute = attribute.expect("the default member attribute");
    assert_eq!(attribute.type_().namespace().as_deref(), Some("System.Reflection"));
    assert_eq!(attribute.parameters(), [XamlValue::String("Item".to_string())]);

    let item = probe.properties().into_iter().find(|p| p.name() == "Item").expect("the indexer");
    assert_eq!(item.property_type().full_name(), "System.Int32");
    let parameters: Vec<String> = item.indexer_parameters().iter().map(|p| p.full_name()).collect();
    assert_eq!(parameters, ["System.String"]);
    // A property that is not an indexer has no index parameters.
    let name = probe.properties().into_iter().find(|p| p.name() == "Name").expect("the property");
    assert!(name.indexer_parameters().is_empty());

    let instance: MarkupValue = boxed(Rc::new(projection::Probe::default()));
    let set = method(&probe, "set_Item");
    let get = method(&probe, "get_Item");
    assert_eq!(set.parameters().len(), 2);
    set.invoke(&[instance.clone(), boxed("a".to_string()), boxed(7i32)]).expect("set");
    assert_eq!(from_markup_value::<i32>(&get.invoke(&[instance.clone(), boxed("a".to_string())]).unwrap()), Some(7));
    // A failing getter is a failed invocation, not a panic.
    assert!(get.invoke(&[instance, boxed("missing".to_string())]).is_err());
}

#[test]
fn runtime_library_collections_have_count_and_an_indexer() {
    let ts = system();
    for (name, index, writable) in [
        ("System.Collections.Generic.List`1", "System.Int32", true),
        ("System.Collections.Generic.IList`1", "System.Int32", true),
        ("System.Collections.Generic.IReadOnlyList`1", "System.Int32", false),
    ] {
        let definition = ts.find_type(name).expect("the definition");
        let list = definition.make_generic_type(&[ts.get("System.String")]).expect("the instantiation");
        assert!(
            list.custom_attributes().iter().any(|a| a.type_().name() == "DefaultMemberAttribute"),
            "{name} has a default member"
        );
        let item = list.properties().into_iter().find(|p| p.name() == "Item").expect("the indexer");
        assert_eq!(item.property_type().full_name(), "System.String", "{name}");
        assert_eq!(item.indexer_parameters()[0].full_name(), index, "{name}");
        assert_eq!(item.setter().is_some(), writable, "{name}");
    }
    let dictionary = ts.find_type("System.Collections.Generic.Dictionary`2").unwrap();
    let dictionary = dictionary.make_generic_type(&[ts.get("System.String"), ts.get("System.Int32")]).unwrap();
    let item = dictionary.properties().into_iter().find(|p| p.name() == "Item").expect("the indexer");
    assert_eq!(item.indexer_parameters()[0].full_name(), "System.String");
    assert_eq!(item.property_type().full_name(), "System.Int32");
    assert!(dictionary.properties().iter().any(|p| p.name() == "Count"));

    // `string.Length` counts UTF-16 code units.
    let length = method(&ts.get("System.String"), "get_Length");
    assert_eq!(from_markup_value::<i32>(&length.invoke(&[boxed("h\u{e9}llo \u{1F600}".to_string())]).unwrap()), Some(8));
    assert!(ts.get("System.Array").properties().iter().any(|p| p.name() == "Length"));
}

#[test]
fn a_list_of_the_runtime_library_is_created_for_any_element_type() {
    let ts = system();
    let known = ts.well_known_types();
    let definition = ts.find_type("System.Collections.Generic.List`1").expect("the definition");
    // The assembly markup names for the list (`assembly=System.Collections`).
    assert!(ts.find_type_in_assembly("System.Collections.Generic.List`1", "System.Collections").is_some());

    // No metadata registers `List<double>`.
    let list_type = definition.make_generic_type(&[ts.get("System.Double")]).expect("the instantiation");
    assert_eq!(list_type.full_name(), "System.Collections.Generic.List`1[System.Double]");
    assert!(list_type.generic_type_definition().unwrap().equals(&*definition));
    assert!(known.i_enumerable.is_assignable_from(&*list_type));
    let ilist = known.i_list_of_t.make_generic_type(&[ts.get("System.Double")]).unwrap();
    assert!(ilist.is_assignable_from(&*list_type));

    let constructor = list_type.find_constructor(None).expect("the constructor");
    let constructor = constructor.as_any().downcast_ref::<RuntimeConstructor>().expect("a run-time constructor");
    let list = constructor.invoke(&[]).expect("a list");
    assert!(ts.runtime_type_of(list.as_ref().unwrap()).equals(&*list_type));
    assert!(ts.is_instance(&list, &*known.i_enumerable));

    let add = method(&list_type, "Add");
    assert!(add.is_invocable() && add.return_type().equals(&*known.void));
    assert_eq!(add.parameters()[0].full_name(), "System.Double");
    add.invoke(&[list.clone(), boxed(1.5f64)]).expect("add");
    // The method of the list contract dispatches to the list.
    let contract_add = ilist.get_all_interfaces().iter().flat_map(|i| i.methods()).find(|m| m.name() == "Add").expect("Add");
    let contract_add = contract_add.as_any().downcast_ref::<RuntimeMethod>().unwrap();
    assert!(!contract_add.is_invocable());
    contract_add.invoke(&[list.clone(), boxed(2.5f64)]).expect("dispatch");

    let count = method(&list_type, "get_Count");
    assert_eq!(from_markup_value::<i32>(&count.invoke(&[list.clone()]).unwrap()), Some(2));
    let get = method(&list_type, "get_Item");
    assert_eq!(from_markup_value::<f64>(&get.invoke(&[list.clone(), boxed(1i32)]).unwrap()), Some(2.5));
    assert!(get.invoke(&[list.clone(), boxed(2i32)]).is_err());
    assert!(get.invoke(&[list.clone(), boxed(-1i32)]).is_err());
    method(&list_type, "set_Item").invoke(&[list.clone(), boxed(0i32), boxed(3.5f64)]).expect("set");

    let items = from_markup_value::<RuntimeList>(&list).expect("the run-time list");
    assert!(items.element_type().is_some_and(|element_type| element_type.full_name() == "System.Double"));
    assert_eq!(items.count(), 2);
    assert_eq!(from_markup_value::<f64>(&items.get(0)), Some(3.5));
    // An untyped target takes the list itself.
    assert!(to_exact_value(&list, ValueType::object()).is_ok());
    assert!(items.to_declared(ValueType::object()).is_none());

    // A registered instantiation keeps the members and the values of its metadata.
    let registered = definition.make_generic_type(&[ts.get("System.Int32")]).unwrap();
    assert!(registered.equals(&*ts.resolve(ValueType::of::<Rc<TestList<i32>>>())));
}

#[test]
fn an_array_list_takes_any_item() {
    let ts = system();
    let known = ts.well_known_types();
    let list_type = ts.find_type("System.Collections.ArrayList").expect("the type");
    assert!(ts.find_type_in_assembly("System.Collections.ArrayList", "System.Collections.NonGeneric").is_some());
    assert_eq!(list_type.base_type().map(|b| b.full_name()).as_deref(), Some("System.Object"));
    assert!(known.i_list.is_assignable_from(&*list_type));
    assert!(known.i_enumerable.is_assignable_from(&*list_type));

    let constructor = list_type.find_constructor(None).expect("the constructor");
    let constructor = constructor.as_any().downcast_ref::<RuntimeConstructor>().expect("a run-time constructor");
    let list = constructor.invoke(&[]).expect("a list");
    assert!(ts.runtime_type_of(list.as_ref().unwrap()).equals(&*list_type));

    // `ArrayList.Add(object)` returns the index of the item; null is an item.
    let add = method(&list_type, "Add");
    assert!(add.return_type().equals(&*known.int32) && add.parameters()[0].equals(&*known.object));
    assert_eq!(from_markup_value::<i32>(&add.invoke(&[list.clone(), None]).unwrap()), Some(0));
    assert_eq!(from_markup_value::<i32>(&add.invoke(&[list.clone(), boxed("Hello".to_string())]).unwrap()), Some(1));
    assert_eq!(from_markup_value::<i32>(&add.invoke(&[list.clone(), boxed(7i32)]).unwrap()), Some(2));

    let items = from_markup_value::<RuntimeList>(&list).expect("the run-time list");
    assert!(items.element_type().is_none());
    assert_eq!(items.count(), 3);
    assert!(items.get(0).is_none());
    assert_eq!(from_markup_value::<String>(&items.get(1)).as_deref(), Some("Hello"));
    let get = method(&list_type, "get_Item");
    assert_eq!(from_markup_value::<i32>(&get.invoke(&[list.clone(), boxed(2i32)]).unwrap()), Some(7));

    // The shared list of the items is what a collection handle is cast from.
    let shared: BoxedValue = Rc::new(items.items().clone());
    ValueTypes::register_cast::<Rc<ferroui_base::collections::FerroList<MarkupValue>>, ListProbe>(|list| ListProbe(list.count()));
    let probe = to_exact_value(&list, ValueType::of::<ListProbe>()).expect("the cast of the shared list");
    assert_eq!(probe.downcast_ref::<ListProbe>(), Some(&ListProbe(3)));
    assert!(ValueTypes::try_cast(&shared, ValueType::of::<ListProbe>()).is_some());
    // Without a registered cast the list is not a value of the type.
    assert!(to_exact_value(&list, ValueType::of::<String>()).is_err());
}

/// What a crate registers a cast of the shared list of items to: a collection handle.
#[derive(Clone, Debug, PartialEq)]
struct ListProbe(usize);

#[test]
fn a_static_value_declared_as_a_field_is_a_field_only() {
    let ts = projection_system();
    let probe = ts.find_type("RtProjection.Probe").expect("the type");
    let field = probe.fields().into_iter().find(|f| f.name() == "Shared").expect("the field");
    assert!(field.is_static());
    assert!(!probe.properties().iter().any(|p| p.name() == "Shared"));

    // The definitions of registered properties are fields only.
    let layoutable = ts.find_type("FerroUI.Layout.Layoutable").expect("the class");
    assert!(layoutable.fields().iter().any(|f| f.name() == "WidthProperty"));
    assert!(!layoutable.properties().iter().any(|p| p.name() == "WidthProperty"));
}

#[test]
fn a_constructor_parameter_has_the_attributes_its_declaration_states() {
    let ts = projection_system();
    // The positional form states none.
    let probe = ts.find_type("RtProjection.Probe").expect("the type");
    let constructor = probe.constructors().into_iter().find(|c| c.parameters().len() == 1).expect("the constructor");
    assert!(constructor.get_parameter_info(0).expect("the parameter").custom_attributes().is_empty());

    // The binding to the templated parent: its property is looked up in the control
    // template scope, also when it is given as the constructor argument.
    let binding = ts.find_type("FerroUI.Data.TemplateBinding").expect("the type");
    let constructor = binding.constructors().into_iter().find(|c| c.parameters().len() == 1).expect("the constructor");
    let parameter = constructor.get_parameter_info(0).expect("the parameter");
    assert_eq!(attribute_names(parameter.custom_attributes()), ["FerroUI.Metadata.InheritDataTypeFromAttribute"]);
}

#[test]
fn a_class_without_a_constructor_has_a_constructor_that_is_not_public() {
    let ts = system();
    let style_base = ts.find_type("FerroUI.Styling.StyleBase").expect("the class");
    let constructors = style_base.constructors();
    assert_eq!(constructors.len(), 1);
    assert!(!constructors[0].is_public() && constructors[0].parameters().is_empty());
    assert!(style_base.find_constructor(None).is_none());
    let runtime_constructors = runtime(&style_base).runtime_constructors();
    assert!(runtime_constructors[0].invoke(&[]).is_err());

    // A class with a constructor keeps its public one only.
    let style = ts.find_type("FerroUI.Styling.Style").expect("the class");
    assert!(style.constructors().iter().all(|c| c.is_public()));
    assert!(style.find_constructor(None).is_some());
}

#[test]
fn collections_derived_from_the_notifying_list_implement_its_contracts() {
    let ts = projection_system();
    let interfaces = |type_: &Rc<dyn IXamlType>| -> Vec<String> {
        type_.get_all_interfaces().iter().map(|i| i.full_name()).collect()
    };
    // `Points : FerroList<Point>`, with the registered cast of a collection to its list.
    let points = ts.find_type("FerroUI.Points").expect("the collection");
    let point_list = ts.resolve(ValueType::of::<ferroui_base::collections::FerroList<ferroui_base::Point>>());
    assert!(point_list.is_assignable_from(&*points));
    let implemented = interfaces(&points);
    for expected in [
        "System.Collections.Generic.IList`1[FerroUI.Point]",
        "System.Collections.Generic.IEnumerable`1[FerroUI.Point]",
        "System.Collections.Generic.IReadOnlyList`1[FerroUI.Point]",
        "System.Collections.IList",
        "System.Collections.Specialized.INotifyCollectionChanged",
    ] {
        assert!(implemented.iter().any(|i| i == expected), "{expected} is missing from {implemented:?}");
    }

    // Without the cast the members of the list cannot be called on the collection: it is
    // not projected as a list.
    let detached = ts.find_type("RtProjection.Detached").expect("the collection");
    let string_list = ts.resolve(ValueType::of::<ferroui_base::collections::FerroList<String>>());
    assert!(!string_list.is_assignable_from(&*detached));
    assert!(!interfaces(&detached).iter().any(|i| i.starts_with("System.Collections.Generic.IEnumerable`1")));
}

#[test]
fn an_attached_property_is_a_styled_property_of_a_class_that_adds_itself_as_owner() {
    let ts = system();
    // `InputElement.TabIndexProperty = KeyboardNavigation.TabIndexProperty.AddOwner<InputElement>()`.
    let input_element = ts.find_type("FerroUI.Input.InputElement").expect("the class");
    let field = input_element.fields().into_iter().find(|f| f.name() == "TabIndexProperty").expect("the field");
    assert!(field.field_type().name().starts_with("StyledProperty"), "{}", field.field_type().full_name());
    assert!(input_element.properties().iter().any(|p| p.name() == "TabIndex"));
    let owner = ts.find_type("FerroUI.Input.KeyboardNavigation").expect("the owner");
    let field = owner.fields().into_iter().find(|f| f.name() == "TabIndexProperty").expect("the field");
    assert!(field.field_type().name().starts_with("AttachedProperty"), "{}", field.field_type().full_name());
}

#[test]
fn an_array_argument_of_an_attribute_is_an_array_value() {
    let ts = projection_system();
    let declared = ts.find_type("RtProjection.Declared").expect("the type");
    let attributes = declared.custom_attributes();
    let list = attributes.iter().find(|a| a.type_().name() == "FerroListAttribute").expect("the attribute");
    assert_eq!(
        list.properties().get("Separators"),
        Some(&XamlValue::Array(vec![XamlValue::String(",".to_string()), XamlValue::String(" ".to_string())]))
    );
    let sample = attributes.iter().find(|a| a.type_().name() == "SampleAttribute").expect("the attribute");
    assert_eq!(
        sample.parameters(),
        [XamlValue::Array(vec![
            XamlValue::Int32(1),
            XamlValue::Array(vec![XamlValue::String("a".to_string()), XamlValue::Null]),
        ])]
    );
    match sample.properties().get("Types") {
        Some(XamlValue::Array(types)) => match types.as_slice() {
            [XamlValue::Type(type_)] => assert!(type_.equals(&*declared)),
            other => panic!("unexpected items: {}", other.len()),
        },
        _ => panic!("the named argument is an array"),
    }
}

#[test]
fn a_declared_constructor_parameter_has_its_declared_attributes() {
    let ts = projection_system();
    let declared = ts.find_type("RtProjection.Declared").expect("the type");
    let constructors = declared.constructors();
    let named = constructors.iter().find(|c| c.parameters().len() == 2).expect("the constructor");
    let name = named.get_parameter_info(0).expect("the parameter");
    assert_eq!(attribute_names(name.custom_attributes()), ["FerroUI.Metadata.InheritDataTypeFromAttribute"]);
    // The declared attributes, not the ones of the constructor argument property.
    assert_eq!(name.custom_attributes()[0].parameters(), [XamlValue::Int32(1)]);
    assert!(named.get_parameter_info(1).expect("the parameter").custom_attributes().is_empty());
    assert!(named.get_parameter_info(2).is_err());

    // The positional form states no parameter attributes.
    let positional = constructors.iter().find(|c| c.parameters().len() == 1).expect("the constructor");
    assert!(positional.get_parameter_info(0).expect("the parameter").custom_attributes().is_empty());
}

#[test]
fn a_static_property_has_static_accessors_and_no_field() {
    let ts = projection_system();
    let declared = ts.find_type("RtProjection.Declared").expect("the type");

    let read_only = declared.properties().into_iter().find(|p| p.name() == "ReadOnly").expect("the property");
    let getter = read_only.getter().expect("the getter");
    assert!(getter.is_static() && getter.parameters().is_empty());
    assert!(read_only.setter().is_none());
    assert_eq!(read_only.property_type().full_name(), "System.String");
    assert_eq!(attribute_names(read_only.custom_attributes()), ["FerroUI.Metadata.UnstableAttribute"]);
    assert!(!declared.fields().iter().any(|f| f.name() == "ReadOnly"));

    let current = declared.properties().into_iter().find(|p| p.name() == "Current").expect("the property");
    let setter = current.setter().expect("the setter");
    assert!(setter.is_static());
    assert_eq!(setter.parameters().len(), 1);
    method(&declared, "set_Current").invoke(&[boxed("changed".to_string())]).expect("set");
    let value = method(&declared, "get_Current").invoke(&[]).expect("get");
    assert_eq!(from_markup_value::<String>(&value).as_deref(), Some("changed"));

    // A type that declares static properties has made the distinction: its fields are
    // fields only.
    let constant = declared.fields().into_iter().find(|f| f.name() == "Constant").expect("the field");
    assert!(constant.is_static());
    assert!(!declared.properties().iter().any(|p| p.name() == "Constant"));
}
