//! Tests of the type system helpers on the fake type system.

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::XamlLineInfo;
use crate::exceptions::XamlError;
use crate::transform::{NamespaceInfoHelper, XamlTransformHelpers};
use crate::type_system::*;

use super::test_xaml_language::{TestHost, NS};

fn list_of(host: &TestHost, item: Rc<dyn IXamlType>) -> Rc<dyn IXamlType> {
    host.ts
        .get("System.Collections.Generic.List`1")
        .make_generic_type(&[item])
        .expect("List<T>")
}

#[test]
fn assignability_follows_the_metadata_rules() {
    let host = TestHost::new();
    let object = host.ts.get("System.Object");
    let int32 = host.ts.get("System.Int32");
    let control = host.t("Control");
    let content_control = host.t("ContentControl");
    let enumerable = host.ts.get("System.Collections.IEnumerable");
    let nullable_int = host
        .ts
        .get("System.Nullable`1")
        .make_generic_type(&[int32.clone()])
        .expect("Nullable<int>");

    assert!(control.is_assignable_from(&*content_control));
    assert!(!content_control.is_assignable_from(&*control));
    assert!(object.is_assignable_from(&*control));
    assert!(object.is_assignable_from(&*int32));
    assert!(object.is_assignable_from(&*enumerable));
    assert!(enumerable.is_assignable_from(&*list_of(&host, control.clone())));
    assert!(!enumerable.is_assignable_from(&*control));

    // x:Null
    assert!(control.is_assignable_from(&*XamlPseudoType::null()));
    assert!(!int32.is_assignable_from(&*XamlPseudoType::null()));
    assert!(nullable_int.is_assignable_from(&*XamlPseudoType::null()));
    assert!(nullable_int.is_assignable_from(&*int32));
    assert!(nullable_int.is_nullable_of(&*int32));
    assert!(nullable_int.accepts_null() && control.accepts_null() && !int32.accepts_null());

    assert!(int32.is_directly_assignable_from(&*int32));
    assert!(!object.is_directly_assignable_from(&*int32));
}

#[test]
fn pseudo_types_use_reference_equality() {
    assert!(XamlPseudoType::null().equals(&*XamlPseudoType::null()));
    assert!(!XamlPseudoType::null().equals(&*XamlPseudoType::unknown()));
    assert!(XamlPseudoType::is_unknown(&*XamlPseudoType::unknown()));
    let unresolved = XamlPseudoType::unresolved("Foo");
    assert_eq!(unresolved.name(), "{Unresolved type: 'Foo'}");
    assert!(!unresolved.equals(&*XamlPseudoType::unresolved("Foo")));
    assert!(unresolved.make_generic_type(&[]).is_err());
}

#[test]
fn generic_types_substitute_members() {
    let host = TestHost::new();
    let control = host.t("Control");
    let list = list_of(&host, control.clone());
    assert!(list.equals(&*list_of(&host, control.clone())));
    assert_eq!(
        list.full_name(),
        format!("System.Collections.Generic.List`1[{NS}.Control]")
    );
    assert_eq!(
        list.get_fqn(),
        "System.Runtime:System.Collections.Generic.List`1"
    );
    assert!(list
        .generic_type_definition()
        .is_some_and(|d| d.equals(&*host.ts.get("System.Collections.Generic.List`1"))));

    let add = list.get_method(|m| m.name() == "Add").expect("Add");
    assert!(add.parameters()[0].equals(&*control));
    assert!(add.declaring_type().equals(&*list));
    assert!(add.this_or_first_parameter().expect("this").equals(&*list));
    assert_eq!(add.parameters_with_this().len(), 2);

    let interfaces: Vec<String> = list
        .get_all_interfaces()
        .iter()
        .map(|i| i.full_name())
        .collect();
    assert!(interfaces.contains(&format!("System.Collections.Generic.IList`1[{NS}.Control]")));
    assert!(interfaces.contains(&format!(
        "System.Collections.Generic.ICollection`1[{NS}.Control]"
    )));
    assert!(interfaces.contains(&"System.Collections.IEnumerable".to_string()));

    // Members inherited from a constructed base type
    let collection = host.t("InlineCollection");
    let adders = collection.find_methods(|m| m.name() == "Add");
    let parameters: Vec<String> = adders.iter().map(|m| m.parameters()[0].name()).collect();
    assert_eq!(parameters, vec!["String", "Inline"]);

    let mut map: HashMap<XamlTypeKey, i32> = HashMap::new();
    map.insert(XamlTypeKey(list.clone()), 1);
    assert_eq!(map.get(&XamlTypeKey(list_of(&host, control))), Some(&1));
}

#[test]
fn method_and_constructor_lookup() {
    let host = TestHost::new();
    let string = host.ts.get("System.String");
    let object = host.ts.get("System.Object");
    let int32 = host.ts.get("System.Int32");
    let void = host.ts.get("System.Void");
    let control = host.t("Control");
    let content_control = host.t("ContentControl");

    // Declared on the base type
    assert!(content_control
        .find_method_by_name("set_StrProp", &*void, false, &[string.clone()])
        .is_some());
    assert!(content_control
        .find_method_by_name("set_StrProp", &*void, false, &[object.clone()])
        .is_none());
    // allowDowncast uses assignability
    assert!(content_control
        .find_method_by_name("set_Content", &*void, true, &[string.clone()])
        .is_some());
    assert!(content_control
        .find_method_by_name("set_Content", &*void, false, &[string.clone()])
        .is_none());

    let mut signature =
        FindMethodMethodSignature::new("set_StrProp", void.clone(), vec![string.clone()]);
    assert!(content_control
        .find_method_by_signature(&signature)
        .is_some());
    signature.declaring_only = true;
    assert!(content_control
        .find_method_by_signature(&signature)
        .is_none());
    assert!(matches!(
        content_control.get_method_by_signature(&signature),
        Err(XamlError::TypeSystem(_))
    ));
    assert_eq!(
        signature.to_string(),
        "instance System.Void,System.Runtime set_StrProp (System.String,System.Runtime) (exact match: True, declaring only: True)"
    );

    assert!(control.find_constructor(None).is_some());
    let ctor_class = host.t("CtorClass");
    assert!(ctor_class.find_constructor(None).is_none());
    assert!(ctor_class
        .find_constructor(Some(&[int32.clone(), string.clone()]))
        .is_some());
    let error = ctor_class
        .get_constructor(Some(&[string.clone()]))
        .err()
        .expect("no such constructor");
    assert_eq!(
        error.message(),
        format!("Constructor with arguments System.String,System.Runtime is not found on type {NS}:{NS}.CtorClass")
    );
    assert!(matches!(
        get_type(&*host.ts, "Nope.Missing"),
        Err(XamlError::TypeSystem(_))
    ));

    assert!(XamlTransformHelpers::get_common_base_class(&[
        content_control.clone(),
        control.clone()
    ])
    .expect("common base")
    .equals(&*control));
    assert!(
        XamlTransformHelpers::get_common_base_class(&[content_control, string])
            .expect("common base")
            .equals(&*object)
    );
}

#[test]
fn constants_are_parsed_with_dotnet_rules() {
    let host = TestHost::new();
    let li = XamlLineInfo::new(3, 7);
    let parse = |type_name: &str, text: &str| {
        TypeSystemHelpers::parse_constant_if_type_allows(text, &host.ts.get(type_name), &li)
            .map(|node| node.map(|n| n.constant.clone()))
    };

    assert_eq!(
        parse("System.Int32", " -12 ").expect("ok"),
        Some(XamlValue::Int32(-12))
    );
    assert_eq!(
        parse("System.Byte", "255").expect("ok"),
        Some(XamlValue::Byte(255))
    );
    assert_eq!(
        parse("System.UInt64", "18446744073709551615").expect("ok"),
        Some(XamlValue::UInt64(u64::MAX))
    );
    assert_eq!(
        parse("System.Double", "1,000.5e1").expect("ok"),
        Some(XamlValue::Double(10005.0))
    );
    assert_eq!(
        parse("System.Single", ".5").expect("ok"),
        Some(XamlValue::Single(0.5))
    );
    assert_eq!(
        parse("System.Double", "-Infinity").expect("ok"),
        Some(XamlValue::Double(f64::NEG_INFINITY))
    );
    assert_eq!(
        parse("System.Boolean", " TRUE ").expect("ok"),
        Some(XamlValue::Boolean(true))
    );
    assert_eq!(
        parse("System.Char", "x").expect("ok"),
        Some(XamlValue::Char('x'))
    );
    assert_eq!(parse("System.String", "x").expect("ok"), None);
    assert_eq!(parse(&format!("{NS}.Control"), "x").expect("ok"), None);

    // FormatException becomes XamlParseException with line info
    for (type_name, text) in [
        ("System.Int32", "1.5"),
        ("System.Int32", ""),
        ("System.Double", "abc"),
        ("System.Double", "inf"),
        ("System.Boolean", "yes"),
        ("System.Char", "xy"),
    ] {
        match parse(type_name, text) {
            Err(XamlError::Parse(e)) => assert_eq!((e.line_number, e.line_position), (3, 7)),
            other => panic!("Expected a parse exception for {type_name} '{text}', got {other:?}"),
        }
    }

    // OverflowException is not converted
    for (type_name, text) in [
        ("System.Byte", "256"),
        ("System.Int32", "2147483648"),
        ("System.UInt32", "-1"),
    ] {
        match parse(type_name, text) {
            Err(XamlError::Internal(e)) => assert_eq!(e.type_name, "OverflowException"),
            other => panic!("Expected an overflow for {type_name} '{text}', got {other:?}"),
        }
    }
}

#[test]
fn enum_values_are_resolved() {
    let host = TestHost::new();
    let test_enum = host.t("TestEnum");
    let flags_enum = host.t("FlagsEnum");
    let value = |t: &Rc<dyn IXamlType>, s: &str, ignore_case: bool| {
        TypeSystemHelpers::try_get_enum_value(t, s, ignore_case).expect("no error")
    };

    assert_eq!(value(&test_enum, "B", false), Some(XamlValue::Int32(2)));
    assert_eq!(value(&test_enum, "b", false), None);
    assert_eq!(value(&test_enum, "b", true), Some(XamlValue::Int32(2)));
    assert_eq!(value(&test_enum, "17", false), Some(XamlValue::Int32(17)));
    assert_eq!(value(&test_enum, "A, B", false), None);
    assert_eq!(
        value(&flags_enum, "One, Four", false),
        Some(XamlValue::Int32(5))
    );
    assert_eq!(value(&flags_enum, "One,Missing", false), None);

    assert_eq!(
        TypeSystemHelpers::convert_literal_to_int(&XamlValue::UInt32(u32::MAX)).expect("ok"),
        -1
    );
    assert_eq!(
        TypeSystemHelpers::convert_literal_to_long(&XamlValue::UInt64(u64::MAX)).expect("ok"),
        -1
    );
    assert_eq!(
        TypeSystemHelpers::convert_literal_to_int(&XamlValue::Byte(7)).expect("ok"),
        7
    );
    assert!(TypeSystemHelpers::convert_literal_to_int(&XamlValue::Int64(i64::MAX)).is_err());
}

#[test]
fn xml_namespaces_are_resolved_to_clr_namespaces() {
    let host = TestHost::new();
    let config = &host.configuration;

    let mapped = NamespaceInfoHelper::try_resolve(config, Some("test")).expect("mapped namespace");
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].clr_namespace, NS);
    assert!(mapped[0].assembly.as_ref().is_some_and(|a| a.name() == NS));

    let clr = NamespaceInfoHelper::try_resolve(config, Some("clr-namespace:Some.Ns"))
        .expect("clr namespace");
    assert_eq!(
        (
            clr[0].clr_namespace.as_str(),
            clr[0].assembly_name.as_deref()
        ),
        ("Some.Ns", Some(NS))
    );

    let clr =
        NamespaceInfoHelper::try_resolve(config, Some("clr-namespace:Some.Ns;assembly= Other "))
            .expect("clr namespace");
    assert_eq!(
        (
            clr[0].clr_namespace.as_str(),
            clr[0].assembly_name.as_deref()
        ),
        ("Some.Ns", Some("Other"))
    );

    let using =
        NamespaceInfoHelper::try_resolve(config, Some("using:Some.Ns")).expect("using namespace");
    assert_eq!(
        (
            using[0].clr_namespace.as_str(),
            using[0].assembly_name.as_deref()
        ),
        ("Some.Ns", None)
    );

    assert!(NamespaceInfoHelper::try_resolve(config, Some("http://unknown")).is_none());
    assert!(NamespaceInfoHelper::try_resolve(config, None).is_none());
}

#[test]
fn locals_pool_reuses_released_locals() {
    struct Local(usize);
    impl IXamlLocal for Local {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    let host = TestHost::new();
    let int32 = host.ts.get("System.Int32");
    let string = host.ts.get("System.String");
    let counter = std::cell::Cell::new(0usize);
    let pool = XamlLocalsPool::new(move |_| {
        counter.set(counter.get() + 1);
        Rc::new(Local(counter.get()))
    });
    let id = |local: &PooledLocal| {
        local
            .local()
            .expect("live")
            .as_any()
            .downcast_ref::<Local>()
            .expect("Local")
            .0
    };

    let first = pool.get_local(&int32);
    let second = pool.get_local(&int32);
    assert_eq!((id(&first), id(&second)), (1, 2));
    drop(first);
    let other_type = pool.get_local(&string);
    assert_eq!(id(&other_type), 3);
    let mut reused = pool.get_local(&int32);
    assert_eq!(id(&reused), 1);
    reused.dispose();
    assert!(reused.local().is_err());
    assert_eq!(id(&pool.get_local(&int32)), 1);
}
