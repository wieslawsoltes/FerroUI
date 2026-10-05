//! Tests of the context factory of compiled markup ([`super::compiled`]).
//! Not tests of upstream: upstream's compiler generates the context class;
//! these check that the factory gives generated code the context the
//! run-time loader's framework language creates.

use std::rc::Rc;

use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::BoxedValue;

use super::compiled::{create_context, populate_context, to_value, XmlNamespaceTable, FRAMEWORK_CONTEXT};
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
    let context = populate_context(None, None, NAMESPACES, to_value(7i32));
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
