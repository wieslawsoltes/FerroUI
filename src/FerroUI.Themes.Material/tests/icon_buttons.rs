//! Native icon/FAB geometry and state contracts exercised through real themes and peers.

use super::support::start_themed_application;
use crate::MaterialRipple;
use ferroui_base::controls::ResourceKey;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Color, Colors, FlowDirection};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ControlTheme;
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::automation::peers::{
    AutomationControlType, ControlAutomationPeer, ToggleButtonAutomationPeer,
};
use ferroui_controls::automation::provider::ToggleState;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Application, Border, Button, Control, Window};
use std::cell::Cell;
use std::rc::Rc;

fn theme(key: &str) -> Ref<ControlTheme> {
    let value = Application::current()
        .unwrap()
        .try_get_resource(&ResourceKey::from(key), None)
        .flatten();
    from_markup_value::<Ref<ControlTheme>>(&value)
        .unwrap_or_else(|| panic!("missing icon/FAB resource {key}"))
}

fn color(control: &Control, key: &str) -> Color {
    let value = control
        .try_find_resource(&ResourceKey::from(key), control.actual_theme_variant().as_ref())
        .flatten();
    from_markup_value::<Color>(&value).unwrap_or_else(|| panic!("missing color {key}"))
}

fn host(control: &Control) -> Ref<Window> {
    let frame = Border::new();
    frame.set_horizontal_alignment(HorizontalAlignment::Left);
    frame.set_vertical_alignment(VerticalAlignment::Top);
    frame.set_child(control.to_ref());
    let window = Window::new();
    window.set_width(480.0);
    window.set_height(320.0);
    window.set_content(Some(Control::boxed(&frame)));
    window.show();
    window.update_layout();
    window
}

fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|node| node.name().as_deref() == Some(name))
        .find_map(|node| node.cast::<T>())
        .unwrap_or_else(|| panic!("missing icon/FAB part {name}"))
}

#[test]
fn every_icon_size_and_width_preserves_native_container_and_minimum_touch_target() {
    let _app = start_themed_application();
    // Values follow the effective Small/Medium/Large icon-button token files.
    for (size, height, normal_width, narrow_width, wide_width, icon_size) in [
        ("extraSmall", 32.0, 32.0, 28.0, 40.0, 20.0),
        ("small", 40.0, 40.0, 32.0, 52.0, 24.0),
        ("medium", 56.0, 56.0, 48.0, 72.0, 24.0),
        ("large", 96.0, 96.0, 64.0, 128.0, 32.0),
        ("extraLarge", 136.0, 136.0, 104.0, 184.0, 40.0),
    ] {
        for (width_class, width) in [
            ("", normal_width),
            ("narrow", narrow_width),
            ("wide", wide_width),
        ] {
            let button = Button::new();
            button.set_theme(theme("MaterialFilledIconButton"));
            button.classes().add(size);
            if !width_class.is_empty() {
                button.classes().add(width_class);
            }
            let window = host(&button);
            let container: Ref<Border> = named(&button, "MaterialIconContainer");
            let icon: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
            assert_eq!(width, container.bounds().width, "{size} {width_class}");
            assert_eq!(height, container.bounds().height, "{size} {width_class}");
            assert_eq!(icon_size, icon.bounds().width, "{size}");
            assert_eq!(height / 2.0, container.corner_radius().top_left);
            assert_eq!(width.max(48.0), button.bounds().width);
            assert_eq!(height.max(48.0), button.bounds().height);
            window.close();
        }
    }
}

#[test]
fn selected_shapes_swap_round_and_square_and_pressed_shape_takes_precedence() {
    let _app = start_themed_application();
    for (size, height, square, pressed) in [
        ("extraSmall", 32.0, 12.0, 8.0),
        ("small", 40.0, 12.0, 8.0),
        ("medium", 56.0, 16.0, 12.0),
        ("large", 96.0, 28.0, 16.0),
        ("extraLarge", 136.0, 28.0, 16.0),
    ] {
        let button = ToggleButton::new();
        button.set_theme(theme("MaterialTonalIconToggleButton"));
        button.classes().add(size);
        let window = host(&button);
        let container: Ref<Border> = named(&button, "MaterialIconContainer");
        button.set_is_checked(Some(true));
        assert_eq!(square, container.corner_radius().top_left, "{size}");
        button.pseudo_classes().set(":pressed", true);
        assert_eq!(
            pressed,
            container.corner_radius().top_left,
            "pressed {size}"
        );
        button.pseudo_classes().set(":pressed", false);
        assert_eq!(square, container.corner_radius().top_left);
        button.classes().add("square");
        assert_eq!(
            height / 2.0,
            container.corner_radius().top_left,
            "selected square {size}"
        );
        button.set_is_checked(Some(false));
        assert_eq!(
            square,
            container.corner_radius().top_left,
            "resting square {size}"
        );
        window.close();
    }
}

#[test]
fn icon_families_use_correct_container_content_and_selected_roles() {
    let _app = start_themed_application();
    for (key, unchecked_fill, unchecked_content, checked_fill, checked_content) in [
        (
            "MaterialIconToggleButton",
            None,
            "MaterialOnSurfaceVariantColor",
            None,
            "MaterialPrimaryColor",
        ),
        (
            "MaterialFilledIconToggleButton",
            Some("MaterialSurfaceContainerColor"),
            "MaterialOnSurfaceVariantColor",
            Some("MaterialPrimaryColor"),
            "MaterialOnPrimaryColor",
        ),
        (
            "MaterialTonalIconToggleButton",
            Some("MaterialSecondaryContainerColor"),
            "MaterialOnSecondaryContainerColor",
            Some("MaterialSecondaryColor"),
            "MaterialOnSecondaryColor",
        ),
        (
            "MaterialOutlinedIconToggleButton",
            None,
            "MaterialOnSurfaceVariantColor",
            Some("MaterialInverseSurfaceColor"),
            "MaterialInverseOnSurfaceColor",
        ),
    ] {
        let button = ToggleButton::new();
        button.set_theme(theme(key));
        let window = host(&button);
        let initial_fill = button
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color();
        assert_eq!(
            unchecked_fill.map_or(Colors::TRANSPARENT, |key| color(&button, key)),
            initial_fill,
            "{key}"
        );
        assert_eq!(
            color(&button, unchecked_content),
            button
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            "{key}"
        );
        button.set_is_checked(Some(true));
        assert_eq!(
            checked_fill.map_or(Colors::TRANSPARENT, |key| color(&button, key)),
            button
                .background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            "selected {key}"
        );
        assert_eq!(
            color(&button, checked_content),
            button
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            "selected {key}"
        );
        if key == "MaterialOutlinedIconToggleButton" {
            assert_eq!(0.0, button.border_thickness().left);
        }
        window.close();
    }
}

#[test]
fn disabled_icon_roles_are_independent_and_never_fade_the_whole_content_twice() {
    let _app = start_themed_application();
    for key in [
        "MaterialFilledIconToggleButton",
        "MaterialTonalIconToggleButton",
        "MaterialOutlinedIconToggleButton",
    ] {
        let button = ToggleButton::new();
        button.set_theme(theme(key));
        button.set_is_checked(Some(true));
        button.set_is_enabled(false);
        button.pseudo_classes().set(":pointerover", true);
        let window = host(&button);
        let surface: Ref<Border> = named(&button, "MaterialIconContainer");
        let state: Ref<Border> = named(&button, "MaterialIconStateLayer");
        let ripple: Ref<MaterialRipple> = named(&button, "MaterialIconRipple");
        assert_eq!(1.0, surface.opacity());
        assert_eq!(0.1, surface.background().unwrap().opacity(), "{key}");
        assert_eq!(0.38, button.foreground().unwrap().opacity(), "{key}");
        assert_eq!(0.0, state.opacity());
        assert!(!ripple.is_visible());
        window.close();
    }
    let outline_button = Button::new();
    outline_button.set_theme(theme("MaterialOutlinedIconButton"));
    outline_button.classes().add("extraLarge");
    outline_button.set_is_enabled(false);
    let _window = host(&outline_button);
    let outline: Ref<Border> = named(&outline_button, "MaterialIconOutline");
    assert_eq!(3.0, outline.border_thickness().left);
    assert_eq!(0.38, outline.opacity());
}

#[test]
fn icon_toggle_preserves_click_toggle_and_accessibility_contracts() {
    let _app = start_themed_application();
    let button = ToggleButton::new();
    button.set_theme(theme("MaterialFilledIconToggleButton"));
    AutomationProperties::set_name(&button, Some("Favorite"));
    let clicks = Rc::new(Cell::new(0));
    let count = clicks.clone();
    button.click(move |_, _| count.set(count.get() + 1));
    let _window = host(&button);
    let peer = ControlAutomationPeer::create_peer_for_element(&button);
    assert_eq!(
        AutomationControlType::Button,
        peer.get_automation_control_type()
    );
    assert_eq!("Favorite", peer.get_name());
    let peer = peer
        .cast::<ToggleButtonAutomationPeer>()
        .expect("native toggle peer");
    peer.toggle().unwrap();
    assert_eq!(Some(true), button.is_checked());
    assert_eq!(ToggleState::On, peer.toggle_state());
    peer.toggle().unwrap();
    assert_eq!(Some(false), button.is_checked());
    assert_eq!(2, clicks.get());
    button.set_is_enabled(false);
    assert!(peer.toggle().is_err());
    assert_eq!(2, clicks.get());
}

#[test]
fn fab_variants_use_effective_icon_sizes_and_do_not_take_icon_button_press_shapes() {
    let _app = start_themed_application();
    for (key, painted, target, icon_size, radius) in [
        ("MaterialSmallFab", 40.0, 48.0, 24.0, 12.0),
        ("MaterialFloatingActionButton", 56.0, 56.0, 24.0, 16.0),
        ("MaterialMediumFab", 80.0, 80.0, 28.0, 20.0),
        ("MaterialLargeFab", 96.0, 96.0, 36.0, 28.0),
    ] {
        let button = Button::new();
        button.set_theme(theme(key));
        let window = host(&button);
        let surface: Ref<Border> = named(&button, "MaterialIconContainer");
        let icon: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
        assert_eq!(painted, surface.bounds().width, "{key}");
        assert_eq!(painted, surface.bounds().height, "{key}");
        assert_eq!(target, button.bounds().width, "{key}");
        assert_eq!(icon_size, icon.bounds().width, "{key}");
        assert_eq!(radius, surface.corner_radius().top_left);
        button.pseudo_classes().set(":pressed", true);
        assert_eq!(radius, surface.corner_radius().top_left, "pressed {key}");
        window.close();
    }
}

#[test]
fn fab_color_families_resolve_theme_roles_and_disabled_feedback() {
    let _app = start_themed_application();
    for (key, background, foreground) in [
        (
            "MaterialPrimaryFab",
            "MaterialPrimaryContainerColor",
            "MaterialOnPrimaryContainerColor",
        ),
        (
            "MaterialSurfaceFab",
            "MaterialSurfaceContainerHighColor",
            "MaterialPrimaryColor",
        ),
        (
            "MaterialSecondaryFab",
            "MaterialSecondaryContainerColor",
            "MaterialOnSecondaryContainerColor",
        ),
        (
            "MaterialTertiaryFab",
            "MaterialTertiaryContainerColor",
            "MaterialOnTertiaryContainerColor",
        ),
    ] {
        let button = Button::new();
        button.set_theme(theme(key));
        let window = host(&button);
        assert_eq!(
            color(&button, background),
            button
                .background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            "{key}"
        );
        assert_eq!(
            color(&button, foreground),
            button
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            "{key}"
        );
        button.set_is_enabled(false);
        assert_eq!(0.1, button.background().unwrap().opacity());
        assert_eq!(0.38, button.foreground().unwrap().opacity());
        window.close();
    }
}

#[test]
fn toggle_fab_collapses_to_56dp_at_logical_end_while_preserving_its_footprint() {
    let _app = start_themed_application();
    for (key, footprint) in [
        ("MaterialToggleFab", 56.0),
        ("MaterialMediumToggleFab", 80.0),
        ("MaterialLargeToggleFab", 96.0),
    ] {
        let button = ToggleButton::new();
        button.set_theme(theme(key));
        let window = host(&button);
        let surface: Ref<Border> = named(&button, "MaterialIconContainer");
        let icon: Ref<ContentPresenter> = named(&button, "PART_ContentPresenter");
        button.set_is_checked(Some(true));
        window.update_layout();
        assert_eq!(footprint, button.bounds().width, "{key}");
        assert_eq!(footprint, button.bounds().height, "{key}");
        assert_eq!(56.0, surface.bounds().width, "{key}");
        assert_eq!(56.0, surface.bounds().height, "{key}");
        assert_eq!(footprint - 56.0, surface.bounds().x, "{key}");
        assert_eq!(0.0, surface.bounds().y);
        assert_eq!(28.0, surface.corner_radius().top_left);
        assert_eq!(20.0, icon.bounds().width);
        button.set_flow_direction(FlowDirection::RightToLeft);
        window.update_layout();
        assert_eq!(footprint - 56.0, surface.bounds().x);
        // The platform mirrors the logical position exactly once at the visual boundary.
        assert!(surface.transform_to_visual(&window).unwrap().m11 < 0.0);
        window.close();
    }
}
