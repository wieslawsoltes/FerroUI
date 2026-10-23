//! Reveal focus (WinUI `FocusVisualKind.Reveal`; `dxaml/xcp/components/FocusRect`:
//! `RevealFocusSource`, `RevealFocusAnimator`, `RevealMotion` and
//! `RevealFocusDefaultValue`): a glow around the element that keyboard
//! focus moves to, lit by a default ambient light, a spot light that
//! travels across the element in the direction focus moved, a pulsing
//! ambient light while the element keeps focus, and a press light while a
//! key presses it (winui-materials.md).

use crate::hosting::ElementCompositionPreview;
use crate::primitives::AdornerLayer;
use crate::{Border, TopLevel};
use ferroui_base::input::{FocusChangedEventArgs, InputElement, Key, KeyEventArgs, NavigationMethod};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::media::{Color, MaterialPolicy};
use ferroui_base::numerics::Vector2;
use ferroui_base::rendering::composition::animations::{AnimationDelayBehavior, AnimationIterationBehavior};
use ferroui_base::rendering::composition::effects::{CanvasComposite, ColorSourceEffect, CompositeEffect};
use ferroui_base::rendering::composition::{
    AmbientLight, CompositionEffectSourceParameter, CompositionStretch, Compositor, ICompositionObjectAnimations,
    ICompositionSurface, LoadedImageSurface, SpotLight, SpriteVisual,
};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_markup_enum, Point, Ref, Thickness, Vector, Vector3D, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The kind of focus visual of the application (WinUI
/// `Microsoft.UI.Xaml.FocusVisualKind`, the `FocusVisualKind` of the
/// application).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FocusVisualKind {
    DottedLine = 0,
    /// The focus visuals of the controls (FerroUI's focus adorners).
    #[default]
    HighVisibility = 1,
    /// The focus visuals of the controls with a Reveal glow around them.
    Reveal = 2,
}

ferro_markup_enum!(FocusVisualKind { DottedLine, HighVisibility, Reveal }, { namespace: "FerroUI.Media" });

/// The glow image of Reveal focus (`NineGridFocusGlow.png` of WinUI, 205 by
/// 205 pixels), stretched as a nine-grid around the element.
pub const REVEAL_FOCUS_GLOW_PNG: &[u8] = include_bytes!("assets/nine_grid_focus_glow.png");

// `RevealFocusDefaultValue.cpp`.
const ALPHA_MASK: f32 = 0.75;
const AMBIENT_LIGHT_DURATION: Duration = Duration::from_secs(1);
const THICKNESS_TO_GLOW_SIZE_SCALE: f64 = 3.5;
const MINIMUM_GLOW_SIZE: f64 = 12.0;
const DEFAULT_LIGHT_INTENSITY_MIN: f32 = 0.4;
const DEFAULT_LIGHT_INTENSITY_MAX: f32 = 1.0;
const DEFAULT_LIGHT_INTENSITY: f32 = 0.5;
const PULSING_LIGHT_INTENSITY_LOW: f32 = 0.0;
const PULSING_LIGHT_INTENSITY_MID: f32 = 0.1;
const PULSING_LIGHT_INTENSITY_HIGH: f32 = 0.4;
const PULSING_ANIMATION_DURATION: Duration = Duration::from_secs(6);
const PULSING_ANIMATION_DELAY: Duration = Duration::from_secs(2);
const CONE_INTENSITY_MIN: f32 = 0.3;
const CONE_INTENSITY_MAX: f32 = 0.6;
const SPOT_LIGHT_INTENSITY_DURATION_FACTOR: f64 = 0.6225;
const SPOT_LIGHT_SPEED: f64 = 700.0;
const SPOT_LIGHT_START_OFFSET_FACTOR: f64 = 2.0;
const SMALL_TRAVELING_SPOT_LIGHT_THRESHOLD: f64 = 70.0;
const INNER_CONE_ANGLE: f32 = 30.0;
const OUTER_CONE_ANGLE: f32 = 45.0;
const PRESS_LIGHT_DURATION: Duration = Duration::from_millis(800);

/// The thickness of the primary border of the focus visual the glow is
/// sized from (the 2 pixels of WinUI's high visibility focus visual).
const PRIMARY_FOCUS_THICKNESS: f64 = 2.0;

/// The direction focus moved in (WinUI `FocusNavigationDirection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealFocusDirection {
    None,
    Up,
    Down,
    Left,
    Right,
}

/// `RevealMotion::CalculateFrom`: the direction focus moved in, from the
/// position of the element that had focus relative to the one that has it.
pub fn direction_from(previous_offset: Point) -> RevealFocusDirection {
    let (x, y) = (previous_offset.x, previous_offset.y);
    if y.abs() >= x.abs() {
        if y < 0.0 {
            return RevealFocusDirection::Down;
        } else if y > 0.0 {
            return RevealFocusDirection::Up;
        }
    } else if x < 0.0 {
        return RevealFocusDirection::Right;
    } else if x > 0.0 {
        return RevealFocusDirection::Left;
    }
    RevealFocusDirection::None
}

/// `GetInsetsFromThickness`: how far the glow reaches beyond the element.
pub fn glow_inset(primary_thickness: f64) -> f64 {
    if primary_thickness > 0.0 {
        THICKNESS_TO_GLOW_SIZE_SCALE * primary_thickness + MINIMUM_GLOW_SIZE
    } else {
        0.0
    }
}

/// The traveling distance of the spot light for an element of `size`.
fn traveling_distance(direction: RevealFocusDirection, size: Vector) -> f64 {
    match direction {
        RevealFocusDirection::Up | RevealFocusDirection::Down => size.y,
        RevealFocusDirection::Left | RevealFocusDirection::Right => size.x,
        RevealFocusDirection::None => 0.0,
    }
}

/// `IsTravelingFocusEnabled`: the spot light travels across elements that
/// are large enough in the direction focus moved.
pub fn is_traveling_focus_enabled(direction: RevealFocusDirection, size: Vector) -> bool {
    traveling_distance(direction, size) > SMALL_TRAVELING_SPOT_LIGHT_THRESHOLD
}

/// `RevealMotion::GetStartX/Y` and `GetDistanceX/Y`: where the spot light
/// starts and ends, in the coordinates of the element, and its height.
pub fn spot_light_path(direction: RevealFocusDirection, size: Vector) -> (Vector3D, Vector3D) {
    let spot_size = size.x.max(size.y);
    let start_x = match direction {
        RevealFocusDirection::Up | RevealFocusDirection::Down | RevealFocusDirection::None => size.x / 2.0,
        RevealFocusDirection::Left => spot_size * SPOT_LIGHT_START_OFFSET_FACTOR + size.x,
        RevealFocusDirection::Right => -spot_size * SPOT_LIGHT_START_OFFSET_FACTOR,
    };
    let start_y = match direction {
        RevealFocusDirection::Left | RevealFocusDirection::Right | RevealFocusDirection::None => size.y / 2.0,
        RevealFocusDirection::Up => spot_size * SPOT_LIGHT_START_OFFSET_FACTOR + size.y,
        RevealFocusDirection::Down => -spot_size * SPOT_LIGHT_START_OFFSET_FACTOR,
    };
    (Vector3D::new(start_x, start_y, spot_size), Vector3D::new(size.x / 2.0, size.y / 2.0, spot_size))
}

/// `GetSpotLightDuration`: the distance at 700 pixels a second.
pub fn spot_light_duration(direction: RevealFocusDirection, size: Vector) -> Duration {
    Duration::from_secs_f64((traveling_distance(direction, size) / SPOT_LIGHT_SPEED).max(0.0))
}

/// The lights of the glow (`RevealFocusLight`).
struct RevealFocusLights {
    default_ambient: AmbientLight,
    pulsing_ambient: AmbientLight,
    traveling_spot: SpotLight,
    press_ambient: AmbientLight,
}

/// The glow that is shown.
struct ShownGlow {
    element: WeakRef<Visual>,
    host: Ref<Border>,
    layer: WeakRef<AdornerLayer>,
    sprite: Option<SpriteVisual>,
    lights: Option<RevealFocusLights>,
    pressed: Cell<bool>,
}

ferroui_base::ferro_thread_local! {
    static KIND: Cell<FocusVisualKind> = const { Cell::new(FocusVisualKind::HighVisibility) };
    static SHOWN: RefCell<Option<Rc<ShownGlow>>> = const { RefCell::new(None) };
}

/// Reveal focus: the focus visual kind of the application, and the glow
/// it shows around the element keyboard focus moves to.
pub struct RevealFocus;

impl RevealFocus {
    /// `Application.FocusVisualKind`.
    pub fn focus_visual_kind() -> FocusVisualKind {
        KIND.with(Cell::get)
    }

    /// Sets `Application.FocusVisualKind`; a glow that is shown goes when
    /// the kind is no longer `Reveal`.
    pub fn set_focus_visual_kind(kind: FocusVisualKind) {
        KIND.with(|current| current.set(kind));
        if kind != FocusVisualKind::Reveal {
            hide();
        }
    }

    /// The element the glow is shown around, if one is.
    pub fn focused_element() -> Option<Ref<Visual>> {
        SHOWN.with(|shown| shown.borrow().as_ref().and_then(|glow| glow.element.upgrade()))
    }

    /// Whether the press light of the glow is on.
    pub fn is_pressed() -> bool {
        SHOWN.with(|shown| shown.borrow().as_ref().is_some_and(|glow| glow.pressed.get()))
    }

    /// The host of the glow in the adorner layer, if a glow is shown.
    pub fn glow_host() -> Option<Ref<Border>> {
        SHOWN.with(|shown| shown.borrow().as_ref().map(|glow| glow.host.clone()))
    }
}

/// Follows the keys of the elements of a window (the press light of the
/// glow).
pub(crate) fn track_focus(top_level: &TopLevel) {
    let routes = RoutingStrategies::BUBBLE;
    let _ = top_level.add_handler_with(
        &InputElement::key_down_event(),
        |_: &Interactive, e: &KeyEventArgs| on_key(e, true),
        routes,
        true,
    );
    let _ = top_level.add_handler_with(&InputElement::key_up_event(), |_: &Interactive, e: &KeyEventArgs| on_key(e, false), routes, true);
}

/// A control got keyboard focus (`Control::on_got_focus`): with the
/// `Reveal` kind, the glow is shown around `target` (the element its focus
/// visual adorns; `None` for a control without a focus visual) when focus
/// moved by the keyboard, and hidden otherwise.
pub(crate) fn on_control_got_focus(target: Option<&Visual>, e: &FocusChangedEventArgs) {
    if RevealFocus::focus_visual_kind() != FocusVisualKind::Reveal {
        return;
    }
    let Some(target) = target else {
        hide();
        return;
    };
    if e.navigation_method != NavigationMethod::Tab && e.navigation_method != NavigationMethod::Directional {
        // Focus that a pointer moved shows no focus visual.
        hide();
        return;
    }
    let target = target.to_ref();
    let previous: Option<Ref<Visual>> = e.old_focused_element.clone().map(|previous| previous.upcast());
    let direction = previous
        .as_ref()
        .and_then(|previous| previous.translate_point(Point::new(0.0, 0.0), &target))
        .map_or(RevealFocusDirection::None, direction_from);
    show(&target, direction);
}

/// A control lost keyboard focus: the glow around it (or around `target`,
/// the element its focus visual adorns) goes.
pub(crate) fn on_control_lost_focus(control: &Visual, target: Option<&Visual>) {
    let Some(element) = RevealFocus::focused_element() else { return };
    let is = |visual: &Visual| std::ptr::eq::<Visual>(&*element, visual);
    if is(control) || target.is_some_and(is) {
        hide();
    }
}

fn on_key(e: &KeyEventArgs, down: bool) {
    if !matches!(e.key, Key::Space | Key::Enter) {
        return;
    }
    let Some(glow) = SHOWN.with(|shown| shown.borrow().clone()) else { return };
    let Some(lights) = glow.lights.as_ref() else {
        glow.pressed.set(down);
        return;
    };
    if glow.pressed.get() == down {
        return;
    }
    glow.pressed.set(down);
    animate_press_light(&lights.press_ambient, down);
}

/// Removes the glow that is shown.
fn hide() {
    let Some(glow) = SHOWN.with(|shown| shown.borrow_mut().take()) else { return };
    if let Some(lights) = glow.lights.as_ref() {
        lights.traveling_spot.stop_animation("Offset");
        lights.traveling_spot.stop_animation("InnerConeIntensity");
        lights.traveling_spot.stop_animation("OuterConeIntensity");
        lights.pulsing_ambient.stop_animation("Intensity");
        for light in [&*lights.default_ambient, &*lights.pulsing_ambient, &*lights.press_ambient] {
            light.targets().remove_all();
        }
        lights.traveling_spot.targets().remove_all();
    }
    if let Some(layer) = glow.layer.upgrade() {
        layer.children().remove(glow.host.clone().upcast());
    }
}

/// The colour of the glow: the primary focus colour of the theme of the
/// element (white in the dark theme, black in the light one), with the
/// alpha mask of Reveal focus.
fn glow_color(element: &Visual) -> Color {
    let dark = element.actual_theme_variant().is_some_and(|variant| variant == ThemeVariant::dark());
    let base = if dark { Color::from_rgb(255, 255, 255) } else { Color::from_rgb(0, 0, 0) };
    Color { a: (255.0 * ALPHA_MASK).round() as u8, ..base }
}

/// `BuildRevealFocusVisual` and `AnimateLights`: shows the glow around
/// `element` and starts its lights.
fn show(element: &Ref<Visual>, direction: RevealFocusDirection) {
    hide();
    let Some(layer) = AdornerLayer::get_adorner_layer(element) else { return };
    let inset = glow_inset(PRIMARY_FOCUS_THICKNESS);
    let host = Border::new();
    host.set_is_hit_test_visible(false);
    host.set_margin(Thickness::uniform(-inset));
    AdornerLayer::set_adorned_element(&host, &**element);
    layer.children().add(host.clone().upcast());
    let glow = Rc::new(ShownGlow {
        element: element.downgrade(),
        host: host.clone(),
        layer: layer.downgrade(),
        sprite: None,
        lights: None,
        pressed: Cell::new(false),
    });
    SHOWN.with(|shown| *shown.borrow_mut() = Some(glow.clone()));
    let color = glow_color(element);
    let size = element.bounds().size();
    let element_size = Vector::new(size.width, size.height);
    // The visual of the host exists once the renderer has seen it.
    let weak_host = host.downgrade();
    let _ = Dispatcher::ui_thread().invoke_async_local_with_priority(
        move || {
            let Some(host) = weak_host.upgrade() else { return };
            let still_shown = SHOWN.with(|shown| {
                shown.borrow().as_ref().is_some_and(|glow| std::ptr::eq::<Border>(&*glow.host, &*host))
            });
            if !still_shown {
                return;
            }
            let Some(compositor) = ElementCompositionPreview::get_element_visual(&host).map(|visual| visual.compositor().clone())
            else {
                return;
            };
            let (sprite, lights) = build_glow(&compositor, &host, color, inset, element_size, direction);
            SHOWN.with(|shown| {
                let mut shown = shown.borrow_mut();
                if let Some(glow) = shown.as_mut() {
                    let pressed = glow.pressed.get();
                    let rebuilt = ShownGlow {
                        element: glow.element.clone(),
                        host: glow.host.clone(),
                        layer: glow.layer.clone(),
                        sprite: Some(sprite),
                        lights: Some(lights),
                        pressed: Cell::new(false),
                    };
                    *glow = Rc::new(rebuilt);
                    if pressed {
                        if let Some(lights) = glow.lights.as_ref() {
                            glow.pressed.set(true);
                            animate_press_light(&lights.press_ambient, true);
                        }
                    }
                }
            });
        },
        DispatcherPriority::BACKGROUND,
    );
}

fn build_glow(
    compositor: &Rc<Compositor>,
    host: &Ref<Border>,
    color: Color,
    inset: f64,
    element_size: Vector,
    direction: RevealFocusDirection,
) -> (SpriteVisual, RevealFocusLights) {
    // The glow image as a hollow nine-grid whose edges are the insets
    // (`CreateEffectSourceBrush`): a quarter of the image on each side.
    let surface = LoadedImageSurface::start_load_from_stream(compositor, &mut std::io::Cursor::new(REVEAL_FOCUS_GLOW_PNG));
    let image_size = surface.decoded_size();
    let surface_brush = compositor.create_surface_brush_with_surface(surface as Rc<dyn ICompositionSurface>);
    surface_brush.set_stretch(CompositionStretch::Fill);
    let nine_grid = compositor.create_nine_grid_brush();
    nine_grid.set_is_center_hollow(true);
    let (quarter_x, quarter_y) = ((image_size.width / 4.0).max(1.0) as f32, (image_size.height / 4.0).max(1.0) as f32);
    nine_grid.set_insets_with(quarter_x, quarter_y, quarter_x, quarter_y);
    nine_grid.set_inset_scales_with(
        inset as f32 / quarter_x,
        inset as f32 / quarter_y,
        inset as f32 / quarter_x,
        inset as f32 / quarter_y,
    );
    nine_grid.set_source(Some((*surface_brush).clone()));
    // The colour where the glow is (`CreateColorEffect`: a destination-in
    // composite of the colour and the glow).
    let effect = CompositeEffect {
        name: String::new(),
        mode: CanvasComposite::DestinationIn,
        sources: vec![
            ColorSourceEffect { name: "Color".into(), color }.into(),
            CompositionEffectSourceParameter::new("nineGridBrush").into(),
        ],
    };
    let brush = compositor.create_effect_factory(effect).create_brush();
    brush.set_source_parameter("nineGridBrush", Some((*nine_grid).clone()));
    let sprite = compositor.create_sprite_visual();
    sprite.set_brush(Some(brush.brush().clone()));
    let host_size = host.bounds().size();
    sprite.set_size(Vector::new(host_size.width, host_size.height));
    if let Some(host_visual) = ElementCompositionPreview::get_element_visual(host) {
        let size = compositor.create_expression_animation_with("hostVisual.Size");
        size.set_reference_parameter("hostVisual", host_visual);
        sprite.start_animation("Size", &*size);
    }
    ElementCompositionPreview::set_element_child_visual(host, Some(sprite.visual().clone()));

    // `InitializeLightingEffects`: the four lights of the glow.
    let white = Color::from_rgb(255, 255, 255);
    let target = sprite.visual().clone();
    let ambient = |intensity: f32| {
        let light = compositor.create_ambient_light();
        light.set_color(white);
        light.set_intensity(intensity);
        light.targets().add(target.clone());
        light
    };
    let default_ambient = ambient(DEFAULT_LIGHT_INTENSITY);
    let pulsing_ambient = ambient(PULSING_LIGHT_INTENSITY_MID);
    let traveling_spot = compositor.create_spot_light();
    traveling_spot.set_inner_cone_color(white);
    traveling_spot.set_coordinate_space(Some(target.clone()));
    traveling_spot.set_inner_cone_angle_in_degrees(INNER_CONE_ANGLE);
    traveling_spot.set_outer_cone_angle_in_degrees(OUTER_CONE_ANGLE);
    traveling_spot.set_offset(Vector3D::new(0.0, 0.0, 0.0));
    traveling_spot.targets().add(target.clone());
    let press_ambient = ambient(0.0);
    press_ambient.set_is_enabled(false);
    let lights = RevealFocusLights { default_ambient, pulsing_ambient, traveling_spot, press_ambient };
    animate_lights(compositor, &lights, inset, element_size, direction);
    (sprite, lights)
}

fn scalar_frames(compositor: &Rc<Compositor>, frames: &[(f32, f32)], duration: Duration) -> Rc<ferroui_base::rendering::composition::animations::ScalarKeyFrameAnimation> {
    let animation = compositor.create_scalar_key_frame_animation();
    for &(progress, value) in frames {
        animation.insert_key_frame(progress, value);
    }
    animation.set_duration(if duration.is_zero() { Duration::from_millis(1) } else { duration });
    animation
}

/// `AnimateLights`: the default light, the traveling spot light and the
/// pulsing light.
fn animate_lights(
    compositor: &Rc<Compositor>,
    lights: &RevealFocusLights,
    inset: f64,
    element_size: Vector,
    direction: RevealFocusDirection,
) {
    let traveling = is_traveling_focus_enabled(direction, element_size);
    // The default light: a short pulse, shorter when the spot light travels.
    let frames: &[(f32, f32)] = if traveling {
        &[(0.0, DEFAULT_LIGHT_INTENSITY_MAX), (1.0, DEFAULT_LIGHT_INTENSITY)]
    } else {
        &[(0.0, DEFAULT_LIGHT_INTENSITY_MIN), (0.1, DEFAULT_LIGHT_INTENSITY_MAX), (1.0, DEFAULT_LIGHT_INTENSITY)]
    };
    lights.default_ambient.start_animation("Intensity", &*scalar_frames(compositor, frames, AMBIENT_LIGHT_DURATION));

    if traveling {
        // The spot light in the coordinates of the glow, which starts
        // `inset` before the element.
        let (start, end) = spot_light_path(direction, element_size);
        let shift = |point: Vector3D| Vector3D::new(point.x + inset, point.y + inset, point.z);
        let duration = spot_light_duration(direction, element_size);
        let easing = compositor.create_cubic_bezier_easing_function(Vector2::new(0.17, 0.8), Vector2::new(0.33, 0.99));
        let offset = compositor.create_vector3d_key_frame_animation();
        offset.insert_key_frame_with_easing(0.0, shift(start), easing.clone());
        offset.insert_key_frame_with_easing(1.0, shift(end), easing);
        offset.set_duration(duration);
        lights.traveling_spot.start_animation("Offset", &*offset);
        let intensity_duration = duration.mul_f64(SPOT_LIGHT_INTENSITY_DURATION_FACTOR);
        let cone = [(0.0, CONE_INTENSITY_MIN), (0.2, CONE_INTENSITY_MAX), (1.0, CONE_INTENSITY_MIN)];
        lights.traveling_spot.start_animation("InnerConeIntensity", &*scalar_frames(compositor, &cone, intensity_duration));
        lights.traveling_spot.start_animation("OuterConeIntensity", &*scalar_frames(compositor, &cone, intensity_duration));
    } else {
        // A spot light that does not travel lights nothing.
        lights.traveling_spot.set_inner_cone_intensity(0.0);
        lights.traveling_spot.set_outer_cone_intensity(0.0);
    }

    // `ShouldBreathe`: the pulsing light, unless the energy saver is on.
    if !MaterialPolicy::platform_settings().energy_saver_on {
        let pulse = scalar_frames(
            compositor,
            &[(0.0, PULSING_LIGHT_INTENSITY_MID), (0.5, PULSING_LIGHT_INTENSITY_HIGH), (1.0, PULSING_LIGHT_INTENSITY_LOW)],
            PULSING_ANIMATION_DURATION,
        );
        pulse.set_delay_behavior(AnimationDelayBehavior::SetInitialValueBeforeDelay);
        pulse.set_delay_time(PULSING_ANIMATION_DELAY);
        pulse.set_iteration_behavior(AnimationIterationBehavior::Forever);
        lights.pulsing_ambient.start_animation("Intensity", &*pulse);
    }
}

/// `AnimatePressLight`: on, the press light rises to the default intensity
/// over 0.8 s; off, it falls back to nothing.
fn animate_press_light(light: &AmbientLight, on: bool) {
    if on && !light.is_enabled() {
        light.set_is_enabled(true);
    }
    let compositor = light.compositor().clone();
    let (from, to) = if on { (0.0, DEFAULT_LIGHT_INTENSITY) } else { (DEFAULT_LIGHT_INTENSITY, 0.0) };
    light.start_animation("Intensity", &*scalar_frames(&compositor, &[(0.0, from), (1.0, to)], PRESS_LIGHT_DURATION));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_direction_follows_where_the_previous_element_was() {
        assert_eq!(RevealFocusDirection::Down, direction_from(Point::new(0.0, -40.0)));
        assert_eq!(RevealFocusDirection::Up, direction_from(Point::new(10.0, 40.0)));
        assert_eq!(RevealFocusDirection::Right, direction_from(Point::new(-80.0, 10.0)));
        assert_eq!(RevealFocusDirection::Left, direction_from(Point::new(80.0, 10.0)));
        assert_eq!(RevealFocusDirection::None, direction_from(Point::new(0.0, 0.0)));
    }

    #[test]
    fn the_glow_and_the_spot_light_follow_the_constants_of_winui() {
        assert_eq!(19.0, glow_inset(2.0));
        assert_eq!(0.0, glow_inset(0.0));
        let size = Vector::new(120.0, 32.0);
        assert!(is_traveling_focus_enabled(RevealFocusDirection::Right, size));
        assert!(!is_traveling_focus_enabled(RevealFocusDirection::Down, size));
        let (start, end) = spot_light_path(RevealFocusDirection::Right, size);
        assert_eq!(Vector3D::new(-240.0, 16.0, 120.0), start);
        assert_eq!(Vector3D::new(60.0, 16.0, 120.0), end);
        let (start, _) = spot_light_path(RevealFocusDirection::Left, size);
        assert_eq!(360.0, start.x);
        assert_eq!(Duration::from_secs_f64(120.0 / 700.0), spot_light_duration(RevealFocusDirection::Right, size));
    }
}
