//! Tests of the runtime helpers: deferred content, the service providers
//! and the application of non-matching markup extension values.

use super::*;
use crate::markup_extensions::DynamicResourceExtension;
use crate::test_support::{boxed, TestServiceProvider};
use crate::{register_types, IRootObjectProvider, IXamlTypeResolver, ServiceProviderExtensions, XamlResourceNode};
use ferroui_base::controls::{IDeferredContent, NameScope, NameScopeRef, ResourceDictionary};
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, into_markup_value, IServiceProvider, MarkupTyped};
use ferroui_base::{BoxedValue, FerroProperty, Ref, StaticType};
use ferroui_controls::{Border, Button, Control, TextBlock};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn declaration_context(parents: Vec<BoxedValue>, root: BoxedValue, scope: Option<NameScopeRef>) -> Rc<dyn IServiceProvider> {
    let provider = TestServiceProvider::new().with_parents(parents).with_root(Some(root));
    match scope {
        Some(scope) => provider.with_name_scope(scope).sp(),
        None => provider.sp(),
    }
}

fn button_builder() -> DeferredContentBuilder {
    DeferredContentBuilder::new(|sp| {
        let button = Button::new();
        sp.get_name_scope().unwrap().register("PART_Button", button.clone().upcast());
        Some(boxed(button))
    })
}

#[test]
fn every_instantiation_gets_a_new_name_scope() {
    let root = Border::new();
    let provider = declaration_context(vec![boxed(root.clone())], boxed(root), None);
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(button_builder(), &provider);

    let first = content.build_with(None);
    let second = content.build_with(None);

    assert!(first.name_scope != second.name_scope);
    assert!(first.name_scope.is_completed() && second.name_scope.is_completed());
    let first_button = from_markup_value::<Ref<Button>>(&first.result).unwrap();
    let second_button = from_markup_value::<Ref<Button>>(&second.result).unwrap();
    assert!(first_button != second_button);
    assert!(first.name_scope.find("PART_Button").unwrap().ptr_eq(&first_button));
    assert!(second.name_scope.find("PART_Button").unwrap().ptr_eq(&second_button));
}

#[test]
fn the_name_scope_of_an_instantiation_is_a_child_of_the_declaring_scope() {
    let outer = NameScopeRef::new(NameScope::new());
    let named = TextBlock::new();
    outer.register("Outer", named.clone().upcast());
    let root = Border::new();
    let provider = declaration_context(vec![], boxed(root), Some(outer.clone()));
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(button_builder(), &provider);

    let built = content.build_with(None);
    // Names of the declaring scope are found from the inner scope, not the
    // other way round.
    assert!(built.name_scope.find("Outer").unwrap().ptr_eq(&named));
    assert!(built.name_scope.find("PART_Button").is_some());
    assert!(outer.find("PART_Button").is_none());
}

#[test]
fn the_builder_sees_the_captured_context() {
    let root = Border::new();
    let outer = Border::new();
    let resources = ResourceDictionary::new();
    // Only resource nodes are captured: the string is not one.
    let provider = declaration_context(
        vec![boxed(outer.clone()), boxed("not a node".to_string()), boxed(resources.clone())],
        boxed(root.clone()),
        None,
    );

    let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let expected_root = root.clone();
    let expected_outer = outer.clone();
    let builder = DeferredContentBuilder::new(move |sp| {
        let stack = sp.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
        let eager = stack.clone().as_eager_parent_stack_provider().expect("an eager provider");
        let direct = eager.direct_parents_stack();
        // The immediate parent is last.
        assert_eq!(direct.len(), 2);
        assert!(from_markup_value::<Ref<Border>>(&Some(direct[0].clone())).unwrap() == expected_outer);
        assert!(direct[1].is::<Ref<ResourceDictionary>>());
        // Enumerated, the immediate parent is first.
        let parents = stack.parents();
        assert!(parents[0].is::<Ref<ResourceDictionary>>());
        assert_eq!(parents.len(), 2);
        assert!(eager.parent_provider().is_none());

        let roots = sp.get_required_service::<Rc<dyn IRootObjectProvider>>();
        assert!(from_markup_value::<Ref<Border>>(&roots.root_object()).unwrap() == expected_root);
        assert!(from_markup_value::<Ref<Border>>(&roots.intermediate_root_object()).unwrap() == expected_root);
        assert!(sp.is_in_control_template());
        assert!(sp.get_name_scope().is_some());
        assert_eq!(sp.get_parents::<XamlResourceNode>().len(), 2);
        log.borrow_mut().push("built".to_string());
        Some(boxed(Border::new()))
    });

    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder, &provider);
    // Changing the declaring context afterwards does not change what was captured.
    drop(provider);
    assert!(content.build_with(None).result.is_some());
    assert_eq!(*seen.borrow(), ["built"]);
}

#[test]
fn the_service_provider_of_the_instantiation_is_the_parent_of_the_captured_one() {
    let root = Border::new();
    let declared = declaration_context(vec![boxed(root.clone())], boxed(root.clone()), None);

    let host = Button::new();
    let instantiation = TestServiceProvider::new().with_parents(vec![boxed(host.clone())]).with_base_uri("ferres://app/Host.xaml").sp();

    let expected_host = host.clone();
    let builder = DeferredContentBuilder::new(move |sp| {
        // Services the captured context does not answer are the ones of the
        // place the content is instantiated in.
        assert_eq!(sp.get_context_base_uri().unwrap().absolute_uri(), "ferres://app/Host.xaml");
        // The parents of that place follow the captured ones.
        let buttons = sp.get_parents::<Ref<Button>>();
        assert_eq!(buttons.len(), 1);
        assert!(buttons[0] == expected_host);
        assert_eq!(sp.get_parents::<Ref<Control>>().len(), 2);
        assert!(sp.get_first_parent::<Ref<Border>>().is_some());
        Some(boxed(Border::new()))
    });
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder, &declared);
    assert!(content.build_with(Some(&instantiation)).result.is_some());
}

#[test]
fn resource_node_stacks_of_the_same_parents_are_shared() {
    let root = Border::new();
    let provider = declaration_context(vec![boxed(root.clone()), boxed(ResourceDictionary::new())], boxed(root), None);
    let stacks: Rc<RefCell<Vec<Rc<Vec<BoxedValue>>>>> = Rc::new(RefCell::new(Vec::new()));
    let builder = || {
        let stacks = stacks.clone();
        DeferredContentBuilder::new(move |sp| {
            let stack = sp.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
            stacks.borrow_mut().push(stack.as_eager_parent_stack_provider().unwrap().direct_parents_stack());
            None
        })
    };

    let first = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder(), &provider);
    let second = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder(), &provider);
    first.build_with(None);
    second.build_with(None);

    let stacks = stacks.borrow();
    assert!(Rc::ptr_eq(&stacks[0], &stacks[1]));
}

/// Not a port: upstream's deferred content holds what it captured, and its
/// collector frees the cycle (DEVIATIONS.md, Markup).
#[test]
fn deferred_content_does_not_keep_what_it_was_declared_under_alive() {
    let root = Border::new();
    let provider = declaration_context(vec![boxed(root.clone())], boxed(root.clone()), None);
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(button_builder(), &provider);
    drop(provider);

    // The root owns the content, as an element owns the template declared
    // under it.
    root.set_tag(Some(content as BoxedValue));
    let weak = root.downgrade();
    drop(root);
    assert!(weak.upgrade().is_none());
}

/// Not a port: an instantiation sees the captured objects that are alive.
#[test]
fn deferred_content_is_instantiated_with_the_captured_objects_that_are_alive() {
    let root = Border::new();
    let gone = Border::new();
    let provider = declaration_context(vec![boxed(root.clone()), boxed(gone.clone())], boxed(root.clone()), None);
    let seen: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    let builder = {
        let seen = seen.clone();
        DeferredContentBuilder::new(move |sp| {
            seen.borrow_mut().push(sp.get_parents::<Ref<Border>>().len());
            None
        })
    };
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder, &provider);
    drop(provider);

    content.build_with(None);
    drop(gone);
    content.build_with(None);
    assert_eq!(vec![2, 1], *seen.borrow());
}

#[test]
fn deferred_content_builds_the_object_as_the_deferred_content_contract() {
    let root = Border::new();
    let provider = declaration_context(vec![], boxed(root), None);
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(button_builder(), &provider);

    let deferred: Rc<dyn IDeferredContent> = content.as_deferred_content();
    assert!(deferred.build(None).unwrap().is::<Ref<Button>>());

    // It is what a resource dictionary stores for a deferred resource.
    let resources = ResourceDictionary::new();
    resources.add_deferred("button", deferred);
    let key: ferroui_base::controls::ResourceKey = "button".into();
    assert!(resources.try_get_value(&key).unwrap().unwrap().is::<Ref<Button>>());
}

#[test]
#[should_panic(expected = "Unable to cast object of type")]
fn building_content_of_the_wrong_type_panics() {
    let provider = declaration_context(vec![], boxed(Border::new()), None);
    let builder = DeferredContentBuilder::new(|_| Some(boxed("text".to_string())));
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder, &provider);
    content.build_with(None);
}

#[test]
fn the_older_factories_create_the_same_deferred_content() {
    let provider = declaration_context(vec![], boxed(Border::new()), None);
    let v1 = XamlIlRuntimeHelpers::deferred_transformation_factory_v1(button_builder(), &provider);
    assert_eq!(v1.result_type(), ValueType::of::<Ref<Control>>());
    assert!(v1.build_with(None).result.is_some());

    let v2 = XamlIlRuntimeHelpers::deferred_transformation_factory_v2::<Ref<Button>>(button_builder(), &provider);
    assert_eq!(v2.result_type(), ValueType::of::<Ref<Button>>());
    assert!(v2.build_with(None).result.is_some());
}

#[test]
#[should_panic(expected = "hasn't been registered")]
fn the_factory_requires_a_parent_stack() {
    let provider = TestServiceProvider::new().with_root(Some(boxed(1i32))).sp();
    let _ = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(button_builder(), &provider);
}

#[test]
fn builders_compare_by_identity() {
    let a = button_builder();
    let b = a.clone();
    assert!(a == b);
    assert!(a != button_builder());
    let shared: Rc<dyn Fn(&Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, crate::XamlLoadException>> =
        Rc::new(|_| Ok(None));
    assert!(DeferredContentBuilder::from_rc(shared.clone()) == DeferredContentBuilder::from_rc(shared));
}

#[test]
fn root_service_provider_has_a_name_scope_and_the_parents_of_its_parent() {
    let root = XamlIlRuntimeHelpers::create_root_service_provider_v2();
    let scope = root.get_name_scope().unwrap();
    assert!(!scope.is_completed());
    // Without an application the parent stack is empty.
    let stack = root.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
    assert!(stack.parents().is_empty());
    let eager = stack.as_eager_parent_stack_provider().unwrap();
    assert!(eager.direct_parents_stack().is_empty() && eager.parent_provider().is_none());
    // Other services are not offered.
    assert!(root.get_service_of::<Rc<dyn IRootObjectProvider>>().is_none());

    let border = Border::new();
    let parent = TestServiceProvider::new().with_parents(vec![boxed(border.clone())]).with_base_uri("ferres://a/b").sp();
    let child = XamlIlRuntimeHelpers::create_root_service_provider_v3(Some(parent));
    assert!(child.get_first_parent::<Ref<Border>>().unwrap() == border);
    // Only the parent stack comes from the parent provider.
    assert!(child.get_context_base_uri().is_none());
    // Each root has its own scope.
    let other = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    assert!(!std::ptr::addr_eq(Rc::as_ptr(&child.get_name_scope().unwrap()), Rc::as_ptr(&other.get_name_scope().unwrap())));
    // The name scope is also offered as the property-value handle.
    assert!(child.get_service_of::<NameScopeRef>().is_some());
}

#[test]
fn a_provider_that_only_enumerates_is_wrapped_as_an_eager_one() {
    let border = Border::new();
    let button = Button::new();
    let lazy = TestServiceProvider::new().with_parents(vec![boxed(border), boxed(button)]).lazy().sp();
    let provider = lazy.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
    assert!(provider.clone().as_eager_parent_stack_provider().is_none());

    let eager = XamlIlRuntimeHelpers::as_eager_parent_stack_provider(provider.clone());
    let stack = eager.direct_parents_stack();
    assert_eq!(stack.len(), 2);
    // The immediate parent is last in the direct stack and first when enumerated.
    assert!(stack[1].is::<Ref<Button>>());
    assert!(eager.parents()[0].is::<Ref<Button>>());
    assert!(eager.parent_provider().is_none());
    // The snapshot is taken once.
    assert!(Rc::ptr_eq(&stack, &eager.direct_parents_stack()));

    // An eager provider is returned as it is.
    let already = TestServiceProvider::new().with_parents(vec![]).sp();
    let provider = already.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
    let eager = XamlIlRuntimeHelpers::as_eager_parent_stack_provider(provider.clone());
    assert!(std::ptr::addr_eq(Rc::as_ptr(&eager), Rc::as_ptr(&provider)));
}

fn namespaces(entries: &[(&str, &[(&str, &str)])]) -> XmlNamespaces {
    let mut map = HashMap::new();
    for (prefix, infos) in entries {
        map.insert(
            prefix.to_string(),
            infos.iter().map(|(namespace, assembly)| FerroXamlIlXmlNamespaceInfo::with(namespace, assembly)).collect(),
        );
    }
    Rc::new(map)
}

#[test]
fn inner_service_provider_resolves_types_with_the_namespaces_of_the_document() {
    register_types();
    let compiled = TestServiceProvider::new()
        .with_namespaces(namespaces(&[
            ("", &[("FerroUI.Controls.Primitives", "FerroUI.Controls"), ("FerroUI.Controls", "FerroUI.Controls")]),
            ("m", &[("FerroUI.Markup.Xaml.MarkupExtensions", "FerroUI.Markup.Xaml")]),
            ("unknown", &[("Nowhere", "")]),
        ]))
        .sp();
    let inner = XamlIlRuntimeHelpers::create_inner_service_provider_v1(compiled);
    let resolver = inner.get_required_service::<Rc<dyn IXamlTypeResolver>>();
    // The resolver is created once.
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(&resolver),
        Rc::as_ptr(&inner.get_required_service::<Rc<dyn IXamlTypeResolver>>())
    ));
    // The inner provider offers nothing else.
    assert!(inner.get_name_scope().is_none());

    match resolver.resolve("Border").ok().unwrap() {
        CastTarget::Class(class) => assert!(std::ptr::eq(class, <Border as StaticType>::TYPE)),
        CastTarget::Value(_) => panic!("a class was expected"),
    }
    // Through the service provider lookups.
    assert!(matches!(inner.resolve_type(None, "Button"), Ok(CastTarget::Class(_))));
    assert!(matches!(inner.resolve_type(Some(""), "Button"), Ok(CastTarget::Class(_))));
    // A type that is not a class resolves to its handle type.
    match inner.resolve_type(Some("m"), "StaticResourceExtension").ok().unwrap() {
        CastTarget::Value(type_) => {
            assert_eq!(type_, ValueType::of::<crate::markup_extensions::StaticResourceExtension>())
        }
        CastTarget::Class(_) => panic!("a value type was expected"),
    }

    let error = resolver.resolve("x:Border").err().unwrap();
    assert_eq!(error.message(), "Unable to resolve namespace for type x:Border");
    let error = resolver.resolve("m:Border").err().unwrap();
    assert_eq!(
        error.message(),
        "Unable to resolve type m:Border from any of the following locations: \
         `clr-namespace:FerroUI.Markup.Xaml.MarkupExtensions;assembly=FerroUI.Markup.Xaml`"
    );
    // Entries without an assembly cannot be resolved.
    assert!(resolver.resolve("unknown:Border").err().unwrap().message().ends_with("locations: "));
}

#[test]
fn a_binding_that_does_not_match_the_property_type_is_bound() {
    let border = Border::new();
    border.resources().add("tag", Some(boxed("bound".to_string())));
    let property: &'static FerroProperty = Control::tag_property();
    let prov = TestServiceProvider::new().sp();
    let binding = DynamicResourceExtension::with_resource_key(Some(boxed("tag".to_string())));

    let target = boxed(border.clone());
    XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&target),
        Some(&boxed(property)),
        &prov,
        Some(&boxed(binding.as_binding())),
    )
    .unwrap();
    assert_eq!(border.tag().unwrap().downcast_ref::<String>().unwrap(), "bound");

    // The extension object itself is a binding too.
    let other = Border::new();
    other.resources().add("tag", Some(boxed(1i32)));
    let extension: BoxedValue = binding;
    XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&boxed(other.clone())),
        Some(&boxed(property)),
        &prov,
        Some(&extension),
    )
    .unwrap();
    assert_eq!(other.tag().unwrap().downcast_ref::<i32>(), Some(&1));
}

#[test]
fn the_unset_value_clears_the_property() {
    let border = Border::new();
    border.set_tag(Some(boxed(1i32)));
    let property: &'static FerroProperty = Control::tag_property();
    let prov = TestServiceProvider::new().sp();

    XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&boxed(border.clone())),
        Some(&boxed(property)),
        &prov,
        Some(&FerroProperty::unset_value()),
    )
    .unwrap();
    assert!(border.tag().is_none());

    // On anything that is not a registered property the unset value is ignored.
    XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&boxed(border)),
        Some(&boxed("Tag".to_string())),
        &prov,
        Some(&FerroProperty::unset_value()),
    )
    .unwrap();
}

#[test]
fn values_that_cannot_be_applied_are_errors() {
    let border = boxed(Border::new());
    let property: &'static FerroProperty = Control::tag_property();
    let prov = TestServiceProvider::new().sp();
    let binding = boxed(DynamicResourceExtension::with_resource_key(Some(boxed("k".to_string()))).as_binding());

    let error = XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&border),
        Some(&boxed("Tag".to_string())),
        &prov,
        Some(&binding),
    )
    .unwrap_err();
    assert_eq!(error.message(), "Attempt to apply binding to non-ferro property Tag");

    let error = XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&border),
        Some(&boxed(property)),
        &prov,
        Some(&boxed(5i32)),
    )
    .unwrap_err();
    assert_eq!(error.message(), "Don't know what to do with i32");

    // A binding cannot be applied to an object without registered properties.
    let error = XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&boxed(5i32)),
        Some(&boxed(property)),
        &prov,
        Some(&binding),
    )
    .unwrap_err();
    assert!(error.message().starts_with("Unable to cast object of type 'i32'"));
}

#[test]
fn the_helpers_are_invocable_through_their_metadata() {
    register_types();
    let markup = <XamlIlRuntimeHelpers as MarkupTyped>::MARKUP;
    assert_eq!(markup.namespace(), "FerroUI.Markup.Xaml.XamlIl.Runtime");
    let method = |name: &str| markup.find_methods(name).next().unwrap();

    let root = (method("CreateRootServiceProviderV2").invoke)(&[]).unwrap();
    assert!(from_markup_value::<Rc<dyn IServiceProvider>>(&root).unwrap().get_name_scope().is_some());
    let root = (method("CreateRootServiceProviderV3").invoke)(&[None]).unwrap();
    assert!(root.is_some());
    let border = Border::new();
    let parent = TestServiceProvider::new().with_parents(vec![boxed(border.clone())]).with_root(Some(boxed(border))).sp();
    let root = (method("CreateRootServiceProviderV3").invoke)(&[into_markup_value(parent.clone())]).unwrap();
    let root = from_markup_value::<Rc<dyn IServiceProvider>>(&root).unwrap();
    assert!(root.get_first_parent::<Ref<Border>>().is_some());

    let inner = (method("CreateInnerServiceProviderV1").invoke)(&[into_markup_value(parent.clone())]).unwrap();
    assert!(inner.is_some());

    let stack = parent.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
    let eager = (method("AsEagerParentStackProvider").invoke)(&[into_markup_value(stack)]).unwrap();
    assert!(from_markup_value::<Rc<dyn IFerroXamlIlEagerParentStackProvider>>(&eager).is_some());

    for factory in
        ["DeferredTransformationFactoryV1", "DeferredTransformationFactoryV2", "DeferredTransformationFactoryV3"]
    {
        let content =
            (method(factory).invoke)(&[into_markup_value(button_builder()), into_markup_value(parent.clone())]).unwrap();
        let content = from_markup_value::<Rc<DeferredContent>>(&content).unwrap();
        assert!(content.build_with(None).result.unwrap().is::<Ref<Button>>());

        // The deferred content object is invocable through its own metadata.
        let build = <DeferredContent as MarkupTyped>::MARKUP.find_methods("Build").next().unwrap();
        let built = (build.invoke)(&[into_markup_value(content), None]).unwrap();
        assert!(built.unwrap().is::<Ref<Button>>());
    }

    let target = Border::new();
    let property: &'static FerroProperty = Control::tag_property();
    target.set_tag(Some(boxed(1i32)));
    (method("ApplyNonMatchingMarkupExtensionV1").invoke)(&[
        into_markup_value(target.clone()),
        into_markup_value(property),
        into_markup_value(parent),
        Some(FerroProperty::unset_value()),
    ])
    .unwrap();
    assert!(target.tag().is_none());
}

#[test]
fn a_document_loaded_on_its_own_has_the_application_as_its_parent() {
    let application = ferroui_controls::Application::new();
    application.resources().add("app", Some(boxed("from the application".to_string())));
    ferroui_controls::Application::bind_current(application.clone());

    let root = XamlIlRuntimeHelpers::create_root_service_provider_v2();
    let stack = root.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
    let parents = stack.parents();
    assert_eq!(parents.len(), 1);
    assert!(from_markup_value::<Ref<ferroui_controls::Application>>(&Some(parents[0].clone())).unwrap() == application);
    let eager = stack.clone().as_eager_parent_stack_provider().unwrap();
    assert_eq!(eager.direct_parents_stack().len(), 1);
    assert!(eager.parent_provider().is_none());

    // The provider is the same for the same application.
    let again = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    let other = again.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&stack), Rc::as_ptr(&other)));

    // The application is a resource node: its resources are static resources.
    let extension =
        crate::markup_extensions::StaticResourceExtension::with_resource_key(Some(boxed("app".to_string())));
    let value = extension.provide_value(&root).unwrap().unwrap();
    assert_eq!(value.downcast_ref::<String>().unwrap(), "from the application");
    // And it provides a data context: it is the default anchor.
    assert!(root.get_default_anchor().unwrap().ptr_eq(&application));
}

#[test]
fn a_failing_builder_is_the_error_of_the_instantiation() {
    use crate::templates::{ControlTemplate, TemplateContent};
    use crate::XamlLoadException;
    use ferroui_base::metadata::MarkupInvokeError;

    register_types();
    let provider = declaration_context(vec![], boxed(Border::new()), None);
    let builder = DeferredContentBuilder::try_new(|_| Err(XamlLoadException::with_message("Static resource 'x' not found.")));
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder.clone(), &provider);

    assert_eq!(content.try_build_with(None).err().unwrap().message(), "Static resource 'x' not found.");
    let untyped: BoxedValue = content.clone();
    let error = TemplateContent::try_load_as::<Ref<Control>>(Some(&untyped)).err().unwrap();
    assert_eq!(error.message(), "Static resource 'x' not found.");

    // Through metadata: the deferred content, the loader of template
    // content and a template.
    let failed = Err(MarkupInvokeError::Failed("Static resource 'x' not found.".to_string()));
    let build = <DeferredContent as MarkupTyped>::MARKUP.find_methods("Build").next().unwrap();
    assert_eq!((build.invoke)(&[Some(untyped.clone()), None]), failed);
    let load = <TemplateContent as MarkupTyped>::MARKUP.find_methods("Load").next().unwrap();
    assert_eq!((load.invoke)(&[Some(untyped.clone())]), failed);
    let template = ControlTemplate::new();
    template.set_content(Some(untyped));
    let template: BoxedValue = template;
    let build = <ControlTemplate as MarkupTyped>::MARKUP.find_methods("Build").next().unwrap();
    let control = ferroui_controls::primitives::TemplatedControl::new();
    assert_eq!((build.invoke)(&[Some(template), into_markup_value(control)]), failed);

    // A builder that fails once succeeds on the next instantiation.
    let attempts = Rc::new(std::cell::Cell::new(0));
    let counter = attempts.clone();
    let flaky = DeferredContentBuilder::try_new(move |_| {
        counter.set(counter.get() + 1);
        if counter.get() == 1 {
            Err(XamlLoadException::with_message("first"))
        } else {
            Ok(Some(boxed(Button::new())))
        }
    });
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(flaky, &provider);
    assert!(content.try_build_with(None).is_err());
    assert!(content.try_build_with(None).unwrap().result.is_some());
    let _ = builder;
}

#[test]
#[should_panic(expected = "deferred resource failed")]
fn a_failing_builder_panics_through_the_infallible_contract() {
    let provider = declaration_context(vec![], boxed(Border::new()), None);
    let builder = DeferredContentBuilder::try_new(|_| Err(crate::XamlLoadException::with_message("deferred resource failed")));
    let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder, &provider);
    let _ = content.as_deferred_content().build(None);
}

#[test]
fn the_bindings_of_the_binding_extensions_are_bound() {
    use crate::markup_extensions::{CompiledBindingExtension, ReflectionBindingExtension};
    use ferroui_base::data::CompiledBindingPathBuilder;

    register_types();
    let prov = TestServiceProvider::new().sp();
    let property: &'static FerroProperty = Control::tag_property();

    // A reflection binding, as its extension provides it and in its untyped form.
    let border = Border::new();
    border.set_data_context(Some(boxed("context".to_string())));
    let binding = ReflectionBindingExtension::with_path(".").provide_value(&prov);
    XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&boxed(border.clone())),
        Some(&boxed(property)),
        &prov,
        into_markup_value(binding).as_ref(),
    )
    .unwrap();
    assert_eq!(border.tag().unwrap().downcast_ref::<String>().unwrap(), "context");

    // A compiled binding.
    let border = Border::new();
    let extension = CompiledBindingExtension::with_path(CompiledBindingPathBuilder::new().build());
    extension.set_source(Some(boxed("source".to_string())));
    let binding = extension.provide_value(Some(&prov));
    XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&boxed(border.clone())),
        Some(&boxed(property)),
        &prov,
        into_markup_value(binding).as_ref(),
    )
    .unwrap();
    assert_eq!(border.tag().unwrap().downcast_ref::<String>().unwrap(), "source");

    // The extension objects themselves are bindings too.
    let border = Border::new();
    let extension: BoxedValue = extension;
    XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
        Some(&boxed(border.clone())),
        Some(&boxed(property)),
        &prov,
        Some(&extension),
    )
    .unwrap();
    assert_eq!(border.tag().unwrap().downcast_ref::<String>().unwrap(), "source");
}
