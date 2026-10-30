//! The rules of the theme tests of the upstream project
//! (`tests/Themes.UnitTests/ControlThemeTests.cs`, ported for the Fluent theme in
//! `src/FerroUI.Themes.Fluent/tests/control_theme_tests.rs`), for the macOS theme.
//!
//! Differences from the tests of the Fluent theme:
//! - Every templated control has a default control theme or is in
//!   [`PENDING_CONTROLS`]; the pending ones are reported, not failed, and a
//!   control of the list that has a theme fails the test, so that the list is
//!   emptied as the batches port the controls.
//! - The default control themes of the port are based on the keyed theme of
//!   the WinUI style (`{x:Type Button}` on `DefaultButtonStyle`), so the
//!   template of a theme is looked for in its base before the default template
//!   of a content control, and the checks of the setters cover every control
//!   theme of the theme, keyed ones included.

use super::support::*;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{BoxShadows, Color, Colors, IBrush};
use ferroui_base::metadata::{from_markup_value, into_markup_value, MarkupAttributeValue};
use ferroui_base::styling::{ControlTheme, Setter, SetterValue, StyleBase, ThemeVariant};
use ferroui_base::{BoxedValue, FerroObject, Ref, StyledElement, TypeInfo};
use ferroui_controls::chrome::{IWindowDrawnDecorationsTemplate, WindowDrawnDecorations};
use ferroui_controls::primitives::{
    HeaderedItemsControl, HeaderedSelectingItemsControl, SelectingItemsControl, TemplatedControl, Thumb,
};
use ferroui_controls::templates::IControlTemplate;
use ferroui_controls::{ContentControl, Control, NativeMenuBar, TopLevel, UserControl, Window};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

/// The control themes of the theme that are not defined per theme variant, with their keys.
fn control_themes(theme: &Ref<crate::MacOSTheme>) -> Vec<(ResourceKey, Ref<ControlTheme>)> {
    enumerate_resources(&theme.as_style())
        .into_iter()
        // Control themes must not be defined per specific ThemeVariant.
        .filter(|entry| entry.theme_variant == ThemeVariant::default())
        .filter_map(|entry| entry.control_theme().map(|theme| (entry.key, theme)))
        .collect()
}

/// The default control themes: the ones keyed by a type.
fn default_control_themes(theme: &Ref<crate::MacOSTheme>) -> Vec<(ResourceKey, Ref<ControlTheme>)> {
    control_themes(theme).into_iter().filter(|(key, _)| matches!(key, ResourceKey::Type(_))).collect()
}

/// The global styles the theme has on purpose: the icon of a menu item is a logical child of the
/// item, which neither a style of the item's template nor a nested style of a control theme can
/// name (the nesting selector drops what precedes it), so the theme scales it as WinUI's with a
/// style of its own (DEVIATIONS.md, PI-3). It targets a type, which keeps it cheap to match.
const GLOBAL_STYLES: [&str; 1] = ["MenuItem > PathIcon"];

#[test]
fn should_not_contain_any_global_styles() {
    // For performance reasons, we want all styles to be enclosed in ControlThemes,
    // without selectors that apply globally.
    let _app = start_application();
    let theme = create_attached_theme();
    let all_styles = enumerate_styles(&theme.as_style());

    let global: Vec<String> = all_styles
        .iter()
        .filter(|style| style.parent().is_none())
        .map(|style| style.downcast_ref::<ferroui_base::styling::Style>().and_then(|s| s.selector()).map(|s| s.to_string()).unwrap_or_default())
        .collect();
    assert_eq!(GLOBAL_STYLES.to_vec(), global, "the global styles of the theme");
}

/// The templated controls of the controls and the dialogs assemblies, by their style keys, as
/// the Fluent theme test collects them.
fn templated_controls() -> Vec<&'static TypeInfo> {
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
        if !(type_.module_path().starts_with("ferroui_controls") || type_.module_path().starts_with("ferroui_dialogs"))
            || type_.module_path().contains("::testing")
            || !TemplatedControl::TYPE.is_assignable_from(type_)
            || type_.default_constructor().is_none()
        {
            continue;
        }
        let style_key = if TopLevel::TYPE.is_assignable_from(type_) {
            type_
        } else {
            let instance = type_.create_instance().expect("a default constructor");
            instance.cast::<StyledElement>().map(|element| element.style_key()).unwrap_or(type_)
        };
        if UserControl::TYPE.is_assignable_from(style_key)
            || (Window::TYPE.is_assignable_from(style_key) && !std::ptr::eq(style_key, Window::TYPE))
            || ignored_controls.iter().any(|ignored| std::ptr::eq(*ignored, style_key))
            || templated_controls.iter().any(|known| std::ptr::eq(*known, style_key))
        {
            continue;
        }
        templated_controls.push(style_key);
    }
    templated_controls
}

#[test]
fn should_define_control_theme_for_built_in_templated_controls() {
    let _app = start_application();
    let templated_controls = templated_controls();
    let theme = create_attached_theme();
    let themed: Vec<&'static TypeInfo> = default_control_themes(&theme)
        .into_iter()
        .filter_map(|(key, _)| if let ResourceKey::Type(type_) = key { Some(type_) } else { None })
        .collect();

    assert!(templated_controls.len() > 1);
    let mut missing = Vec::new();
    let mut pending = Vec::new();
    for control in &templated_controls {
        let has_theme = themed.iter().any(|themed| std::ptr::eq(*themed, *control));
        match (has_theme, is_pending(control.name())) {
            (true, true) => panic!("{} has a control theme: remove it from PENDING_CONTROLS", control.name()),
            (false, true) => pending.push(control.name()),
            (false, false) => missing.push(control.name()),
            (true, false) => {}
        }
    }
    pending.sort_unstable();
    println!("controls without a control theme yet ({}, pending in a batch): {}", pending.len(), pending.join(", "));
    assert!(missing.is_empty(), "templated controls without a control theme that are not pending: {}", missing.join(", "));
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

/// `ControlThemeExtensions.ResolveTemplate`, with the base of the theme before the default
/// template of a content control (see the header of the module).
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
                if is_template {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        }
    });
    if template.is_some() {
        return template;
    }

    if let Some(based_on) = style.cast::<ControlTheme>().and_then(|theme| theme.based_on()) {
        if let Some(template) = resolve_template(&based_on.upcast(), target_type) {
            return Some(template);
        }
    }

    // ContentControl derived controls inherit its template.
    if ContentControl::TYPE.is_assignable_from(target_type) {
        let default_template = TemplatedControl::template_property()
            .get_default_value(ContentControl::TYPE)
            .expect("ContentControl must always have default template");
        return into_markup_value(default_template);
    }
    None
}

#[test]
fn should_define_all_requested_template_parts() {
    let _app = start_application();
    let theme = create_attached_theme();
    let default_control_themes = default_control_themes(&theme);

    // ScrollBar and Slider define their templates per pseudo-class (:vertical, :horizontal), as in
    // the Fluent theme, which makes them harder to test here; tests/control_templates.rs checks
    // their parts in a window.
    let control_types_to_skip: [&str; 2] = ["ScrollBar", "Slider"];

    assert!(!default_control_themes.is_empty());
    let mut checked = 0;
    for (_, control_theme) in default_control_themes {
        let target_type = control_theme.target_type().expect("a control theme has a target type");
        if control_types_to_skip.contains(&target_type.name()) {
            continue;
        }
        let mut requested = requested_parts(target_type);
        // AutoCompleteBox has the optional part PART_SelectionAdapter, which no template of
        // FerroUI's themes defines (as the tests of the Fluent theme skip it).
        if target_type.name() == "AutoCompleteBox" {
            requested.retain(|part| {
                !matches!(part.arguments.first(), Some(MarkupAttributeValue::Str(name)) if *name == "PART_SelectionAdapter")
            });
        }
        if requested.is_empty() {
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

fn setters_of(style: &Ref<StyleBase>) -> Vec<Rc<dyn ferroui_base::styling::SetterBase>> {
    style.setters().snapshot().iter().cloned().collect()
}

#[test]
fn should_define_setters_with_valid_dynamic_resources() {
    // Validate that <Setter Value="{DynamicResource}"> points to an actually defined resource.
    let _app = start_application();
    let theme = create_attached_theme();
    let control_themes = control_themes(&theme);
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
                let key = Some(extension.resource_key().expect("a resource key"));
                let key = match from_markup_value::<String>(&key) {
                    Some(text) => ResourceKey::from(text),
                    None => ResourceKey::Type(from_markup_value::<&'static TypeInfo>(&key).expect("a string or type key")),
                };
                // The macOS theme also supplies live size, accent and accessibility resources
                // through its resource provider. Resolve them as a real consumer does, for
                // every variant, instead of assuming all keys are static dictionary entries.
                for variant in variants() {
                    assert!(theme.try_get_resource(&key, Some(&variant)).is_some(),
                        "the dynamic resource {key:?} is not defined for {variant:?}");
                }
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
    let control_themes = control_themes(&theme);

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
                    // Clearing a shadow for an edge placement or a disabled control is
                    // semantic, like clearing a brush to Transparent. Actual shadows
                    // still have to come from shared theme resources.
                    from_markup_value::<BoxShadows>(&value).is_some_and(|shadows| shadows.count() != 0)
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
