//! `RevealBrush`, `RevealBackgroundBrush` and `RevealBorderBrush` of WinUI
//! (`controls/dev/Materials/Reveal/RevealBrush.cpp` of microsoft-ui-xaml):
//! brushes lit by the Reveal lights near the pointer (winui-materials.md).

use super::reveal_lights::{
    RevealBorderLight, RevealHoverLight, REVEAL_BORDER_LIGHT_DARK_THEME_ID, REVEAL_BORDER_LIGHT_LIGHT_THEME_ID,
    REVEAL_HOVER_LIGHT_ID, XAML_AMBIENT_LIGHT_ID,
};
use super::XamlAmbientLight;
use crate::TopLevel;
use ferroui_base::media::{
    BrushImpl, Color, ColorMatrix, ColorMatrixAlphaMode, MaterialPolicy, XamlCompositionBrushBase,
    XamlCompositionBrushBaseImpl, XamlLight,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::effects::{
    AlphaMaskEffect, ArithmeticCompositeEffect, BorderEffect, CanvasComposite, CanvasEdgeBehavior, ColorMatrixEffect,
    ColorSourceEffect, CompositeEffect, CompositionEffectFactory, CompositionEffectSourceParameter, GraphicsEffect,
    SceneLightingEffect,
};
use ferroui_base::rendering::composition::{CompositionBrush, Compositor};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_property, instantiate, AttachedProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty, Visual, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The theme an element is drawn for (WinUI `Microsoft.UI.Xaml.ApplicationTheme`):
/// the type of [`RevealBrush::target_theme`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ApplicationTheme {
    #[default]
    Light = 0,
    Dark = 1,
}

ferro_markup_enum!(ApplicationTheme { Light, Dark }, { namespace: "FerroUI.Media" });

/// The Reveal state of an element (WinUI
/// `Microsoft.UI.Xaml.Media.RevealBrushState`), set by the visual states of
/// a control through the attached [`RevealBrush::state_property`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RevealBrushState {
    #[default]
    Normal = 0,
    PointerOver = 1,
    Pressed = 2,
}

ferro_markup_enum!(RevealBrushState { Normal, PointerOver, Pressed }, { namespace: "FerroUI.Media" });

/// `sc_ambientContributionColor`.
const AMBIENT_CONTRIBUTION_COLOR: Color = Color::from_argb(255, 127, 127, 127);
/// `sc_diffuseAmount`, `sc_diffuseAmountBorder`.
const DIFFUSE_AMOUNT: f32 = 0.2;

/// `sc_colorToAlphaMatrix`: the red of the noise becomes an alpha from 85 %
/// to 100 %.
pub const COLOR_TO_ALPHA_MATRIX: ColorMatrix = ColorMatrix::new([
    [1.0, 0.0, 0.0, 0.15],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.85],
]);

/// `sc_luminanceToAlphaMatrix`: the luminance becomes the alpha, the colour
/// is kept.
pub const LUMINANCE_TO_ALPHA_MATRIX: ColorMatrix = ColorMatrix::new([
    [1.0, 0.0, 0.0, 0.2125],
    [0.0, 1.0, 0.0, 0.7154],
    [0.0, 0.0, 1.0, 0.0721],
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
]);

/// `sc_revealBorderColorMatrix`: twice the luminance-to-alpha matrix.
pub const REVEAL_BORDER_COLOR_MATRIX: ColorMatrix = ColorMatrix::new([
    [2.0, 0.0, 0.0, 2.0 * 0.2125],
    [0.0, 2.0, 0.0, 2.0 * 0.7154],
    [0.0, 0.0, 2.0, 2.0 * 0.0721],
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
]);

/// `sc_revealInvertedBorderColorMatrix`: the above with the colours
/// negated, for the light theme.
pub const REVEAL_INVERTED_BORDER_COLOR_MATRIX: ColorMatrix = ColorMatrix::new([
    [-2.0, 0.0, 0.0, 2.0 * 0.2125],
    [0.0, -2.0, 0.0, 2.0 * 0.7154],
    [0.0, 0.0, -2.0, 2.0 * 0.0721],
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
]);

const BASE_COLOR_EFFECT_COLOR: &str = "BaseColorEffect.Color";

fn scene_lighting() -> SceneLightingEffect {
    SceneLightingEffect {
        name: String::new(),
        ambient_amount: 0.0,
        diffuse_amount: DIFFUSE_AMOUNT,
        specular_amount: 0.0,
        specular_shine: 0.0,
        normal_map_source: None,
    }
}

/// `CreateRevealHoverEffect`: the lighting masked by the noise (to hide
/// banding), added to the ambient contribution, its luminance as alpha, and
/// mixed with the base colour (`2 s + 2 b - 2 s b - 1`).
pub fn reveal_hover_effect() -> GraphicsEffect {
    let noise_border = BorderEffect {
        name: String::new(),
        extend_x: CanvasEdgeBehavior::Wrap,
        extend_y: CanvasEdgeBehavior::Wrap,
        source: Some(CompositionEffectSourceParameter::new("Noise").into()),
    };
    let noise_opacity = ColorMatrixEffect {
        name: String::new(),
        color_matrix: COLOR_TO_ALPHA_MATRIX,
        alpha_mode: ColorMatrixAlphaMode::Straight,
        clamp_output: false,
        source: Some(noise_border.into()),
    };
    let masked_lighting = AlphaMaskEffect {
        name: String::new(),
        source: Some(scene_lighting().into()),
        alpha_mask: Some(noise_opacity.into()),
    };
    let composite = CompositeEffect {
        name: String::new(),
        mode: CanvasComposite::Add,
        sources: vec![
            ColorSourceEffect { name: String::new(), color: AMBIENT_CONTRIBUTION_COLOR }.into(),
            masked_lighting.into(),
        ],
    };
    let color_matrix = ColorMatrixEffect {
        name: String::new(),
        color_matrix: LUMINANCE_TO_ALPHA_MATRIX,
        alpha_mode: ColorMatrixAlphaMode::Straight,
        clamp_output: false,
        source: Some(composite.into()),
    };
    ArithmeticCompositeEffect {
        name: String::new(),
        multiply_amount: -2.0,
        source1_amount: 2.0,
        source2_amount: 2.0,
        offset: -1.0,
        clamp_output: false,
        source1: Some(color_matrix.into()),
        source2: Some(ColorSourceEffect { name: "BaseColorEffect".into(), color: Color::default() }.into()),
    }
    .into()
}

/// `CreateRevealBorderEffect`: the lighting through the border matrix
/// (inverted for the light theme), over the base colour when there is one.
pub fn reveal_border_effect(is_inverted: bool, has_base_color: bool) -> GraphicsEffect {
    let color_matrix: GraphicsEffect = ColorMatrixEffect {
        name: String::new(),
        color_matrix: if is_inverted { REVEAL_INVERTED_BORDER_COLOR_MATRIX } else { REVEAL_BORDER_COLOR_MATRIX },
        alpha_mode: ColorMatrixAlphaMode::Straight,
        clamp_output: false,
        source: Some(scene_lighting().into()),
    }
    .into();
    if !has_base_color {
        return color_matrix;
    }
    CompositeEffect {
        name: String::new(),
        mode: CanvasComposite::SourceOver,
        sources: vec![
            ColorSourceEffect { name: "BaseColorEffect".into(), color: Color::default() }.into(),
            color_matrix.into(),
        ],
    }
    .into()
}

ferroui_base::ferro_thread_local! {
    /// The effect factories of Reveal per compositor (`RevealBrushCacheFlags`).
    static FACTORIES: RefCell<Vec<(Weak<Compositor>, u8, Rc<CompositionEffectFactory>)>> = const { RefCell::new(Vec::new()) };
    /// The number of connected Reveal brushes.
    static CONNECTED_BRUSHES: Cell<usize> = const { Cell::new(0) };
    /// The windows the Reveal lights are attached to while brushes are
    /// connected.
    static TOP_LEVELS: RefCell<Vec<WeakRef<TopLevel>>> = const { RefCell::new(Vec::new()) };
    /// The lights attached to the root of each window, removed when the
    /// last brush disconnects (`TrackRevealLightsToRemove`).
    static ATTACHED_LIGHTS: RefCell<Vec<(WeakRef<Visual>, Vec<Ref<XamlLight>>)>> = const { RefCell::new(Vec::new()) };
}

fn factory(compositor: &Rc<Compositor>, is_border: bool, is_inverted: bool, has_base_color: bool) -> Rc<CompositionEffectFactory> {
    let key = u8::from(is_border) | (u8::from(is_inverted) << 1) | (u8::from(has_base_color) << 2);
    let cached = FACTORIES.with(|factories| {
        let mut factories = factories.borrow_mut();
        factories.retain(|(owner, ..)| owner.strong_count() > 0);
        factories
            .iter()
            .find(|(owner, existing, _)| *existing == key && owner.upgrade().is_some_and(|owner| Rc::ptr_eq(&owner, compositor)))
            .map(|(.., factory)| factory.clone())
    });
    if let Some(cached) = cached {
        return cached;
    }
    let effect = if is_border { reveal_border_effect(is_inverted, has_base_color) } else { reveal_hover_effect() };
    let animatable: &[&str] = if has_base_color { &[BASE_COLOR_EFFECT_COLOR] } else { &[] };
    let factory = compositor.create_effect_factory_with_properties(effect, animatable);
    FACTORIES.with(|factories| factories.borrow_mut().push((Rc::downgrade(compositor), key, factory.clone())));
    factory
}

/// Records a window, to which the Reveal lights are attached while Reveal
/// brushes are connected (WinUI attaches them to the root of the window).
pub(crate) fn on_top_level_created(top_level: &TopLevel) {
    TOP_LEVELS.with(|top_levels| {
        let mut top_levels = top_levels.borrow_mut();
        top_levels.retain(|existing| existing.upgrade().is_some());
        top_levels.push(top_level.to_ref().downgrade());
    });
    if CONNECTED_BRUSHES.with(Cell::get) > 0 {
        attach_lights_to_element(top_level);
    }
}

/// `AttachLightsToElement`: two border lights (normal and wide) for each
/// theme and an ambient light, unless the element has border lights.
fn attach_lights_to_element(element: &Visual) {
    let lights = element.lights();
    if lights.to_vec().iter().any(|light| light.is::<RevealBorderLight>()) {
        return;
    }
    let added: Vec<Ref<XamlLight>> = vec![
        RevealBorderLight::with_options(true, false).upcast(),
        RevealBorderLight::with_options(true, true).upcast(),
        RevealBorderLight::with_options(false, false).upcast(),
        RevealBorderLight::with_options(false, true).upcast(),
        XamlAmbientLight::new().upcast(),
    ];
    for light in &added {
        lights.add(light.clone());
    }
    ATTACHED_LIGHTS.with(|attached| attached.borrow_mut().push((element.to_ref().downgrade(), added)));
}

/// `MaterialHelper::OnRevealBrushConnected`: the first connected brush
/// attaches the lights to every window.
fn on_reveal_brush_connected() {
    let count = CONNECTED_BRUSHES.with(|count| {
        count.set(count.get() + 1);
        count.get()
    });
    if count != 1 {
        return;
    }
    let top_levels: Vec<Ref<TopLevel>> =
        TOP_LEVELS.with(|top_levels| top_levels.borrow().iter().filter_map(WeakRef::upgrade).collect());
    for top_level in top_levels {
        attach_lights_to_element(&top_level);
    }
}

/// `MaterialHelper::OnRevealBrushDisconnected`: the last disconnected brush
/// removes the lights it attached.
fn on_reveal_brush_disconnected() {
    let count = CONNECTED_BRUSHES.with(|count| {
        count.set(count.get().saturating_sub(1));
        count.get()
    });
    if count != 0 {
        return;
    }
    let attached = ATTACHED_LIGHTS.with(|attached| std::mem::take(&mut *attached.borrow_mut()));
    for (element, lights) in attached {
        if let Some(element) = element.upgrade() {
            let collection = element.lights();
            for light in lights {
                collection.remove(&light);
            }
        }
    }
}

#[derive(Default)]
struct RevealState {
    brush: Option<Rc<CompositionBrush>>,
    is_connected: bool,
    is_border_light_set: bool,
    is_hover_light_set: bool,
    is_ambient_light_set: bool,
    /// The border light id the brush targets while it is set.
    border_light_id: Option<&'static str>,
    has_base_color: bool,
    is_disabled_by_policy: bool,
    policy_subscription: Option<Rc<dyn IDisposable>>,
}

/// The base of the Reveal brushes (WinUI `Microsoft.UI.Xaml.Media.RevealBrush`):
/// a brush lit by the Reveal lights near the pointer.
///
/// A [`RevealBackgroundBrush`] is lit by the hover and press lights the
/// element whose [`state`](Self::state_property) is set holds; a
/// [`RevealBorderBrush`] by the border lights of the window. In fallback
/// mode ([`always_use_fallback`](Self::always_use_fallback), or when the
/// [`MaterialPolicy`] disables materials) the brush is its
/// [`fallback_color`](XamlCompositionBrushBase::fallback_color).
#[repr(C)]
pub struct RevealBrush {
    base: XamlCompositionBrushBase,
    state: RefCell<RevealState>,
}

ferro_class!(RevealBrush: XamlCompositionBrushBase);
ferroui_base::ferro_class_info!(RevealBrush {
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(RevealBrush: BrushImpl);

impl FerroObjectImpl for RevealBrush {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::color_property().as_property() {
            this.update_reveal_brush();
        } else if property == Self::target_theme_property().as_property()
            || property == Self::always_use_fallback_property().as_property()
        {
            this.update_light_targets(false);
            this.update_reveal_brush();
        } else if property == XamlCompositionBrushBase::fallback_color_property().as_property() && this.is_in_fallback_mode() {
            this.update_light_targets(false);
            this.update_reveal_brush();
        }
    }
}

impl XamlCompositionBrushBaseImpl for RevealBrush {
    fn on_connected(this: &Self) {
        on_reveal_brush_connected();
        this.state.borrow_mut().is_connected = true;
        let weak = this.to_ref().downgrade();
        let subscription = MaterialPolicy::policy_changed(move |disabled| {
            if let Some(this) = weak.upgrade() {
                this.state.borrow_mut().is_disabled_by_policy = disabled;
                if this.state.borrow().is_connected && this.state.borrow().policy_subscription.is_some() {
                    this.update_light_targets(true);
                    this.update_reveal_brush();
                }
            }
        });
        this.state.borrow_mut().policy_subscription = Some(subscription);
        this.update_light_targets(true);
        this.update_reveal_brush();
    }

    fn on_disconnected(this: &Self) {
        this.state.borrow_mut().is_connected = false;
        this.update_light_targets(true);
        let (brush, subscription) = {
            let mut state = this.state.borrow_mut();
            (state.brush.take(), state.policy_subscription.take())
        };
        if let Some(brush) = brush {
            brush.object().dispose();
        }
        this.set_composition_brush(None);
        on_reveal_brush_disconnected();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

ferroui_base::ferro_properties! { impl RevealBrush {
    ferro_property!(
        /// `Color` (default transparent white): the base colour the light is
        /// mixed with.
        pub fn color_property() -> StyledProperty<Color> {
            FerroProperty::register::<RevealBrush, _>("Color", Color::from_argb(0, 255, 255, 255))
        }
    );

    ferro_property!(
        /// `TargetTheme` (default light): the theme the brush is drawn for;
        /// the border light of the light theme darkens.
        pub fn target_theme_property() -> StyledProperty<ApplicationTheme> {
            FerroProperty::register::<RevealBrush, _>("TargetTheme", ApplicationTheme::Light)
        }
    );

    ferro_property!(
        /// `AlwaysUseFallback` (default `false`).
        pub fn always_use_fallback_property() -> StyledProperty<bool> {
            FerroProperty::register::<RevealBrush, _>("AlwaysUseFallback", false)
        }
    );

    ferro_property!(
        /// `State` (attached): the Reveal state of an element, which drives
        /// its hover and press lights.
        pub fn state_property() -> AttachedProperty<RevealBrushState> {
            FerroProperty::register_attached::<RevealBrush, Visual, _>("State", RevealBrushState::Normal)
        }
    );

    ferro_property!(
        /// `IsContainer` (attached, internal in WinUI): the element holds a
        /// hover and a press light.
        pub fn is_container_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RevealBrush, Visual, _>("IsContainer", false)
        }
    );
} }

impl RevealBrush {
    pub fn construct() -> Self {
        Self { base: XamlCompositionBrushBase::construct(), state: RefCell::new(RevealState::default()) }
    }

    fn static_constructor() {
        // `OnIsContainerPropertyChanged`.
        let _ = Self::is_container_property().changed().subscribe(|e| {
            let Some(element) = e.sender().downcast_ref::<Visual>() else { return };
            let lights = element.lights();
            if e.get_new_value::<bool>() {
                lights.add(RevealHoverLight::new().upcast());
                lights.add(RevealHoverLight::new_press_light().upcast());
            } else {
                for light in lights.to_vec() {
                    if light.is::<RevealHoverLight>() {
                        lights.remove(&light);
                    }
                }
            }
        });
        // `OnStatePropertyChanged`.
        let _ = Self::state_property().changed().subscribe(|e| {
            let (old, new) = e.get_old_and_new_value::<RevealBrushState>();
            if old == new {
                return;
            }
            let Some(element) = e.sender().downcast_ref::<Visual>() else { return };
            if new != RevealBrushState::Normal {
                element.set_value(Self::is_container_property(), true);
            }
            for light in element.lights().to_vec() {
                if let Some(hover) = light.downcast_ref::<RevealHoverLight>() {
                    hover.go_to_state(new);
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

    pub fn target_theme(&self) -> ApplicationTheme {
        self.get_value(Self::target_theme_property())
    }

    pub fn set_target_theme(&self, value: ApplicationTheme) {
        self.set_value(Self::target_theme_property(), value)
    }

    pub fn always_use_fallback(&self) -> bool {
        self.get_value(Self::always_use_fallback_property())
    }

    pub fn set_always_use_fallback(&self, value: bool) {
        self.set_value(Self::always_use_fallback_property(), value)
    }

    /// `GetState`.
    pub fn get_state(element: &Visual) -> RevealBrushState {
        element.get_value(Self::state_property())
    }

    /// `SetState`.
    pub fn set_state(element: &Visual, value: RevealBrushState) {
        element.set_value(Self::state_property(), value)
    }

    /// Whether the brush is a border brush.
    pub fn is_border(&self) -> bool {
        self.is::<RevealBorderBrush>()
    }

    /// `IsInFallbackMode`: the policy disables materials or
    /// `AlwaysUseFallback` is set.
    ///
    /// Deviation (winui-materials.md): WinUI 3 always draws the fallback
    /// colour (its policy callback passes `true`); FerroUI draws Reveal
    /// unless the policy disables materials.
    pub fn is_in_fallback_mode(&self) -> bool {
        self.state.borrow().is_disabled_by_policy || self.always_use_fallback()
    }

    fn border_light_id(&self) -> &'static str {
        if self.target_theme() == ApplicationTheme::Light {
            REVEAL_BORDER_LIGHT_LIGHT_THEME_ID
        } else {
            REVEAL_BORDER_LIGHT_DARK_THEME_ID
        }
    }

    /// `UpdateLightTargets`: the brush targets the hover light (or the
    /// border light of its theme) and the ambient light while it is
    /// connected and not in fallback mode.
    fn update_light_targets(&self, ambient_too: bool) {
        let active = self.state.borrow().is_connected && !self.is_in_fallback_mode();
        let is_border = self.is_border();
        if !active {
            let (border, hover, ambient) = {
                let state = self.state.borrow();
                (state.border_light_id.filter(|_| state.is_border_light_set), state.is_hover_light_set, state.is_ambient_light_set)
            };
            if let Some(id) = border.filter(|_| is_border) {
                self.remove_light_target_id(id);
                self.state.borrow_mut().is_border_light_set = false;
            } else if hover {
                self.remove_light_target_id(REVEAL_HOVER_LIGHT_ID);
                self.state.borrow_mut().is_hover_light_set = false;
            }
            if ambient_too && ambient {
                self.remove_light_target_id(XAML_AMBIENT_LIGHT_ID);
                self.state.borrow_mut().is_ambient_light_set = false;
            }
            return;
        }
        if is_border {
            // The border light of the theme may change: always refreshed.
            let previous = self.state.borrow().border_light_id.filter(|_| self.state.borrow().is_border_light_set);
            if let Some(previous) = previous {
                self.remove_light_target_id(previous);
            }
            let id = self.border_light_id();
            self.add_light_target_id(id);
            let mut state = self.state.borrow_mut();
            state.border_light_id = Some(id);
            state.is_border_light_set = true;
        } else if !self.state.borrow().is_hover_light_set {
            self.add_light_target_id(REVEAL_HOVER_LIGHT_ID);
            self.state.borrow_mut().is_hover_light_set = true;
        }
        if ambient_too && !self.state.borrow().is_ambient_light_set {
            self.add_light_target_id(XAML_AMBIENT_LIGHT_ID);
            self.state.borrow_mut().is_ambient_light_set = true;
        }
    }

    /// `UpdateRevealBrush` / `CreateRevealBrush`.
    fn update_reveal_brush(&self) {
        if !self.state.borrow().is_connected {
            return;
        }
        let Some(compositor) = self.compositor() else { return };
        let brush: Option<Rc<CompositionBrush>> = if !self.is_in_fallback_mode() {
            let color = self.color();
            // The base colour version of the effect once the colour is not
            // transparent black.
            if color != Color::from_argb(0, 0, 0, 0) {
                self.state.borrow_mut().has_base_color = true;
            }
            let has_base_color = self.state.borrow().has_base_color;
            let is_border = self.is_border();
            let is_inverted = is_border && self.target_theme() == ApplicationTheme::Light;
            let effect_brush = factory(&compositor, is_border, is_inverted, has_base_color).create_brush();
            if has_base_color {
                effect_brush.properties().insert_color(BASE_COLOR_EFFECT_COLOR, color);
            }
            if !is_border {
                // The border brush covers a few pixels and shows no banding: no
                // noise.
                effect_brush.set_source_parameter("Noise", Some((*MaterialPolicy::noise_brush(&compositor)).clone()));
            }
            Some(effect_brush.brush().clone())
        } else {
            let fallback = self.fallback_color();
            // A transparent fallback draws nothing.
            (fallback.a != 0).then(|| (*compositor.create_color_brush_with_color(fallback)).clone())
        };
        let old = std::mem::replace(&mut self.state.borrow_mut().brush, brush.clone());
        self.set_composition_brush(brush);
        if let Some(old) = old {
            old.object().dispose();
        }
    }
}

/// The Reveal brush of backgrounds (WinUI
/// `Microsoft.UI.Xaml.Media.RevealBackgroundBrush`): lit by the hover and
/// press lights of the element whose Reveal state is set.
#[repr(C)]
pub struct RevealBackgroundBrush {
    base: RevealBrush,
}

ferro_class!(RevealBackgroundBrush: RevealBrush);
ferroui_base::ferro_class_info!(RevealBackgroundBrush {
    new: RevealBackgroundBrush::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(RevealBackgroundBrush: FerroObjectImpl, BrushImpl, XamlCompositionBrushBaseImpl);

impl RevealBackgroundBrush {
    pub fn construct() -> Self {
        Self { base: RevealBrush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The Reveal brush of borders (WinUI
/// `Microsoft.UI.Xaml.Media.RevealBorderBrush`): lit by the border lights of
/// the window near the pointer.
#[repr(C)]
pub struct RevealBorderBrush {
    base: RevealBrush,
}

ferro_class!(RevealBorderBrush: RevealBrush);
ferroui_base::ferro_class_info!(RevealBorderBrush {
    new: RevealBorderBrush::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(RevealBorderBrush: FerroObjectImpl, BrushImpl, XamlCompositionBrushBaseImpl);

impl RevealBorderBrush {
    pub fn construct() -> Self {
        Self { base: RevealBrush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The number of connected Reveal brushes (for tests).
pub fn connected_reveal_brushes() -> usize {
    CONNECTED_BRUSHES.with(Cell::get)
}
