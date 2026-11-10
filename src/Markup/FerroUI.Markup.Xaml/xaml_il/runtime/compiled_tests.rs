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
    &[("", &[("FerroUI.Controls", "FerroUI.Controls", &[]), ("FerroUI.Media", "FerroUI.Base", &[])]), ("x", &[])];

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
    // The base URI is the URL of the canonical text of the document's URI: the scheme and the
    // authority in lower case, as the `Uri` of the text compared and resolved it (C5 of #423).
    assert_eq!(uri_context.base_uri().as_ref().map(url::Url::as_str), Some("ferres://assembly/Views/Main.xaml"));
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
fn the_namespace_information_has_the_types_the_compiler_resolved() {
    use ferroui_base::data::core::expression_nodes::CastTarget;
    use ferroui_base::StaticType;
    const TYPED: XmlNamespaceTable = &[(
        "",
        &[
            ("FerroUI.Controls", "FerroUI.Controls", &[("TextBox", || CastTarget::Class(<ferroui_controls::TextBox as StaticType>::TYPE))]),
            ("FerroUI.Media", "FerroUI.Base", &[]),
        ],
    )];
    let context = create_context(None, None, TYPED);
    let provider: Rc<dyn IServiceProvider> = context;
    let info = provider
        .get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>()
        .expect("the namespace information");
    let namespaces = info.xml_namespaces();
    let controls = &namespaces[""][0];
    assert_eq!(controls.types().iter().map(|(name, _)| *name).collect::<Vec<_>>(), ["TextBox"]);
    match controls.find_type("TextBox") {
        Some(CastTarget::Class(class)) => assert!(std::ptr::eq(class, <ferroui_controls::TextBox as StaticType>::TYPE)),
        _ => panic!("the class of the entry was expected"),
    }
    assert!(controls.find_type("Button").is_none());
    assert!(namespaces[""][1].types().is_empty());
}

#[test]
fn compiled_namespace_scopes_shadow_restore_and_keep_captured_tables_stable() {
    use super::compiled::push_namespace_scope;
    let context = create_context(None, None, NAMESPACES);
    let provider: Rc<dyn IServiceProvider> = context.clone();
    let namespaces = || provider.get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>().unwrap().xml_namespaces();
    let outer = push_namespace_scope(&context, &[("p", &[("Outer.Views", "App", &[])])]);
    let captured = namespaces();
    assert_eq!(captured["p"][0].clr_namespace(), "Outer.Views");
    let inner = push_namespace_scope(&context, &[("p", &[("Inner.Views", "App", &[])])]);
    assert_eq!(namespaces()["p"][0].clr_namespace(), "Inner.Views");
    // Out-of-order cleanup must not expose a dropped outer scope underneath the active one.
    drop(outer);
    assert_eq!(namespaces()["p"][0].clr_namespace(), "Inner.Views");
    drop(inner);
    assert!(!namespaces().contains_key("p"));
    assert!(namespaces().contains_key(""));
    assert_eq!(captured["p"][0].clr_namespace(), "Outer.Views");
    let failed = (|| -> Result<(), ()> {
        let _guard = push_namespace_scope(&context, &[("p", &[("Failed.Views", "App", &[])])]);
        Err(())
    })();
    assert!(failed.is_err());
    assert!(!namespaces().contains_key("p"));
}

#[test]
fn emitted_deferred_context_uses_declaration_namespaces_after_parent_scope_ends() {
    use super::compiled::{deferred_context, push_namespace_scope};
    let context = create_context(None, None, NAMESPACES);
    let declaration: XmlNamespaceTable = &[("local", &[("Inner.Views", "App", &[])])];
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

// --- the binds of a template build -----------------------------------------------------------
//
// `bind_dynamic_resource` and `bind_template_binding` make the expression of the binding
// without the extension the statements of generated code created. Each is compared here with
// those statements, written out as the emitter wrote them.

mod template_binds {
    use std::rc::Rc;

    use ferroui_base::controls::ResourceDictionary;
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::media::{Color, Colors, SolidColorBrush};
    use ferroui_base::metadata::IServiceProvider;
    use ferroui_base::styling::ThemeVariant;
    use ferroui_base::{FerroObject, FerroProperty, Ref};
    use ferroui_controls::primitives::TemplatedControl;
    use ferroui_controls::{Border, Button};

    use super::super::compiled::{
        bind, bind_dynamic_resource, bind_template_binding, create_context, property_value, provide_value, string_object,
        to_value,
    };
    use super::super::IFerroXamlIlEagerParentStackProvider;
    use super::{values, NAMESPACES};
    use crate::markup_extensions::DynamicResourceExtension;
    use crate::test_support::{boxed, TestServiceProvider};
    use crate::xaml_il::runtime::XamlIlContext;

    type Bind = fn(&Rc<XamlIlContext>, &Ref<FerroObject>, &'static FerroProperty, &str);

    /// The bind of the runtime.
    fn direct(context: &Rc<XamlIlContext>, target: &Ref<FerroObject>, property: &'static FerroProperty, key: &str) {
        bind_dynamic_resource(context, target, property, key, 3, 9).expect("the resource is bound");
    }

    /// The statements of generated code the bind of the runtime stands for.
    fn statements(context: &Rc<XamlIlContext>, target: &Ref<FerroObject>, property: &'static FerroProperty, key: &str) {
        let extension = DynamicResourceExtension::__markup_new_1(string_object(key));
        let provided =
            provide_value(context, property_value(property), &extension, DynamicResourceExtension::__markup_ProvideValue_0);
        bind(target, property, provided, 3, 9).expect("the resource is bound");
    }

    const BINDS: [(&str, Bind); 2] = [("direct", direct), ("statements", statements)];

    fn brush(color: Color) -> Option<ferroui_base::BoxedValue> {
        Some(boxed(SolidColorBrush::with_color(color)))
    }

    fn background_color(border: &Border) -> Option<Color> {
        border.background().map(|brush| brush.as_solid_color_brush().expect("a solid color brush").color())
    }

    /// A border with the resource `brush`, and a context whose parent service provider is
    /// `parent`, with the border pushed as the element being built.
    fn element(parent: Rc<dyn IServiceProvider>) -> (Ref<Border>, Rc<XamlIlContext>) {
        let border = Border::new();
        border.resources().add("brush", brush(Colors::BLUE));
        let context = create_context(Some(parent), None, NAMESPACES);
        context.push_parent(to_value(border.clone()));
        (border, context)
    }

    #[test]
    fn a_dynamic_resource_of_a_template_is_bound_at_the_template_priority() {
        for (name, bind) in BINDS {
            let (border, context) = element(TestServiceProvider::new().in_control_template().sp());
            bind(&context, border.upcast_ref::<FerroObject>(), Border::background_property(), "brush");
            assert_eq!(background_color(&border), Some(Colors::BLUE), "{name}");
            // A local value is above the binding, and does not replace it.
            border.set_value(Border::background_property(), Some(SolidColorBrush::with_color(Colors::RED).into()));
            assert_eq!(background_color(&border), Some(Colors::RED), "{name}");
            border.clear_value(Border::background_property());
            assert_eq!(background_color(&border), Some(Colors::BLUE), "{name}");
            // The target property was cleared after the bind, as after `ProvideValue`.
            assert!(context.target_property().is_none(), "{name}");
        }
    }

    #[test]
    fn a_dynamic_resource_outside_a_template_is_bound_as_a_local_value() {
        for (name, bind) in BINDS {
            let (border, context) = element(TestServiceProvider::new().sp());
            bind(&context, border.upcast_ref::<FerroObject>(), Border::background_property(), "brush");
            assert_eq!(background_color(&border), Some(Colors::BLUE), "{name}");
            // A local value replaces the binding.
            border.set_value(Border::background_property(), Some(SolidColorBrush::with_color(Colors::RED).into()));
            border.clear_value(Border::background_property());
            assert_eq!(background_color(&border), None, "{name}");
        }
    }

    #[test]
    fn a_dynamic_resource_of_an_object_that_is_no_element_finds_resources_from_its_nearest_parent() {
        for (name, bind) in BINDS {
            let host = Border::new();
            host.resources().add("accent", Some(boxed(Colors::BLUE)));
            let context = create_context(Some(TestServiceProvider::new().sp()), None, NAMESPACES);
            context.push_parent(to_value(host.clone()));
            let brush = SolidColorBrush::new();
            context.push_parent(to_value(brush.clone()));
            bind(&context, brush.upcast_ref::<FerroObject>(), SolidColorBrush::color_property(), "accent");
            context.pop_parent();
            assert_eq!(brush.color(), Colors::BLUE, "{name}");
            host.resources().set("accent", Some(boxed(Colors::RED)));
            assert_eq!(brush.color(), Colors::RED, "{name}");
        }
    }

    #[test]
    fn a_dynamic_resource_in_a_theme_dictionary_is_of_the_variant_of_the_dictionary() {
        for (name, bind) in BINDS {
            let themed = ResourceDictionary::new();
            let dark = ResourceDictionary::new();
            dark.add("accent", Some(boxed(Colors::BLUE)));
            // The key of a theme dictionary, as the markup that declares it sets it.
            dark.set_key(Some(ThemeVariant::dark()));
            let light = ResourceDictionary::new();
            light.add("accent", Some(boxed(Colors::RED)));
            themed.add_theme_dictionary(ThemeVariant::dark(), dark.clone());
            themed.add_theme_dictionary(ThemeVariant::light(), light);

            let host = Border::new();
            host.resources().add_merged_dictionary(themed);
            host.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));

            // The brush is declared in the dark dictionary: the dictionary is its nearest parent,
            // and its variant is the one the resource is looked up in, whatever the host has.
            let context = create_context(Some(TestServiceProvider::new().sp()), None, NAMESPACES);
            context.push_parent(to_value(dark.clone()));
            let brush = SolidColorBrush::new();
            context.push_parent(to_value(brush.clone()));
            bind(&context, brush.upcast_ref::<FerroObject>(), SolidColorBrush::color_property(), "accent");
            context.pop_parent();
            assert_eq!(brush.color(), Colors::BLUE, "{name}");
        }
    }

    #[test]
    fn a_template_binding_is_bound_as_the_statements_of_generated_code_bind_it() {
        fn direct(target: &Ref<FerroObject>, property: &'static FerroProperty, source: &'static FerroProperty) {
            bind_template_binding(target, property, source, 5, 11).expect("the binding is bound");
        }
        fn statements(target: &Ref<FerroObject>, property: &'static FerroProperty, source: &'static FerroProperty) {
            let binding = TemplateBinding::__markup_new_1(source);
            let provided = TemplateBinding::__markup_ProvideValue_0(&binding);
            bind(target, property, provided, 5, 11).expect("the binding is bound");
        }
        let binds: [(&str, fn(&Ref<FerroObject>, &'static FerroProperty, &'static FerroProperty)); 2] =
            [("direct", direct), ("statements", statements)];
        for (name, bind) in binds {
            let button = Button::new();
            button.set_value(TemplatedControl::background_property(), Some(SolidColorBrush::with_color(Colors::BLUE).into()));
            let border = Border::new();
            bind(border.upcast_ref::<FerroObject>(), Border::background_property(), TemplatedControl::background_property());
            assert_eq!(background_color(&border), None, "{name}");
            border.set_templated_parent(Some(button.upcast_ref::<FerroObject>().clone()));
            assert_eq!(background_color(&border), Some(Colors::BLUE), "{name}");
            button.set_value(TemplatedControl::background_property(), Some(SolidColorBrush::with_color(Colors::RED).into()));
            assert_eq!(background_color(&border), Some(Colors::RED), "{name}");
        }
    }

    #[test]
    fn the_direct_parents_are_the_objects_of_the_stack_and_a_list_handed_out_does_not_change() {
        let context = create_context(None, None, NAMESPACES);
        let eager: Rc<dyn IFerroXamlIlEagerParentStackProvider> = context.clone();
        assert!(eager.direct_parents_stack().is_empty());
        context.push_parent(to_value(1i32));
        // A null parent is on the stack and not among the objects.
        context.push_parent(None);
        context.push_parent(to_value(2i32));
        let held = eager.direct_parents_stack();
        assert_eq!(values(&held), [1, 2]);
        // The list is the one the context keeps, not a copy, until the stack changes.
        assert!(Rc::ptr_eq(&held, &eager.direct_parents_stack()));
        context.push_parent(to_value(3i32));
        assert_eq!(values(&held), [1, 2]);
        assert_eq!(values(&eager.direct_parents_stack()), [1, 2, 3]);
        context.pop_parent();
        context.pop_parent();
        assert_eq!(values(&eager.direct_parents_stack()), [1]);
        // Popping the null parent takes no object away.
        context.pop_parent();
        assert_eq!(values(&eager.direct_parents_stack()), [1]);
        assert_eq!(context.parents_stack().len(), 1);
        context.pop_parent();
        assert!(eager.direct_parents_stack().is_empty());
        assert_eq!(values(&held), [1, 2]);
    }
}
