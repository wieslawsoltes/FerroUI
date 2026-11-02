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
    let snapshot = eager.direct_parents_stack();
    assert!(Rc::ptr_eq(&snapshot, &eager.direct_parents_stack()));
    context.pop_parent();
    assert_eq!(values(&snapshot), [1, 2]);
    assert_eq!(values(&eager.direct_parents_stack()), [1]);
    assert_eq!(values(&stack.parents()), [1, 11, 10]);
}

#[test]
fn standalone_populates_have_fresh_scopes_and_nested_populates_keep_their_provider() {
    let document = DocumentInfo { base_uri: None, namespaces: NAMESPACES };
    let first = populate_context(None, &document, None).name_scope_field().unwrap();
    let second = populate_context(None, &document, None).name_scope_field().unwrap();
    assert!(!Rc::ptr_eq(&first, &second));

    let parent = TestServiceProvider::new().with_name_scope(NameScopeRef(first.clone())).sp();
    let nested = populate_context(Some(parent.clone()), &document, None).name_scope_field().unwrap();
    assert!(Rc::ptr_eq(&first, &nested));
    let deferred = super::compiled::deferred_context(&parent, &document).name_scope_field().unwrap();
    assert!(Rc::ptr_eq(&first, &deferred));
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
fn compact_template_setter_preserves_priority_null_and_error_location() {
    use super::compiled::{bind_or_set_priority, no_setter};
    use ferroui_base::data::{core::ValueType, BindingPriority};
    use ferroui_base::{FerroObject, Visual};
    use ferroui_controls::TextBlock;

    crate::register_types();
    let text = TextBlock::new();
    let object = text.upcast_ref::<FerroObject>();
    let priority = BindingPriority::Template;
    bind_or_set_priority(object, Visual::opacity_property, to_value(0.25_f64), priority,
        ValueType::of::<f64>, false, "Visual.Opacity", 8, 13).unwrap();
    assert_eq!(text.get_value(Visual::opacity_property()), 0.25);
    text.set_value(Visual::opacity_property(), 0.75);
    assert_eq!(text.get_value(Visual::opacity_property()), 0.75);
    text.clear_value(Visual::opacity_property());
    assert_eq!(text.get_value(Visual::opacity_property()), 0.25);

    for invalid in [None, to_value("bad".to_owned())] {
        let expected = no_setter("Opacity", &invalid, 8, 13);
        let error = bind_or_set_priority(object, Visual::opacity_property, invalid, priority,
            ValueType::of::<f64>, false, "Visual.Opacity", 8, 13).unwrap_err();
        assert_eq!(error.message(), expected.message());
    }
    bind_or_set_priority(object, TextBlock::text_property, None, priority, ValueType::of::<String>,
        true, "TextBlock.Text", 8, 13).unwrap();
    assert_eq!(text.get_value(TextBlock::text_property()), None);
}

#[test]
fn nullable_binding_overload_matches_the_expanded_calls_for_both_branches() {
    use super::compiled::{bind, bind_or_set_null};
    use ferroui_base::{data::BindingPriority, FerroObject};
    use ferroui_controls::TextBlock;

    crate::register_types();
    let text = TextBlock::new();
    let target = text.upcast_ref::<FerroObject>();
    let property = TextBlock::text_property;
    text.set_value_with_priority(property(), Some("initial".into()), BindingPriority::Template);
    bind_or_set_null(target, property, &None, BindingPriority::Template, "TextBlock.Text", 8, 13).unwrap();
    assert_eq!(text.text(), None);
    let invalid = to_value("bad binding".to_owned());
    let expected = bind(target, property(), invalid.clone(), 8, 13).unwrap_err();
    let actual = bind_or_set_null(target, property, &invalid, BindingPriority::Template, "TextBlock.Text", 8, 13).unwrap_err();
    assert_eq!(actual.message(), expected.message());
}

#[test]
fn local_setter_checks_the_overloads_before_registering_the_property() {
    use super::compiled::{no_setter, set_local_value};
    use ferroui_base::{data::core::ValueType, FerroObject, FerroProperty, Visual};
    use ferroui_controls::TextBlock;

    crate::register_types();
    let text = TextBlock::new();
    let object = text.upcast_ref::<FerroObject>();
    let member = "FerroUI.Visual.set_Opacity";
    set_local_value(object, Visual::opacity_property, to_value(0.25_f64), ValueType::of::<f64>,
        false, member, "Opacity", 7, 19).unwrap();
    assert_eq!(text.get_value(Visual::opacity_property()), 0.25);
    set_local_value(object, Visual::opacity_property, Some(FerroProperty::unset_value()),
        || panic!("unset must bypass the expected value type"), false, member, "Opacity", 7, 19).unwrap();
    assert_eq!(text.get_value(Visual::opacity_property()), 1.0);
    let wrong = to_value("x".to_owned());
    let expected = no_setter("Opacity", &wrong, 7, 19);
    let actual = set_local_value::<f64>(object, || panic!("an invalid value must not read the accessor"),
        wrong, ValueType::of::<f64>, false, member, "Opacity", 7, 19).unwrap_err();
    assert_eq!(actual.message(), expected.message());
    set_local_value(object, TextBlock::text_property, None, ValueType::of::<String>,
        true, "FerroUI.Controls.TextBlock.set_Text", "Text", 7, 19).unwrap();
    assert_eq!(text.get_value(TextBlock::text_property()), None);
}

#[test]
fn template_binding_rows_resolve_operands_in_order_and_stop_on_unwind() {
    use ferroui_base::{FerroObject, FerroProperty};
    use ferroui_controls::Border;
    use std::cell::RefCell;
    use super::compiled::{bind_template_rows, property};

    ferroui_base::ferro_thread_local! { static TRACE: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) }; }
    fn source() -> &'static FerroProperty {
        TRACE.with(|trace| trace.borrow_mut().push("source"));
        property(Border::background_property())
    }
    fn destination() -> &'static FerroProperty {
        TRACE.with(|trace| trace.borrow_mut().push("destination"));
        property(Border::background_property())
    }
    fn fail() -> &'static FerroProperty {
        TRACE.with(|trace| trace.borrow_mut().push("fail"));
        panic!("property initializer");
    }
    let target = Border::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bind_template_rows(target.upcast_ref::<FerroObject>(), &[
            (&(source, destination), 2, 3),
            (&(fail, destination), 3, 4),
            (&(source, destination), 4, 5),
        ]).unwrap();
    }));
    assert!(result.is_err());
    TRACE.with(|trace| assert_eq!(*trace.borrow(), ["source", "destination", "fail"]));
}
