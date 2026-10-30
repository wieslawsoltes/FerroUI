//! `AcrylicBrush` of WinUI (`Microsoft.UI.Xaml.Media.AcrylicBrush`,
//! `controls/dev/Materials/Acrylic/AcrylicBrush.cpp` of microsoft-ui-xaml):
//! the in-app acrylic material, a blurred backdrop with a luminosity and a
//! tint, and noise (winui-materials.md).

use crate::media::{
    BrushImpl, Color, MaterialPolicy, XamlCompositionBrushBase, XamlCompositionBrushBaseImpl,
};
use crate::numerics::Vector2;
use crate::reactive::IDisposable;
use crate::rendering::composition::effects::{
    BlendEffect, BlendEffectMode, BorderEffect, CanvasComposite, CanvasEdgeBehavior, ColorSourceEffect,
    CompositeEffect, CompositionEffectFactory, CompositionEffectSourceParameter, CrossFadeEffect, EffectBorderMode,
    GaussianBlurEffect, GraphicsEffect, GraphicsEffectSource, OpacityEffect,
};
use crate::rendering::composition::{
    CompositionBatchTypes, CompositionBrush, CompositionScopedBatch, Compositor, ICompositionObjectAnimations,
};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledProperty, StyledPropertyOptions,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// `sc_blurRadius`: the blur amount of the backdrop.
pub const ACRYLIC_BLUR_AMOUNT: f32 = 30.0;
/// `sc_noiseOpacity`: the opacity of the noise.
pub const ACRYLIC_NOISE_OPACITY: f32 = 0.02;
/// The duration of the cross-fade between the material and its fallback
/// colour.
pub const ACRYLIC_FALLBACK_CROSS_FADE: Duration = Duration::from_millis(167);

const TINT_COLOR_COLOR: &str = "TintColor.Color";
const LUMINOSITY_COLOR_COLOR: &str = "LuminosityColor.Color";
const FALLBACK_COLOR_COLOR: &str = "FallbackColor.Color";
const OPAQUE_FALLBACK_COLOR_COLOR: &str = "OpaqueFallbackColor.Color";
const FADE_IN_OUT_WEIGHT: &str = "FadeInOut.Weight";

// --- the colour math of WinUI --------------------------------------------------------

/// A colour in hue (degrees), saturation and value (0 to 1).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hsv {
    h: f64,
    s: f64,
    v: f64,
}

fn rgb_to_hsv(r: f64, g: f64, b: f64) -> Hsv {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let h = if delta <= 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let s = if max <= 0.0 { 0.0 } else { delta / max };
    Hsv { h, s, v: max }
}

fn hsv_to_rgb(hsv: Hsv) -> (f64, f64, f64) {
    let c = hsv.v * hsv.s;
    let h = hsv.h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = hsv.v - c;
    (r + m, g + m, b + m)
}

fn channels(color: Color) -> (f64, f64, f64) {
    (color.r as f64 / 255.0, color.g as f64 / 255.0, color.b as f64 / 255.0)
}

fn channel(value: f64) -> u8 {
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

/// `GetTintOpacityModifier`: how much of the tint opacity a colour keeps.
/// Pure white keeps 45 %, pure black 85 %, grey 90 %; the suppression grows
/// linearly with the distance of the value from 50 % and shrinks as the
/// saturation grows.
pub fn tint_opacity_modifier(tint_color: Color) -> f64 {
    const MID_POINT: f64 = 0.50;
    const WHITE_MAX_OPACITY: f64 = 0.45;
    const MID_POINT_MAX_OPACITY: f64 = 0.90;
    const BLACK_MAX_OPACITY: f64 = 0.85;

    let (r, g, b) = channels(tint_color);
    let hsv = rgb_to_hsv(r, g, b);
    let mut opacity_modifier = MID_POINT_MAX_OPACITY;
    if hsv.v != MID_POINT {
        let mut lowest_max_opacity = MID_POINT_MAX_OPACITY;
        let mut max_deviation = MID_POINT;
        if hsv.v > MID_POINT {
            lowest_max_opacity = WHITE_MAX_OPACITY;
            max_deviation = 1.0 - max_deviation;
        } else if hsv.v < MID_POINT {
            lowest_max_opacity = BLACK_MAX_OPACITY;
        }
        let mut max_opacity_suppression = MID_POINT_MAX_OPACITY - lowest_max_opacity;
        let deviation = (hsv.v - MID_POINT).abs();
        let normalized_deviation = deviation / max_deviation;
        if hsv.s > 0.0 {
            max_opacity_suppression *= (1.0 - hsv.s * 2.0).max(0.0);
        }
        let opacity_suppression = max_opacity_suppression * normalized_deviation;
        opacity_modifier = MID_POINT_MAX_OPACITY - opacity_suppression;
    }
    opacity_modifier
}

/// `GetLuminosityColor`: the colour of the luminosity blend for a tint
/// colour whose alpha is the tint opacity the user set. With a luminosity
/// opacity it is the tint colour at that opacity; without one, the tint
/// colour with its value clamped to 12.5 %..96.5 %, at an opacity mapped
/// from the tint opacity onto 15 %..103 % (at most 100 %).
pub fn luminosity_color(tint_color: Color, luminosity_opacity: Option<f64>) -> Color {
    let (r, g, b) = channels(tint_color);
    if let Some(opacity) = luminosity_opacity {
        return Color::from_argb(channel(opacity.clamp(0.0, 1.0)), tint_color.r, tint_color.g, tint_color.b);
    }
    const MIN_HSV_V: f64 = 0.125;
    const MAX_HSV_V: f64 = 0.965;
    let hsv = rgb_to_hsv(r, g, b);
    let (lr, lg, lb) = hsv_to_rgb(Hsv { v: hsv.v.clamp(MIN_HSV_V, MAX_HSV_V), ..hsv });
    const MIN_LUMINOSITY_OPACITY: f64 = 0.15;
    const MAX_LUMINOSITY_OPACITY: f64 = 1.03;
    let range = MAX_LUMINOSITY_OPACITY - MIN_LUMINOSITY_OPACITY;
    let mapped = (tint_color.a as f64 / 255.0) * range + MIN_LUMINOSITY_OPACITY;
    Color::from_argb(channel(mapped.min(1.0)), channel(lr), channel(lg), channel(lb))
}

// --- the effect graph ------------------------------------------------------------------

fn parameter(name: &str) -> Option<GraphicsEffectSource> {
    Some(CompositionEffectSourceParameter::new(name).into())
}

/// What an acrylic brush blurs (WinUI 2 `AcrylicBrushBackgroundSource`, the
/// `BackgroundSource` of `AcrylicBrush`; WinUI 3 has the backdrop only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AcrylicBrushBackgroundSource {
    /// What is behind the window, blurred by the platform (window acrylic).
    HostBackdrop = 0,
    /// What the window draws behind the element (in-app acrylic).
    #[default]
    Backdrop = 1,
}

crate::ferro_markup_enum!(AcrylicBrushBackgroundSource { HostBackdrop, Backdrop }, { namespace: "FerroUI.Media" });

/// The effect of WinUI's acrylic ("shadow-friendly luminosity-based
/// recipe"): an opaque tint with noise, or the backdrop over the opaque
/// fallback colour, blurred by 30 (with hard edges), its luminosity taken
/// from the luminosity colour, its colour from the tint, with noise; with
/// `use_cross_fade`, cross-faded with the fallback colour. Returns the
/// effect and its animatable properties.
pub fn acrylic_effect(
    should_brush_be_opaque: bool,
    use_cross_fade: bool,
    tint_color: Color,
    luminosity_color: Color,
    fallback_color: Color,
) -> (GraphicsEffect, Vec<&'static str>) {
    acrylic_effect_with_source(
        should_brush_be_opaque,
        use_cross_fade,
        AcrylicBrushBackgroundSource::Backdrop,
        tint_color,
        luminosity_color,
        fallback_color,
    )
}

/// [`acrylic_effect`] for a background source. The host backdrop (WinUI 2's
/// window acrylic) is blurred by the platform behind the window, so it is
/// not blurred again; the compositor cannot read it, so the luminosity and
/// the tint are composed over it source-over (winui-materials.md,
/// deviations), the host backdrop brush first, so that it shows through the
/// window where the brush paints.
pub fn acrylic_effect_with_source(
    should_brush_be_opaque: bool,
    use_cross_fade: bool,
    source: AcrylicBrushBackgroundSource,
    tint_color: Color,
    luminosity_color: Color,
    fallback_color: Color,
) -> (GraphicsEffect, Vec<&'static str>) {
    let tint: GraphicsEffect = ColorSourceEffect { name: "TintColor".into(), color: tint_color }.into();
    let mut animatable = vec![TINT_COLOR_COLOR];

    let tint_output: GraphicsEffect = if should_brush_be_opaque {
        tint
    } else if source == AcrylicBrushBackgroundSource::HostBackdrop {
        animatable.push(LUMINOSITY_COLOR_COLOR);
        CompositeEffect {
            name: String::new(),
            mode: CanvasComposite::SourceOver,
            sources: vec![
                CompositionEffectSourceParameter::new("Backdrop").into(),
                ColorSourceEffect { name: "LuminosityColor".into(), color: luminosity_color }.into(),
                tint.into(),
            ],
        }
        .into()
    } else {
        // The backdrop over the opaque fallback colour, so that the blur
        // does not blur with transparency.
        let opaque_fallback = Color { a: 255, ..fallback_color };
        animatable.push(OPAQUE_FALLBACK_COLOR_COLOR);
        let blended_backdrop = CompositeEffect {
            name: String::new(),
            mode: CanvasComposite::SourceOver,
            sources: vec![
                ColorSourceEffect { name: "OpaqueFallbackColor".into(), color: opaque_fallback }.into(),
                CompositionEffectSourceParameter::new("Backdrop").into(),
            ],
        };
        let blur = GaussianBlurEffect {
            name: "Blur".into(),
            blur_amount: ACRYLIC_BLUR_AMOUNT,
            border_mode: EffectBorderMode::Hard,
            source: Some(blended_backdrop.into()),
            ..Default::default()
        };
        animatable.push(LUMINOSITY_COLOR_COLOR);
        // The hue and saturation of the blurred backdrop with the
        // luminosity of the luminosity colour.
        //
        // Deviation (winui-materials.md): WinUI writes `BlendEffectMode::Color`
        // here, because the names of `Color` and `Luminosity` are swapped in
        // its blend effect; FerroUI's blend effect has the meaning Win2D
        // documents, so the mode that does what WinUI draws is `Luminosity`.
        let luminosity_blend = BlendEffect {
            name: String::new(),
            mode: BlendEffectMode::Luminosity,
            background: Some(blur.into()),
            foreground: Some(ColorSourceEffect { name: "LuminosityColor".into(), color: luminosity_color }.into()),
        };
        // The hue and saturation of the tint with the luminosity of the
        // above (WinUI writes `Luminosity`, see above).
        BlendEffect {
            name: String::new(),
            mode: BlendEffectMode::Color,
            background: Some(luminosity_blend.into()),
            foreground: Some(tint.into()),
        }
        .into()
    };

    // The noise, tiled, at 2 %, over the tint.
    let noise = OpacityEffect {
        name: "NoiseOpacity".into(),
        opacity: ACRYLIC_NOISE_OPACITY,
        source: Some(
            BorderEffect {
                name: String::new(),
                extend_x: CanvasEdgeBehavior::Wrap,
                extend_y: CanvasEdgeBehavior::Wrap,
                source: parameter("Noise"),
            }
            .into(),
        ),
    };
    let outer: GraphicsEffect = CompositeEffect {
        name: String::new(),
        mode: CanvasComposite::SourceOver,
        sources: vec![tint_output.into(), noise.into()],
    }
    .into();

    if !use_cross_fade {
        return (outer, animatable);
    }
    animatable.push(FALLBACK_COLOR_COLOR);
    animatable.push(FADE_IN_OUT_WEIGHT);
    let fade = CrossFadeEffect {
        name: "FadeInOut".into(),
        weight: 1.0,
        source1: Some(ColorSourceEffect { name: "FallbackColor".into(), color: fallback_color }.into()),
        source2: Some(outer.into()),
    };
    (fade.into(), animatable)
}

crate::ferro_thread_local! {
    /// The effect factories of acrylic per compositor and form (WinUI's
    /// `MaterialHelper` cache), keyed by opaque and cross-fade.
    static FACTORIES: RefCell<Vec<(Weak<Compositor>, bool, bool, AcrylicBrushBackgroundSource, Rc<CompositionEffectFactory>)>> = const { RefCell::new(Vec::new()) };
}

fn factory(
    compositor: &Rc<Compositor>,
    opaque: bool,
    cross_fade: bool,
    source: AcrylicBrushBackgroundSource,
    tint: Color,
    luminosity: Color,
    fallback: Color,
) -> Rc<CompositionEffectFactory> {
    let cached = FACTORIES.with(|factories| {
        let mut factories = factories.borrow_mut();
        factories.retain(|(owner, ..)| owner.strong_count() > 0);
        factories
            .iter()
            .find(|(owner, o, c, s, _)| {
                *o == opaque && *c == cross_fade && *s == source && owner.upgrade().is_some_and(|owner| Rc::ptr_eq(&owner, compositor))
            })
            .map(|(.., factory)| factory.clone())
    });
    if let Some(cached) = cached {
        return cached;
    }
    let (effect, animatable) = acrylic_effect_with_source(opaque, cross_fade, source, tint, luminosity, fallback);
    let factory = compositor.create_effect_factory_with_properties(effect, &animatable);
    FACTORIES.with(|factories| factories.borrow_mut().push((Rc::downgrade(compositor), opaque, cross_fade, source, factory.clone())));
    factory
}

// --- the brush -----------------------------------------------------------------------

#[derive(Default)]
struct AcrylicState {
    /// The composition brush: the effect brush of the material, or a colour
    /// brush of the fallback colour.
    brush: Option<Rc<CompositionBrush>>,
    is_using_acrylic: bool,
    is_using_opaque: bool,
    /// The background source the brush was created for.
    is_using_host_backdrop: bool,
    is_disabled_by_policy: bool,
    /// The cross-fade between the material and the fallback colour that is
    /// running, and the subscription to its end.
    waiting_for_fallback: Option<(Rc<CompositionScopedBatch>, Rc<dyn IDisposable>)>,
    policy_subscription: Option<Rc<dyn IDisposable>>,
    activation_subscription: Option<Rc<dyn IDisposable>>,
}

/// A brush that paints the acrylic material of WinUI (WinUI
/// `Microsoft.UI.Xaml.Media.AcrylicBrush`): what is behind the element,
/// blurred, with its luminosity taken from a luminosity colour and its
/// colour from the tint, and a little noise; or, when the tint is opaque,
/// the tint with noise.
///
/// The brush falls back to its [`fallback_color`](XamlCompositionBrushBase::fallback_color)
/// when [`always_use_fallback`](Self::always_use_fallback) is set or the
/// [`MaterialPolicy`] disables materials (energy saver, transparency effects
/// off), cross-fading over 167 ms. With the [`background_source`](Self::background_source)
/// `HostBackdrop` (WinUI 2's window acrylic) it shows what is behind the
/// window, and also falls back while no window of the application is
/// active; in a window the platform shows nothing behind (an opaque
/// window) it paints its fallback colour. Changes of the tint animate over
/// [`tint_transition_duration`](Self::tint_transition_duration).
#[repr(C)]
pub struct AcrylicBrush {
    base: XamlCompositionBrushBase,
    state: RefCell<AcrylicState>,
}

ferro_class!(AcrylicBrush: XamlCompositionBrushBase);
crate::ferro_class_info!(AcrylicBrush {
    new: AcrylicBrush::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(AcrylicBrush: BrushImpl);

impl FerroObjectImpl for AcrylicBrush {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::tint_color_property().as_property()
            || property == Self::tint_opacity_property().as_property()
            || property == Self::tint_luminosity_opacity_property().as_property()
        {
            this.on_tint_changed(property == Self::tint_luminosity_opacity_property().as_property());
        } else if property == Self::always_use_fallback_property().as_property()
            || property == Self::background_source_property().as_property()
        {
            this.update_acrylic_brush();
        } else if property == XamlCompositionBrushBase::fallback_color_property().as_property() {
            this.on_fallback_color_changed();
        }
    }
}

impl XamlCompositionBrushBaseImpl for AcrylicBrush {
    fn on_connected(this: &Self) {
        let weak = this.to_ref().downgrade();
        // Called at once with the current state, before the brush counts as
        // connected (as in WinUI).
        let subscription = MaterialPolicy::policy_changed(move |disabled| {
            if let Some(this) = weak.upgrade() {
                this.state.borrow_mut().is_disabled_by_policy = disabled;
                if this.state.borrow().policy_subscription.is_some() {
                    this.update_acrylic_brush();
                }
            }
        });
        this.state.borrow_mut().policy_subscription = Some(subscription);
        let weak = this.to_ref().downgrade();
        let activation = MaterialPolicy::window_activation_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                if this.background_source() == AcrylicBrushBackgroundSource::HostBackdrop {
                    this.update_acrylic_brush();
                }
            }
        });
        this.state.borrow_mut().activation_subscription = Some(activation);
        this.update_acrylic_brush();
    }

    fn on_disconnected(this: &Self) {
        this.cancel_fallback_animation_complete_wait();
        let (subscriptions, brush) = {
            let mut state = this.state.borrow_mut();
            state.is_using_acrylic = false;
            ([state.policy_subscription.take(), state.activation_subscription.take()], state.brush.take())
        };
        for subscription in subscriptions.into_iter().flatten() {
            subscription.dispose();
        }
        if let Some(brush) = brush {
            brush.object().dispose();
        }
        this.set_composition_brush(None);
    }
}

crate::ferro_properties! { impl AcrylicBrush {
    ferro_property!(
        /// `TintColor` (default `#CCFFFFFF`).
        pub fn tint_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<AcrylicBrush, _>("TintColor", Color::from_argb(204, 255, 255, 255))
        }
    );

    ferro_property!(
        /// `TintOpacity` (default 1), coerced to 0..1.
        pub fn tint_opacity_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<AcrylicBrush, _>(
                "TintOpacity",
                StyledPropertyOptions::new(1.0).coerce(|_, value: f64| if value.is_nan() { value } else { value.clamp(0.0, 1.0) }),
            )
        }
    );

    ferro_property!(
        /// `TintLuminosityOpacity` (default none), coerced to 0..1.
        pub fn tint_luminosity_opacity_property() -> StyledProperty<Option<f64>> {
            FerroProperty::register_with::<AcrylicBrush, _>(
                "TintLuminosityOpacity",
                StyledPropertyOptions::new(None).coerce(|_, value: Option<f64>| {
                    value.map(|value| if value.is_nan() { value } else { value.clamp(0.0, 1.0) })
                }),
            )
        }
    );

    ferro_property!(
        /// `TintTransitionDuration` (default 500 ms): how long a change of
        /// the tint or of the fallback colour animates.
        pub fn tint_transition_duration_property() -> StyledProperty<Duration> {
            FerroProperty::register::<AcrylicBrush, _>("TintTransitionDuration", Duration::from_millis(500))
        }
    );

    ferro_property!(
        /// `BackgroundSource` (WinUI 2; default `Backdrop`): what the brush
        /// blurs.
        pub fn background_source_property() -> StyledProperty<AcrylicBrushBackgroundSource> {
            FerroProperty::register::<AcrylicBrush, _>("BackgroundSource", AcrylicBrushBackgroundSource::Backdrop)
        }
    );

    ferro_property!(
        /// `AlwaysUseFallback` (default `false`).
        pub fn always_use_fallback_property() -> StyledProperty<bool> {
            FerroProperty::register::<AcrylicBrush, _>("AlwaysUseFallback", false)
        }
    );
} }

impl AcrylicBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: XamlCompositionBrushBase::construct(), state: RefCell::new(AcrylicState::default()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn tint_color(&self) -> Color {
        self.get_value(Self::tint_color_property())
    }

    pub fn set_tint_color(&self, value: Color) {
        self.set_value(Self::tint_color_property(), value)
    }

    pub fn tint_opacity(&self) -> f64 {
        self.get_value(Self::tint_opacity_property())
    }

    pub fn set_tint_opacity(&self, value: f64) {
        self.set_value(Self::tint_opacity_property(), value)
    }

    pub fn tint_luminosity_opacity(&self) -> Option<f64> {
        self.get_value(Self::tint_luminosity_opacity_property())
    }

    pub fn set_tint_luminosity_opacity(&self, value: Option<f64>) {
        self.set_value(Self::tint_luminosity_opacity_property(), value)
    }

    pub fn tint_transition_duration(&self) -> Duration {
        self.get_value(Self::tint_transition_duration_property())
    }

    pub fn set_tint_transition_duration(&self, value: Duration) {
        self.set_value(Self::tint_transition_duration_property(), value)
    }

    pub fn background_source(&self) -> AcrylicBrushBackgroundSource {
        self.get_value(Self::background_source_property())
    }

    pub fn set_background_source(&self, value: AcrylicBrushBackgroundSource) {
        self.set_value(Self::background_source_property(), value)
    }

    pub fn always_use_fallback(&self) -> bool {
        self.get_value(Self::always_use_fallback_property())
    }

    pub fn set_always_use_fallback(&self, value: bool) {
        self.set_value(Self::always_use_fallback_property(), value)
    }

    /// `GetEffectiveTintColor`: the tint colour with its alpha times the
    /// tint opacity (and the tint opacity modifier when no luminosity
    /// opacity is set).
    pub fn effective_tint_color(&self) -> Color {
        let mut tint = self.tint_color();
        let opacity = self.tint_opacity();
        let modifier = if self.tint_luminosity_opacity().is_some() { 1.0 } else { tint_opacity_modifier(tint) };
        tint.a = (tint.a as f64 * opacity * modifier).round().clamp(0.0, 255.0) as u8;
        tint
    }

    /// `GetEffectiveLuminosityColor`: the luminosity colour of the tint at
    /// the tint opacity the user set.
    pub fn effective_luminosity_color(&self) -> Color {
        let mut tint = self.tint_color();
        tint.a = (tint.a as f64 * self.tint_opacity()).round().clamp(0.0, 255.0) as u8;
        luminosity_color(tint, self.tint_luminosity_opacity())
    }

    /// Whether the brush draws the material now (rather than its fallback
    /// colour).
    pub fn is_using_acrylic_brush(&self) -> bool {
        self.state.borrow().is_using_acrylic
    }

    /// Whether the cross-fade between the material and the fallback colour
    /// is running.
    pub fn is_waiting_for_fallback_animation(&self) -> bool {
        self.state.borrow().waiting_for_fallback.is_some()
    }

    fn on_tint_changed(&self, luminosity_only: bool) {
        let should_use_opaque = self.effective_tint_color().a == 255;
        if self.state.borrow().is_using_opaque != should_use_opaque {
            self.update_acrylic_brush();
        }
        let (brush, animate) = {
            let state = self.state.borrow();
            (state.brush.clone(), state.is_using_acrylic || state.waiting_for_fallback.is_some())
        };
        let Some(brush) = brush.filter(|_| animate) else { return };
        let duration = self.tint_transition_duration();
        if !luminosity_only {
            animate_color(&brush, TINT_COLOR_COLOR, self.effective_tint_color(), duration);
        }
        if !should_use_opaque {
            animate_color(&brush, LUMINOSITY_COLOR_COLOR, self.effective_luminosity_color(), duration);
        }
    }

    fn on_fallback_color_changed(&self) {
        let (brush, using_opaque, using_acrylic, waiting) = {
            let state = self.state.borrow();
            (state.brush.clone(), state.is_using_opaque, state.is_using_acrylic, state.waiting_for_fallback.is_some())
        };
        let Some(brush) = brush else { return };
        let fallback = self.fallback_color();
        if !using_opaque && (using_acrylic || waiting) {
            brush.object().properties().insert_color(OPAQUE_FALLBACK_COLOR_COLOR, Color { a: 255, ..fallback });
        }
        if using_acrylic && !waiting {
            // The material without the cross-fade has no fallback colour.
            return;
        }
        let name = if using_acrylic || waiting { FALLBACK_COLOR_COLOR } else { "Color" };
        animate_color(&brush, name, fallback, self.tint_transition_duration());
    }

    /// `UpdateAcrylicBrush`: creates the brush the first time, again when
    /// the tint becomes opaque or stops being opaque, or cross-fades to or
    /// from the fallback colour when the policy or `AlwaysUseFallback`
    /// changes.
    fn update_acrylic_brush(&self) {
        if !self.is_connected() {
            return;
        }
        let use_host_backdrop = self.background_source() == AcrylicBrushBackgroundSource::HostBackdrop;
        let is_using_acrylic = !(self.state.borrow().is_disabled_by_policy
            || self.always_use_fallback()
            || (use_host_backdrop && !MaterialPolicy::is_window_active()));
        let should_use_opaque = self.effective_tint_color().a == 255;
        let (has_brush, using_opaque, currently_acrylic, using_host_backdrop) = {
            let state = self.state.borrow();
            (state.brush.is_some(), state.is_using_opaque, state.is_using_acrylic, state.is_using_host_backdrop)
        };
        self.set_needs_transparent_window(use_host_backdrop);
        if !has_brush || using_opaque != should_use_opaque || using_host_backdrop != use_host_backdrop {
            self.state.borrow_mut().is_using_host_backdrop = use_host_backdrop;
            self.state.borrow_mut().is_using_acrylic = is_using_acrylic;
            self.create_acrylic_brush(false, false);
            return;
        }
        if currently_acrylic == is_using_acrylic {
            return;
        }
        self.cancel_fallback_animation_complete_wait();
        // The material with the cross-fade, animated between the fallback
        // colour (weight 0) and the material (weight 1).
        self.create_acrylic_brush(true, true);
        self.state.borrow_mut().is_using_acrylic = is_using_acrylic;
        let (start, end) = if is_using_acrylic { (0.0, 1.0) } else { (1.0, 0.0) };
        let Some(brush) = self.state.borrow().brush.clone() else { return };
        let compositor = brush.compositor().clone();
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        let animation = compositor.create_scalar_key_frame_animation();
        let easing = compositor.create_cubic_bezier_easing_function(Vector2::new(0.5, 0.0), Vector2::new(0.0, 0.9));
        animation.insert_key_frame(0.0, start);
        animation.insert_key_frame_with_easing(1.0, end, easing);
        animation.set_duration(ACRYLIC_FALLBACK_CROSS_FADE);
        brush.start_animation(FADE_IN_OUT_WEIGHT, &*animation);
        batch.end();
        let weak = self.to_ref().downgrade();
        let subscription = batch.completed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.cancel_fallback_animation_complete_wait();
                // The material (or the fallback colour) without the
                // cross-fade.
                this.create_acrylic_brush(false, false);
            }
        });
        self.state.borrow_mut().waiting_for_fallback = Some((batch, subscription));
    }

    fn cancel_fallback_animation_complete_wait(&self) {
        let waiting = self.state.borrow_mut().waiting_for_fallback.take();
        if let Some((_, subscription)) = waiting {
            subscription.dispose();
        }
    }

    /// `CreateAcrylicBrush`: the effect brush of the material (with the
    /// cross-fade when `use_cross_fade`), when the brush uses the material or
    /// `force` is set; a colour brush of the fallback colour otherwise.
    fn create_acrylic_brush(&self, use_cross_fade: bool, force: bool) {
        self.cancel_fallback_animation_complete_wait();
        let Some(compositor) = self.compositor() else { return };
        let fallback = self.fallback_color();
        let using_acrylic = self.state.borrow().is_using_acrylic;
        let brush: Rc<CompositionBrush> = if force || using_acrylic {
            let tint = self.effective_tint_color();
            let luminosity = self.effective_luminosity_color();
            let opaque = tint.a == 255;
            self.state.borrow_mut().is_using_opaque = opaque;
            let host = self.background_source() == AcrylicBrushBackgroundSource::HostBackdrop;
            let source = if host { AcrylicBrushBackgroundSource::HostBackdrop } else { AcrylicBrushBackgroundSource::Backdrop };
            let effect_brush = factory(&compositor, opaque, use_cross_fade, source, tint, luminosity, fallback).create_brush();
            effect_brush.set_source_parameter("Noise", Some((*MaterialPolicy::noise_brush(&compositor)).clone()));
            if !opaque {
                let backdrop: Rc<CompositionBrush> = if host {
                    (*compositor.create_host_backdrop_brush()).clone()
                } else {
                    (*compositor.create_backdrop_brush()).clone()
                };
                effect_brush.set_source_parameter("Backdrop", Some(backdrop));
            }
            let properties = effect_brush.properties();
            properties.insert_color(TINT_COLOR_COLOR, tint);
            if !opaque {
                properties.insert_color(LUMINOSITY_COLOR_COLOR, luminosity);
                if !host {
                    properties.insert_color(OPAQUE_FALLBACK_COLOR_COLOR, Color { a: 255, ..fallback });
                }
            }
            if use_cross_fade {
                properties.insert_color(FALLBACK_COLOR_COLOR, fallback);
            }
            effect_brush.brush().clone()
        } else {
            (*compositor.create_color_brush_with_color(fallback)).clone()
        };
        let old = self.state.borrow_mut().brush.replace(brush.clone());
        self.set_composition_brush(Some(brush));
        if let Some(old) = old {
            old.object().dispose();
        }
    }
}

/// Animates the colour `name` of `brush` to `color` over `duration` (at
/// least 1 ms, as WinUI's `MakeColorAnimation`).
fn animate_color(brush: &Rc<CompositionBrush>, name: &str, color: Color, duration: Duration) {
    let compositor = brush.compositor().clone();
    let animation = compositor.create_color_key_frame_animation();
    animation.insert_key_frame(1.0, color);
    animation.set_duration(if duration.is_zero() { Duration::from_millis(1) } else { duration });
    brush.start_animation(name, &*animation);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tint_opacity_modifier_follows_winui() {
        // Pure white keeps 45 %, pure black 85 %.
        assert!((tint_opacity_modifier(Color::from_rgb(255, 255, 255)) - 0.45).abs() < 1e-9);
        assert!((tint_opacity_modifier(Color::from_rgb(0, 0, 0)) - 0.85).abs() < 1e-9);
        // A saturated colour is not suppressed past half its saturation.
        assert!((tint_opacity_modifier(Color::from_rgb(255, 0, 0)) - 0.90).abs() < 1e-9);
    }

    #[test]
    fn the_luminosity_colour_follows_winui() {
        // With a luminosity opacity: the tint at that opacity.
        let color = luminosity_color(Color::from_argb(38, 44, 44, 44), Some(0.96));
        assert_eq!((color.a, color.r, color.g, color.b), (245, 44, 44, 44));
        // Without: the value clamped to 12.5 %, the opacity mapped from the
        // tint opacity.
        let color = luminosity_color(Color::from_argb(0, 0, 0, 0), None);
        assert_eq!(color.r, 32);
        assert_eq!(color.a, (0.15f64 * 255.0).round() as u8);
        let opaque = luminosity_color(Color::from_argb(255, 255, 255, 255), None);
        assert_eq!(opaque.a, 255);
        assert_eq!(opaque.r, (0.965f64 * 255.0).round() as u8);
    }

    #[test]
    fn the_effective_colours_of_the_theme_brushes() {
        // AcrylicInAppFillColorDefaultBrush of the dark theme.
        let brush = AcrylicBrush::new();
        brush.set_tint_color(Color::from_rgb(0x2c, 0x2c, 0x2c));
        brush.set_tint_opacity(0.15);
        brush.set_tint_luminosity_opacity(Some(0.96));
        assert_eq!(brush.effective_tint_color().a, (255.0f64 * 0.15).round() as u8);
        assert_eq!(brush.effective_luminosity_color().a, (0.96f64 * 255.0).round() as u8);
        // Coercion.
        brush.set_tint_opacity(3.0);
        assert_eq!(brush.tint_opacity(), 1.0);
        brush.set_tint_luminosity_opacity(Some(-1.0));
        assert_eq!(brush.tint_luminosity_opacity(), Some(0.0));
    }

    #[test]
    fn the_effect_has_the_animatable_properties_of_winui() {
        let (_, translucent) = acrylic_effect(false, false, Color::default(), Color::default(), Color::default());
        assert_eq!(translucent, vec![TINT_COLOR_COLOR, OPAQUE_FALLBACK_COLOR_COLOR, LUMINOSITY_COLOR_COLOR]);
        let (_, opaque_fading) = acrylic_effect(true, true, Color::default(), Color::default(), Color::default());
        assert_eq!(opaque_fading, vec![TINT_COLOR_COLOR, FALLBACK_COLOR_COLOR, FADE_IN_OUT_WEIGHT]);
    }
}
