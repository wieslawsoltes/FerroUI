//! Ports of `BasicCompilerTests.cs`, `ContentAttributeTests.cs`,
//! `ListTests.cs`, `DictionaryTests.cs`, `IntrinsicsTests.cs`,
//! `InitializationTests.cs`, `DeferredContentTests.cs`, `ConvertersTests.cs`,
//! `SpecialPropertiesTests.cs` and `GenericTypeWithPropertyElement.cs`.

use std::rc::Rc;

use ferroui_base::metadata::IServiceProvider;

use crate::runtime::type_system::DeferredContentFactory;

use super::classes::*;
use super::{boxed, get, HostOptions, TestHost, TestServiceProvider, X};

fn text(value: &Object) -> String {
    get::<String>(value)
}

// --- BasicCompilerTests ------------------------------------------------------

const SIMPLE: &str = "
<SimpleClass xmlns='rt-test' Test='123'>
    <SimpleSubClass Test='test'/>
    <SimpleClass.Test2>321</SimpleClass.Test2>
    <SimpleSubClass Test='test2'/>
</SimpleClass>";

fn assert_simple(res: &Rc<SimpleClass>) {
    assert_eq!(res.test.borrow().as_deref(), Some("123"));
    assert_eq!(res.test2.borrow().as_deref(), Some("321"));
    let children = res.children.items();
    assert_eq!(children[0].test.borrow().as_deref(), Some("test"));
    assert_eq!(children[1].test.borrow().as_deref(), Some("test2"));
}

#[test]
fn compiler_should_compile_simple_xaml_build() {
    let host = TestHost::new();
    assert_simple(&host.run::<Rc<SimpleClass>>(SIMPLE));
}

#[test]
fn compiler_should_compile_simple_xaml_populate() {
    let host = TestHost::new();
    let res = Rc::new(SimpleClass::default());
    host.populate(SIMPLE, None, &boxed(res.clone())).expect("populate");
    assert_simple(&res);
}

#[test]
fn compiler_should_compile_xaml_with_i_add_child() {
    let host = TestHost::new();
    let xaml = format!("<ObjectWithAddChild xmlns='rt-test' xmlns:x='{X}'>123</ObjectWithAddChild>");
    let res = host.run::<Rc<ObjectWithAddChild>>(&xaml);
    assert_eq!(text(&res.child.borrow()), "123");
    // Populating the built object again, as upstream does.
    host.populate(&xaml, None, &boxed(res.clone())).expect("populate");
    assert_eq!(text(&res.child.borrow()), "123");
}

#[test]
fn compiler_should_compile_xaml_with_generic_i_add_child() {
    let host = TestHost::new();
    let res = host.run::<Rc<ObjectWithGenericAddChild>>(&format!(
        "<ObjectWithGenericAddChild xmlns='rt-test' xmlns:x='{X}'>123</ObjectWithGenericAddChild>"
    ));
    assert!(res.child.borrow().is_none());
    assert_eq!(res.text.borrow().as_deref(), Some("123"));
}

#[test]
fn compiler_should_populate_xaml_without_matching_ctor() {
    let host = TestHost::new();
    let res = Rc::new(ObjectWithoutMatchingCtor { arg: Some("123".to_string()).into(), prop: None.into() });
    host.populate("<ObjectWithoutMatchingCtor xmlns='rt-test' Prop='321' />", None, &boxed(res.clone())).expect("populate");
    assert_eq!(res.arg.borrow().as_deref(), Some("123"));
    assert_eq!(res.prop.borrow().as_deref(), Some("321"));
}

#[test]
fn compiler_should_fail_to_build_xaml_without_matching_ctor() {
    let host = TestHost::new();
    let error = host.error("<ObjectWithoutMatchingCtor xmlns='rt-test' Prop='321' />", None);
    assert_eq!(error.type_name(), "InvalidOperationException", "{}", error.message());
}

// --- ContentAttributeTests, ListTests, DictionaryTests -----------------------

#[test]
fn compiler_should_support_content_attribute() {
    let host = TestHost::new();
    let res = host.run::<Rc<RtClassWithContentAttribute>>(&format!(
        "<RtClassWithContentAttribute xmlns='rt-test' xmlns:x='{X}'>123</RtClassWithContentAttribute>"
    ));
    assert_eq!(res.text.borrow().as_deref(), Some("123"));
}

#[test]
fn compiler_should_support_content_attribute_override() {
    let host = TestHost::new();
    let res = host.run::<Rc<RtSubClassWithContentAttributeOverride>>(&format!(
        "<RtSubClassWithContentAttributeOverride xmlns='rt-test' xmlns:x='{X}'>123</RtSubClassWithContentAttributeOverride>"
    ));
    assert!(res.base.text.borrow().is_none());
    assert_eq!(res.other_text.borrow().as_deref(), Some("123"));
}

#[test]
fn compiler_should_fail_to_build_xaml_when_multiple_content_attributes_defined() {
    let host = TestHost::new();
    let error = host.error(
        "<RtClassWithTwoContentAttributes xmlns='rt-test'>123</RtClassWithTwoContentAttributes>",
        None,
    );
    assert!(error.message().contains("Content attribute is declared on multiple properties"), "{}", error.message());
}

#[test]
fn enumerable_properties_should_be_treated_as_lists() {
    let host = TestHost::new();
    let res = host.run::<Rc<EnumerableContentClass>>(
        "
<EnumerableContentClass xmlns='rt-test'>
    <SimpleSubClass Test='test'/>
    <SimpleSubClass Test='test2'/>
</EnumerableContentClass>",
    );
    let children = res.children.borrow().0.items();
    assert_eq!(children.first().unwrap().test.borrow().as_deref(), Some("test"));
    assert_eq!(children.last().unwrap().test.borrow().as_deref(), Some("test2"));
}

#[test]
fn compiler_should_be_able_to_populate_dictionary_content() {
    let host = TestHost::new();
    let res = host.run::<Rc<SimpleClassWithDictionaryContent>>(&format!(
        "
<SimpleClassWithDictionaryContent xmlns='rt-test'  xmlns:x='{X}'>
    <SimpleClassWithDictionaryContent Test='123' x:Key='test'/>
    <SimpleClassWithDictionaryContent Test='321' x:Key='{{x:Type SimpleClassWithDictionaryContent}}'/>
    <SimpleClassWithDictionaryContent.NonContentChildren>
        <SimpleClassWithDictionaryContent Test='ch' x:Key='test2'/>
    </SimpleClassWithDictionaryContent.NonContentChildren>
</SimpleClassWithDictionaryContent>"
    ));
    let test_of = |value: &Object| get::<Rc<SimpleClassWithDictionaryContent>>(value).test.borrow().clone();
    let children = res.children.0.borrow();
    assert_eq!(children.len(), 2);
    assert_eq!(text(&children[0].0), "test");
    assert_eq!(test_of(&children[0].1).as_deref(), Some("123"));
    // An untyped target receives a type that is not a class of the object model
    // as the value type of its canonical handle.
    let key = get::<ferroui_base::data::core::ValueType>(&children[1].0);
    assert!(key.name().contains("SimpleClassWithDictionaryContent"), "{}", key.name());
    let expected = host.ts.get("RtXamlParserTests.SimpleClassWithDictionaryContent");
    let expected = expected.as_any().downcast_ref::<crate::runtime::type_system::RuntimeType>().unwrap().handle();
    assert_eq!(Some(key), expected);
    assert_eq!(test_of(&children[1].1).as_deref(), Some("321"));
    let other = res.non_content_children.0.borrow();
    assert_eq!(other.len(), 1);
    assert_eq!(text(&other[0].0), "test2");
    assert_eq!(test_of(&other[0].1).as_deref(), Some("ch"));
}

// --- IntrinsicsTests ---------------------------------------------------------

fn intrinsics(body: &str) -> String {
    format!("<IntrinsicsTestsClass xmlns='rt-test' xmlns:x='{X}'>{body}</IntrinsicsTestsClass>")
}

#[test]
fn null_extension_should_be_operational() {
    let host = TestHost::new();
    let res = host.run::<Rc<IntrinsicsTestsClass>>(&intrinsics(
        "<IntrinsicsTestsClass.ObjectProperty><x:Null/></IntrinsicsTestsClass.ObjectProperty>",
    ));
    assert!(res.object_property.borrow().is_none());
}

#[test]
fn null_extension_should_cause_compilation_error_when_applied_to_value_type() {
    let host = TestHost::new();
    let error =
        host.error(&intrinsics("<IntrinsicsTestsClass.IntProperty><x:Null/></IntrinsicsTestsClass.IntProperty>"), None);
    assert_eq!(error.type_name(), "XamlLoadException", "{}", error.message());
}

#[test]
fn null_extension_should_disregard_value_type_overloads() {
    let host = TestHost::new();
    let res = host.run::<Rc<IntrinsicsListTestsClass>>(&format!(
        "<IntrinsicsListTestsClass xmlns='rt-test' xmlns:x='{X}'><x:Null /><x:Null /></IntrinsicsListTestsClass>"
    ));
    assert_eq!(res.add_int32_call_count.get(), 0);
    assert_eq!(res.add_object_call_count.get(), 2);
}

#[test]
fn type_extension_resolves_types() {
    let host = TestHost::new();
    let res = host.run::<Rc<IntrinsicsTestsClass>>(&intrinsics(
        "<IntrinsicsTestsClass.TypeProperty><x:Type TypeName='IntrinsicsTestsClass' /></IntrinsicsTestsClass.TypeProperty>",
    ));
    let type_ = res.type_property.borrow().clone().expect("a type");
    assert_eq!(type_.type_().full_name(), "RtXamlParserTests.IntrinsicsTestsClass");
    assert!(type_.markup().is_some());
}

fn static_value(member: &str) -> Object {
    let host = TestHost::new();
    let res = host.run::<Rc<IntrinsicsTestsClass>>(&intrinsics(&format!(
        "<IntrinsicsTestsClass.ObjectProperty><x:Static Member='{member}'/></IntrinsicsTestsClass.ObjectProperty>"
    )));
    let value = res.object_property.borrow().clone();
    value
}

#[test]
fn static_extension_resolves_values() {
    assert_eq!(text(&static_value("IntrinsicsTestsClass.StaticProp")), "StaticPropValue");
    assert_eq!(text(&static_value("IntrinsicsTestsClass.StaticField")), "StaticFieldValue");
    assert_eq!(text(&static_value("IntrinsicsTestsClass.StringConstant")), "ConstantValue");
    assert_eq!(get::<i32>(&static_value("IntrinsicsTestsClass.IntConstant")), 100);
    assert_eq!(get::<f32>(&static_value("IntrinsicsTestsClass.FloatConstant")), 2.0);
    assert_eq!(get::<f64>(&static_value("IntrinsicsTestsClass.DoubleConstant")), 3.0);
    assert_eq!(text(&static_value("IntrinsicsTestsDerivedClass.StaticProp")), "StaticPropValue");
    assert_eq!(text(&static_value("IntrinsicsTestsDerivedClass.StaticField")), "StaticFieldValue");
}

#[test]
fn static_extension_resolves_enum_values() {
    assert_eq!(get::<IntrinsicsTestsEnum>(&static_value("IntrinsicsTestsEnum.Foo")), IntrinsicsTestsEnum::Foo);
}

#[test]
fn static_extension_reports_errors() {
    let host = TestHost::new();
    let error = host.error(
        &intrinsics(
            "<IntrinsicsTestsClass.ObjectProperty><x:Static Member='IntrinsicsTestsClass.StaticPropDoesntExist1'/></IntrinsicsTestsClass.ObjectProperty>
             <IntrinsicsTestsClass.BoolProperty><x:Static Member='IntrinsicsTestsClass.StaticPropDoesntExist2'/></IntrinsicsTestsClass.BoolProperty>",
        ),
        None,
    );
    let xamlx::exceptions::XamlError::Aggregate(errors) = &error else {
        panic!("expected an aggregate, got {}: {}", error.type_name(), error.message());
    };
    assert_eq!(errors.len(), 2);
    assert!(errors[0].message().contains("StaticPropDoesntExist1"), "{}", errors[0].message());
    assert!(errors[1].message().contains("StaticPropDoesntExist2"), "{}", errors[1].message());
}

#[test]
fn boolean_extension_can_be_set() {
    for (expected, value) in [(true, "x:True"), (false, "x:False")] {
        let host = TestHost::new();
        let res = host.run::<Rc<IntrinsicsTestsClass>>(&intrinsics(&format!(
            "<IntrinsicsTestsClass.ObjectProperty><{value}/></IntrinsicsTestsClass.ObjectProperty>
             <IntrinsicsTestsClass.BoolProperty><{value}/></IntrinsicsTestsClass.BoolProperty>
             <IntrinsicsTestsClass.NullableBoolProperty><{value}/></IntrinsicsTestsClass.NullableBoolProperty>"
        )));
        assert_eq!(get::<bool>(&res.object_property.borrow()), expected);
        assert_eq!(*res.bool_property.borrow(), expected);
        assert_eq!(*res.nullable_bool_property.borrow(), Some(expected));

        // Used as a markup extension.
        let res = host.run::<Rc<IntrinsicsTestsClass>>(&format!(
            "<IntrinsicsTestsClass xmlns='rt-test' xmlns:x='{X}' ObjectProperty='{{{value}}}' />"
        ));
        assert_eq!(get::<bool>(&res.object_property.borrow()), expected);
    }
}

#[test]
fn boolean_extension_should_cause_compilation_error_when_applied_to_wrong_type() {
    let host = TestHost::new();
    let error = host.error(&format!("<IntrinsicsTestsClass xmlns='rt-test' xmlns:x='{X}' IntProperty='{{x:True}}' />"), None);
    assert_eq!(error.type_name(), "XamlLoadException", "{}", error.message());
}

// --- InitializationTests -----------------------------------------------------

fn initialization_events(xaml: &str) -> Vec<String> {
    reset_initialization();
    let host = TestHost::new();
    host.build(xaml, None).unwrap_or_else(|e| panic!("{}", e.message()));
    INIT_EVENTS.with(|e| e.borrow().clone())
}

#[test]
fn initialization_events_should_be_triggered_for_supports_initialize() {
    let events = initialization_events(
        "
<InitializationTestsSupportInitializeClass xmlns='rt-test' Property='123'>
    <InitializationTestsSupportInitializeClass.Child>
        <InitializationTestsSupportInitializeClass Property='321'/>
    </InitializationTestsSupportInitializeClass.Child>
    <InitializationTestsSupportInitializeClass Property='321'/>
</InitializationTestsSupportInitializeClass>",
    );
    assert_eq!(
        events,
        [
            "1:BeginInit",
            "1:PropertySet",
            "2:BeginInit",
            "2:PropertySet",
            "2:EndInit",
            "1:ChildAdded:2",
            "3:BeginInit",
            "3:PropertySet",
            "3:EndInit",
            "1:ChildAdded:3",
            "1:EndInit"
        ]
    );
}

#[test]
fn usable_during_initialization_should_revert_initialization_order() {
    let events = initialization_events(
        "
<InitializationTestsTopDownClass xmlns='rt-test' Property='123'>
    <InitializationTestsTopDownClass.Child>
        <InitializationTestsTopDownClass Property='321'/>
    </InitializationTestsTopDownClass.Child>
    <InitializationTestsTopDownClass Property='321'/>
</InitializationTestsTopDownClass>",
    );
    assert_eq!(
        events,
        [
            "1:BeginInit",
            "1:PropertySet",
            "2:BeginInit",
            "1:ChildAdded:2",
            "2:PropertySet",
            "2:EndInit",
            "3:BeginInit",
            "1:ChildAdded:3",
            "3:PropertySet",
            "3:EndInit",
            "1:EndInit"
        ]
    );
}

// --- DeferredContentTests ----------------------------------------------------

fn callback_provider(callback: Callback) -> Option<Rc<dyn IServiceProvider>> {
    Some(TestServiceProvider::new().with(callback))
}

fn root_object_callback() -> Callback {
    use crate::runtime::interpreter::services::IRootObjectProvider;
    Callback(Rc::new(|provider| {
        provider.get_service_of::<Rc<dyn IRootObjectProvider>>().expect("root object provider").root_object()
    }))
}

#[test]
fn deferred_content_should_generate_delegate_in_the_target_property() {
    let host = TestHost::new();
    let res = host.run::<Rc<DeferredContentTestsClass>>(
        "
<DeferredContentTestsClass xmlns='rt-test' ObjectProperty='123'>
    <DeferredContentTestsClass ObjectProperty='321'/>
</DeferredContentTestsClass>",
    );
    assert_eq!(text(&res.object_property.borrow()), "123");
    let factory = get::<DeferredContentFactory>(&res.deferred_content.borrow());
    let e1 = get::<Rc<DeferredContentTestsClass>>(&factory.invoke(None).expect("build"));
    assert_eq!(text(&e1.object_property.borrow()), "321");
    let e2 = get::<Rc<DeferredContentTestsClass>>(&factory.invoke(None).expect("build"));
    assert_eq!(text(&e2.object_property.borrow()), "321");
    assert!(!Rc::ptr_eq(&e1, &e2));
}

#[test]
fn deferred_content_delegate_should_be_transformed_when_configured() {
    let host = TestHost::with_options(HostOptions {
        deferred_customization: Some("DelegateCustomizer"),
        ..HostOptions::default()
    });
    let res = host.run_with::<Rc<DeferredContentTestsClass>>(
        "
<DeferredContentTestsClass xmlns='rt-test'>
    <DeferredContentTestsClass ObjectProperty='{Callback}'/>
</DeferredContentTestsClass>",
        callback_provider(root_object_callback()),
    );
    let factory = get::<DeferredContentFactory>(&res.deferred_content.borrow());
    let generated = get::<Rc<DeferredContentTestsClass>>(&factory.invoke(None).expect("build"));
    let root = get::<Rc<DeferredContentTestsClass>>(&generated.object_property.borrow());
    assert!(Rc::ptr_eq(&res, &root));
}

#[test]
fn deferred_content_delegate_should_be_transformed_with_changed_return_type_when_configured() {
    let host = TestHost::with_options(HostOptions {
        deferred_customization: Some("CustomizerWithChangedReturnType"),
        ..HostOptions::default()
    });
    let res = host.run_with::<Rc<DeferredContentTestsClass>>(
        "
<DeferredContentTestsClass xmlns='rt-test'>
    <DeferredContentTestsClass ObjectProperty='abc'/>
</DeferredContentTestsClass>",
        callback_provider(root_object_callback()),
    );
    let deferred = get::<Rc<DeferredValue>>(&res.deferred_content.borrow());
    let content = get::<Rc<DeferredContentTestsClass>>(&deferred.original_factory.invoke(None).expect("build"));
    assert_eq!(text(&content.object_property.borrow()), "abc");
}

// --- ConvertersTests ---------------------------------------------------------

fn converted(property: &str, value: &str) -> Rc<ConvertersTestClass> {
    let host = TestHost::new();
    host.run::<Rc<ConvertersTestClass>>(&format!(
        "
<ConvertersTestClass
    xmlns='rt-test'
    xmlns:x='{X}'
    xmlns:scg='clr-namespace:System.Collections.Generic' {property}='{value}'
/>"
    ))
}

#[test]
fn converters_are_operational() {
    assert_eq!(*converted("Int64Property", "1").int64_property.borrow(), 1);
    assert!(*converted("BoolProperty", "True").bool_property.borrow());
    assert_eq!(*converted("DoubleProperty", "1.5").double_property.borrow(), 1.5);
    assert_eq!(*converted("FloatProperty", "2.5").float_property.borrow(), 2.5);
    assert_eq!(converted("CustomProperty", "Custom").custom_property.borrow().value, "Custom");
    assert_eq!(converted("TimeSpanProperty", "01:10:00").time_span_property.borrow().to_string(), "01:10:00");
    let res = converted("TypeWithConverterProperty", "CustomConverter");
    let value = res.type_with_converter_property.borrow().clone().expect("converted");
    assert_eq!(value.value.borrow().as_deref(), Some("CustomConverter"));
    let res = converted("PropertyWithConverter", "CustomConverterProperty");
    let value = res.property_with_converter.borrow().clone().expect("converted");
    assert_eq!(value.value.borrow().as_deref(), Some("CustomConverterProperty"));
    assert_eq!(*converted("UriKindProperty", "Relative").uri_kind_property.borrow(), RtUriKind::Relative);
    assert_eq!(*converted("EnumProperty", "Second").enum_property.borrow(), ConvertersTestsEnum::SECOND);
    assert_eq!(
        *converted("EnumProperty", "First, Third").enum_property.borrow(),
        ConvertersTestsEnum::FIRST | ConvertersTestsEnum::THIRD
    );
    // Numeric text is the value of the enumeration.
    assert_eq!(*converted("EnumProperty", "6").enum_property.borrow(), ConvertersTestsEnum::SECOND | ConvertersTestsEnum::THIRD);
    assert_eq!(*converted("UriKindProperty", "2").uri_kind_property.borrow(), RtUriKind::Relative);
    assert_eq!(*converted("IntPropertyWithNegativeConverter", "5").int_property_with_negative_converter.borrow(), -5);
}

/// Upstream converts `EnumProperty='100500'` and `UriKindProperty='150'` to enumeration values
/// without a member, which a Rust enumeration cannot hold: such text is an error naming the type.
#[test]
fn numeric_enum_text_without_a_value_is_an_error() {
    for (property, value) in [("EnumProperty", "100500"), ("UriKindProperty", "150")] {
        let host = TestHost::new();
        let error = host.error(
            &format!("<ConvertersTestClass xmlns='rt-test' {property}='{value}' />"),
            None,
        );
        assert_eq!(error.type_name(), "XamlLoadException", "{}", error.message());
        assert!(error.message().contains(value), "{}", error.message());
    }
}

#[test]
fn type_properties_are_converted() {
    let res = converted("TypeProperty", "ConvertersTestClass");
    let type_ = res.type_property.borrow().clone().expect("a type");
    assert_eq!(type_.to_string(), "RtXamlParserTests.ConvertersTestClass");
}

#[test]
fn primitive_types_are_properly_parsed() {
    use ferroui_base::animation::TimeSpan;
    let host = TestHost::new();
    let parsed = |type_: &str, value: &str| -> Object {
        let res = host.run::<Rc<ConvertersTestClass>>(&format!(
            "
<ConvertersTestClass
    xmlns='rt-test'
    xmlns:x='{X}'
    xmlns:sys='clr-namespace:System;assembly=netstandard'>
    <{type_}>{value}</{type_}>
</ConvertersTestClass>"
        ));
        let content = res.content_property.borrow().clone();
        content
    };
    for prefix in ["x", "sys"] {
        assert_eq!(get::<i32>(&parsed(&format!("{prefix}:Int32"), "1")), 1);
        assert_eq!(get::<f64>(&parsed(&format!("{prefix}:Double"), "1.5")), 1.5);
        assert_eq!(get::<f32>(&parsed(&format!("{prefix}:Single"), "2.5")), 2.5);
        assert_eq!(text(&parsed(&format!("{prefix}:String"), "Some string")), "Some string");
        assert_eq!(get::<TimeSpan>(&parsed(&format!("{prefix}:TimeSpan"), "01:10:00")).to_string(), "01:10:00");
        assert_eq!(get::<char>(&parsed(&format!("{prefix}:Char"), "x")), 'x');
    }
}

#[test]
fn constructor_parameters_should_be_converted() {
    let host = TestHost::new();
    let res = host.run::<Rc<ConvertersTestsClassWithConstructor>>(&format!(
        "
<ConvertersTestsClassWithConstructor
    xmlns='rt-test'
    xmlns:x='{X}'
    xmlns:sys='clr-namespace:System;assembly=netstandard'>
    <x:Arguments>
        <sys:String>123</sys:String>
        <sys:String>01:10:00</sys:String>
    </x:Arguments>
</ConvertersTestsClassWithConstructor>"
    ));
    assert_eq!(res.int, 123);
    assert_eq!(res.converted.to_string(), "01:10:00");
}

// --- SpecialPropertiesTests, GenericTypeWithPropertyElement ------------------

#[test]
fn init_properties_should_be_set() {
    let host = TestHost::new();
    let res = host.run::<Rc<InitPropertiesTestClass>>(
        "<InitPropertiesTestClass xmlns='clr-namespace:RtXamlParserTests' Prop1='foo' Prop2='42' />",
    );
    assert_eq!(res.prop1.borrow().as_deref(), Some("foo"));
    assert_eq!(*res.prop2.borrow(), 42);
}

#[test]
fn generic_type_with_property_element() {
    let host = TestHost::new();
    let res = host.run::<Rc<RootNode>>(&format!(
        "
    <RootNode xmlns='rt-test' xmlns:x='{X}'>
        <GenericClass x:TypeArguments='TypeArgument'>
            <GenericClass.Items>
                <ItemNode />
            </GenericClass.Items>
        </GenericClass>
    </RootNode>"
    ));
    let children = res.children.items();
    assert_eq!(children.len(), 1);
    let typed_child = get::<Rc<GenericClassOfTypeArgument>>(&children[0]);
    let items = typed_child.items.items();
    assert_eq!(items.len(), 1);
    let _ = get::<Rc<ItemNode>>(&items[0]);
}
