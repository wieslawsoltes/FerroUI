//! Tests of the Reveal brushes and lights and of the system backdrops of
//! windows (winui-materials.md), in a window rendered by the test
//! compositor.

use super::*;
use crate::compositor_hit_testing_tests::CompositorTestServices;
use crate::templates::FuncControlTemplate;
use crate::{Border, Control, Panel};
use ferroui_base::media::{Colors, IBrush, XamlCompositionBrushBase};
use ferroui_base::rendering::composition::system_backdrops::{MicaKind, SystemBackdropState};
use ferroui_base::{FerroObject, Ref, Size, Thickness};
use std::rc::Rc;
use std::time::{Duration, Instant};

fn run_until(services: &CompositorTestServices, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() && started.elapsed() < Duration::from_secs(2) {
        services.run_jobs();
    }
}

fn hover_lights(element: &ferroui_base::Visual) -> Vec<Ref<RevealHoverLight>> {
    element
        .lights()
        .to_vec()
        .into_iter()
        .filter_map(|light| light.downcast_ref::<RevealHoverLight>().map(|light| light.to_ref()))
        .collect()
}

#[test]
fn the_reveal_state_adds_a_hover_and_a_press_light_and_drives_them() {
    let services = CompositorTestServices::new(Size::new(400.0, 300.0));
    let border = Border::new();
    border.set_width(100.0);
    border.set_height(40.0);
    let brush: Rc<dyn IBrush> = RevealBackgroundBrush::new().into();
    border.set_background(Some(brush));
    services.set_content(&border.clone().upcast::<Control>());
    services.run_jobs();

    RevealBrush::set_state(&border, RevealBrushState::PointerOver);
    assert!(border.get_value(RevealBrush::is_container_property()));
    let lights = hover_lights(&border);
    assert_eq!(2, lights.len());
    let hover = lights.iter().find(|light| !light.is_press_light()).expect("a hover light").clone();
    let press = lights.iter().find(|light| light.is_press_light()).expect("a press light").clone();
    assert!(hover.is_connected());
    assert!(hover.spot_light().is_some());
    // The hover light is switched on at once (its hover state is not animated).
    assert!(hover.is_on());
    assert_eq!(Some(REVEAL_HOVER_LIGHT_ID.to_owned()), hover.spot_light().unwrap().xaml_id());

    // Pressed: the press light flares and fades.
    RevealBrush::set_state(&border, RevealBrushState::Pressed);
    assert!(press.is_on());
    RevealBrush::set_state(&border, RevealBrushState::PointerOver);
    run_until(&services, || false);

    // Normal: the hover light goes out.
    RevealBrush::set_state(&border, RevealBrushState::Normal);
    run_until(&services, || !hover.is_on());
    assert!(!hover.is_on());

    // No longer a container: the lights go.
    border.set_value(RevealBrush::is_container_property(), false);
    assert!(hover_lights(&border).is_empty());
}

#[test]
fn a_connected_reveal_brush_attaches_the_border_lights_to_the_window() {
    let services = CompositorTestServices::new(Size::new(400.0, 300.0));
    let top_level = services.top_level().clone();
    let border = Border::new();
    border.set_width(100.0);
    border.set_height(40.0);
    border.set_border_thickness(Thickness::uniform(1.0));
    let brush = RevealBorderBrush::new();
    brush.set_target_theme(ApplicationTheme::Dark);
    brush.set_fallback_color(Colors::GRAY);
    border.set_border_brush(Some(brush.clone().into()));
    services.set_content(&border.clone().upcast::<Control>());
    services.run_jobs();

    assert!(brush.is_connected());
    assert_eq!(1, connected_reveal_brushes());
    let lights = top_level.lights().to_vec();
    assert_eq!(5, lights.len(), "four border lights and an ambient light");
    assert_eq!(4, lights.iter().filter(|light| light.is::<RevealBorderLight>()).count());
    // The brush targets the border light of its theme and the ambient light.
    let ids = brush.light_target_ids();
    assert!(ids.iter().any(|id| id == REVEAL_BORDER_LIGHT_DARK_THEME_ID));
    assert!(ids.iter().any(|id| id == XAML_AMBIENT_LIGHT_ID));
    assert!(brush.composition_brush().is_some());

    // The light theme changes the light.
    brush.set_target_theme(ApplicationTheme::Light);
    let ids = brush.light_target_ids();
    assert!(ids.iter().any(|id| id == REVEAL_BORDER_LIGHT_LIGHT_THEME_ID));
    assert!(!ids.iter().any(|id| id == REVEAL_BORDER_LIGHT_DARK_THEME_ID));

    // Fallback: the brush is its fallback colour and no longer targets its border light; the
    // ambient light stays, as WinUI's `OnPropertyChanged` updates the targets without it
    // (`UpdateLightTargets(false /* ambientToo */)`).
    brush.set_always_use_fallback(true);
    let ids = brush.light_target_ids();
    assert!(!ids.iter().any(|id| id == REVEAL_BORDER_LIGHT_LIGHT_THEME_ID || id == REVEAL_BORDER_LIGHT_DARK_THEME_ID));
    assert!(ids.iter().any(|id| id == XAML_AMBIENT_LIGHT_ID));

    // The last brush that disconnects removes the lights.
    services.set_content(&Border::new().upcast::<Control>());
    services.run_jobs();
    assert!(!brush.is_connected());
    assert_eq!(0, connected_reveal_brushes());
    assert!(top_level.lights().to_vec().is_empty());
}

#[test]
fn the_effects_of_reveal_load() {
    let services = CompositorTestServices::new(Size::new(100.0, 100.0));
    let compositor = ferroui_base::rendering::composition::Compositor::try_get_default_compositor()
        .unwrap_or_else(|| services.top_level().renderer().compositor().expect("a compositor"));
    use ferroui_base::rendering::composition::CompositionEffectFactoryLoadStatus;
    assert_eq!(CompositionEffectFactoryLoadStatus::Success, compositor.create_effect_factory(reveal_hover_effect()).load_status());
    for inverted in [false, true] {
        for base in [false, true] {
            let factory = compositor.create_effect_factory(reveal_border_effect(inverted, base));
            assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
        }
    }
}

fn template_with_fallback() -> Option<Rc<dyn crate::templates::IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, scope| {
        let panel = Panel::new();
        let fallback = Border::new();
        fallback.set_name(Some("PART_TransparencyFallback".to_owned()));
        scope.register("PART_TransparencyFallback", fallback.clone().upcast::<FerroObject>());
        panel.children().add(fallback.upcast::<Control>());
        panel.upcast()
    }))
}

#[test]
fn a_system_backdrop_of_a_window_shows_its_fallback_where_the_platform_draws_none() {
    let services = CompositorTestServices::new(Size::new(200.0, 100.0));
    let top_level = services.top_level().clone();
    top_level.set_template(template_with_fallback());
    top_level.apply_template();
    services.run_jobs();

    let backdrop = MicaBackdrop::new();
    backdrop.set_kind(MicaKind::BaseAlt);
    top_level.set_system_backdrop(Some(backdrop.clone().upcast()));
    services.run_jobs();

    assert!(top_level.pseudo_classes().contains(":system-backdrop"));
    let target = top_level.system_backdrop_target();
    let as_target: Rc<dyn ferroui_base::rendering::composition::system_backdrops::ICompositionSupportsSystemBackdrop> =
        target.clone();
    let controller = backdrop.controller_of(&as_target).expect("a controller per target");
    // The mock platform draws no system backdrop: the fallback colour.
    assert_eq!(SystemBackdropState::Fallback, controller.state());
    assert!(!target.is_platform_active());
    let configuration =
        backdrop.get_default_system_backdrop_configuration(&as_target, None).expect("a default configuration");
    assert!(configuration.is_input_active());

    let fallback_border = top_level
        .get_visual_children()
        .first()
        .and_then(|panel| panel.clone().cast::<Panel>())
        .and_then(|panel| panel.children().get(0).cast::<Border>())
        .expect("the template was applied");
    let background = fallback_border.background().expect("the backdrop brush");
    let xaml = XamlCompositionBrushBase::of(&*background).expect("the brush of the backdrop");
    assert!(xaml.composition_brush().is_some());

    top_level.set_system_backdrop(None);
    services.run_jobs();
    assert!(!top_level.pseudo_classes().contains(":system-backdrop"));
    assert!(backdrop.controller_of(&as_target).is_none());
    assert!(DesktopAcrylicBackdrop::new().controller_of(&as_target).is_none());
}

#[test]
fn reveal_focus_shows_a_glow_around_the_element_keyboard_focus_moves_to() {
    use crate::primitives::{AdornerLayer, VisualLayerManager};
    use crate::test_support::TestRoot;
    use crate::StackPanel;
    use ferroui_base::input::{KeyModifiers, NavigationMethod};

    let _scope = crate::test_support::test_scope();
    let _focus = crate::test_support_buttons::focus_scope();
    let focusable = || {
        let control = Control::new();
        control.set_focusable(true);
        control.set_width(120.0);
        control.set_height(32.0);
        control
    };
    let (first, second) = (focusable(), focusable());
    let panel = StackPanel::new();
    panel.children().add(&first);
    panel.children().add(&second);
    let manager = VisualLayerManager::new();
    manager.set_child(panel);
    let root = TestRoot::with_child(&manager);
    root.execute_initial_layout_pass();
    // The focus visual of the controls (Reveal focus glows around it).
    let layer = manager.adorner_layer().expect("an adorner layer");
    let template: Rc<dyn crate::templates::ITemplateOf<Option<Ref<Control>>>> =
        crate::templates::FuncTemplate::new(|| -> Option<Ref<Control>> { Some(Border::new().upcast()) });
    layer.set_default_focus_adorner(Some(template));

    // The default kind shows no glow.
    assert!(first.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    assert!(RevealFocus::glow_host().is_none());

    RevealFocus::set_focus_visual_kind(FocusVisualKind::Reveal);
    assert!(second.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    let host = RevealFocus::glow_host().expect("a glow");
    assert!(RevealFocus::focused_element().is_some_and(|element| same(&element, &second)));
    let adorned = AdornerLayer::get_adorned_element(&host).expect("adorned");
    assert!(same(&adorned, &second));
    assert_eq!(Thickness::uniform(-glow_inset(2.0)), host.margin());
    assert!(layer.children().snapshot().iter().any(|child| same(child, &host)));

    // Focus moved by the pointer shows none.
    assert!(first.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE));
    assert!(RevealFocus::glow_host().is_none());
    assert!(!layer.children().snapshot().iter().any(|child| same(child, &host)));

    // Back to the default kind: the glow goes.
    assert!(second.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(RevealFocus::glow_host().is_some());
    RevealFocus::set_focus_visual_kind(FocusVisualKind::HighVisibility);
    assert!(RevealFocus::glow_host().is_none());
}

fn same(a: &ferroui_base::Visual, b: &ferroui_base::Visual) -> bool {
    std::ptr::eq(a, b)
}
