//! Tests of the context factory of compiled markup ([`super::compiled`]).
//! Not tests of upstream: upstream's compiler generates the context class;
//! these check that the factory gives generated code the context the
//! run-time loader's framework language creates.

use std::rc::Rc;

use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::BoxedValue;

use super::compiled::{create_context, populate_context, to_value, DocumentInfo, XmlNamespaceTable, FRAMEWORK_CONTEXT};
use super::{IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider, IFerroXamlIlXmlNamespaceInfoProvider};
use crate::test_support::{boxed, TestServiceProvider};
use crate::{IRootObjectProvider, IUriContext};

const NAMESPACES: XmlNamespaceTable =
    &[("", &[("FerroUI.Controls", "FerroUI.Controls"), ("FerroUI.Media", "FerroUI.Base")]), ("x", &[])];

fn values(list: &[BoxedValue]) -> Vec<i32> {
    list.iter().map(|value| *value.downcast_ref::<i32>().expect("an i32")).collect()
}

#[test]
fn the_context_has_every_service_of_the_framework_language() {
    let context = create_context(None, Some("ferres://Assembly/Views/Main.xaml"), NAMESPACES);
    assert_eq!(context.definition(), FRAMEWORK_CONTEXT);
    let provider: Rc<dyn IServiceProvider> = context.clone();
    assert!(provider.get_service_of::<Rc<dyn IRootObjectProvider>>().is_some());
    assert!(provider.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>().is_some());
    let uri_context = provider.get_service_of::<Rc<dyn IUriContext>>().expect("a URI context");
    assert_eq!(
        uri_context.base_uri().map(|uri| uri.original_string().to_string()).as_deref(),
        Some("ferres://Assembly/Views/Main.xaml")
    );
}

#[test]
fn the_namespace_information_is_the_table_of_the_document() {
    let context = create_context(None, None, NAMESPACES);
    let provider: Rc<dyn IServiceProvider> = context;
    let info = provider
        .get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>()
        .expect("the namespace information");
    let namespaces = info.xml_namespaces();
    assert_eq!(namespaces.len(), 2);
    let default: Vec<(String, String)> =
        namespaces[""].iter().map(|info| (info.clr_namespace(), info.clr_assembly_name())).collect();
    assert_eq!(
        default,
        [
            ("FerroUI.Controls".to_string(), "FerroUI.Controls".to_string()),
            ("FerroUI.Media".to_string(), "FerroUI.Base".to_string())
        ]
    );
    assert!(namespaces["x"].is_empty());
}

#[test]
fn compiled_namespace_scopes_shadow_restore_and_keep_captured_tables_stable() {
    use super::compiled::push_namespace_scope;
    let context = create_context(None, None, NAMESPACES);
    let provider: Rc<dyn IServiceProvider> = context.clone();
    let namespaces = || provider.get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>().unwrap().xml_namespaces();
    let outer = push_namespace_scope(&context, &[("p", &[("Outer.Views", "App")])]);
    let captured = namespaces();
    assert_eq!(captured["p"][0].clr_namespace(), "Outer.Views");
    let inner = push_namespace_scope(&context, &[("p", &[("Inner.Views", "App")])]);
    assert_eq!(namespaces()["p"][0].clr_namespace(), "Inner.Views");
    // Out-of-order cleanup must not expose a dropped outer scope underneath the active one.
    drop(outer);
    assert_eq!(namespaces()["p"][0].clr_namespace(), "Inner.Views");
    drop(inner);
    assert!(!namespaces().contains_key("p"));
    assert!(namespaces().contains_key(""));
    assert_eq!(captured["p"][0].clr_namespace(), "Outer.Views");
    let failed = (|| -> Result<(), ()> {
        let _guard = push_namespace_scope(&context, &[("p", &[("Failed.Views", "App")])]);
        Err(())
    })();
    assert!(failed.is_err());
    assert!(!namespaces().contains_key("p"));
}

#[test]
fn emitted_deferred_context_uses_declaration_namespaces_after_parent_scope_ends() {
    use super::compiled::{deferred_context, push_namespace_scope};
    let context = create_context(None, None, NAMESPACES);
    let declaration: XmlNamespaceTable = &[("local", &[("Inner.Views", "App")])];
    let guard = push_namespace_scope(&context, declaration);
    let provider: Rc<dyn IServiceProvider> = context;
    drop(guard);
    let deferred = deferred_context(&provider, &DocumentInfo {
        base_uri: Some("ferres://App/Template.xaml"), namespaces: declaration,
    });
    let deferred_provider: Rc<dyn IServiceProvider> = deferred;
    let info = deferred_provider.get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>().unwrap();
    assert_eq!(info.xml_namespaces()["local"][0].clr_namespace(), "Inner.Views");
    assert!(!provider.get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>().unwrap().xml_namespaces().contains_key("local"));
}

#[test]
fn the_name_scope_field_is_the_scope_of_the_parent() {
    let scope = NameScopeRef::new(NameScope::new());
    let parent = TestServiceProvider::new().with_name_scope(scope.clone()).sp();
    let context = create_context(Some(parent), None, NAMESPACES);
    let field = context.name_scope_field().expect("the name scope field");
    assert!(Rc::ptr_eq(&field, &scope.0));

    let orphan = create_context(None, None, NAMESPACES);
    assert!(orphan.name_scope_field().is_none());
}

#[test]
fn the_parent_stack_is_the_pushed_objects_then_the_parents_of_the_parent() {
    // The parents of the parent provider, the outermost first: 10 holds 11.
    let parent = TestServiceProvider::new().with_parents(vec![boxed(10i32), boxed(11i32)]).sp();
    let context = create_context(Some(parent), None, NAMESPACES);
    context.push_parent(to_value(1i32));
    context.push_parent(to_value(2i32));
    let provider: Rc<dyn IServiceProvider> = context.clone();
    let stack = provider.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>().expect("a parent stack");
    assert_eq!(values(&stack.parents()), [2, 1, 11, 10]);
    let eager: Rc<dyn IFerroXamlIlEagerParentStackProvider> = context.clone();
    assert_eq!(values(&eager.direct_parents_stack()), [1, 2]);
    context.pop_parent();
    assert_eq!(values(&stack.parents()), [1, 11, 10]);
}

#[test]
fn the_populate_context_has_the_root_object() {
    let context = populate_context(None, &DocumentInfo { base_uri: None, namespaces: NAMESPACES }, to_value(7i32));
    let provider: Rc<dyn IServiceProvider> = context;
    let root = provider.get_service_of::<Rc<dyn IRootObjectProvider>>().expect("a root object provider");
    assert_eq!(root.root_object().and_then(|root| root.downcast_ref::<i32>().copied()), Some(7));
}

#[test]
fn an_object_is_an_instance_of_its_class_and_its_bases() {
    use ferroui_base::StaticType;
    use ferroui_controls::{Button, Control, TextBlock};
    use super::compiled::{class_handle, is_instance};
    let button = to_value(Button::new());
    assert!(is_instance(&button, class_handle(<Button as StaticType>::TYPE)));
    assert!(is_instance(&button, class_handle(<Control as StaticType>::TYPE)));
    assert!(!is_instance(&button, class_handle(<TextBlock as StaticType>::TYPE)));
    assert!(!is_instance(&None, class_handle(<Button as StaticType>::TYPE)));
}

#[test]
fn a_value_in_a_nullable_form_is_an_instance_of_what_it_holds() {
    use super::compiled::{is_instance, markup_handle};
    use ferroui_base::media::{Brushes, IBrush};
    use ferroui_base::metadata::MarkupTyped;
    crate::register_types();
    // What `ResolveByName` provides for a property of type `Option<Rc<dyn IBrush>>`: the
    // element in the form the property stores.
    let brush: Rc<dyn IBrush> = Brushes::red();
    let stored: BoxedValue = Rc::new(Some(brush));
    let value = Some(stored);
    let contract = markup_handle(<dyn IBrush as MarkupTyped>::MARKUP);
    assert!(is_instance(&value, contract));
    // The null of the form is not an instance (`isinst` of null).
    let null: BoxedValue = Rc::new(Option::<Rc<dyn IBrush>>::None);
    assert!(!is_instance(&Some(null), contract));
}

#[test]
fn a_value_cast_to_a_handle_is_held_untyped() {
    use super::compiled::cast_untyped;
    use ferroui_base::data::core::ValueType;
    crate::register_types();
    // A value of a type with a registered cast: the value as the target, then untyped.
    let value = cast_untyped(Some(String::from("text")), ValueType::of::<String>(), 1, 2).expect("the text");
    assert_eq!(value.and_then(|value| value.downcast_ref::<String>().cloned()).as_deref(), Some("text"));
    let null = cast_untyped(Option::<String>::None, ValueType::of::<Option<String>>(), 1, 2).expect("null");
    assert!(null.is_none());
    let error = cast_untyped(1.5_f64, ValueType::of::<String>(), 3, 4).expect_err("a number is no text");
    assert!(error.message().starts_with("Unable to cast the value to "), "{}", error.message());
    assert!(error.message().ends_with("Line 3, position 4. (line 3 position 4)"), "{}", error.message());
}

#[test]
fn a_value_is_an_instance_of_the_types_it_is_assignable_to() {
    use ferroui_base::data::core::ValueType;
    use super::compiled::is_instance;
    let text = to_value(String::from("text"));
    assert!(is_instance(&text, ValueType::of::<String>()));
    assert!(!is_instance(&text, ValueType::of::<f64>()));
}

#[test]
fn an_untyped_value_converts_to_the_declared_type_or_fails_as_the_loader_does() {
    use super::compiled::exact;
    let value: Option<String> = exact(to_value(String::from("a")), "T.M", 0, 1, 2).expect("a string");
    assert_eq!(value.as_deref(), Some("a"));
    let null: Option<String> = exact(None, "T.M", 0, 1, 2).expect("null converts to a nullable");
    assert_eq!(null, None);
    let error = exact::<f64>(to_value(String::from("a")), "T.M", 0, 3, 4).expect_err("a string is no number");
    assert!(error.message().starts_with("T.M: "), "{}", error.message());
    assert!(error.message().ends_with("Line 3, position 4. (line 3 position 4)"), "{}", error.message());
}

#[test]
fn the_error_of_an_assignment_without_a_setter_names_the_type_of_the_value() {
    use super::compiled::no_setter;
    use ferroui_base::StaticType;
    use ferroui_controls::Button;
    // The type is named by the full name of its class.
    let error = no_setter("Background", &to_value(Button::new()), 1, 2);
    assert_eq!(
        error.message(),
        format!(
            "No setter of property Background accepts a value of type '{}' Line 1, position 2. (line 1 position 2)",
            <Button as StaticType>::TYPE.full_name()
        )
    );
    let error = no_setter("Background", &None, 1, 2);
    assert_eq!(error.message(), "No setter of property Background accepts null Line 1, position 2. (line 1 position 2)");
}

#[test]
fn enum_integer_constants_use_the_converter_and_preserve_failures_and_positions() {
    use super::compiled::{enum_value, CompiledLoadError};
    use ferroui_base::metadata::{MarkupEnumMember, MarkupType, MarkupTypeKind};
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct IntegerValue(i32);
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static OPEN: MarkupType = {
        let mut m = MarkupType::new("IntegerValue", MarkupTypeKind::Enum, "Fixture");
        m.enum_from_value = Some(|raw| {
            CALLS.fetch_add(1, Ordering::SeqCst);
            i32::try_from(raw)
                .ok()
                .map(|v| Rc::new(IntegerValue(v)) as BoxedValue)
        });
        m
    };
    static FALLBACK: MarkupType = {
        let mut m = MarkupType::new("ClosedValue", MarkupTypeKind::Enum, "Fixture");
        m.enum_members = &[MarkupEnumMember {
            name: "Zero",
            value: 0,
            get: || Rc::new(IntegerValue(0)),
            rust_variant: Some("Zero"),
        }];
        m
    };
    static REFUSAL: MarkupType = {
        let mut m = MarkupType::new("Refusal", MarkupTypeKind::Enum, "Fixture");
        m.enum_members = FALLBACK.enum_members;
        m.enum_from_value = Some(|_| None);
        m
    };
    static WRONG_TYPE: MarkupType = {
        let mut m = MarkupType::new("WrongType", MarkupTypeKind::Enum, "Fixture");
        m.enum_from_value = Some(|_| Some(Rc::new("wrong".to_string())));
        m
    };
    assert!(!OPEN.is_flags);
    for raw in [i32::MIN, -1, 0, 1, i32::MAX] {
        let before = CALLS.load(Ordering::SeqCst);
        assert_eq!(
            enum_value::<IntegerValue>(&OPEN, i64::from(raw), 5, 7).unwrap(),
            IntegerValue(raw)
        );
        assert_eq!(CALLS.load(Ordering::SeqCst), before + 1);
    }
    for (markup, raw) in [
        (&OPEN, i64::MIN),
        (&OPEN, i64::MAX),
        (&FALLBACK, -1),
        (&REFUSAL, 0),
    ] {
        let error = enum_value::<IntegerValue>(markup, raw, 5, 7).unwrap_err();
        let inner = error
            .inner_exception()
            .unwrap()
            .downcast_ref::<CompiledLoadError>()
            .unwrap();
        assert_eq!(inner.type_name(), "XamlLoadException");
        assert_eq!(inner.line_number(), 5);
        assert_eq!(inner.line_position(), 7);
        assert!(error.message().contains(&format!(
            "{raw} is not a value of the enumeration {}",
            markup.full_name()
        )));
    }
    assert_eq!(
        enum_value::<IntegerValue>(&FALLBACK, 0, 0, 0).unwrap(),
        IntegerValue(0)
    );
    let error = enum_value::<IntegerValue>(&WRONG_TYPE, 0, 2, 3).unwrap_err();
    assert_eq!(
        error
            .inner_exception()
            .unwrap()
            .downcast_ref::<CompiledLoadError>()
            .unwrap()
            .type_name(),
        "InvalidCastException"
    );
}

#[derive(PartialEq)]
struct MethodPathClock;
ferroui_base::ferro_markup_type!(class MethodPathClock {
    namespace: "CompiledMethodPathTests", this: Rc<MethodPathClock>, handles: [Rc<MethodPathClock>],
    methods: [
        fn Now()->ferroui_base::utilities::DateTime => (|_: &Rc<MethodPathClock>| ferroui_base::utilities::DateTime::from_ticks(10)),
        fn Now(i32)->ferroui_base::utilities::DateTime => (|_: &Rc<MethodPathClock>, value:i32| ferroui_base::utilities::DateTime::from_ticks(10+i64::from(value))),
        static fn Static()->ferroui_base::utilities::DateTime => (||ferroui_base::utilities::DateTime::from_ticks(30)),
        try fn Fail()->ferroui_base::utilities::DateTime => (|_: &Rc<MethodPathClock>|Err::<ferroui_base::utilities::DateTime,_>("declared-clock-failure"))
    ]
});
#[test]
fn declared_method_paths_preserve_overload_static_signature_target_and_failures() {
    use crate::markup_extensions::CompiledBindingExtension;
    use ferroui_base::{
        FerroObject, FerroProperty,
        data::{CompiledBindingPathBuilder, core::ValueType},
        metadata::{
            MarkupDelegate, MarkupInvokeError, MarkupTyped, from_markup_value, into_markup_value,
        },
        utilities::DateTime,
    };
    ferroui_base::register_types();
    let source = Rc::new(MethodPathClock);
    for (name, is_static, parameters, arguments, expected) in [
        ("Now", false, vec![], vec![], Some(10)),
        (
            "Now",
            false,
            vec![ValueType::of::<i32>()],
            vec![into_markup_value(7i32)],
            Some(17),
        ),
        ("Static", true, vec![], vec![], Some(30)),
        ("Fail", false, vec![], vec![], None),
    ] {
        let builder = super::compiled::path_method(
            &CompiledBindingPathBuilder::new(),
            MethodPathClock::MARKUP,
            name,
            is_static,
            &parameters,
            Some(ValueType::of::<DateTime>()),
            false,
        );
        let binding = CompiledBindingExtension::with_path(builder.build());
        binding.set_source(into_markup_value(source.clone()));
        let target = FerroObject::new();
        let property =
            FerroProperty::register::<FerroObject, _>("DeclaredMethodValue", None::<BoxedValue>);
        let subscription = target.bind_binding(property, binding.as_ref());
        let value = target.get_value(property).expect("method delegate value");
        let delegate = value
            .downcast_ref::<MarkupDelegate>()
            .expect("declared delegate");
        assert_eq!(delegate.method().unwrap().name, name);
        assert_eq!(
            delegate.method().unwrap().parameters.len(),
            parameters.len()
        );
        assert_eq!(delegate.target().is_none(), is_static);
        let result = delegate.try_invoke(&arguments);
        match expected {
            Some(ticks) => assert_eq!(
                from_markup_value::<DateTime>(&result.unwrap())
                    .unwrap()
                    .ticks(),
                ticks
            ),
            None => assert!(
                matches!(result,Err(MarkupInvokeError::Failed(v)) if v=="declared-clock-failure")
            ),
        }
        subscription.dispose();
    }
}

#[derive(PartialEq)]
struct IndexerSignatureProbe;
ferroui_base::ferro_markup_type!(class IndexerSignatureProbe as "IndexerSignatureProbe" {
    namespace: "Tests", this: Rc<IndexerSignatureProbe>, handles: [Rc<IndexerSignatureProbe>],
    indexers: [
        (i32) -> i32 { get: |_: &Rc<IndexerSignatureProbe>, key: i32| key + 10 },
        (Option<ferroui_base::Utf16Text>) -> i32 { get: |_: &Rc<IndexerSignatureProbe>, key: Option<ferroui_base::Utf16Text>| key.map_or(-1, |v| v.units().len() as i32) },
        (u8) -> i32 { get: |_: &Rc<IndexerSignatureProbe>, key: u8| i32::from(key) + 20 },
        (i32, i32) -> i32 { get: |_: &Rc<IndexerSignatureProbe>, a: i32, b: i32| a * 100 + b }
    ]
});
#[test]
fn compiled_indexers_select_exact_native_overloads_and_preserve_arguments() {
    use crate::markup_extensions::CompiledBindingExtension;
    use ferroui_base::{FerroObject, FerroProperty, data::{CompiledBindingPathBuilder, core::ValueType}, metadata::{MarkupTyped, into_markup_value}};
    ferroui_base::register_types();
    let source = Rc::new(IndexerSignatureProbe);
    let text = ferroui_base::Utf16Text::from_units(vec![0xd800, 97]);
    for (parameters, arguments, expected) in [
        (vec![ValueType::of::<i32>()], vec![into_markup_value(7_i32)], 17),
        (vec![ValueType::of::<Option<ferroui_base::Utf16Text>>()], vec![into_markup_value(Some(text))], 2),
        (vec![ValueType::of::<Option<ferroui_base::Utf16Text>>()], vec![None], -1),
        (vec![ValueType::of::<u8>()], vec![into_markup_value(7_u8)], 27),
        (vec![ValueType::of::<i32>(), ValueType::of::<i32>()], vec![into_markup_value(7_i32), into_markup_value(9_i32)], 709),
    ] {
        let builder = super::compiled::path_indexer_arguments(&CompiledBindingPathBuilder::new(), IndexerSignatureProbe::MARKUP, &parameters, arguments, None);
        let binding = CompiledBindingExtension::with_path(builder.build());
        binding.set_source(into_markup_value(source.clone()));
        let target = FerroObject::new();
        let property = FerroProperty::register::<FerroObject, _>("IndexerSignatureValue", 0_i32);
        let subscription = target.bind_binding(property, binding.as_ref());
        assert_eq!(target.get_value(property), expected);
        subscription.dispose();
    }
    // Keep the existing integer helper and its notifying collection accessor usable.
    for notifying in [false, true] {
        let builder = super::compiled::path_indexer(&CompiledBindingPathBuilder::new(), IndexerSignatureProbe::MARKUP, 3, notifying);
        let binding = CompiledBindingExtension::with_path(builder.build());
        binding.set_source(into_markup_value(source.clone()));
        let target = FerroObject::new();
        let property = FerroProperty::register::<FerroObject, _>("IntegerIndexerValue", 0_i32);
        let subscription = target.bind_binding(property, binding.as_ref());
        assert_eq!(target.get_value(property), 13);
        subscription.dispose();
    }
}
