//! Tests of the dynamic resource expression, through the bindings that
//! create it.

use super::*;
use crate::test_support::boxed;
use ferroui_base::controls::{ResourceDictionary, ResourceHostRef, ResourceKey};
use ferroui_base::data::BindingPriority;
use ferroui_base::media::{Colors, SolidColorBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{FerroProperty, Ref};
use ferroui_controls::{Border, Control};
use std::rc::Rc;

fn key(text: &str) -> ResourceKey {
    text.into()
}

fn tag_text(control: &Control) -> Option<String> {
    control.tag().and_then(|v| v.downcast_ref::<String>().cloned())
}

#[test]
fn describes_itself_by_its_key() {
    use ferroui_base::data::core::UntypedBindingExpression;

    let expression = DynamicResourceExpression::new(key("Accent"), None, None, BindingPriority::Template);
    assert_eq!(expression.description(), "DynamicResource Accent");
    assert_eq!(expression.base().default_priority(), BindingPriority::Template);
    assert!(!expression.base().is_running());
    assert!(expression.resource_key() == &key("Accent"));
}

#[test]
fn publishes_the_resource_of_the_target_and_follows_its_changes() {
    let border = Border::new();
    let property: &'static FerroProperty = Control::tag_property();
    // Nothing is published before the expression is attached and started.
    let unattached = DynamicResourceExpression::new(key("text"), None, None, BindingPriority::LocalValue);
    border.resources().add("text", Some(boxed("first".to_string())));
    assert_eq!(tag_text(&border), None);
    drop(unattached);

    let expression = DynamicResourceExpression::new(key("text"), None, None, BindingPriority::LocalValue);
    let instance = border.bind_binding(property, &ExpressionBinding(expression));
    assert_eq!(tag_text(&border).as_deref(), Some("first"));

    border.resources().set("text", Some(boxed("second".to_string())));
    assert_eq!(tag_text(&border).as_deref(), Some("second"));

    // A removed resource is no value: the property falls back to its default.
    border.resources().remove(&key("text"));
    assert_eq!(tag_text(&border), None);
    border.resources().add("text", Some(boxed("third".to_string())));
    assert_eq!(tag_text(&border).as_deref(), Some("third"));

    instance.dispose();
    border.resources().set("text", Some(boxed("ignored".to_string())));
    assert_eq!(tag_text(&border), None);
}

/// A binding that instantiates one prepared expression.
struct ExpressionBinding(Rc<DynamicResourceExpression>);

impl ferroui_base::data::BindingBase for ExpressionBinding {
    fn create_instance(
        &self,
        _target: &ferroui_base::FerroObject,
        _target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<ferroui_base::FerroObject>>,
    ) -> Rc<dyn ferroui_base::data::BindingExpressionBase> {
        self.0.clone()
    }
}

#[test]
fn a_provider_anchor_follows_its_owner() {
    // The anchor is a dictionary that is not hosted yet.
    let dictionary = ResourceDictionary::new();
    let provider: Rc<dyn ferroui_base::controls::IResourceProvider> = dictionary.clone().into();
    let brush = SolidColorBrush::new();
    let default = brush.color();
    let property: &'static FerroProperty = SolidColorBrush::color_property();

    let expression = DynamicResourceExpression::new(
        key("accent"),
        Some(DynamicResourceAnchor::Provider(provider)),
        None,
        BindingPriority::LocalValue,
    );
    let instance = brush.bind_binding(property, &ExpressionBinding(expression));
    assert_eq!(brush.color(), default);

    // When the dictionary gets an owner, the resources of the owner are used.
    let host = Border::new();
    host.resources().add("accent", Some(boxed(Colors::RED)));
    let owner: ResourceHostRef = host.clone().into();
    dictionary.add_owner(&owner);
    assert_eq!(brush.color(), Colors::RED);

    host.resources().set("accent", Some(boxed(Colors::BLUE)));
    assert_eq!(brush.color(), Colors::BLUE);

    // Without an owner there is no value again.
    dictionary.remove_owner(&owner);
    assert_eq!(brush.color(), default);
    host.resources().set("accent", Some(boxed(Colors::RED)));
    assert_eq!(brush.color(), default);

    instance.dispose();
}

#[test]
fn the_theme_variant_of_a_theme_dictionary_overrides_the_one_of_the_host() {
    let themed = ResourceDictionary::new();
    themed.add_theme_dictionary(ThemeVariant::dark(), dictionary_with("accent", "dark"));
    themed.add_theme_dictionary(ThemeVariant::light(), dictionary_with("accent", "light"));
    let host = Border::new();
    host.resources().add_merged_dictionary(themed);

    let anchor = ResourceDictionary::new();
    let provider: Rc<dyn ferroui_base::controls::IResourceProvider> = anchor.clone().into();
    let owner: ResourceHostRef = host.clone().into();
    anchor.add_owner(&owner);

    for (variant, expected) in [(ThemeVariant::dark(), "dark"), (ThemeVariant::light(), "light")] {
        let target = Border::new();
        let property: &'static FerroProperty = Control::tag_property();
        // The target is an element without the resource; the anchor is used
        // only when the target is no resource host, so bind an object.
        let _ = (&target, property);
        let holder = Holder::new();
        let expression = DynamicResourceExpression::new(
            key("accent"),
            Some(DynamicResourceAnchor::Provider(provider.clone())),
            Some(variant),
            BindingPriority::LocalValue,
        );
        let instance = holder.bind_binding(Holder::text_property().as_property(), &ExpressionBinding(expression));
        assert_eq!(holder.text(), expected);
        instance.dispose();
    }
}

fn dictionary_with(key: &str, value: &str) -> Ref<ResourceDictionary> {
    let dictionary = ResourceDictionary::new();
    dictionary.add(key, Some(boxed(value.to_string())));
    dictionary
}

/// An object of the class model that hosts no resources.
#[repr(C)]
struct Holder {
    base: ferroui_base::FerroObject,
}

use ferroui_base::FerroObject;
ferroui_base::ferro_class!(Holder: FerroObject);
ferroui_base::ferro_impl_classes!(Holder: ferroui_base::FerroObjectImpl);

ferroui_base::ferro_properties! {
    impl Holder {
        fn text_property() -> ferroui_base::StyledProperty<String> {
            FerroProperty::register::<Holder, _>("Text", String::new())
        }
    }
}

impl Holder {
    fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self { base: ferroui_base::FerroObject::construct() })
    }

    fn text(&self) -> String {
        self.get_value(Self::text_property())
    }
}

#[test]
fn an_element_or_host_anchor_is_the_resource_host() {
    let host = Border::new();
    host.resources().add("text", Some(boxed("from element".to_string())));

    let holder = Holder::new();
    let expression = DynamicResourceExpression::new(
        key("text"),
        Some(DynamicResourceAnchor::Element(host.clone().upcast())),
        None,
        BindingPriority::LocalValue,
    );
    let instance = holder.bind_binding(Holder::text_property().as_property(), &ExpressionBinding(expression));
    assert_eq!(holder.text(), "from element");
    instance.dispose();

    let holder = Holder::new();
    let expression = DynamicResourceExpression::new(
        key("text"),
        Some(DynamicResourceAnchor::Host(host.clone().into())),
        None,
        BindingPriority::LocalValue,
    );
    let _instance = holder.bind_binding(Holder::text_property().as_property(), &ExpressionBinding(expression));
    assert_eq!(holder.text(), "from element");
    host.resources().set("text", Some(boxed("changed".to_string())));
    assert_eq!(holder.text(), "changed");

    // A resource of another type than the property is not a value for it.
    host.resources().set("text", Some(boxed(Colors::RED)));
    assert_eq!(holder.text(), "");
}

// --- `{DynamicResource Key}` on a brush-typed property, applied as the
// --- compiler's binding setter applies it: `target.Bind(property, binding)`.

mod brush_property {
    use super::*;
    use crate::markup_extensions::DynamicResourceExtension;
    use crate::test_support::TestServiceProvider;
    use ferroui_base::data::BindingExpressionBase;
    use ferroui_base::media::{Brushes, Color, IBrush};
    use ferroui_base::BoxedValue;

    fn background_color(border: &Border) -> Option<Color> {
        border.background().map(|brush| brush.as_solid_color_brush().expect("a solid color brush").color())
    }

    /// Provides the extension for `Background` of `border` (target and
    /// parent stack as the compiler passes them) and binds the result.
    fn bind_background(border: &Ref<Border>, key: &str, parents: Vec<BoxedValue>) -> Rc<dyn BindingExpressionBase> {
        let property: &'static FerroProperty = Border::background_property();
        let mut stack = parents;
        stack.push(boxed(border.clone()));
        let sp = TestServiceProvider::new()
            .with_parents(stack)
            .with_target(Some(boxed(border.clone())), Some(boxed(property)))
            .sp();
        let binding = DynamicResourceExtension::with_resource_key(Some(boxed(key.to_string()))).provide_value(&sp);
        border.bind_binding(property, &binding)
    }

    #[test]
    fn a_brush_object_resource_reaches_the_property() {
        let border = Border::new();
        let brush = SolidColorBrush::with_color(Colors::RED);
        border.resources().add("brush", Some(boxed(brush.clone())));

        let _expression = bind_background(&border, "brush", vec![]);
        assert_eq!(background_color(&border), Some(Colors::RED));
        // It is the resource itself, not a copy: a change of the brush shows.
        brush.set_color(Colors::BLUE);
        assert_eq!(background_color(&border), Some(Colors::BLUE));
    }

    #[test]
    fn brush_resources_of_every_boxing_form_reach_the_property() {
        let immutable: Rc<dyn IBrush> = Brushes::red();
        let resources: Vec<(&str, BoxedValue)> = vec![
            ("object handle", boxed(SolidColorBrush::with_color(Colors::RED))),
            ("contract handle", boxed(immutable.clone())),
            // What markup creates for `<SolidColorBrush>#..</SolidColorBrush>`: the handle of
            // the concrete immutable brush, which is a brush only through its registered cast.
            ("concrete immutable brush handle", boxed(Brushes::red())),
            ("nullable concrete immutable brush handle", boxed(Some(Brushes::red()))),
            ("nullable contract handle", boxed(Some(immutable))),
            ("color", boxed(Colors::RED)),
        ];
        for (form, resource) in resources {
            let border = Border::new();
            border.resources().add("brush", Some(resource));
            let _expression = bind_background(&border, "brush", vec![]);
            assert_eq!(background_color(&border), Some(Colors::RED), "{form}");
        }
    }

    #[test]
    fn a_resource_added_after_the_binding_reaches_the_property() {
        let border = Border::new();
        let _expression = bind_background(&border, "brush", vec![]);
        assert_eq!(background_color(&border), None);

        border.resources().add("brush", Some(boxed(SolidColorBrush::with_color(Colors::RED))));
        assert_eq!(background_color(&border), Some(Colors::RED));

        border.resources().set("brush", Some(boxed(SolidColorBrush::with_color(Colors::BLUE))));
        assert_eq!(background_color(&border), Some(Colors::BLUE));
    }

    #[test]
    fn a_removed_resource_clears_the_property() {
        let border = Border::new();
        border.resources().add("brush", Some(boxed(SolidColorBrush::with_color(Colors::RED))));
        let expression = bind_background(&border, "brush", vec![]);
        assert_eq!(background_color(&border), Some(Colors::RED));

        border.resources().remove(&key("brush"));
        assert_eq!(background_color(&border), None);

        // After the binding is disposed nothing arrives any more.
        expression.dispose();
        border.resources().add("brush", Some(boxed(SolidColorBrush::with_color(Colors::RED))));
        assert_eq!(background_color(&border), None);
    }

    #[test]
    fn a_resource_of_a_parent_reaches_the_property() {
        let parent = Border::new();
        parent.resources().add("brush", Some(boxed(SolidColorBrush::with_color(Colors::RED))));
        let border = Border::new();
        parent.set_child(&border);

        let _expression = bind_background(&border, "brush", vec![boxed(parent.clone())]);
        assert_eq!(background_color(&border), Some(Colors::RED));
    }

    #[test]
    fn a_resource_of_the_application_reaches_a_non_element_target_through_the_anchor() {
        let application = ferroui_controls::Application::new();
        application.resources().add("accent", Some(boxed(Colors::RED)));

        // The target is a brush (it hosts no resources): the nearest resource
        // host above the place the resource is used is the application.
        let brush = SolidColorBrush::new();
        let property: &'static FerroProperty = SolidColorBrush::color_property();
        let sp = TestServiceProvider::new()
            .with_parents(vec![boxed(application.clone())])
            .with_target(Some(boxed(brush.clone())), Some(boxed(property)))
            .sp();
        let binding = DynamicResourceExtension::with_resource_key(Some(boxed("accent".to_string()))).provide_value(&sp);
        let _expression = brush.bind_binding(property, &binding);
        assert_eq!(brush.color(), Colors::RED);

        application.resources().set("accent", Some(boxed(Colors::BLUE)));
        assert_eq!(brush.color(), Colors::BLUE);
    }

    #[test]
    fn a_theme_variant_change_selects_the_resource_of_the_new_variant() {
        let border = Border::new();
        let themed = ResourceDictionary::new();
        let dark = ResourceDictionary::new();
        dark.add("brush", Some(boxed(SolidColorBrush::with_color(Colors::BLUE))));
        let light = ResourceDictionary::new();
        light.add("brush", Some(boxed(SolidColorBrush::with_color(Colors::RED))));
        themed.add_theme_dictionary(ThemeVariant::dark(), dark);
        themed.add_theme_dictionary(ThemeVariant::light(), light);
        border.resources().add_merged_dictionary(themed);

        border.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));
        let _expression = bind_background(&border, "brush", vec![]);
        assert_eq!(background_color(&border), Some(Colors::RED));

        border.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
        assert_eq!(background_color(&border), Some(Colors::BLUE));

        border.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));
        assert_eq!(background_color(&border), Some(Colors::RED));
    }
}
