//! Port of `tests/Themes.UnitTests/ControlThemeTests.cs`.
//!
//! Not ported: the parts of `Should_Define_Setters_With_Valid_Dynamic_Resources`
//! and `Should_Not_Define_Setters_With_Hardcoded_Brushes_Or_Colors` that
//! inspect the applied value frames of template children (the value store
//! diagnostics they read are not ported); the setters of the styles are
//! checked.

use super::support::*;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{BoxShadows, Color, Colors, IBrush};
use ferroui_base::metadata::{from_markup_value, into_markup_value, MarkupAttributeValue};
use ferroui_base::styling::{ControlTheme, Setter, SetterValue, StyleBase, ThemeVariant};
use ferroui_base::{BoxedValue, FerroObject, Ref, StyledElement, TypeInfo};
use ferroui_controls::chrome::{IWindowDrawnDecorationsTemplate, WindowDrawnDecorations};
use ferroui_controls::primitives::{
    HeaderedItemsControl, HeaderedSelectingItemsControl, SelectingItemsControl,
    TemplatedControl, Thumb,
};
use ferroui_controls::templates::IControlTemplate;
use ferroui_controls::{ContentControl, Control, NativeMenuBar, TopLevel, UserControl, Window};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

fn default_control_themes(theme: &Ref<crate::FluentTheme>) -> Vec<(ResourceKey, Ref<ControlTheme>)> {
    enumerate_resources(&theme.as_style())
        .into_iter()
        // Default ControlThemes must not be defined per specific ThemeVariant.
        .filter(|entry| entry.theme_variant == ThemeVariant::default())
        .filter_map(|entry| entry.control_theme().map(|theme| (entry.key, theme)))
        .collect()
}

#[test]
fn should_not_contain_any_global_styles() {
    // For performance reasons, we want all styles to be enclosed in ControlThemes,
    // without selectors that apply globally.
    let _app = start_application();
    let theme = create_attached_theme();
    let all_styles = enumerate_styles(&theme.as_style());

    assert!(all_styles.iter().all(|style| style.parent().is_some()));
}

#[test]
fn should_define_control_theme_for_built_in_templated_controls() {
    let _app = start_application();
    let ignored_controls: [&'static TypeInfo; 8] = [
        Control::TYPE,
        TemplatedControl::TYPE,
        ContentControl::TYPE,
        NativeMenuBar::TYPE,
        HeaderedItemsControl::TYPE,
        HeaderedSelectingItemsControl::TYPE,
        SelectingItemsControl::TYPE,
        Thumb::TYPE,
    ];

    let mut templated_controls: Vec<&'static TypeInfo> = Vec::new();
    // WindowDrawnDecorations is the only StyleElement that is not Control but has ControlTheme
    templated_controls.push(WindowDrawnDecorations::TYPE);
    for type_ in TypeInfo::registered_types() {
        // Resolve all public non-abstract TemplatedControls of the controls assembly.
        // Technically, any StyledElement can have a control theme,
        // but templated control are ones that won't work without one.
        if !type_.module_path().starts_with("ferroui_controls")
            || type_.module_path().contains("::testing")
            || !TemplatedControl::TYPE.is_assignable_from(type_)
            || type_.default_constructor().is_none()
        {
            continue;
        }
        // Respect StyleKeyOverride. The managed test reads the style key of an uninitialised
        // object; here an instance is created, except of top levels (whose constructors need a
        // platform implementation and whose style key is their own type).
        let style_key = if TopLevel::TYPE.is_assignable_from(type_) {
            type_
        } else {
            let instance = type_.create_instance().expect("a default constructor");
            instance.cast::<StyledElement>().map(|element| element.style_key()).unwrap_or(type_)
        };
        // Filter common types that we don't style
        if UserControl::TYPE.is_assignable_from(style_key)
            || (Window::TYPE.is_assignable_from(style_key) && !std::ptr::eq(style_key, Window::TYPE))
            || ignored_controls.iter().any(|ignored| std::ptr::eq(*ignored, style_key))
            || templated_controls.iter().any(|known| std::ptr::eq(*known, style_key))
        {
            continue;
        }
        templated_controls.push(style_key);
    }

    let theme = create_attached_theme();
    let default_control_themes: Vec<&'static TypeInfo> = default_control_themes(&theme)
        .into_iter()
        .filter_map(|(key, _)| if let ResourceKey::Type(type_) = key { Some(type_) } else { None })
        .collect();

    assert!(templated_controls.len() > 1);
    // Every templated control has a control theme, except the ones whose document is left out of
    // the theme for now (`Controls/excluded.txt`); a control with a theme is not in that list.
    for control in &templated_controls {
        let themed = default_control_themes.iter().any(|themed| std::ptr::eq(*themed, *control));
        // The themes of the picker presenters are part of the document of their picker.
        let document = match control.name() {
            "DatePickerPresenter" => "DatePicker",
            "TimePickerPresenter" => "TimePicker",
            name => name,
        };
        let excluded = crate::assets::is_excluded(&format!("/Controls/{document}.xaml"));
        assert_eq!(!excluded, themed, "{}", control.name());
    }
}

/// The value of a setter as a control template (`value is IControlTemplate`).
fn control_template(value: &BoxedValue) -> Option<Rc<dyn IControlTemplate>> {
    value
        .downcast_ref::<Option<Rc<dyn IControlTemplate>>>()
        .cloned()
        .flatten()
        .or_else(|| from_markup_value::<Rc<dyn IControlTemplate>>(&Some(value.clone())))
}

/// The value of a setter as a decorations template.
fn decorations_template(value: &BoxedValue) -> Option<Rc<dyn IWindowDrawnDecorationsTemplate>> {
    value
        .downcast_ref::<Option<Rc<dyn IWindowDrawnDecorationsTemplate>>>()
        .cloned()
        .flatten()
        .or_else(|| from_markup_value::<Rc<dyn IWindowDrawnDecorationsTemplate>>(&Some(value.clone())))
}

/// `ControlThemeExtensions.ResolveTemplate`.
fn resolve_template(style: &Ref<StyleBase>, target_type: &'static TypeInfo) -> Option<BoxedValue> {
    let template = style.setters().snapshot().iter().find_map(|setter| {
        let setter = setter.as_any()?.downcast_ref::<Setter>()?;
        let property = setter.property()?;
        if !std::ptr::eq(property, TemplatedControl::template_property().as_property())
            && !std::ptr::eq(property, WindowDrawnDecorations::template_property().as_property())
        {
            return None;
        }
        match setter.value()? {
            SetterValue::Value(value) => {
                let is_template = control_template(&value).is_some() || decorations_template(&value).is_some();
                if is_template { Some(value) } else { None }
            }
            _ => None,
        }
    });
    if template.is_some() {
        return template;
    }

    // ContentControl derived controls inherit its template.
    if ContentControl::TYPE.is_assignable_from(target_type) {
        let default_template = TemplatedControl::template_property()
            .get_default_value(ContentControl::TYPE)
            .expect("ContentControl must always have default template");
        return into_markup_value(default_template);
    }

    // If we don't find any template in the current StyleBase we look for one in its BaseOn style
    let based_on = style.cast::<ControlTheme>().and_then(|theme| theme.based_on())?;
    resolve_template(&based_on.upcast(), target_type)
}

#[test]
fn should_define_all_requested_template_parts() {
    let _app = start_application();
    let theme = create_attached_theme();
    let default_control_themes = default_control_themes(&theme);

    // AutoCompleteBox has optional PART_SelectionAdapter, that was never defined in templates for "historical reasons".
    // ScrollBar, SplitView and Slider define templates per specific pseudoclasses, making it harder to test.
    let control_types_to_skip: [(&str, &[&str]); 4] =
        [("AutoCompleteBox", &["PART_SelectionAdapter"]), ("ScrollBar", &[]), ("SplitView", &[]), ("Slider", &[])];

    assert!(!default_control_themes.is_empty());
    let mut checked = 0;
    for (key, control_theme) in default_control_themes {
        // Only the default control themes: the ones with a type as their key.
        if !matches!(key, ResourceKey::Type(_)) {
            continue;
        }
        let target_type = control_theme.target_type().expect("a control theme has a target type");

        // TemplatePart can be IsRequired=false, if it's not essential for the control to function.
        // But for built-in default themes we expect all of them to be present.
        // Exception is optional template parts from the base class, that were inherited and ignored by the control.
        let mut requested = requested_parts(target_type);
        if let Some((_, skip_parts)) = control_types_to_skip.iter().find(|(name, _)| *name == target_type.name()) {
            if skip_parts.is_empty() {
                continue;
            }
            requested.retain(|part| {
                !matches!(part.arguments.first(), Some(MarkupAttributeValue::Str(name)) if skip_parts.contains(name))
            });
        }
        if requested.is_empty() {
            continue;
        }
        // While a document is left out of the theme, the templates that name one of its resources
        // cannot be built (the static resource is not found).
        if blocked_by_excluded_document(target_type.name()) {
            continue;
        }

        let template = resolve_template(&control_theme.clone().upcast(), target_type)
            .unwrap_or_else(|| panic!("{} has no template", target_type.name()));
        let name_scope = if let Some(template) = control_template(&template) {
            let control = TemplatedControl::new();
            template.build(&control).expect("a template result").name_scope().clone()
        } else {
            decorations_template(&template).expect("a template").build_typed().name_scope().clone()
        };

        for part in requested {
            let Some(MarkupAttributeValue::Str(name)) = part.arguments.first() else { panic!("a part without a name") };
            let found: Option<Ref<FerroObject>> = name_scope.find(name);
            let found = found.unwrap_or_else(|| panic!("{}: the part {name} is not in the template", target_type.name()));
            if let Some(MarkupAttributeValue::Type(part_type)) = part.arguments.get(1) {
                let part_type = part_type();
                assert!(
                    is_instance_of(&found, part_type.name()),
                    "{}: the part {name} is a {}, not a {}",
                    target_type.name(),
                    found.get_type().name(),
                    part_type.name()
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0);
}

/// Whether a document that is left out of the theme defines the resource `key`.
fn defined_in_excluded_document(key: &ResourceKey) -> bool {
    let Some(key) = key.as_str() else { return false };
    let definition = format!("x:Key=\"{key}\"");
    crate::excluded_documents().iter().any(|excluded| {
        crate::assets::document(excluded.path)
            .and_then(|content| std::str::from_utf8(content).ok())
            .is_some_and(|text| text.contains(&definition))
    })
}

fn setters_of(style: &Ref<StyleBase>) -> Vec<Rc<dyn ferroui_base::styling::SetterBase>> {
    style.setters().snapshot().iter().cloned().collect()
}

#[test]
fn should_define_setters_with_valid_dynamic_resources() {
    // Validate that <Setter Value="{DynamicResource}"> points to an actually defined resource.
    let _app = start_application();
    let theme = create_attached_theme();
    let all_resources = enumerate_resources(&theme.as_style());
    let default_keys: Vec<&ResourceKey> =
        all_resources.iter().filter(|r| r.theme_variant == ThemeVariant::default()).map(|r| &r.key).collect();

    let control_themes = default_control_themes(&theme);
    assert!(!control_themes.is_empty());
    let mut dynamic_resources = 0;
    for (_, control_theme) in control_themes {
        for style in enumerate_styles(&control_theme.clone().into()) {
            for setter in setters_of(&style) {
                let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else { continue };
                let Some(SetterValue::BindingBase(binding)) = setter.value() else { continue };
                let Some(extension) = binding.as_any().and_then(|any| any.downcast_ref::<DynamicResourceExtension>())
                else {
                    continue;
                };
                let key = extension.resource_key().expect("a resource key");
                let key = Some(key);
                let key = match from_markup_value::<String>(&key) {
                    Some(text) => ResourceKey::from(text),
                    None => ResourceKey::Type(from_markup_value::<&'static TypeInfo>(&key).expect("a string or type key")),
                };
                // A resource of a document that is left out of the theme for now
                // (`Controls/excluded.txt`) is defined once that document is included.
                let defined = default_keys.contains(&&key) || defined_in_excluded_document(&key);
                assert!(defined, "the dynamic resource {key:?} is not defined");
                dynamic_resources += 1;
            }
        }
    }
    assert!(dynamic_resources > 0);
}

#[test]
fn should_not_define_setters_with_hardcoded_brushes_or_colors() {
    let _app = start_application();
    let theme = create_attached_theme();
    let control_themes = default_control_themes(&theme);

    fn is_transparent_or_empty(color: Color) -> bool {
        color == Colors::TRANSPARENT || color == Color::default()
    }

    assert!(!control_themes.is_empty());
    for (_, control_theme) in control_themes {
        for style in enumerate_styles(&control_theme.clone().into()) {
            for setter in setters_of(&style) {
                let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else { continue };
                let Some(SetterValue::Value(value)) = setter.value() else { continue };
                let value = Some(value);
                let hardcoded = if let Some(color) = from_markup_value::<Color>(&value) {
                    !is_transparent_or_empty(color)
                } else if let Some(brush) = from_markup_value::<Rc<dyn IBrush>>(&value) {
                    brush.as_solid_color_brush().is_none_or(|solid| !is_transparent_or_empty(solid.color()))
                } else {
                    from_markup_value::<BoxShadows>(&value).is_some()
                };
                assert!(
                    !hardcoded,
                    "{}: the setter of {} has a hardcoded brush or colour",
                    control_theme.target_type().map(|t| t.name()).unwrap_or_default(),
                    setter.property().map(|p| p.name()).unwrap_or_default()
                );
            }
        }
    }
}
