//! The XAML lights of Reveal (`controls/dev/Lights` of microsoft-ui-xaml):
//! `RevealHoverLight` (the hover light and, as a press light, the press
//! light), `RevealBorderLight` and `XamlAmbientLight` (winui-materials.md).

use super::RevealBrushState;
use crate::hosting::ElementCompositionPreview;
use crate::TopLevel;
use ferroui_base::input::{InputElement, PointerEventArgs, PointerPressedEventArgs, PointerType};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::media::{Color, Colors, IBrush, MaterialPolicy, XamlLight, XamlLightImpl};
use ferroui_base::numerics::Vector2;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{
    AmbientLight, CompositionBatchTypes, CompositionPropertySet, CompositionScopedBatch, Compositor,
    ICompositionObjectAnimations, SpotLight,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, Ref, StyledProperty, Visual, WeakRef,
};
use ferroui_base::rendering::composition::animations::ExpressionAnimation;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The id of the hover and press lights (`RevealHoverLight::GetLightIdStatic`).
pub const REVEAL_HOVER_LIGHT_ID: &str = "Microsoft.UI.Xaml.Media.RevealHoverLight";
/// The id of the border lights of the light theme.
pub const REVEAL_BORDER_LIGHT_LIGHT_THEME_ID: &str = "Microsoft.UI.Xaml.Media.RevealBorderLight_LightTheme";
/// The id of the border lights of the dark theme.
pub const REVEAL_BORDER_LIGHT_DARK_THEME_ID: &str = "Microsoft.UI.Xaml.Media.RevealBorderLight_DarkTheme";
/// The id of the ambient light (`XamlAmbientLight::GetLightIdStatic`).
pub const XAML_AMBIENT_LIGHT_ID: &str = "Microsoft.UI.Xaml.Media.XamlAmbientLight";

// --- the animation tables of SpotLightStateHelper ---------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Easing {
    Default,
    Linear,
    CubicBezier(Vector2, Vector2),
}

const CUBIC_EASING_1: Easing = Easing::CubicBezier(Vector2::new(0.5, 0.0), Vector2::new(0.6, 1.0));
const CUBIC_EASING_RELEASE: Easing = Easing::CubicBezier(Vector2::new(0.33, 0.0), Vector2::new(0.67, 1.0));
const HALF_DURATION: Duration = Duration::from_millis(166);

/// One property of a state (`RevealHoverSpotlightAnimationInfo`): ignored,
/// set at once, or animated over a duration with an easing.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    ignore: bool,
    animate: bool,
    duration: Duration,
    value: f32,
    easing: Easing,
}

const fn set(duration: Duration, value: f32, easing: Easing) -> Frame {
    Frame { ignore: false, animate: false, duration, value, easing }
}

const fn animate(duration: Duration, value: f32, easing: Easing) -> Frame {
    Frame { ignore: false, animate: true, duration, value, easing }
}

const IGNORE: Frame = Frame { ignore: true, animate: false, duration: Duration::ZERO, value: 0.0, easing: Easing::Default };

/// A state of a hover light (`RevealHoverSpotlightStateDesc`): the cone
/// colours are kept white (WinUI animates the intensities on the systems it
/// supports), the intensities and the scale of the outer angle change.
#[derive(Clone, Copy, Debug, PartialEq)]
struct StateDesc {
    inner_cone_intensity: Frame,
    outer_cone_intensity: Frame,
    outer_angle_scale: Frame,
}

impl StateDesc {
    fn is_animated(&self) -> bool {
        let animated = |frame: &Frame| !frame.ignore && frame.animate;
        animated(&self.inner_cone_intensity) || animated(&self.outer_cone_intensity) || animated(&self.outer_angle_scale)
    }

    fn is_ignored(&self) -> bool {
        self.inner_cone_intensity.ignore && self.outer_cone_intensity.ignore && self.outer_angle_scale.ignore
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpotState {
    AnimToOff = 0,
    AnimToHover = 1,
    Pressing = 2,
    FastRelease = 3,
    SlowRelease = 4,
}

/// `sc_revealHoverSpotlightStates`.
const HOVER_STATES: [StateDesc; 5] = [
    // AnimToOff
    StateDesc {
        inner_cone_intensity: set(HALF_DURATION, 0.0, Easing::Default),
        outer_cone_intensity: set(HALF_DURATION, 0.0, Easing::Default),
        outer_angle_scale: set(HALF_DURATION, 1.0, Easing::Default),
    },
    // AnimToHover
    StateDesc {
        inner_cone_intensity: set(HALF_DURATION, 1.0, Easing::Default),
        outer_cone_intensity: set(HALF_DURATION, 1.0, Easing::Default),
        outer_angle_scale: set(HALF_DURATION, 1.0, Easing::Default),
    },
    // Pressing
    StateDesc { inner_cone_intensity: IGNORE, outer_cone_intensity: IGNORE, outer_angle_scale: IGNORE },
    // FastRelease
    StateDesc { inner_cone_intensity: IGNORE, outer_cone_intensity: IGNORE, outer_angle_scale: IGNORE },
    // SlowRelease
    StateDesc { inner_cone_intensity: IGNORE, outer_cone_intensity: IGNORE, outer_angle_scale: IGNORE },
];

/// `sc_pressSpotLightStates`.
const PRESS_STATES: [StateDesc; 5] = [
    // AnimToOff
    StateDesc {
        inner_cone_intensity: set(HALF_DURATION, 0.0, CUBIC_EASING_1),
        outer_cone_intensity: set(HALF_DURATION, 0.0, CUBIC_EASING_1),
        outer_angle_scale: set(HALF_DURATION, 1.0, CUBIC_EASING_1),
    },
    // AnimToHover
    StateDesc {
        inner_cone_intensity: set(HALF_DURATION, 0.0, CUBIC_EASING_1),
        outer_cone_intensity: set(HALF_DURATION, 0.0, CUBIC_EASING_1),
        outer_angle_scale: set(HALF_DURATION, 1.0, CUBIC_EASING_1),
    },
    // Pressing
    StateDesc {
        inner_cone_intensity: animate(Duration::from_millis(66), 1.0, Easing::Linear),
        outer_cone_intensity: animate(Duration::from_millis(66), 1.0, CUBIC_EASING_1),
        outer_angle_scale: animate(Duration::from_millis(66), 1.0, Easing::Linear),
    },
    // FastRelease
    StateDesc {
        inner_cone_intensity: animate(Duration::from_millis(200), 0.0, CUBIC_EASING_RELEASE),
        outer_cone_intensity: animate(Duration::from_millis(266), 0.0, CUBIC_EASING_RELEASE),
        outer_angle_scale: animate(Duration::from_millis(266), 4.44, CUBIC_EASING_RELEASE),
    },
    // SlowRelease
    StateDesc {
        inner_cone_intensity: animate(Duration::from_millis(1000), 0.0, CUBIC_EASING_RELEASE),
        outer_cone_intensity: animate(Duration::from_millis(1500), 0.0, CUBIC_EASING_RELEASE),
        outer_angle_scale: animate(Duration::from_millis(1500), 4.44, CUBIC_EASING_RELEASE),
    },
];

/// `c_PointerOffsetExpression` of the hover light.
const POINTER_OFFSET_EXPRESSION: &str = "pointer.Position + Vector3(0, 0, 256)";
/// `c_CenteredOffsetExpression`.
const CENTERED_OFFSET_EXPRESSION: &str = "Vector3((visual.Size.X / 2), (visual.Size.Y / 2), 256)";
/// `c_OuterAngleExpression`.
const OUTER_ANGLE_EXPRESSION: &str =
    "ATan((Clamp(Max(visual.Size.X, visual.Size.Y) + 12, 16, 512) * props.OuterAngleScale) / 256)";
/// `c_PressOuterAngleExpression`.
const PRESS_OUTER_ANGLE_EXPRESSION: &str = "ATan(((47.25) * props.OuterAngleScale) / 256)";

fn easing_of(compositor: &Rc<Compositor>, easing: Easing) -> Option<Rc<dyn ferroui_base::animation::easings::IEasing>> {
    match easing {
        Easing::Default => None,
        Easing::Linear => Some(compositor.create_linear_easing_function() as Rc<dyn ferroui_base::animation::easings::IEasing>),
        Easing::CubicBezier(p1, p2) => {
            Some(compositor.create_cubic_bezier_easing_function(p1, p2) as Rc<dyn ferroui_base::animation::easings::IEasing>)
        }
    }
}

/// `AnimateTo` of a scalar of a light or a property set.
fn animate_to(target: &dyn ICompositionObjectAnimations, compositor: &Rc<Compositor>, name: &str, frame: &Frame, set_direct: &dyn Fn(f32)) {
    if frame.ignore {
        return;
    }
    target.stop_animation(name);
    if !frame.animate {
        set_direct(frame.value);
        return;
    }
    let animation = compositor.create_scalar_key_frame_animation();
    animation.set_duration(frame.duration);
    match easing_of(compositor, frame.easing) {
        Some(easing) => animation.insert_key_frame_with_easing(1.0, frame.value, easing),
        None => animation.insert_key_frame(1.0, frame.value),
    }
    target.start_animation(name, &*animation);
}

/// `CreateSpotLightColorsProxy`: the cone colours of the light follow the
/// `InnerConeColor`, `OuterConeColor` and `LightIntensity` entries of its
/// properties (the light has no linear intensity of its own).
fn create_colors_proxy(light: &SpotLight) -> Rc<CompositionPropertySet> {
    let properties = light.object().properties();
    properties.insert_color("InnerConeColor", Colors::WHITE);
    properties.insert_color("OuterConeColor", Colors::WHITE);
    properties.insert_scalar("LightIntensity", 0.0);
    let compositor = light.compositor().clone();
    for (property, entry) in [("InnerConeColor", "InnerConeColor"), ("OuterConeColor", "OuterConeColor")] {
        let expression = compositor.create_expression_animation_with(&format!(
            "ColorLerp(ColorRgb(0,0,0,0), target.{entry}, target.LightIntensity)"
        ));
        expression.set_reference_parameter("target", properties.clone());
        light.start_animation(property, &*expression);
    }
    properties
}

// --- RevealHoverLight -------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LightState {
    Off,
    AnimToHover,
    AnimToOff,
    Hover,
    Pressing,
    FastRelease,
    SlowRelease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LightEvent {
    GotoNormal,
    GotoPointerOver,
    GotoPressed,
    AnimationComplete,
}

struct HoverResources {
    light: SpotLight,
    offset_animation: Rc<ExpressionAnimation>,
    outer_angle_animation: Rc<ExpressionAnimation>,
    offset_props: Rc<CompositionPropertySet>,
    colors_proxy: Rc<CompositionPropertySet>,
    pointer_pressed: Option<(WeakRef<Interactive>, RoutedEventHandlerToken)>,
}

#[derive(Default)]
struct HoverState {
    target: Option<WeakRef<Visual>>,
    resources: Option<HoverResources>,
    current: Option<LightState>,
    is_press_light: bool,
    is_pressed: bool,
    is_pointer_over: bool,
    should_light_be_on: bool,
    center_light: bool,
    is_hover_animation_active: bool,
    is_disabled_by_policy: bool,
    policy_subscription: Option<Rc<dyn IDisposable>>,
    /// The batch of the state animation that is running, and the
    /// subscription that continues the state machine when it completes.
    continuation: Option<(Rc<CompositionScopedBatch>, Rc<dyn IDisposable>)>,
}

/// The hover light of Reveal (WinUI's internal `RevealHoverLight`): a spot
/// light 256 above the element under the pointer, whose cone covers the
/// element (`Clamp(Max(width, height) + 12, 16, 512)`), switched on while
/// the pointer is over the element. As a press light
/// ([`RevealHoverLight::new_press_light`]) it is a narrower light (47.25)
/// that flares when the element is pressed and fades out over 0.2 s (a
/// quick press) or 1 to 1.5 s (a long one).
///
/// [`RevealBrush::set_state`](super::RevealBrush::set_state) adds a hover
/// and a press light to an element and drives them.
#[repr(C)]
pub struct RevealHoverLight {
    base: XamlLight,
    state: RefCell<HoverState>,
}

ferro_class!(RevealHoverLight: XamlLight);
ferroui_base::ferro_class_info!(RevealHoverLight {
    new: RevealHoverLight::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(RevealHoverLight: FerroObjectImpl);

/// The press light of Reveal: a [`RevealHoverLight`] made by
/// [`RevealHoverLight::new_press_light`] (WinUI's `SetIsPressLight(true)`).
pub struct RevealPressLight;

impl RevealPressLight {
    /// A press light.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Ref<RevealHoverLight> {
        RevealHoverLight::new_press_light()
    }
}

impl XamlLightImpl for RevealHoverLight {
    fn get_id(_this: &Self) -> String {
        REVEAL_HOVER_LIGHT_ID.to_owned()
    }

    fn on_connected(this: &Self, new_element: &Visual) {
        this.state.borrow_mut().target = Some(new_element.to_ref().downgrade());
        let weak = this.to_ref().downgrade();
        let subscription = MaterialPolicy::policy_changed(move |disabled| {
            if let Some(this) = weak.upgrade() {
                this.on_policy_changed(disabled);
            }
        });
        this.state.borrow_mut().policy_subscription = Some(subscription);
        if !this.state.borrow().is_disabled_by_policy {
            this.ensure_composition_resources();
        }
    }

    fn on_disconnected(this: &Self, _old_element: &Visual) {
        this.release_composition_resources();
        let subscription = {
            let mut state = this.state.borrow_mut();
            state.target = None;
            state.policy_subscription.take()
        };
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

impl RevealHoverLight {
    pub fn construct() -> Self {
        Self { base: XamlLight::construct(), state: RefCell::new(HoverState { center_light: true, ..Default::default() }) }
    }

    /// A hover light.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A press light (WinUI's `SetIsPressLight(true)`).
    pub fn new_press_light() -> Ref<Self> {
        let light = Self::new();
        light.state.borrow_mut().is_press_light = true;
        light
    }

    /// Whether the light is a press light.
    pub fn is_press_light(&self) -> bool {
        self.state.borrow().is_press_light
    }

    /// The spot light, while the light is connected and the policy allows
    /// it (the `GetLight` of the test API).
    pub fn spot_light(&self) -> Option<SpotLight> {
        self.state.borrow().resources.as_ref().map(|resources| resources.light.clone())
    }

    /// Whether the light is switched on.
    pub fn is_on(&self) -> bool {
        self.state.borrow().should_light_be_on
    }

    fn states(&self) -> &'static [StateDesc; 5] {
        if self.is_press_light() {
            &PRESS_STATES
        } else {
            &HOVER_STATES
        }
    }

    fn on_policy_changed(&self, disabled: bool) {
        let changed = {
            let mut state = self.state.borrow_mut();
            let changed = state.is_disabled_by_policy != disabled;
            state.is_disabled_by_policy = disabled;
            changed
        };
        // Called once at subscription, before the resources exist.
        if !changed {
            return;
        }
        if disabled {
            self.release_composition_resources();
        } else if self.state.borrow().target.is_some() {
            self.ensure_composition_resources();
        }
    }

    fn ensure_composition_resources(&self) {
        if self.state.borrow().resources.is_some() {
            return;
        }
        let Some(element) = self.state.borrow().target.as_ref().and_then(WeakRef::upgrade) else { return };
        let Some(element_visual) = ElementCompositionPreview::get_element_visual(&element) else { return };
        let compositor = element_visual.compositor().clone();

        let light = compositor.create_spot_light();
        light.set_constant_attenuation(2.0);
        light.set_linear_attenuation(0.5);
        light.set_inner_cone_angle_in_degrees(0.0);
        light.set_outer_cone_angle_in_degrees(0.0);
        let offset_props = CompositionPropertySet::new(&compositor);
        let offset_animation = compositor.create_expression_animation_with(CENTERED_OFFSET_EXPRESSION);
        offset_animation.set_reference_parameter("props", offset_props.clone());
        let outer_angle_animation = compositor.create_expression_animation_with(if self.is_press_light() {
            PRESS_OUTER_ANGLE_EXPRESSION
        } else {
            OUTER_ANGLE_EXPRESSION
        });
        outer_angle_animation.set_reference_parameter("props", offset_props.clone());
        let colors_proxy = create_colors_proxy(&light);

        // Whether a press comes from the pointer or the keyboard.
        let weak = self.to_ref().downgrade();
        let interactive = element.clone().cast::<Interactive>();
        let pointer_pressed = interactive.map(|interactive| {
            let token = interactive.add_handler_with(
                &InputElement::pointer_pressed_event(),
                move |_: &Interactive, _: &PointerPressedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.on_pointer_pressed();
                    }
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                true,
            );
            (interactive.downgrade(), token)
        });

        let pointer = ElementCompositionPreview::get_pointer_position_property_set(&element);
        if let Some(pointer) = &pointer {
            offset_animation.set_reference_parameter("pointer", pointer.clone());
        }
        offset_animation.set_reference_parameter("visual", element_visual.clone());
        outer_angle_animation.set_reference_parameter("visual", element_visual);

        self.state.borrow_mut().resources = Some(HoverResources {
            light: light.clone(),
            offset_animation,
            outer_angle_animation,
            offset_props,
            colors_proxy,
            pointer_pressed,
        });
        self.set_spot_light_state_immediate(SpotState::AnimToOff);
        self.set_composition_light(Some((*light).clone()));
        if self.state.borrow().should_light_be_on {
            self.switch_light(true);
        }
    }

    fn release_composition_resources(&self) {
        self.cancel_current_press_state_continuation();
        self.disable_hover_animation();
        let resources = self.state.borrow_mut().resources.take();
        if let Some(resources) = resources {
            if let Some((element, token)) = resources.pointer_pressed {
                if let Some(element) = element.upgrade() {
                    let _ = element.remove_handler(&InputElement::pointer_pressed_event(), token);
                }
            }
            resources.light.dispose();
        }
        self.set_composition_light(None);
    }

    fn set_spot_light_state_immediate(&self, state: SpotState) {
        let desc = self.states()[state as usize];
        let resources = self.state.borrow();
        let Some(resources) = resources.resources.as_ref() else { return };
        if desc.is_ignored() {
            return;
        }
        if !desc.inner_cone_intensity.ignore {
            resources.light.set_inner_cone_intensity(desc.inner_cone_intensity.value);
        }
        if !desc.outer_cone_intensity.ignore {
            resources.light.set_outer_cone_intensity(desc.outer_cone_intensity.value);
        }
        if !desc.outer_angle_scale.ignore {
            resources.offset_props.insert_scalar("OuterAngleScale", desc.outer_angle_scale.value);
        }
    }

    /// `PlaySpotLightStateAnimation`: animates to `state` and continues the
    /// state machine when it is done (at once when nothing is animated).
    fn play_spot_light_state_animation(&self, state: SpotState) {
        let desc = self.states()[state as usize];
        let resources = self.state.borrow().resources.as_ref().map(|resources| {
            (resources.light.clone(), resources.offset_props.clone())
        });
        let Some((light, offset_props)) = resources else {
            self.goto_light_state(LightEvent::AnimationComplete);
            return;
        };
        let compositor = light.compositor().clone();
        let apply = || {
            let light_for_inner = light.clone();
            animate_to(&**light, &compositor, "InnerConeIntensity", &desc.inner_cone_intensity, &move |value| {
                light_for_inner.set_inner_cone_intensity(value)
            });
            let light_for_outer = light.clone();
            animate_to(&**light, &compositor, "OuterConeIntensity", &desc.outer_cone_intensity, &move |value| {
                light_for_outer.set_outer_cone_intensity(value)
            });
            let props = offset_props.clone();
            animate_to(&*offset_props, &compositor, "OuterAngleScale", &desc.outer_angle_scale, &move |value| {
                props.insert_scalar("OuterAngleScale", value)
            });
        };
        if !desc.is_animated() {
            apply();
            self.goto_light_state(LightEvent::AnimationComplete);
            return;
        }
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        apply();
        batch.end();
        let weak = self.to_ref().downgrade();
        let subscription = batch.completed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.state.borrow_mut().continuation = None;
                this.goto_light_state(LightEvent::AnimationComplete);
            }
        });
        self.state.borrow_mut().continuation = Some((batch, subscription));
    }

    fn cancel_current_press_state_continuation(&self) {
        let continuation = self.state.borrow_mut().continuation.take();
        if let Some((_, subscription)) = continuation {
            subscription.dispose();
        }
    }

    /// `GoToState`: what the element's Reveal state asks of the light.
    pub fn go_to_state(&self, state: RevealBrushState) {
        self.goto_light_state(match state {
            RevealBrushState::Normal => LightEvent::GotoNormal,
            RevealBrushState::PointerOver => LightEvent::GotoPointerOver,
            RevealBrushState::Pressed => LightEvent::GotoPressed,
        });
    }

    fn set_offset_expression(&self, expression: &str) {
        let resources = self.state.borrow();
        if let Some(resources) = resources.resources.as_ref() {
            resources.offset_animation.set_expression(Some(expression.to_owned()));
            resources.light.start_animation("Offset", &*resources.offset_animation);
        }
    }

    /// `GotoLightState`: the transitions of the state machine.
    fn goto_light_state(&self, event: LightEvent) {
        {
            let mut state = self.state.borrow_mut();
            match event {
                LightEvent::GotoNormal => {
                    state.center_light = true;
                    state.is_pointer_over = false;
                    state.is_pressed = false;
                }
                LightEvent::GotoPointerOver => {
                    state.center_light = false;
                    state.is_pointer_over = true;
                    state.is_pressed = false;
                }
                LightEvent::GotoPressed => state.is_pressed = true,
                LightEvent::AnimationComplete => {}
            }
        }
        let (current, is_pressed, is_pointer_over) = {
            let state = self.state.borrow();
            (state.current.unwrap_or(LightState::Off), state.is_pressed, state.is_pointer_over)
        };
        use LightEvent as E;
        use LightState as S;
        match current {
            S::Off => match event {
                E::GotoPointerOver => self.goto_light_state_helper(S::AnimToHover, false),
                E::GotoPressed => self.goto_light_state_helper(S::Pressing, false),
                E::GotoNormal | E::AnimationComplete => {}
            },
            S::AnimToHover => match event {
                E::GotoNormal => self.goto_light_state_helper(S::AnimToOff, false),
                E::GotoPressed => self.goto_light_state_helper(S::Pressing, false),
                E::AnimationComplete => self.goto_light_state_helper(S::Hover, false),
                E::GotoPointerOver => {}
            },
            S::AnimToOff => match event {
                E::GotoPointerOver => self.goto_light_state_helper(S::AnimToHover, false),
                E::GotoPressed => self.goto_light_state_helper(S::Pressing, false),
                E::AnimationComplete => self.goto_light_state_helper(S::Off, false),
                E::GotoNormal => {}
            },
            S::Hover => match event {
                E::GotoNormal => self.goto_light_state_helper(S::AnimToOff, false),
                E::GotoPressed => self.goto_light_state_helper(S::Pressing, false),
                E::GotoPointerOver | E::AnimationComplete => {}
            },
            S::Pressing => {
                // Once the press is done, the light follows the pointer again.
                self.set_offset_expression(POINTER_OFFSET_EXPRESSION);
                match event {
                    E::GotoNormal => self.goto_light_state_helper(S::AnimToOff, false),
                    E::GotoPressed => {
                        // A press, a release and a press again while the first
                        // press still animates.
                        self.goto_light_state_helper(S::AnimToHover, true);
                        self.goto_light_state_helper(S::Pressing, false);
                    }
                    E::AnimationComplete => {
                        if is_pressed {
                            self.goto_light_state_helper(S::SlowRelease, false)
                        } else {
                            self.goto_light_state_helper(S::FastRelease, false)
                        }
                    }
                    E::GotoPointerOver => {}
                }
            }
            S::FastRelease => match event {
                E::GotoNormal => self.goto_light_state_helper(S::AnimToOff, false),
                E::GotoPressed => {
                    self.goto_light_state_helper(S::AnimToHover, true);
                    self.goto_light_state_helper(S::Pressing, false);
                }
                E::AnimationComplete => {
                    if !is_pressed && is_pointer_over {
                        self.goto_light_state_helper(S::AnimToHover, true);
                    }
                }
                E::GotoPointerOver => {}
            },
            S::SlowRelease => match event {
                E::GotoNormal => self.goto_light_state_helper(S::AnimToOff, false),
                // A press released before the slow release was done.
                E::GotoPointerOver => self.goto_light_state_helper(S::FastRelease, false),
                E::GotoPressed => {
                    self.goto_light_state_helper(S::AnimToHover, true);
                    self.goto_light_state_helper(S::Pressing, false);
                }
                E::AnimationComplete => {
                    if !is_pressed && is_pointer_over {
                        self.goto_light_state_helper(S::AnimToHover, true);
                    }
                }
            },
        }
    }

    /// `GotoLightStateHelper`.
    fn goto_light_state_helper(&self, target: LightState, skip_animation: bool) {
        self.cancel_current_press_state_continuation();
        self.state.borrow_mut().current = Some(target);
        let spot_state = match target {
            LightState::Off => {
                self.switch_light(false);
                return;
            }
            LightState::Hover => {
                self.switch_light(true);
                return;
            }
            LightState::AnimToOff => SpotState::AnimToOff,
            LightState::AnimToHover => {
                self.set_offset_expression(POINTER_OFFSET_EXPRESSION);
                SpotState::AnimToHover
            }
            LightState::Pressing => {
                // The light is centred when the element is invoked from the
                // keyboard (no pointer press came before), and follows the
                // pointer otherwise.
                let center = self.state.borrow().center_light;
                self.set_offset_expression(if center { CENTERED_OFFSET_EXPRESSION } else { POINTER_OFFSET_EXPRESSION });
                SpotState::Pressing
            }
            LightState::FastRelease => SpotState::FastRelease,
            LightState::SlowRelease => SpotState::SlowRelease,
        };
        self.switch_light(true);
        if skip_animation {
            self.set_spot_light_state_immediate(spot_state);
            self.goto_light_state(LightEvent::AnimationComplete);
        } else {
            self.play_spot_light_state_animation(spot_state);
        }
    }

    fn on_pointer_pressed(&self) {
        let is_pressed = {
            let mut state = self.state.borrow_mut();
            state.center_light = false;
            state.is_pressed
        };
        // The press may come after the visual state went to pressed.
        if is_pressed {
            self.set_offset_expression(POINTER_OFFSET_EXPRESSION);
        }
    }

    /// `SwitchLight`: the light and the expressions that place and size it.
    fn switch_light(&self, turn_on: bool) {
        let (has_resources, waiting) = {
            let state = self.state.borrow();
            (state.resources.is_some(), state.continuation.is_some())
        };
        if !has_resources || (waiting && !turn_on) {
            if !has_resources {
                self.state.borrow_mut().should_light_be_on = turn_on;
            }
            return;
        }
        self.state.borrow_mut().should_light_be_on = turn_on;
        if turn_on {
            self.enable_hover_animation();
        } else {
            self.disable_hover_animation();
        }
        let state = self.state.borrow();
        if let Some(resources) = state.resources.as_ref() {
            resources.colors_proxy.insert_scalar("LightIntensity", if turn_on { 1.0 } else { 0.0 });
            resources.light.set_is_enabled(turn_on);
        }
    }

    fn enable_hover_animation(&self) {
        let mut state = self.state.borrow_mut();
        if state.is_hover_animation_active {
            return;
        }
        if let Some(resources) = state.resources.as_ref() {
            resources.light.start_animation("Offset", &*resources.offset_animation);
            resources.light.start_animation("OuterConeAngle", &*resources.outer_angle_animation);
            state.is_hover_animation_active = true;
        }
    }

    fn disable_hover_animation(&self) {
        let mut state = self.state.borrow_mut();
        if !state.is_hover_animation_active {
            return;
        }
        state.is_hover_animation_active = false;
        if let Some(resources) = state.resources.as_ref() {
            resources.light.stop_animation("Offset");
            resources.light.stop_animation("OuterConeAngle");
        }
    }
}

// --- RevealBorderLight ------------------------------------------------------------------

/// `RevealBorderSpotlightStateDesc`: constant 1, linear 3, outer angle
/// 16.94532 degrees, 128 above the pointer; the wide light: linear 0.5,
/// 16.7403 degrees, 256 above.
#[derive(Clone, Copy)]
struct BorderDesc {
    constant_attenuation: f32,
    linear_attenuation: f32,
    outer_cone_angle_in_degrees: f32,
    height: f32,
}

const BORDER_LIGHT: BorderDesc =
    BorderDesc { constant_attenuation: 1.0, linear_attenuation: 3.0, outer_cone_angle_in_degrees: 16.94532, height: 128.0 };
const WIDE_BORDER_LIGHT: BorderDesc =
    BorderDesc { constant_attenuation: 1.0, linear_attenuation: 0.5, outer_cone_angle_in_degrees: 16.7403, height: 256.0 };

/// The duration of the switch of a border light.
pub const BORDER_LIGHT_SWITCH_DURATION: Duration = Duration::from_millis(250);

struct BorderResources {
    light: SpotLight,
    colors_proxy: Rc<CompositionPropertySet>,
    window_handlers: Vec<(WeakRef<TopLevel>, &'static str, RoutedEventHandlerToken)>,
}

#[derive(Default)]
struct BorderState {
    is_light_theme: bool,
    is_wide_light: bool,
    target: Option<WeakRef<Visual>>,
    resources: Option<BorderResources>,
    should_light_be_on: bool,
    is_disabled_by_policy: bool,
    set_disabled_after_turn_off: Rc<Cell<bool>>,
    got_capture_lost_from_touch: bool,
    policy_subscription: Option<Rc<dyn IDisposable>>,
    turn_off: Option<(Rc<CompositionScopedBatch>, Rc<dyn IDisposable>)>,
}

/// A border light of Reveal (WinUI's internal `RevealBorderLight`): a spot
/// light above the pointer that lights the Reveal border brushes near it;
/// switched on while the pointer is over the window (250 ms).
///
/// [`RevealBrush`](super::RevealBrush) attaches four of them (normal and
/// wide, for the light and the dark theme) and an ambient light to the
/// root of every window while a Reveal brush is connected.
#[repr(C)]
pub struct RevealBorderLight {
    base: XamlLight,
    state: RefCell<BorderState>,
}

ferro_class!(RevealBorderLight: XamlLight);
ferroui_base::ferro_class_info!(RevealBorderLight {
    new: RevealBorderLight::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(RevealBorderLight: FerroObjectImpl);

impl XamlLightImpl for RevealBorderLight {
    fn get_id(this: &Self) -> String {
        if this.state.borrow().is_light_theme { REVEAL_BORDER_LIGHT_LIGHT_THEME_ID } else { REVEAL_BORDER_LIGHT_DARK_THEME_ID }
            .to_owned()
    }

    fn on_connected(this: &Self, new_element: &Visual) {
        this.state.borrow_mut().target = Some(new_element.to_ref().downgrade());
        let weak = this.to_ref().downgrade();
        let subscription = MaterialPolicy::policy_changed(move |disabled| {
            if let Some(this) = weak.upgrade() {
                let changed = {
                    let mut state = this.state.borrow_mut();
                    let changed = state.is_disabled_by_policy != disabled;
                    state.is_disabled_by_policy = disabled;
                    changed
                };
                if changed {
                    if disabled {
                        this.release_composition_resources();
                    } else {
                        this.ensure_composition_resources();
                    }
                }
            }
        });
        this.state.borrow_mut().policy_subscription = Some(subscription);
        if !this.state.borrow().is_disabled_by_policy {
            this.ensure_composition_resources();
        }
    }

    fn on_disconnected(this: &Self, _old_element: &Visual) {
        this.release_composition_resources();
        let subscription = {
            let mut state = this.state.borrow_mut();
            state.target = None;
            state.policy_subscription.take()
        };
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

impl RevealBorderLight {
    pub fn construct() -> Self {
        Self { base: XamlLight::construct(), state: RefCell::new(BorderState::default()) }
    }

    /// A border light of the dark theme.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A border light: `light_theme` selects the id of the light theme, `wide`
    /// the wide light.
    pub fn with_options(light_theme: bool, wide: bool) -> Ref<Self> {
        let light = Self::new();
        {
            let mut state = light.state.borrow_mut();
            state.is_light_theme = light_theme;
            state.is_wide_light = wide;
        }
        light
    }

    /// Whether the light has the id of the light theme.
    pub fn is_light_theme(&self) -> bool {
        self.state.borrow().is_light_theme
    }

    /// Whether the light is the wide light.
    pub fn is_wide_light(&self) -> bool {
        self.state.borrow().is_wide_light
    }

    /// Whether the light is switched on.
    pub fn is_on(&self) -> bool {
        self.state.borrow().should_light_be_on
    }

    /// The spot light, while it exists.
    pub fn spot_light(&self) -> Option<SpotLight> {
        self.state.borrow().resources.as_ref().map(|resources| resources.light.clone())
    }

    fn ensure_composition_resources(&self) {
        if self.state.borrow().resources.is_some() {
            return;
        }
        let Some(element) = self.state.borrow().target.as_ref().and_then(WeakRef::upgrade) else { return };
        let Some(element_visual) = ElementCompositionPreview::get_element_visual(&element) else { return };
        let compositor = element_visual.compositor().clone();
        let desc = if self.is_wide_light() { WIDE_BORDER_LIGHT } else { BORDER_LIGHT };

        let light = compositor.create_spot_light();
        let colors_proxy = create_colors_proxy(&light);
        let offset_props = CompositionPropertySet::new(&compositor);
        offset_props.insert_scalar("SpotlightHeight", desc.height);
        offset_props.insert_scalar("OuterAngleScale", 1.0);
        light.set_constant_attenuation(desc.constant_attenuation);
        light.set_linear_attenuation(desc.linear_attenuation);
        light.set_inner_cone_angle_in_degrees(0.0);
        light.set_outer_cone_angle_in_degrees(desc.outer_cone_angle_in_degrees);
        colors_proxy.insert_color("InnerConeColor", Colors::WHITE);
        colors_proxy.insert_color("OuterConeColor", Colors::WHITE);
        light.set_is_enabled(false);

        if let Some(pointer) = ElementCompositionPreview::get_pointer_position_property_set(&element) {
            let offset = compositor.create_expression_animation_with("pointer.Position + Vector3(0,0,props.SpotlightHeight)");
            offset.set_reference_parameter("props", offset_props);
            offset.set_reference_parameter("pointer", pointer);
            light.start_animation("Offset", &*offset);
        }

        let window_handlers = self.hook_window_pointer_handlers(&element);
        self.state.borrow_mut().resources = Some(BorderResources { light: light.clone(), colors_proxy, window_handlers });
        self.set_composition_light(Some((*light).clone()));
    }

    fn release_composition_resources(&self) {
        let (resources, turn_off) = {
            let mut state = self.state.borrow_mut();
            state.should_light_be_on = false;
            (state.resources.take(), state.turn_off.take())
        };
        if let Some((_, subscription)) = turn_off {
            subscription.dispose();
        }
        if let Some(resources) = resources {
            for (top_level, event, token) in resources.window_handlers {
                if let Some(top_level) = top_level.upgrade() {
                    let _ = match event {
                        "entered" => top_level.remove_handler(&InputElement::pointer_entered_event(), token),
                        "exited" => top_level.remove_handler(&InputElement::pointer_exited_event(), token),
                        "moved" => top_level.remove_handler(&InputElement::pointer_moved_event(), token),
                        "pressed" => top_level.remove_handler(&InputElement::pointer_pressed_event(), token),
                        "released" => top_level.remove_handler(&InputElement::pointer_released_event(), token),
                        _ => top_level.remove_handler(&InputElement::pointer_capture_lost_event(), token),
                    };
                }
            }
            resources.light.stop_animation("Offset");
            resources.light.dispose();
        }
        self.set_composition_light(None);
    }

    /// `HookupWindowPointerHandlers`: the pointer events of the window of
    /// `element` switch the light (WinUI listens to the core window).
    fn hook_window_pointer_handlers(&self, element: &Visual) -> Vec<(WeakRef<TopLevel>, &'static str, RoutedEventHandlerToken)> {
        let Some(top_level) = TopLevel::get_top_level(Some(element)) else { return Vec::new() };
        let mut handlers = Vec::new();
        let strategies = RoutingStrategies::TUNNEL | RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE;
        let weak = self.to_ref().downgrade();
        let w = weak.clone();
        handlers.push((
            top_level.downgrade(),
            "entered",
            top_level.add_handler_with(
                &InputElement::pointer_entered_event(),
                move |_: &Interactive, _: &PointerEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.switch_light(true);
                    }
                },
                strategies,
                true,
            ),
        ));
        let w = weak.clone();
        handlers.push((
            top_level.downgrade(),
            "exited",
            top_level.add_handler_with(
                &InputElement::pointer_exited_event(),
                move |_: &Interactive, _: &PointerEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.switch_light(false);
                    }
                },
                strategies,
                true,
            ),
        ));
        let w = weak.clone();
        handlers.push((
            top_level.downgrade(),
            "moved",
            top_level.add_handler_with(
                &InputElement::pointer_moved_event(),
                move |_: &Interactive, e: &PointerEventArgs| {
                    if let Some(this) = w.upgrade() {
                        // A mouse move switches the light on, except the first
                        // one after a touch or pen capture was lost.
                        let touch = this.state.borrow().got_capture_lost_from_touch;
                        if e.pointer().type_() == PointerType::Mouse && !touch {
                            this.switch_light(true);
                        }
                        this.state.borrow_mut().got_capture_lost_from_touch = false;
                    }
                },
                RoutingStrategies::TUNNEL,
                true,
            ),
        ));
        let w = weak.clone();
        handlers.push((
            top_level.downgrade(),
            "pressed",
            top_level.add_handler_with(
                &InputElement::pointer_pressed_event(),
                move |_: &Interactive, _: &PointerPressedEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.switch_light(true);
                    }
                },
                RoutingStrategies::TUNNEL,
                true,
            ),
        ));
        let w = weak.clone();
        handlers.push((
            top_level.downgrade(),
            "released",
            top_level.add_handler_with(
                &InputElement::pointer_released_event(),
                move |_: &Interactive, e: &ferroui_base::input::PointerReleasedEventArgs| {
                    if let Some(this) = w.upgrade() {
                        if e.pointer().type_() == PointerType::Touch {
                            this.switch_light(false);
                        }
                    }
                },
                RoutingStrategies::TUNNEL,
                true,
            ),
        ));
        let w = weak;
        handlers.push((
            top_level.downgrade(),
            "capture-lost",
            top_level.add_handler_with(
                &InputElement::pointer_capture_lost_event(),
                move |_: &Interactive, e: &ferroui_base::input::PointerCaptureLostEventArgs| {
                    if let Some(this) = w.upgrade() {
                        this.switch_light(false);
                        if matches!(e.pointer().type_(), PointerType::Pen | PointerType::Touch) {
                            this.state.borrow_mut().got_capture_lost_from_touch = true;
                        }
                    }
                },
                strategies,
                true,
            ),
        ));
        handlers
    }

    /// `SwitchLight`: animates the intensity of the light to 1 or 0 over
    /// 250 ms; the light is disabled when the turn-off animation is done.
    pub fn switch_light(&self, turn_on: bool) {
        let (colors_proxy, light, flag) = {
            let state = self.state.borrow();
            if state.should_light_be_on == turn_on || state.is_disabled_by_policy {
                return;
            }
            let Some(resources) = state.resources.as_ref() else { return };
            (resources.colors_proxy.clone(), resources.light.clone(), state.set_disabled_after_turn_off.clone())
        };
        self.state.borrow_mut().should_light_be_on = turn_on;
        let compositor = light.compositor().clone();
        let animate = || {
            let animation = compositor.create_scalar_key_frame_animation();
            animation.set_duration(BORDER_LIGHT_SWITCH_DURATION);
            animation.insert_key_frame(1.0, if turn_on { 1.0 } else { 0.0 });
            colors_proxy.start_animation("LightIntensity", &*animation);
        };
        if turn_on {
            flag.set(false);
            light.set_is_enabled(true);
            animate();
            return;
        }
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        animate();
        batch.end();
        flag.set(true);
        let subscription = batch.completed(move |_| {
            if flag.get() {
                light.set_is_enabled(false);
                flag.set(false);
            }
        });
        let previous = self.state.borrow_mut().turn_off.replace((batch, subscription));
        if let Some((_, subscription)) = previous {
            subscription.dispose();
        }
    }
}

// --- XamlAmbientLight -------------------------------------------------------------------

/// The ambient light of Reveal (WinUI's internal `XamlAmbientLight`), also
/// usable on its own: an ambient composition light of its
/// [`color`](Self::color) that lights the targets of its id; the attached
/// [`is_target`](Self::is_target_property) makes an element or a brush a
/// target.
#[repr(C)]
pub struct XamlAmbientLight {
    base: XamlLight,
    light: RefCell<Option<AmbientLight>>,
    policy_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    target: RefCell<Option<WeakRef<Visual>>>,
}

ferro_class!(XamlAmbientLight: XamlLight);
ferroui_base::ferro_class_info!(XamlAmbientLight {
    new: XamlAmbientLight::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});

impl FerroObjectImpl for XamlAmbientLight {
    fn on_property_changed(this: &Self, change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::color_property().as_property() {
            if let Some(light) = this.light.borrow().as_ref() {
                light.set_color(this.color());
            }
        }
    }
}

impl XamlLightImpl for XamlAmbientLight {
    fn get_id(_this: &Self) -> String {
        XAML_AMBIENT_LIGHT_ID.to_owned()
    }

    fn on_connected(this: &Self, new_element: &Visual) {
        *this.target.borrow_mut() = Some(new_element.to_ref().downgrade());
        let weak = this.to_ref().downgrade();
        let subscription = MaterialPolicy::policy_changed(move |disabled| {
            if let Some(this) = weak.upgrade() {
                if disabled {
                    this.release_composition_resources();
                } else {
                    this.ensure_composition_resources();
                }
            }
        });
        *this.policy_subscription.borrow_mut() = Some(subscription);
    }

    fn on_disconnected(this: &Self, _old_element: &Visual) {
        this.release_composition_resources();
        *this.target.borrow_mut() = None;
        if let Some(subscription) = this.policy_subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }
}

ferroui_base::ferro_properties! { impl XamlAmbientLight {
    ferro_property!(
        /// `Color` (default white).
        pub fn color_property() -> StyledProperty<Color> {
            FerroProperty::register::<XamlAmbientLight, _>("Color", Colors::WHITE)
        }
    );

    ferro_property!(
        /// `IsTarget` (attached): the ambient lights light the element or
        /// brush it is set on.
        pub fn is_target_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<XamlAmbientLight, FerroObject, _>("IsTarget", false)
        }
    );
} }

impl XamlAmbientLight {
    pub fn construct() -> Self {
        Self {
            base: XamlLight::construct(),
            light: RefCell::new(None),
            policy_subscription: RefCell::new(None),
            target: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn static_constructor() {
        // `OnAttachedIsTargetPropertyChanged`.
        let _ = Self::is_target_property().changed().subscribe(|e| {
            let is_target = e.get_new_value::<bool>();
            let sender = e.sender();
            if let Some(element) = sender.downcast_ref::<Visual>() {
                if is_target {
                    XamlLight::add_target_element(XAML_AMBIENT_LIGHT_ID, element);
                } else {
                    XamlLight::remove_target_element(XAML_AMBIENT_LIGHT_ID, element);
                }
            } else if let Some(brush) = sender.downcast_ref::<ferroui_base::media::Brush>() {
                let brush: Rc<dyn IBrush> = brush.to_ref().into();
                if is_target {
                    XamlLight::add_target_brush(XAML_AMBIENT_LIGHT_ID, &brush);
                } else {
                    XamlLight::remove_target_brush(XAML_AMBIENT_LIGHT_ID, &brush);
                }
            }
        });
    }

    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// `GetIsTarget`.
    pub fn get_is_target(object: &FerroObject) -> bool {
        object.get_value(Self::is_target_property())
    }

    /// `SetIsTarget`.
    pub fn set_is_target(object: &FerroObject, value: bool) {
        object.set_value(Self::is_target_property(), value)
    }

    fn ensure_composition_resources(&self) {
        if self.light.borrow().is_some() {
            return;
        }
        let Some(element) = self.target.borrow().as_ref().and_then(WeakRef::upgrade) else { return };
        let Some(visual) = ElementCompositionPreview::get_element_visual(&element) else { return };
        let light = visual.compositor().create_ambient_light();
        light.set_color(self.color());
        *self.light.borrow_mut() = Some(light.clone());
        self.set_composition_light(Some((*light).clone()));
    }

    fn release_composition_resources(&self) {
        if let Some(light) = self.light.borrow_mut().take() {
            light.dispose();
            self.set_composition_light(None);
        }
    }
}
