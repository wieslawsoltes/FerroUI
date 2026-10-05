//! Tests of the markup extensions against hand-built service providers,
//! and of the service provider lookups they are built on.

use super::*;
use crate::test_support::{boxed, TestServiceProvider};
use crate::{register_types, IProvideValueTarget, ServiceProviderExtensions};
use ferroui_base::controls::{NameScope, NameScopeRef, ResourceDictionary, ResourceHostRef};
use ferroui_base::data::core::{ClrPropertyInfo, IPropertyInfo, Value, ValueType};
use ferroui_base::data::{BindingMode, BindingPriority, RelativeSourceMode, TreeType};
use ferroui_base::media::SolidColorBrush;
use ferroui_base::media::{Colors, IBrush};
use ferroui_base::metadata::{attributes, from_markup_value, into_markup_value, IServiceProvider, MarkupAttributeValue, MarkupTyped};
use ferroui_base::styling::{Style, ThemeVariant};
use ferroui_base::{BoxedValue, FerroProperty, Ref, StaticType, StyledElement, UnsetValueType};
use ferroui_controls::{Border, Button, Control, TextBlock};
use std::cell::RefCell;
use std::rc::Rc;

fn dictionary(entries: &[(&str, BoxedValue)]) -> Ref<ResourceDictionary> {
    let dictionary = ResourceDictionary::new();
    for (key, value) in entries {
        dictionary.add(*key, Some(value.clone()));
    }
    dictionary
}

fn text(value: &Option<BoxedValue>) -> String {
    value.as_ref().and_then(|v| v.downcast_ref::<String>()).cloned().unwrap()
}

fn is_unset(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|v| v.is::<UnsetValueType>())
}

fn brush_color(value: &Option<BoxedValue>) -> ferroui_base::media::Color {
    let brush = from_markup_value::<Rc<dyn IBrush>>(value).expect("a brush");
    brush.as_solid_color_brush().expect("a solid color brush").color()
}

// --- Service provider lookups ---------------------------------------------

#[test]
fn parents_are_found_nearest_first_through_chained_eager_providers() {
    let outer_border = Border::new();
    let inner_border = Border::new();
    let button = Button::new();

    let outer = TestServiceProvider::new().with_parents(vec![boxed(outer_border.clone())]);
    let inner = TestServiceProvider::new()
        .with_parents(vec![boxed(inner_border.clone()), boxed(button.clone()), boxed("text".to_string())])
        .with_outer_parents(outer);
    let sp = inner.sp();

    assert_eq!(sp.get_first_parent::<Ref<Border>>().unwrap(), inner_border);
    assert_eq!(sp.get_last_parent::<Ref<Border>>().unwrap(), outer_border);
    assert_eq!(sp.get_first_parent::<Ref<Control>>().unwrap(), button);
    let borders = sp.get_parents::<Ref<Border>>();
    assert_eq!(borders.len(), 2);
    assert_eq!(borders[0], inner_border);
    assert_eq!(borders[1], outer_border);
    assert!(sp.get_first_parent::<Ref<TextBlock>>().is_none());
}

#[test]
fn parents_are_found_through_a_provider_that_only_enumerates() {
    let outer_border = Border::new();
    let inner_border = Border::new();
    let sp = TestServiceProvider::new()
        .with_parents(vec![boxed(outer_border.clone()), boxed(inner_border.clone()), boxed(Button::new())])
        .lazy()
        .sp();

    assert_eq!(sp.get_first_parent::<Ref<Border>>().unwrap(), inner_border);
    assert_eq!(sp.get_last_parent::<Ref<Border>>().unwrap(), outer_border);
    assert_eq!(sp.get_parents::<Ref<Border>>().len(), 2);
    assert_eq!(sp.get_parents::<Ref<StyledElement>>().len(), 3);
}

#[test]
fn lookups_without_services_find_nothing() {
    let sp = TestServiceProvider::new().sp();
    assert!(sp.get_first_parent::<Ref<Control>>().is_none());
    assert!(sp.get_last_parent::<Ref<Control>>().is_none());
    assert!(sp.get_parents::<Ref<Control>>().is_empty());
    assert!(sp.get_context_base_uri().is_none());
    assert!(sp.get_name_scope().is_none());
    assert!(!sp.is_in_control_template());
    assert!(sp.get_default_anchor().is_none());
}

#[test]
#[should_panic(expected = "hasn't been registered")]
fn required_service_panics_when_missing() {
    let sp = TestServiceProvider::new().sp();
    let _ = sp.get_required_service::<Rc<dyn IProvideValueTarget>>();
}

#[test]
fn context_services_are_read() {
    let scope = NameScopeRef::new(NameScope::new());
    let sp = TestServiceProvider::new()
        .with_base_uri("ferres://app/Views/Main.xaml")
        .with_name_scope(scope.clone())
        .in_control_template()
        .sp();
    assert_eq!(sp.get_context_base_uri().unwrap().absolute_uri(), "ferres://app/Views/Main.xaml");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&sp.get_name_scope().unwrap()), Rc::as_ptr(&scope.0)));
    assert!(sp.is_in_control_template());
}

#[test]
fn default_anchor_is_the_nearest_control() {
    let border = Border::new();
    let button = Button::new();
    let sp = TestServiceProvider::new().with_parents(vec![boxed(border), boxed(button.clone()), boxed(Style::new())]).sp();
    assert!(sp.get_default_anchor().unwrap().ptr_eq(&button));
}

#[test]
fn default_anchor_falls_back_to_a_data_context_provider_then_to_styles() {
    // An element that is not a control provides a data context.
    let element = StyledElement::new();
    let sp = TestServiceProvider::new().with_parents(vec![boxed(Style::new()), boxed(element.clone())]).sp();
    assert!(sp.get_default_anchor().unwrap().ptr_eq(&element));

    // Without one: the root object if it is a style.
    let root = Style::new();
    let inner = Style::new();
    let sp = TestServiceProvider::new()
        .with_parents(vec![boxed(Style::new()), boxed(inner)])
        .with_root(Some(boxed(root.clone())))
        .sp();
    assert!(sp.get_default_anchor().unwrap().ptr_eq(&root));

    // Otherwise the outermost style.
    let outermost = Style::new();
    let sp = TestServiceProvider::new()
        .with_parents(vec![boxed(outermost.clone()), boxed(Style::new())])
        .with_root(Some(boxed(5i32)))
        .sp();
    assert!(sp.get_default_anchor().unwrap().ptr_eq(&outermost));
}

// --- StaticResource --------------------------------------------------------

#[test]
fn static_resource_is_found_in_the_nearest_resource_node() {
    let outer = dictionary(&[("a", boxed("outer a".to_string())), ("b", boxed("outer b".to_string()))]);
    let inner = Border::new();
    inner.resources().add("a", Some(boxed("inner a".to_string())));

    for lazy in [false, true] {
        let provider = TestServiceProvider::new().with_parents(vec![boxed(outer.clone()), boxed(5i32), boxed(inner.clone())]);
        let sp = if lazy { provider.lazy().sp() } else { provider.sp() };

        let a = StaticResourceExtension::with_resource_key(Some(boxed("a".to_string())));
        assert_eq!(text(&a.provide_value(&sp).unwrap()), "inner a");
        let b = StaticResourceExtension::with_resource_key(Some(boxed("b".to_string())));
        assert_eq!(text(&b.provide_value(&sp).unwrap()), "outer b");
    }
}

#[test]
fn static_resource_is_found_through_chained_providers() {
    let outer = TestServiceProvider::new().with_parents(vec![boxed(dictionary(&[("k", boxed(1i32))]))]);
    let sp = TestServiceProvider::new().with_parents(vec![boxed(Border::new())]).with_outer_parents(outer).sp();
    let extension = StaticResourceExtension::with_resource_key(Some(boxed("k".to_string())));
    assert_eq!(extension.provide_value(&sp).unwrap().unwrap().downcast_ref::<i32>(), Some(&1));
}

#[test]
fn static_resource_requires_a_key() {
    let sp = TestServiceProvider::new().sp();
    let error = StaticResourceExtension::new().provide_value(&sp).unwrap_err();
    assert_eq!(error.message(), "StaticResourceExtension.ResourceKey must be set.");
}

#[test]
fn static_resource_that_is_not_found_is_an_error() {
    let sp = TestServiceProvider::new().with_parents(vec![boxed(dictionary(&[]))]).sp();
    let extension = StaticResourceExtension::with_resource_key(Some(boxed("missing".to_string())));
    assert_eq!(extension.provide_value(&sp).unwrap_err().message(), "Static resource 'missing' not found.");

    // Also without a parent stack.
    let sp = TestServiceProvider::new().sp();
    assert!(extension.provide_value(&sp).is_err());
}

#[test]
fn static_resource_keyed_by_type() {
    let resources = ResourceDictionary::new();
    resources.add(<Button as StaticType>::TYPE, Some(boxed("button theme".to_string())));
    let sp = TestServiceProvider::new().with_parents(vec![boxed(resources)]).sp();

    let extension = StaticResourceExtension::with_resource_key(Some(boxed(<Button as StaticType>::TYPE)));
    assert_eq!(text(&extension.provide_value(&sp).unwrap()), "button theme");
}

#[test]
fn static_resource_converts_a_color_for_a_brush_property() {
    let resources = dictionary(&[("accent", boxed(Colors::RED))]);
    let border = Border::new();
    let property: &'static FerroProperty = Border::background_property();
    let sp = TestServiceProvider::new()
        .with_parents(vec![boxed(resources.clone())])
        .with_target(Some(boxed(border)), Some(boxed(property)))
        .sp();
    let extension = StaticResourceExtension::with_resource_key(Some(boxed("accent".to_string())));
    assert_eq!(brush_color(&extension.provide_value(&sp).unwrap()), Colors::RED);

    // For a property of another type the color stays a color.
    let tag: &'static FerroProperty = Control::tag_property();
    let sp = TestServiceProvider::new()
        .with_parents(vec![boxed(resources.clone())])
        .with_target(Some(boxed(Border::new())), Some(boxed(tag)))
        .sp();
    assert!(extension.provide_value(&sp).unwrap().unwrap().is::<ferroui_base::media::Color>());

}

#[test]
fn static_resource_uses_the_theme_variant_of_the_enclosing_theme_dictionary() {
    let themed = ResourceDictionary::new();
    let dark = dictionary(&[("k", boxed("dark".to_string()))]);
    let light = dictionary(&[("k", boxed("light".to_string()))]);
    themed.add_theme_dictionary(ThemeVariant::dark(), dark.clone());
    themed.add_theme_dictionary(ThemeVariant::light(), light);

    // The place the resource is used in is inside the dark dictionary.
    dark.set_key(Some(ThemeVariant::dark()));
    let sp = TestServiceProvider::new().with_parents(vec![boxed(themed.clone()), boxed(dark.clone())]).sp();
    let stack = sp.get_service_of::<Rc<dyn crate::xaml_il::runtime::IFerroXamlIlParentStackProvider>>();
    assert_eq!(StaticResourceExtension::get_dictionary_variant(stack.as_ref()), Some(ThemeVariant::dark()));
    assert_eq!(StaticResourceExtension::get_dictionary_variant(None), None);

    let lazy = TestServiceProvider::new().with_parents(vec![boxed(themed), boxed(dark)]).lazy().sp();
    let stack = lazy.get_service_of::<Rc<dyn crate::xaml_il::runtime::IFerroXamlIlParentStackProvider>>();
    assert_eq!(StaticResourceExtension::get_dictionary_variant(stack.as_ref()), Some(ThemeVariant::dark()));
}

#[test]
fn static_resource_that_is_not_found_yet_is_applied_when_the_control_is_initialized() {
    let border = Border::new();
    border.begin_init();
    let property: &'static FerroProperty = Control::tag_property();
    let sp = TestServiceProvider::new().with_parents(vec![]).with_target(Some(boxed(border.clone())), Some(boxed(property))).sp();

    let extension = StaticResourceExtension::with_resource_key(Some(boxed("late".to_string())));
    let provided = extension.provide_value(&sp).unwrap();
    assert!(is_unset(&provided));

    border.resources().add("late", Some(boxed("found".to_string())));
    border.end_init();
    // An element is initialized when it is rooted; the test stands in for that.
    border.initialize_if_needed();
    assert_eq!(text(&border.tag()), "found");
}

#[test]
fn static_resource_metadata_constructs_and_provides() {
    register_types();
    let markup = <StaticResourceExtension as MarkupTyped>::MARKUP;
    assert_eq!(markup.name, "StaticResourceExtension");
    assert_eq!(markup.namespace(), "FerroUI.Markup.Xaml.MarkupExtensions");

    let default = (markup.constructors[0].invoke)(&[]).unwrap();
    let key = markup.find_property("ResourceKey").unwrap();
    assert_eq!((key.get.unwrap())(&[default.clone()]).unwrap(), None);
    (key.set.unwrap())(&[default.clone(), into_markup_value("k".to_string())]).unwrap();

    let with_key = (markup.constructors[1].invoke)(&[into_markup_value("k".to_string())]).unwrap();
    let sp = TestServiceProvider::new().with_parents(vec![boxed(dictionary(&[("k", boxed(7i32))]))]).sp();
    let provide = markup.find_methods("ProvideValue").next().unwrap();
    assert_eq!((provide.parameters[0])(), ValueType::of::<Rc<dyn IServiceProvider>>());
    for extension in [default, with_key] {
        let value = (provide.invoke)(&[extension, into_markup_value(sp.clone())]).unwrap();
        assert_eq!(value.unwrap().downcast_ref::<i32>(), Some(&7));
    }
}

// --- DynamicResource -------------------------------------------------------

#[test]
fn dynamic_resource_provides_itself_as_the_binding() {
    let extension = DynamicResourceExtension::with_resource_key(Some(boxed("k".to_string())));
    let sp = TestServiceProvider::new().sp();
    let provided = extension.provide_value(&sp);
    assert!(Rc::ptr_eq(&provided, &extension));
    assert_eq!(text(&extension.resource_key()), "k");
}

#[test]
fn dynamic_resource_follows_the_resources_of_its_target() {
    let border = Border::new();
    border.resources().add("brush", Some(boxed(Colors::RED)));
    let property: &'static FerroProperty = Border::background_property();
    let sp = TestServiceProvider::new().with_target(Some(boxed(border.clone())), Some(boxed(property))).sp();

    let extension = DynamicResourceExtension::with_resource_key(Some(boxed("brush".to_string())));
    let binding = extension.provide_value(&sp);
    let expression = border.bind_binding(property, &*binding);

    // A color resource becomes a brush for a brush property.
    assert_eq!(border.background().unwrap().as_solid_color_brush().unwrap().color(), Colors::RED);

    border.resources().set("brush", Some(boxed(Colors::BLUE)));
    assert_eq!(border.background().unwrap().as_solid_color_brush().unwrap().color(), Colors::BLUE);

    border.resources().remove(&"brush".into());
    assert!(border.background().is_none());

    expression.dispose();
    border.resources().add("brush", Some(boxed(Colors::RED)));
    assert!(border.background().is_none());
}

#[test]
fn dynamic_resource_in_a_control_template_binds_with_template_priority() {
    let border = Border::new();
    border.resources().add("tag", Some(boxed("from resource".to_string())));
    let property: &'static FerroProperty = Control::tag_property();
    let sp = TestServiceProvider::new()
        .with_target(Some(boxed(border.clone())), Some(boxed(property)))
        .in_control_template()
        .sp();

    let extension = DynamicResourceExtension::with_resource_key(Some(boxed("tag".to_string())));
    let binding = extension.provide_value(&sp);
    let expression = border.bind_binding(property, &*binding);
    assert_eq!(expression.priority(), BindingPriority::Template);
    assert_eq!(text(&border.tag()), "from resource");

    // A local value wins over a template binding.
    border.set_tag(Some(boxed("local".to_string())));
    assert_eq!(text(&border.tag()), "local");
}

#[test]
fn dynamic_resource_on_a_non_element_target_is_anchored_to_the_nearest_element() {
    let host = Border::new();
    host.resources().add("accent", Some(boxed(Colors::RED)));
    // The target is an object that hosts no resources: they come from the
    // nearest element above the place the resource is used.
    let brush = SolidColorBrush::new();
    let property: &'static FerroProperty = SolidColorBrush::color_property();
    let sp = TestServiceProvider::new()
        .with_parents(vec![boxed(host.clone())])
        .with_target(Some(boxed(brush.clone())), Some(boxed(property)))
        .sp();
    let extension = DynamicResourceExtension::with_resource_key(Some(boxed("accent".to_string())));
    let binding = extension.provide_value(&sp);

    let expression = brush.bind_binding(property, &*binding);
    assert_eq!(brush.color(), Colors::RED);

    host.resources().set("accent", Some(boxed(Colors::BLUE)));
    assert_eq!(brush.color(), Colors::BLUE);
    expression.dispose();
}

#[test]
fn dynamic_resource_without_a_host_publishes_nothing() {
    let brush = SolidColorBrush::new();
    let default = brush.color();
    let property: &'static FerroProperty = SolidColorBrush::color_property();
    let sp = TestServiceProvider::new().with_target(Some(boxed(brush.clone())), Some(boxed(property))).sp();
    let extension = DynamicResourceExtension::with_resource_key(Some(boxed("accent".to_string())));
    let binding = extension.provide_value(&sp);
    let _expression = brush.bind_binding(property, &*binding);
    assert_eq!(brush.color(), default);
}

#[test]
#[should_panic(expected = "DynamicResource must have a ResourceKey.")]
fn dynamic_resource_without_a_key_cannot_be_instantiated() {
    let border = Border::new();
    let property: &'static FerroProperty = Control::tag_property();
    let extension = DynamicResourceExtension::new();
    let _ = border.bind_binding(property, &*extension);
}

#[test]
fn dynamic_resource_metadata_constructs_and_provides_a_binding() {
    register_types();
    let markup = <DynamicResourceExtension as MarkupTyped>::MARKUP;
    let key = markup.find_property("ResourceKey").unwrap();
    assert_eq!(key.attributes[0].name, attributes::CONSTRUCTOR_ARGUMENT);
    assert_eq!(key.attributes[0].arguments[0], MarkupAttributeValue::Str("resourceKey"));

    let extension = (markup.constructors[1].invoke)(&[into_markup_value("tag".to_string())]).unwrap();
    assert!((markup.constructors[0].invoke)(&[]).unwrap().is_some());

    let border = Border::new();
    border.resources().add("tag", Some(boxed(3i32)));
    let property: &'static FerroProperty = Control::tag_property();
    let sp = TestServiceProvider::new().with_target(Some(boxed(border.clone())), Some(boxed(property))).sp();
    let provide = markup.find_methods("ProvideValue").next().unwrap();
    // The extension provides the concrete binding object: itself.
    let binding = (provide.invoke)(&[extension.clone(), into_markup_value(sp)]).unwrap();
    let binding = from_markup_value::<Rc<DynamicResourceExtension>>(&binding).unwrap();
    assert!(Rc::ptr_eq(&binding, &from_markup_value::<Rc<DynamicResourceExtension>>(&extension).unwrap()));
    let _expression = border.bind_binding(property, &*binding);
    assert_eq!(border.tag().unwrap().downcast_ref::<i32>(), Some(&3));

    // The extension itself is assignable where a binding is expected.
    assert!(from_markup_value::<Rc<dyn ferroui_base::data::BindingBase>>(&extension).is_some());
}

// --- ResolveByName ---------------------------------------------------------

#[test]
fn resolve_by_name_returns_the_named_control() {
    let scope = NameScopeRef::new(NameScope::new());
    let button = Button::new();
    scope.register("ok", button.clone().upcast());
    let sp = TestServiceProvider::new().with_name_scope(scope).sp();

    let found = ResolveByNameExtension::new("ok").provide_value(&sp).unwrap();
    // The control is boxed as the handle of its own class.
    assert!(found.is::<Ref<Button>>());
    assert_eq!(from_markup_value::<Ref<Control>>(&Some(found)).unwrap(), button);
}

#[test]
fn resolve_by_name_without_a_name_scope_is_null() {
    let sp = TestServiceProvider::new().sp();
    assert!(ResolveByNameExtension::new("ok").provide_value(&sp).is_none());
}

#[test]
fn resolve_by_name_of_a_missing_name_in_a_completed_scope_is_null() {
    let scope = NameScopeRef::new(NameScope::new());
    scope.complete();
    let sp = TestServiceProvider::new().with_name_scope(scope).sp();
    assert!(ResolveByNameExtension::new("ok").provide_value(&sp).is_none());
}

#[test]
fn resolve_by_name_sets_the_target_once_the_name_is_registered() {
    let scope = NameScopeRef::new(NameScope::new());
    let border = Border::new();
    let button = Button::new();

    // A registered property as the target.
    let property: &'static FerroProperty = Control::tag_property();
    let sp = TestServiceProvider::new()
        .with_name_scope(scope.clone())
        .with_target(Some(boxed(border.clone())), Some(boxed(property)))
        .sp();
    assert!(is_unset(&ResolveByNameExtension::new("late").provide_value(&sp)));
    assert!(border.tag().is_none());

    // A plain property as the target.
    let seen: Rc<RefCell<Option<Ref<Button>>>> = Rc::new(RefCell::new(None));
    let info: Rc<dyn IPropertyInfo> = Rc::new(ClrPropertyInfo::read_write::<Rc<RefCell<Option<Ref<Button>>>>, Value<Option<Ref<Button>>>>(
        "Target",
        |o| o.borrow().clone(),
        |o, v| *o.borrow_mut() = v,
    ));
    let plain = TestServiceProvider::new()
        .with_name_scope(scope.clone())
        .with_target(Some(boxed(seen.clone())), Some(boxed(info)))
        .sp();
    assert!(is_unset(&ResolveByNameExtension::new("late").provide_value(&plain)));

    scope.register("late", button.clone().upcast());
    assert!(border.tag().unwrap().is::<Ref<Button>>());
    assert_eq!(seen.borrow().clone().unwrap(), button);
}

#[test]
fn resolve_by_name_metadata_constructs_and_provides() {
    register_types();
    let markup = <ResolveByNameExtension as MarkupTyped>::MARKUP;
    let extension = (markup.constructors[0].invoke)(&[into_markup_value("ok".to_string())]).unwrap();
    let name = markup.find_property("Name").unwrap();
    assert!(name.set.is_none());
    assert_eq!(text(&(name.get.unwrap())(&[extension.clone()]).unwrap()), "ok");

    let scope = NameScopeRef::new(NameScope::new());
    let button = Button::new();
    scope.register("ok", button.clone().upcast());
    let sp = TestServiceProvider::new().with_name_scope(scope).sp();
    let provide = markup.find_methods("ProvideValue").next().unwrap();
    let found = (provide.invoke)(&[extension, into_markup_value(sp)]).unwrap();
    assert_eq!(from_markup_value::<Ref<Button>>(&found).unwrap(), button);
}

// --- RelativeSource --------------------------------------------------------

#[test]
fn relative_source_extension_provides_a_relative_source() {
    let sp = TestServiceProvider::new().sp();

    let default = RelativeSourceExtension::new().provide_value(&sp);
    assert_eq!(default.mode(), RelativeSourceMode::FindAncestor);
    assert_eq!(default.ancestor_level(), 1);
    assert_eq!(default.tree(), TreeType::Visual);
    assert!(default.ancestor_type().is_none());

    let extension = RelativeSourceExtension::with_mode(RelativeSourceMode::TemplatedParent);
    assert_eq!(extension.provide_value(&sp).mode(), RelativeSourceMode::TemplatedParent);

    extension.set_mode(RelativeSourceMode::FindAncestor);
    extension.set_ancestor_type(Some(<Border as StaticType>::TYPE));
    extension.set_ancestor_level(3);
    extension.set_tree(TreeType::Logical);
    let source = extension.provide_value(&sp);
    assert!(std::ptr::eq(source.ancestor_type().unwrap(), <Border as StaticType>::TYPE));
    assert_eq!(source.ancestor_level(), 3);
    assert_eq!(source.tree(), TreeType::Logical);
    // Every call provides a new relative source.
    assert!(!Rc::ptr_eq(&source, &extension.provide_value(&sp)));
}

#[test]
#[should_panic(expected = "AncestorLevel may not be set to less than 1.")]
fn relative_source_extension_rejects_an_invalid_ancestor_level() {
    let extension = RelativeSourceExtension::new();
    extension.set_ancestor_level(0);
    let _ = extension.provide_value(&TestServiceProvider::new().sp());
}

#[test]
fn relative_source_extension_metadata_constructs_and_provides() {
    use ferroui_base::data::RelativeSource;
    use ferroui_base::metadata::MarkupInvokeError;

    register_types();
    let markup = <RelativeSourceExtension as MarkupTyped>::MARKUP;
    let extension = (markup.constructors[1].invoke)(&[into_markup_value(RelativeSourceMode::SelfMode)]).unwrap();
    let typed = from_markup_value::<Rc<RelativeSourceExtension>>(&extension).unwrap();
    assert_eq!(typed.mode(), RelativeSourceMode::SelfMode);

    let mode = markup.find_property("Mode").unwrap();
    assert_eq!(mode.attributes[0].name, attributes::CONSTRUCTOR_ARGUMENT);
    let level = markup.find_property("AncestorLevel").unwrap();
    (level.set.unwrap())(&[extension.clone(), into_markup_value(2i32)]).unwrap();
    assert_eq!(typed.ancestor_level(), 2);
    let ancestor_type = markup.find_property("AncestorType").unwrap();
    (ancestor_type.set.unwrap())(&[extension.clone(), into_markup_value(<Border as StaticType>::TYPE)]).unwrap();
    assert!(typed.ancestor_type().is_some());

    let sp = TestServiceProvider::new().sp();
    let provide = markup.find_methods("ProvideValue").next().unwrap();
    assert_eq!((provide.return_type.unwrap())(), ValueType::of::<Rc<RelativeSource>>());
    let source = (provide.invoke)(&[extension.clone(), into_markup_value(sp.clone())]).unwrap();
    let source = from_markup_value::<Rc<RelativeSource>>(&source).unwrap();
    assert_eq!(source.mode(), RelativeSourceMode::SelfMode);
    assert_eq!(source.ancestor_level(), 2);

    // The provided relative source is what a reflection binding takes.
    // (an inherited property: declared by the reflection binding class)
    let binding = <ReflectionBindingExtension as MarkupTyped>::MARKUP;
    let reflection = (binding.constructors[0].invoke)(&[]).unwrap();
    let inherited = <ferroui_base::data::ReflectionBinding as MarkupTyped>::MARKUP;
    let relative_source = inherited.find_property("RelativeSource").unwrap();
    (relative_source.set.unwrap())(&[reflection.clone(), into_markup_value(source.clone())]).unwrap();
    let read = (relative_source.get.unwrap())(&[reflection]).unwrap();
    assert!(Rc::ptr_eq(&from_markup_value::<Rc<RelativeSource>>(&read).unwrap(), &source));

    (ancestor_type.set.unwrap())(&[extension.clone(), None]).unwrap();
    assert!(typed.ancestor_type().is_none());
    // An invalid ancestor level is a failed invocation.
    (level.set.unwrap())(&[extension.clone(), into_markup_value(0i32)]).unwrap();
    assert_eq!(
        (provide.invoke)(&[extension, into_markup_value(sp)]),
        Err(MarkupInvokeError::Failed("AncestorLevel may not be set to less than 1.".to_string()))
    );
}

// --- Binding extensions ----------------------------------------------------

#[test]
fn compiled_binding_extension_provides_a_binding_with_the_default_anchor() {
    let button = Button::new();
    let sp = TestServiceProvider::new().with_parents(vec![boxed(button.clone())]).sp();

    let extension = CompiledBindingExtension::new();
    extension.set_mode(BindingMode::TwoWay);
    extension.set_delay(25);
    extension.set_priority(BindingPriority::Style);
    extension.set_string_format(Some("{0}".to_string()));
    extension.set_source(Some(boxed(1i32)));
    extension.set_converter_parameter(Some(boxed(2i32)));
    extension.set_data_type(Some(ValueType::of::<i32>()));
    assert_eq!(extension.data_type(), Some(ValueType::of::<i32>()));

    let binding = extension.provide_value(Some(&sp));
    assert!(!Rc::ptr_eq(&binding, extension.base()));
    assert_eq!(binding.mode(), BindingMode::TwoWay);
    assert_eq!(binding.delay(), 25);
    assert_eq!(binding.priority(), BindingPriority::Style);
    assert_eq!(binding.string_format().as_deref(), Some("{0}"));
    assert_eq!(binding.source().unwrap().downcast_ref::<i32>(), Some(&1));
    assert!(binding.converter_parameter().is_some());
    assert!(binding.path().is_none());
    assert!(binding.default_anchor().unwrap().upgrade().unwrap().ptr_eq(&button));

    // Without a service provider there is no anchor.
    assert!(extension.provide_value(None).default_anchor().is_none());
}

#[test]
fn compiled_binding_extension_metadata_constructs_and_provides() {
    use ferroui_base::data::{BindingBase, CompiledBinding, CompiledBindingPathBuilder};
    use ferroui_base::data::converters::IValueConverter;

    register_types();
    let markup = <CompiledBindingExtension as MarkupTyped>::MARKUP;
    assert_eq!(markup.constructors.len(), 2);
    assert_eq!((markup.base.unwrap())(), ValueType::of::<Rc<CompiledBinding>>());
    let extension = (markup.constructors[0].invoke)(&[]).unwrap();

    // The inherited properties are the ones of the compiled binding class,
    // set on the extension through its base.
    let inherited = <CompiledBinding as MarkupTyped>::MARKUP;
    assert!(markup.find_property("Mode").is_none());
    let mode = inherited.find_property("Mode").unwrap();
    (mode.set.unwrap())(&[extension.clone(), into_markup_value(BindingMode::OneTime)]).unwrap();
    let source = inherited.find_property("Source").unwrap();
    (source.set.unwrap())(&[extension.clone(), into_markup_value("source".to_string())]).unwrap();
    assert_eq!(text(&(source.get.unwrap())(&[extension.clone()]).unwrap()), "source");
    let data_type = markup.find_property("DataType").unwrap();
    (data_type.set.unwrap())(&[extension.clone(), into_markup_value(ValueType::of::<String>())]).unwrap();

    // The path and the converter are read back as they were set.
    let path = inherited.find_property("Path").unwrap();
    assert_eq!((path.get.unwrap())(&[extension.clone()]), Ok(None));
    let compiled_path = CompiledBindingPathBuilder::new().self_().build();
    (path.set.unwrap())(&[extension.clone(), into_markup_value(compiled_path.clone())]).unwrap();
    assert!((path.get.unwrap())(&[extension.clone()]).unwrap().is_some());
    let with_path = (markup.constructors[1].invoke)(&[into_markup_value(compiled_path)]).unwrap();
    assert!(from_markup_value::<Rc<CompiledBindingExtension>>(&with_path).unwrap().path().is_some());
    let converter = inherited.find_property("Converter").unwrap();
    let color_to_brush: Rc<dyn IValueConverter> = crate::converters::ColorToBrushConverter::new();
    (converter.set.unwrap())(&[extension.clone(), into_markup_value(color_to_brush.clone())]).unwrap();
    let read = (converter.get.unwrap())(&[extension.clone()]).unwrap();
    assert!(from_markup_value::<Rc<dyn IValueConverter>>(&read).unwrap() == color_to_brush);
    // A converter object is assignable through its contract.
    let object: BoxedValue = crate::converters::ColorToBrushConverter::new();
    (converter.set.unwrap())(&[extension.clone(), Some(object)]).unwrap();

    let typed = from_markup_value::<Rc<CompiledBindingExtension>>(&extension).unwrap();
    assert_eq!(typed.mode(), BindingMode::OneTime);
    assert_eq!(typed.data_type(), Some(ValueType::of::<String>()));

    // The extension provides the concrete binding object.
    let provide = markup.find_methods("ProvideValue").next().unwrap();
    assert_eq!((provide.return_type.unwrap())(), ValueType::of::<Rc<CompiledBinding>>());
    let sp = TestServiceProvider::new().sp();
    let binding = (provide.invoke)(&[extension.clone(), into_markup_value(sp)]).unwrap();
    let binding = from_markup_value::<Rc<CompiledBinding>>(&binding).unwrap();
    assert_eq!(binding.mode(), BindingMode::OneTime);
    // A null service provider is accepted.
    assert!((provide.invoke)(&[extension.clone(), None]).unwrap().is_some());

    // The extension is a compiled binding and a binding.
    assert!(Rc::ptr_eq(&from_markup_value::<Rc<CompiledBinding>>(&extension).unwrap(), typed.base()));
    assert!(from_markup_value::<Rc<dyn BindingBase>>(&extension).is_some());
    assert!(from_markup_value::<Option<Rc<dyn BindingBase>>>(&into_markup_value(binding)).unwrap().is_some());
}

#[test]
fn reflection_binding_extension_provides_a_binding_with_its_context() {
    let scope = NameScopeRef::new(NameScope::new());
    let button = Button::new();
    let sp = TestServiceProvider::new().with_parents(vec![boxed(button.clone())]).with_name_scope(scope.clone()).sp();

    let extension = ReflectionBindingExtension::with_path("Foo.Bar");
    extension.set_element_name(Some("source".to_string()));
    extension.set_mode(BindingMode::OneWayToSource);
    extension.set_delay(10);

    let binding = extension.provide_value(&sp);
    assert!(!Rc::ptr_eq(&binding, extension.base()));
    assert_eq!(binding.path(), "Foo.Bar");
    assert_eq!(binding.element_name().as_deref(), Some("source"));
    assert_eq!(binding.mode(), BindingMode::OneWayToSource);
    assert_eq!(binding.delay(), 10);
    assert!(binding.type_resolver().is_some());
    assert!(binding.default_anchor().unwrap().upgrade().unwrap().ptr_eq(&button));
    let name_scope = binding.name_scope().unwrap().upgrade().unwrap();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&name_scope), Rc::as_ptr(&scope.0)));

    assert_eq!(ReflectionBindingExtension::new().path(), "");
}

#[test]
fn reflection_binding_extension_binds_through_the_provided_binding() {
    let border = Border::new();
    border.set_data_context(Some(boxed("hello".to_string())));
    let sp = TestServiceProvider::new().with_parents(vec![boxed(border.clone())]).sp();

    let binding = ReflectionBindingExtension::with_path(".").provide_value(&sp);
    let property: &'static FerroProperty = Control::tag_property();
    let _expression = border.bind_binding(property, &binding);
    assert_eq!(text(&border.tag()), "hello");
}

#[test]
fn reflection_binding_extension_metadata_constructs_and_provides() {
    register_types();
    let markup = <ReflectionBindingExtension as MarkupTyped>::MARKUP;
    let extension = (markup.constructors[1].invoke)(&[into_markup_value("Name".to_string())]).unwrap();
    let inherited = <ferroui_base::data::ReflectionBinding as MarkupTyped>::MARKUP;
    let path = inherited.find_property("Path").unwrap();
    assert_eq!(text(&(path.get.unwrap())(&[extension.clone()]).unwrap()), "Name");
    let element_name = inherited.find_property("ElementName").unwrap();
    (element_name.set.unwrap())(&[extension.clone(), into_markup_value("other".to_string())]).unwrap();
    assert_eq!(text(&(element_name.get.unwrap())(&[extension.clone()]).unwrap()), "other");

    let provide = markup.find_methods("ProvideValue").next().unwrap();
    let sp = TestServiceProvider::new().sp();
    let binding = (provide.invoke)(&[extension.clone(), into_markup_value(sp)]).unwrap();
    let binding = from_markup_value::<Rc<ferroui_base::data::ReflectionBinding>>(&binding).unwrap();
    assert_eq!(binding.path(), "Name");
    assert_eq!(binding.element_name().as_deref(), Some("other"));
    // The extension is a reflection binding and a binding.
    assert!(from_markup_value::<Rc<ferroui_base::data::ReflectionBinding>>(&extension).is_some());
    assert!(from_markup_value::<Rc<dyn ferroui_base::data::BindingBase>>(&extension).is_some());
}

// --- Option extensions -----------------------------------------------------

#[test]
fn on_platform_provides_the_option_of_the_current_operating_system() {
    let current = if cfg!(target_os = "windows") {
        "WINDOWS"
    } else if cfg!(target_os = "macos") {
        "OSX"
    } else if cfg!(target_os = "linux") {
        "LINUX"
    } else {
        return;
    };

    for option in ["WINDOWS", "OSX", "LINUX", "ANDROID", "IOS", "BROWSER"] {
        assert_eq!(OnPlatformExtension::should_provide_option(option), option == current, "{option}");
    }
    assert!(!OnPlatformExtension::should_provide_option("NOT-A-PLATFORM"));
    // Unknown options are compared with the name of the platform, ignoring case.
    assert!(OnPlatformExtension::should_provide_option(&std::env::consts::OS.to_uppercase()));
    assert!(OnPlatformExtensionOf::<i32>::should_provide_option(current));
}

#[test]
fn on_platform_holds_its_options() {
    let extension = OnPlatformExtension::with_default(Some(boxed(1i32)));
    extension.set_windows(Some(boxed(2i32)));
    extension.set_mac_os(Some(boxed(3i32)));
    assert_eq!(extension.default().unwrap().downcast_ref::<i32>(), Some(&1));
    assert_eq!(extension.windows().unwrap().downcast_ref::<i32>(), Some(&2));
    assert_eq!(extension.mac_os().unwrap().downcast_ref::<i32>(), Some(&3));
    assert!(extension.linux().is_none() && extension.android().is_none());
    assert!(extension.ios().is_none() && extension.browser().is_none());
    assert!(Rc::ptr_eq(&extension.provide_value(), &extension));

    let typed = OnPlatformExtensionOf::<f64>::with_default(Some(1.5));
    typed.set_linux(Some(2.5));
    assert_eq!(typed.default(), Some(1.5));
    assert_eq!(typed.linux(), Some(2.5));
}

#[test]
fn on_platform_metadata_declares_the_options() {
    register_types();
    let markup = <OnPlatformExtension as MarkupTyped>::MARKUP;
    assert_eq!(markup.name, "OnPlatformExtension");

    let default = markup.find_property("Default").unwrap();
    assert_eq!(default.attributes[0].name, attributes::MARKUP_EXTENSION_DEFAULT_OPTION);
    for (property, option) in [
        ("Windows", "WINDOWS"),
        ("macOS", "OSX"),
        ("Linux", "LINUX"),
        ("Android", "ANDROID"),
        ("iOS", "IOS"),
        ("Browser", "BROWSER"),
    ] {
        let property = markup.find_property(property).unwrap();
        assert_eq!(property.attributes[0].name, attributes::MARKUP_EXTENSION_OPTION);
        assert_eq!(property.attributes[0].arguments[0], MarkupAttributeValue::Str(option));
    }

    let extension = (markup.constructors[1].invoke)(&[into_markup_value(5i32)]).unwrap();
    assert_eq!((default.get.unwrap())(&[extension.clone()]).unwrap().unwrap().downcast_ref::<i32>(), Some(&5));
    let provide = markup.find_methods("ProvideValue").next().unwrap();
    assert!(provide.parameters.is_empty());
    assert!((provide.invoke)(&[extension.clone()]).unwrap().is_some());

    let should = markup.find_methods("ShouldProvideOption").next().unwrap();
    assert!(should.is_static);
    let result = (should.invoke)(&[into_markup_value("NOT-A-PLATFORM".to_string())]).unwrap().unwrap();
    assert_eq!(result.downcast_ref::<bool>(), Some(&false));

    // Children in element syntax are accepted and ignored.
    let on = (<On as MarkupTyped>::MARKUP.constructors[0].invoke)(&[]).unwrap();
    let add_child = markup.find_methods("AddChild").next().unwrap();
    assert_eq!((add_child.invoke)(&[extension, on]), Ok(None));
}

#[test]
fn on_form_factor_holds_its_options_and_declares_them() {
    register_types();
    let extension = OnFormFactorExtension::with_default(Some(boxed(1i32)));
    extension.set_desktop(Some(boxed(2i32)));
    extension.set_mobile(Some(boxed(3i32)));
    extension.set_tv(Some(boxed(4i32)));
    assert_eq!(extension.desktop().unwrap().downcast_ref::<i32>(), Some(&2));
    assert_eq!(extension.mobile().unwrap().downcast_ref::<i32>(), Some(&3));
    assert_eq!(extension.tv().unwrap().downcast_ref::<i32>(), Some(&4));
    assert!(Rc::ptr_eq(&extension.provide_value(), &extension));
    assert_eq!(OnFormFactorExtensionOf::<bool>::new().default(), None);

    let markup = <OnFormFactorExtension as MarkupTyped>::MARKUP;
    assert_eq!(
        markup.find_property("Default").unwrap().attributes[0].name,
        attributes::MARKUP_EXTENSION_DEFAULT_OPTION
    );
    for (property, option) in [("Desktop", 1), ("Mobile", 2), ("TV", 3)] {
        let property = markup.find_property(property).unwrap();
        assert_eq!(property.attributes[0].name, attributes::MARKUP_EXTENSION_OPTION);
        assert_eq!(property.attributes[0].arguments[0], MarkupAttributeValue::Int(option));
    }
    let built = (markup.constructors[0].invoke)(&[]).unwrap();
    assert!((markup.find_methods("ProvideValue").next().unwrap().invoke)(&[built]).unwrap().is_some());
}

#[test]
fn on_holds_options_and_content() {
    register_types();
    let on = On::<BoxedValue>::new();
    on.add_option("WINDOWS");
    on.add_option("LINUX");
    on.set_content(Some(boxed(1i32)));
    assert_eq!(on.options(), ["WINDOWS", "LINUX"]);
    assert_eq!(on.content().unwrap().downcast_ref::<i32>(), Some(&1));

    let typed = On::<f64>::new();
    typed.set_content(Some(2.0));
    assert_eq!(typed.content(), Some(2.0));

    let markup = <On as MarkupTyped>::MARKUP;
    assert_eq!(markup.content_property, Some("Content"));
    let built = (markup.constructors[0].invoke)(&[]).unwrap();
    let content = markup.find_property("Content").unwrap();
    (content.set.unwrap())(&[built.clone(), into_markup_value("x".to_string())]).unwrap();
    assert_eq!(text(&(content.get.unwrap())(&[built]).unwrap()), "x");
}

#[test]
fn resource_hosts_are_recognised_among_parents() {
    let border = Border::new();
    let resources = ResourceDictionary::new();
    let sp = TestServiceProvider::new().with_parents(vec![boxed(border.clone()), boxed(resources)]).sp();
    assert!(sp.get_first_parent::<ResourceHostRef>().is_some());
    assert!(sp.get_first_parent::<Rc<dyn ferroui_base::controls::IResourceProvider>>().is_some());
    assert_eq!(sp.get_parents::<crate::XamlResourceNode>().len(), 2);
}


#[test]
fn failures_of_metadata_members_are_failed_invocations() {
    use ferroui_base::metadata::MarkupInvokeError;

    register_types();
    let sp = TestServiceProvider::new().sp();

    let markup = <StaticResourceExtension as MarkupTyped>::MARKUP;
    let provide = markup.find_methods("ProvideValue").next().unwrap();
    let without_key = (markup.constructors[0].invoke)(&[]).unwrap();
    assert_eq!(
        (provide.invoke)(&[without_key, into_markup_value(sp.clone())]),
        Err(MarkupInvokeError::Failed("StaticResourceExtension.ResourceKey must be set.".to_string()))
    );
    let missing = (markup.constructors[1].invoke)(&[into_markup_value("missing".to_string())]).unwrap();
    assert_eq!(
        (provide.invoke)(&[missing, into_markup_value(sp.clone())]),
        Err(MarkupInvokeError::Failed("Static resource 'missing' not found.".to_string()))
    );

    // The runtime helpers and the loader.
    let helpers = <crate::xaml_il::runtime::XamlIlRuntimeHelpers as MarkupTyped>::MARKUP;
    let factory = helpers.find_methods("DeferredTransformationFactoryV3").next().unwrap();
    let builder = crate::xaml_il::runtime::DeferredContentBuilder::new(|_| None);
    let result = (factory.invoke)(&[into_markup_value(builder), into_markup_value(sp.clone())]);
    assert_eq!(
        result,
        Err(MarkupInvokeError::Failed("Service IFerroXamlIlParentStackProvider hasn't been registered".to_string()))
    );
    let apply = helpers.find_methods("ApplyNonMatchingMarkupExtensionV1").next().unwrap();
    let result = (apply.invoke)(&[into_markup_value(1i32), None, into_markup_value(sp.clone()), into_markup_value(2i32)]);
    assert_eq!(result, Err(MarkupInvokeError::Failed("Don't know what to do with i32".to_string())));

    let loader = <crate::FerroXamlLoader as MarkupTyped>::MARKUP;
    for load in loader.find_methods("Load").filter(|m| m.return_type.is_none()) {
        let arguments: Vec<_> = load.parameters.iter().map(|_| None).collect();
        assert_eq!(
            (load.invoke)(&arguments),
            Err(MarkupInvokeError::Failed("Value cannot be null. (Parameter 'obj')".to_string()))
        );
    }
    // No asset loader, and a relative URI without a base URI.
    let scope = ferroui_base::FerroLocator::enter_scope();
    let load = loader.find_methods("Load").find(|m| m.parameters.len() == 2 && m.return_type.is_some()).unwrap();
    let result = (load.invoke)(&[into_markup_value(crate::test_support::uri("A.xaml")), None]);
    assert!(matches!(result, Err(MarkupInvokeError::Failed(message)) if message.starts_with("Could not create IAssetLoader")));
    let _assets = crate::test_support::TestAssetLoader::new().install();
    let result = (load.invoke)(&[into_markup_value(crate::test_support::uri("A.xaml")), None]);
    assert_eq!(result, Err(MarkupInvokeError::Failed("Cannot load relative Uri when BaseUri is null".to_string())));
    scope.dispose();

    // Templates.
    use crate::templates::{ItemsPanelTemplate, TemplateContent, WindowDrawnDecorationsTemplate};
    let load = <TemplateContent as MarkupTyped>::MARKUP.find_methods("Load").next().unwrap();
    assert!(matches!(
        (load.invoke)(&[into_markup_value("text".to_string())]),
        Err(MarkupInvokeError::Failed(message)) if message.starts_with("Unexpected content")
    ));
    let markup = <WindowDrawnDecorationsTemplate as MarkupTyped>::MARKUP;
    let template = (markup.constructors[0].invoke)(&[]).unwrap();
    assert!(matches!(
        (markup.find_methods("Build").next().unwrap().invoke)(&[template]),
        Err(MarkupInvokeError::Failed(message)) if message.starts_with("Operation is not valid")
    ));
    let root = Border::new();
    let declared = TestServiceProvider::new().with_parents(vec![]).with_root(Some(boxed(root))).sp();
    let not_a_panel = crate::xaml_il::runtime::DeferredContentBuilder::new(|_| Some(boxed(Button::new())));
    let content = (factory.invoke)(&[into_markup_value(not_a_panel), into_markup_value(declared)]).unwrap();
    let markup = <ItemsPanelTemplate as MarkupTyped>::MARKUP;
    let template = (markup.constructors[0].invoke)(&[]).unwrap();
    (markup.find_property("Content").unwrap().set.unwrap())(&[template.clone(), content]).unwrap();
    assert!(matches!(
        (markup.find_methods("Build").next().unwrap().invoke)(&[template]),
        Err(MarkupInvokeError::Failed(message)) if message.ends_with("to type 'Panel'.")
    ));

    // Source information.
    let info = <crate::diagnostics::XamlSourceInfo as MarkupTyped>::MARKUP;
    let set = info.find_methods("SetXamlSourceInfo").find(|m| m.parameters.len() == 3).unwrap();
    let result = (set.invoke)(&[into_markup_value(5i32), into_markup_value("k".to_string()), None]);
    assert!(matches!(result, Err(MarkupInvokeError::Argument { index: 0, .. })));
    let dictionary = ferroui_base::controls::ResourceDictionary::new();
    let result = (set.invoke)(&[into_markup_value(dictionary), None, None]);
    assert_eq!(result, Err(MarkupInvokeError::Failed("Value cannot be null. (Parameter 'key')".to_string())));
}

#[test]
fn value_knowledge_registered_on_one_thread_applies_on_another() {
    // Registered here (or by any other test, on its thread) ...
    register_types();

    // ... and used on a thread that never called `register_types()`.
    std::thread::spawn(|| {
        use crate::templates::ControlTemplate;
        use ferroui_controls::templates::IControlTemplate;

        // A reference type: constructed through metadata in its untyped
        // form and cast back to its handle and to the contract it implements.
        let markup = <ControlTemplate as MarkupTyped>::MARKUP;
        let template = (markup.constructors[0].invoke)(&[]).unwrap();
        assert!(template.as_ref().unwrap().is::<ControlTemplate>());
        assert!(from_markup_value::<Rc<ControlTemplate>>(&template).is_some());
        assert!(from_markup_value::<Option<Rc<dyn IControlTemplate>>>(&template).unwrap().is_some());

        // Nullable arguments and instance casts of members.
        let target_type = markup.find_property("TargetType").unwrap();
        (target_type.set.unwrap())(&[template.clone(), None]).unwrap();
        (target_type.set.unwrap())(&[template.clone(), into_markup_value(<Button as StaticType>::TYPE)]).unwrap();
        assert!((target_type.get.unwrap())(&[template]).unwrap().is_some());

        let markup = <StaticResourceExtension as MarkupTyped>::MARKUP;
        let extension = (markup.constructors[1].invoke)(&[into_markup_value("k".to_string())]).unwrap();
        let key = markup.find_property("ResourceKey").unwrap();
        assert!((key.get.unwrap())(&[extension]).unwrap().is_some());
    })
    .join()
    .unwrap();
}

#[test]
fn static_resource_takes_the_target_type_from_the_property_of_a_setter() {
    use ferroui_base::styling::Setter;

    let resources = dictionary(&[("accent", boxed(Colors::RED))]);
    let property: &'static FerroProperty = Border::background_property();
    let setter = Setter::empty();
    setter.set_property(Some(property));
    let sp = TestServiceProvider::new().with_parents(vec![boxed(resources)]).with_target(Some(boxed(setter)), None).sp();
    let extension = StaticResourceExtension::with_resource_key(Some(boxed("accent".to_string())));
    assert_eq!(brush_color(&extension.provide_value(&sp).unwrap()), Colors::RED);
}

#[test]
fn on_form_factor_asks_the_runtime_platform_of_the_service_provider() {
    use ferroui_base::platform::{FormFactorType, IRuntimePlatform, RuntimePlatformInfo};
    use ferroui_base::{FerroLocator, LocatorExtensions};

    struct Mobile;
    impl IRuntimePlatform for Mobile {
        fn get_runtime_info(&self) -> RuntimePlatformInfo {
            RuntimePlatformInfo { is_mobile: true, ..RuntimePlatformInfo::default() }
        }
    }

    register_types();
    // A provider without the service provides no option.
    let none = TestServiceProvider::new().sp();
    assert!(!OnFormFactorExtension::should_provide_option(&none, FormFactorType::Mobile));

    // The root and the deferred service providers offer the registered platform.
    let scope = FerroLocator::enter_scope();
    let platform: Rc<dyn IRuntimePlatform> = Rc::new(Mobile);
    FerroLocator::current_mutable().bind::<dyn IRuntimePlatform>().to_constant(platform);
    assert!(FerroLocator::current().get_service::<dyn IRuntimePlatform>().is_some());

    let root = crate::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v2();
    assert!(OnFormFactorExtension::should_provide_option(&root, FormFactorType::Mobile));
    assert!(!OnFormFactorExtension::should_provide_option(&root, FormFactorType::Desktop));
    assert!(!OnFormFactorExtension::should_provide_option(&root, FormFactorType::TV));
    assert!(OnFormFactorExtensionOf::<i32>::should_provide_option(&root, FormFactorType::Mobile));

    let seen = Rc::new(std::cell::Cell::new(false));
    let flag = seen.clone();
    let declared = TestServiceProvider::new().with_parents(vec![]).with_root(Some(boxed(1i32))).sp();
    let builder = crate::xaml_il::runtime::DeferredContentBuilder::new(move |sp| {
        flag.set(OnFormFactorExtension::should_provide_option(sp, FormFactorType::Mobile));
        None
    });
    let content = crate::xaml_il::runtime::XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<BoxedValue>(builder, &declared);
    content.build_with(None);
    assert!(seen.get());

    // Through metadata.
    let markup = <OnFormFactorExtension as MarkupTyped>::MARKUP;
    let should = markup.find_methods("ShouldProvideOption").next().unwrap();
    assert!(should.is_static);
    let result = (should.invoke)(&[into_markup_value(root), into_markup_value(FormFactorType::Mobile)]).unwrap();
    assert_eq!(result.unwrap().downcast_ref::<bool>(), Some(&true));
    scope.dispose();
}

#[test]
fn a_root_service_provider_without_a_registered_platform_reports_it() {
    use crate::xaml_il::runtime::{RuntimePlatformNotRegistered, XamlIlRuntimeHelpers};
    use ferroui_base::metadata::MarkupInvokeError;
    use ferroui_base::platform::{FormFactorType, IRuntimePlatform};
    use ferroui_base::FerroLocator;

    register_types();
    // No platform is registered in this scope.
    let scope = FerroLocator::enter_scope();
    let root = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    assert!(root.get_service_of::<Rc<dyn IRuntimePlatform>>().is_none());
    assert!(root.get_service_of::<RuntimePlatformNotRegistered>().is_some());
    let error = XamlIlRuntimeHelpers::get_runtime_platform(&root).err().unwrap();
    assert_eq!(error.message(), "IRuntimePlatform was not registered");
    assert_eq!(
        OnFormFactorExtension::try_should_provide_option(&root, FormFactorType::Desktop).unwrap_err().message(),
        "IRuntimePlatform was not registered"
    );

    // Through metadata it is a failed invocation, not a panic.
    let should = <OnFormFactorExtension as MarkupTyped>::MARKUP.find_methods("ShouldProvideOption").next().unwrap();
    assert_eq!(
        (should.invoke)(&[into_markup_value(root), into_markup_value(FormFactorType::Desktop)]),
        Err(MarkupInvokeError::Failed("IRuntimePlatform was not registered".to_string()))
    );

    // A provider that simply does not know the service provides no option.
    let other = TestServiceProvider::new().sp();
    assert_eq!(XamlIlRuntimeHelpers::get_runtime_platform(&other).map(|p| p.is_some()), Ok(false));
    assert_eq!(OnFormFactorExtension::try_should_provide_option(&other, FormFactorType::Desktop), Ok(false));
    scope.dispose();
}

#[test]
#[should_panic(expected = "IRuntimePlatform was not registered")]
fn asking_a_root_service_provider_without_a_platform_for_an_option_panics() {
    let _scope = ferroui_base::FerroLocator::enter_scope();
    let root = crate::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v2();
    let _ = OnFormFactorExtension::should_provide_option(&root, ferroui_base::platform::FormFactorType::Mobile);
}

#[test]
fn the_binding_extensions_give_back_their_concrete_type() {
    use ferroui_base::data::BindingBase;

    let dynamic = DynamicResourceExtension::new();
    let binding: Rc<dyn BindingBase> = dynamic.as_binding();
    assert!(std::ptr::eq(binding.as_any().unwrap().downcast_ref::<DynamicResourceExtension>().unwrap(), &*dynamic));

    let compiled = CompiledBindingExtension::new();
    let binding: Rc<dyn BindingBase> = compiled.clone();
    assert!(std::ptr::eq(binding.as_any().unwrap().downcast_ref::<CompiledBindingExtension>().unwrap(), &*compiled));

    let reflection = ReflectionBindingExtension::new();
    let binding: Rc<dyn BindingBase> = reflection.clone();
    assert!(std::ptr::eq(binding.as_any().unwrap().downcast_ref::<ReflectionBindingExtension>().unwrap(), &*reflection));
}

/// The branches of a static resource that is not found, as in the managed original: the
/// lookup is delayed (and the unset marker provided) only when the target is a control and
/// the target property is a property description, which a registered property is too;
/// everything else is the "not found" error.
#[test]
fn a_static_resource_that_is_not_found_is_delayed_for_a_control_property_and_an_error_otherwise() {
    use ferroui_base::data::core::IPropertyInfo;
    use ferroui_base::media::SolidColorBrush;
    use ferroui_base::styling::Setter;

    let extension = StaticResourceExtension::with_resource_key(Some(boxed("Missing".to_string())));
    let registered: &'static FerroProperty = Control::tag_property();
    let description: Rc<dyn IPropertyInfo> = registered.as_property_info();
    let not_found = "Static resource 'Missing' not found.";

    // A registered property of a control, and the same through a property description:
    // delayed; with the parent stack of a template or of a deferred resource as well.
    for (case, parents, property) in [
        ("registered property", vec![], boxed(registered)),
        ("property description", vec![], boxed(description.clone())),
        ("inside a template", vec![boxed(Border::new()), boxed(dictionary(&[]))], boxed(registered)),
    ] {
        let border = Border::new();
        border.begin_init();
        let sp = TestServiceProvider::new().with_parents(parents).with_target(Some(boxed(border.clone())), Some(property)).sp();
        assert!(is_unset(&extension.provide_value(&sp).unwrap()), "{case}");
        // The delayed lookup finds the resource when the control is initialised.
        border.resources().add("Missing", Some(boxed("late".to_string())));
        border.end_init();
        border.initialize_if_needed();
        assert_eq!(text(&border.tag()), "late", "{case}");
    }

    // A target that is not a control (a brush of a deferred resource, a setter): an error.
    let brush = SolidColorBrush::new();
    let color: &'static FerroProperty = SolidColorBrush::color_property();
    let sp = TestServiceProvider::new()
        .with_parents(vec![boxed(dictionary(&[]))])
        .with_target(Some(boxed(brush)), Some(boxed(color)))
        .sp();
    assert_eq!(extension.provide_value(&sp).unwrap_err().message(), not_found);
    let sp = TestServiceProvider::new().with_target(Some(boxed(Setter::empty())), Some(boxed(description))).sp();
    assert_eq!(extension.provide_value(&sp).unwrap_err().message(), not_found);

    // A control without a target property, and no provide-value target at all: an error.
    let sp = TestServiceProvider::new().with_target(Some(boxed(Border::new())), None).sp();
    assert_eq!(extension.provide_value(&sp).unwrap_err().message(), not_found);
    let sp = TestServiceProvider::new().with_parents(vec![boxed(Border::new())]).sp();
    assert_eq!(extension.provide_value(&sp).unwrap_err().message(), not_found);
}

#[test]
fn resolve_by_name_provides_the_element_as_the_target_property_stores_it() {
    use ferroui_base::ElementRef;
    use ferroui_controls::primitives::Popup;
    use ferroui_controls::Label;

    // `Popup.PlacementTarget` holds a reference to an element it does not own.
    let scope = NameScopeRef::new(NameScope::new());
    let border = Border::new();
    scope.register("Background", border.clone().upcast());
    let popup = Popup::new();
    let property: &'static FerroProperty = Popup::placement_target_property();
    let sp = TestServiceProvider::new()
        .with_name_scope(scope.clone())
        .with_target(Some(boxed(popup.clone())), Some(boxed(property)))
        .sp();
    let provided = ResolveByNameExtension::new("Background").provide_value(&sp).unwrap();
    assert!(provided.is::<Option<ElementRef<Control>>>(), "{}", provided.type_name());
    popup.set_value_untyped(property, provided.as_any(), ferroui_base::data::BindingPriority::LocalValue);
    assert_eq!(popup.placement_target().unwrap(), border.clone().upcast::<Control>());

    // A name that is registered later is set on the property in the same form.
    let popup = Popup::new();
    let sp = TestServiceProvider::new()
        .with_name_scope(scope.clone())
        .with_target(Some(boxed(popup.clone())), Some(boxed(property)))
        .sp();
    assert!(is_unset(&ResolveByNameExtension::new("Later").provide_value(&sp)));
    let button = Button::new();
    scope.register("Later", button.clone().upcast());
    assert_eq!(popup.placement_target().unwrap(), button.clone().upcast::<Control>());

    // `Label.Target` and a property of any object.
    let label = Label::new();
    let target: &'static FerroProperty = Label::target_property();
    let sp = TestServiceProvider::new()
        .with_name_scope(scope.clone())
        .with_target(Some(boxed(label.clone())), Some(boxed(target)))
        .sp();
    let provided = ResolveByNameExtension::new("Background").provide_value(&sp).unwrap();
    label.set_value_untyped(target, provided.as_any(), ferroui_base::data::BindingPriority::LocalValue);
    assert!(label.target().is_some());
    let tag: &'static FerroProperty = Control::tag_property();
    let sp = TestServiceProvider::new().with_name_scope(scope).with_target(Some(boxed(label)), Some(boxed(tag))).sp();
    assert!(ResolveByNameExtension::new("Background").provide_value(&sp).unwrap().is::<Ref<Border>>());
}

