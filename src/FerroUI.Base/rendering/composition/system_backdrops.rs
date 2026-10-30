//! `Microsoft.UI.Composition.SystemBackdrops` (winui-materials.md): the
//! controllers of the system backdrops (Mica and desktop acrylic), their
//! configuration and their targets.
//!
//! A controller applies its material to each target: the platform draws it
//! behind the window where it can (DWM system backdrop types on Windows, an
//! `NSVisualEffectView` on macOS); where the platform blurs what is behind a
//! transparent window, the compositor draws the tint and the luminosity of
//! the material over it; elsewhere, and when the material falls back (high
//! contrast, inactive window, energy saver, transparency effects off), the
//! target shows the fallback colour.

use crate::media::{
    acrylic_effect, luminosity_color, Color, MaterialPolicy, ACRYLIC_NOISE_OPACITY,
};
use crate::platform::{IPlatformSettings, PlatformThemeVariant};
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::effects::{
    BorderEffect, CanvasComposite, CanvasEdgeBehavior, ColorSourceEffect, CompositeEffect,
    CompositionEffectSourceParameter, OpacityEffect,
};
use crate::rendering::composition::{CompositionBrush, Compositor};
use crate::utilities::HandlerList;
use crate::{FerroLocator, LocatorExtensions};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The theme of a system backdrop (WinUI `SystemBackdropTheme`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SystemBackdropTheme {
    /// The theme of the platform.
    #[default]
    Default = 0,
    Light = 1,
    Dark = 2,
}

crate::ferro_markup_enum!(SystemBackdropTheme { Default, Light, Dark }, { namespace: "FerroUI.Media" });

/// The state of a system backdrop controller (WinUI `SystemBackdropState`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SystemBackdropState {
    /// The material is drawn.
    Active = 0,
    /// The fallback colour is drawn.
    Fallback = 1,
    /// Nothing is drawn (the controller has no target, or is closed).
    #[default]
    Hidden = 2,
}

/// The kind of Mica (WinUI `MicaKind`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MicaKind {
    /// The material of a window's background.
    #[default]
    Base = 0,
    /// The material of a tabbed window's title bar, more tinted with the
    /// wallpaper.
    BaseAlt = 1,
}

crate::ferro_markup_enum!(MicaKind { Base, BaseAlt }, { namespace: "FerroUI.Media" });

/// The kind of desktop acrylic (WinUI `DesktopAcrylicKind`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DesktopAcrylicKind {
    /// The platform's default (`Base`).
    #[default]
    Default = 0,
    Base = 1,
    /// A more transparent acrylic.
    Thin = 2,
}

crate::ferro_markup_enum!(DesktopAcrylicKind { Default, Base, Thin }, { namespace: "FerroUI.Media" });

/// The material a platform draws behind a window (a FerroUI addition: what
/// a controller asks of its target).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlatformSystemBackdropKind {
    Mica(MicaKind),
    DesktopAcrylic(DesktopAcrylicKind),
}

/// A system backdrop as the platform is asked to draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformSystemBackdrop {
    pub kind: PlatformSystemBackdropKind,
    /// `Light` or `Dark` (never `Default`).
    pub theme: SystemBackdropTheme,
    pub tint_color: Color,
    pub tint_opacity: f32,
    pub luminosity_opacity: f32,
    pub fallback_color: Color,
}

/// How a target can show a system backdrop (a FerroUI addition).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemBackdropSupport {
    /// The platform draws the material itself.
    Native,
    /// The platform blurs what is behind the transparent window: the
    /// compositor draws the tint and the luminosity of the material over it.
    BlurBehind,
    /// The target is in a window (an element of the window, such as the
    /// presenter of a menu in a popup of the window): the compositor draws
    /// the material as in-app acrylic of what is behind the target.
    InApp,
    /// The material cannot be drawn: the fallback colour is shown.
    None,
}

/// A target of a system backdrop (WinUI
/// `Microsoft.UI.Composition.ICompositionSupportsSystemBackdrop`): a window,
/// or a popup window. FerroUI's windows implement it.
pub trait ICompositionSupportsSystemBackdrop {
    /// `SystemBackdrop`: the brush the target draws under its content.
    fn system_backdrop(&self) -> Option<Rc<CompositionBrush>>;

    /// Sets the brush the target draws under its content (`None` draws
    /// nothing, so what the platform draws behind the window shows).
    fn set_system_backdrop(&self, brush: Option<Rc<CompositionBrush>>);

    /// Asks the platform to draw `backdrop` behind the target (`None`
    /// removes it) and says how the target can show it (a FerroUI
    /// addition).
    fn try_set_platform_system_backdrop(&self, _backdrop: Option<&PlatformSystemBackdrop>) -> SystemBackdropSupport {
        SystemBackdropSupport::None
    }

    /// The compositor the target draws with.
    fn compositor(&self) -> Option<Rc<Compositor>>;
}

/// The policy of a system backdrop (WinUI `SystemBackdropConfiguration`):
/// whether its window is active, its theme and the high contrast state. The
/// XAML `SystemBackdrop` keeps one per target up to date.
pub struct SystemBackdropConfiguration {
    is_input_active: Cell<bool>,
    theme: Cell<SystemBackdropTheme>,
    is_high_contrast: Cell<bool>,
    high_contrast_background_color: Cell<Option<Color>>,
    changed: Rc<HandlerList<dyn Fn()>>,
}

impl Default for SystemBackdropConfiguration {
    fn default() -> Self {
        Self {
            is_input_active: Cell::new(false),
            theme: Cell::new(SystemBackdropTheme::Default),
            is_high_contrast: Cell::new(false),
            high_contrast_background_color: Cell::new(None),
            changed: Rc::new(HandlerList::new()),
        }
    }
}

impl SystemBackdropConfiguration {
    /// A configuration: not active, the default theme, no high contrast.
    pub fn new() -> Rc<SystemBackdropConfiguration> {
        Rc::new(Self::default())
    }

    /// `IsInputActive`: whether the window of the target has the input; an
    /// inactive window shows the fallback colour.
    pub fn is_input_active(&self) -> bool {
        self.is_input_active.get()
    }

    pub fn set_is_input_active(&self, value: bool) {
        if self.is_input_active.replace(value) != value {
            self.raise_changed();
        }
    }

    /// `Theme`.
    pub fn theme(&self) -> SystemBackdropTheme {
        self.theme.get()
    }

    pub fn set_theme(&self, value: SystemBackdropTheme) {
        if self.theme.replace(value) != value {
            self.raise_changed();
        }
    }

    /// `IsHighContrast`: the material shows the high contrast background
    /// colour.
    pub fn is_high_contrast(&self) -> bool {
        self.is_high_contrast.get()
    }

    pub fn set_is_high_contrast(&self, value: bool) {
        if self.is_high_contrast.replace(value) != value {
            self.raise_changed();
        }
    }

    /// `HighContrastBackgroundColor`: the colour shown in high contrast
    /// (`None`: the fallback colour).
    pub fn high_contrast_background_color(&self) -> Option<Color> {
        self.high_contrast_background_color.get()
    }

    pub fn set_high_contrast_background_color(&self, value: Option<Color>) {
        if self.high_contrast_background_color.replace(value) != value {
            self.raise_changed();
        }
    }

    /// Raised when a member changes. Disposing the handle unsubscribes.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let list = Rc::downgrade(&self.changed);
        Disposable::create(move || {
            if let Some(list) = list.upgrade() {
                list.remove(token);
            }
        })
    }

    fn raise_changed(&self) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }
}

/// The colours of a material (`TintColor`, `TintOpacity`,
/// `LuminosityOpacity`, `FallbackColor`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemBackdropColors {
    pub tint_color: Color,
    pub tint_opacity: f32,
    pub luminosity_opacity: f32,
    pub fallback_color: Color,
}

const fn colors(tint: u32, tint_opacity: f32, luminosity_opacity: f32, fallback: u32) -> SystemBackdropColors {
    SystemBackdropColors {
        tint_color: Color::from_uint32(0xFF00_0000 | tint),
        tint_opacity,
        luminosity_opacity,
        fallback_color: Color::from_uint32(0xFF00_0000 | fallback),
    }
}

/// The default colours of Mica of a kind in a theme (`ResetProperties`).
///
/// Deviation (winui-materials.md): the controllers of the Windows App SDK
/// do not publish their defaults; these are the values they report, to be
/// confirmed on Windows.
pub fn mica_default_colors(kind: MicaKind, theme: SystemBackdropTheme) -> SystemBackdropColors {
    let dark = theme == SystemBackdropTheme::Dark;
    match (kind, dark) {
        (MicaKind::Base, false) => colors(0xF3F3F3, 0.5, 1.0, 0xF3F3F3),
        (MicaKind::Base, true) => colors(0x202020, 0.8, 1.0, 0x202020),
        (MicaKind::BaseAlt, false) => colors(0xDADADA, 0.5, 1.0, 0xE8E8E8),
        (MicaKind::BaseAlt, true) => colors(0x0A0A0A, 0.0, 1.0, 0x202020),
    }
}

/// The default colours of desktop acrylic of a kind in a theme (see
/// [`mica_default_colors`]).
pub fn desktop_acrylic_default_colors(kind: DesktopAcrylicKind, theme: SystemBackdropTheme) -> SystemBackdropColors {
    let dark = theme == SystemBackdropTheme::Dark;
    match (kind, dark) {
        (DesktopAcrylicKind::Thin, false) => colors(0xD3D3D3, 0.0, 0.44, 0xD3D3D3),
        (DesktopAcrylicKind::Thin, true) => colors(0x545454, 0.0, 0.64, 0x545454),
        (_, false) => colors(0xF3F3F3, 0.0, 0.9, 0xF9F9F9),
        (_, true) => colors(0x202020, 0.5, 0.96, 0x2C2C2C),
    }
}

/// The theme a configuration resolves to: its own, or the platform's.
fn resolve_theme(theme: SystemBackdropTheme) -> SystemBackdropTheme {
    if theme != SystemBackdropTheme::Default {
        return theme;
    }
    let dark = FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .is_some_and(|settings| settings.get_color_values().theme_variant() == PlatformThemeVariant::Dark);
    if dark {
        SystemBackdropTheme::Dark
    } else {
        SystemBackdropTheme::Light
    }
}

/// What a controller decided for a target.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Decision {
    Active(PlatformSystemBackdrop),
    Fallback(Color),
}

struct TargetEntry {
    target: Rc<dyn ICompositionSupportsSystemBackdrop>,
    state: SystemBackdropState,
}

/// What the two controllers share (WinUI's `ISystemBackdropControllerWithTargets`).
struct ControllerCore {
    this: Weak<SystemBackdropController>,
    kind: Cell<PlatformSystemBackdropKind>,
    /// The colours the application set (`None`: the defaults of the kind
    /// and theme).
    tint_color: Cell<Option<Color>>,
    tint_opacity: Cell<Option<f32>>,
    luminosity_opacity: Cell<Option<f32>>,
    fallback_color: Cell<Option<Color>>,
    targets: RefCell<Vec<TargetEntry>>,
    configuration: RefCell<Option<(Rc<SystemBackdropConfiguration>, Rc<dyn IDisposable>)>>,
    state: Cell<SystemBackdropState>,
    state_changed: Rc<HandlerList<dyn Fn(SystemBackdropState)>>,
    policy_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    closed: Cell<bool>,
}

/// A system backdrop controller: the shared implementation of
/// [`MicaController`] and [`DesktopAcrylicController`].
pub struct SystemBackdropController {
    core: ControllerCore,
}

impl SystemBackdropController {
    fn new(kind: PlatformSystemBackdropKind) -> Rc<SystemBackdropController> {
        let controller = Rc::new_cyclic(|this| SystemBackdropController {
            core: ControllerCore {
                this: this.clone(),
                kind: Cell::new(kind),
                tint_color: Cell::new(None),
                tint_opacity: Cell::new(None),
                luminosity_opacity: Cell::new(None),
                fallback_color: Cell::new(None),
                targets: RefCell::new(Vec::new()),
                configuration: RefCell::new(None),
                state: Cell::new(SystemBackdropState::Hidden),
                state_changed: Rc::new(HandlerList::new()),
                policy_subscription: RefCell::new(None),
                closed: Cell::new(false),
            },
        });
        let weak = Rc::downgrade(&controller);
        let subscription = MaterialPolicy::policy_changed(move |_| {
            if let Some(controller) = weak.upgrade() {
                controller.update();
            }
        });
        *controller.core.policy_subscription.borrow_mut() = Some(subscription);
        controller
    }

    /// The theme the controller draws for.
    fn theme(&self) -> SystemBackdropTheme {
        let theme = self.core.configuration.borrow().as_ref().map(|(configuration, _)| configuration.theme());
        resolve_theme(theme.unwrap_or_default())
    }

    fn defaults(&self) -> SystemBackdropColors {
        match self.core.kind.get() {
            PlatformSystemBackdropKind::Mica(kind) => mica_default_colors(kind, self.theme()),
            PlatformSystemBackdropKind::DesktopAcrylic(kind) => desktop_acrylic_default_colors(kind, self.theme()),
        }
    }

    /// The colours in effect: the ones set, else the defaults.
    pub fn colors(&self) -> SystemBackdropColors {
        let defaults = self.defaults();
        SystemBackdropColors {
            tint_color: self.core.tint_color.get().unwrap_or(defaults.tint_color),
            tint_opacity: self.core.tint_opacity.get().unwrap_or(defaults.tint_opacity),
            luminosity_opacity: self.core.luminosity_opacity.get().unwrap_or(defaults.luminosity_opacity),
            fallback_color: self.core.fallback_color.get().unwrap_or(defaults.fallback_color),
        }
    }

    fn decision(&self) -> Decision {
        let colors = self.colors();
        let configuration = self.core.configuration.borrow().as_ref().map(|(configuration, _)| configuration.clone());
        if let Some(configuration) = &configuration {
            if configuration.is_high_contrast() {
                return Decision::Fallback(configuration.high_contrast_background_color().unwrap_or(colors.fallback_color));
            }
            if !configuration.is_input_active() {
                return Decision::Fallback(colors.fallback_color);
            }
        }
        if MaterialPolicy::is_disabled_by_policy() {
            return Decision::Fallback(colors.fallback_color);
        }
        Decision::Active(PlatformSystemBackdrop {
            kind: self.core.kind.get(),
            theme: self.theme(),
            tint_color: colors.tint_color,
            tint_opacity: colors.tint_opacity,
            luminosity_opacity: colors.luminosity_opacity,
            fallback_color: colors.fallback_color,
        })
    }

    /// Applies the material (or the fallback) to every target and updates
    /// the state.
    fn update(&self) {
        if self.core.closed.get() {
            return;
        }
        let decision = self.decision();
        let targets: Vec<Rc<dyn ICompositionSupportsSystemBackdrop>> =
            self.core.targets.borrow().iter().map(|entry| entry.target.clone()).collect();
        let mut states = Vec::with_capacity(targets.len());
        for target in &targets {
            states.push(apply(target, decision));
        }
        for (entry, state) in self.core.targets.borrow_mut().iter_mut().zip(states) {
            entry.state = state;
        }
        let state = {
            let targets = self.core.targets.borrow();
            if targets.is_empty() {
                SystemBackdropState::Hidden
            } else if targets.iter().any(|entry| entry.state == SystemBackdropState::Active) {
                SystemBackdropState::Active
            } else {
                SystemBackdropState::Fallback
            }
        };
        if self.core.state.replace(state) != state {
            for (_, handler) in self.core.state_changed.snapshot().iter() {
                handler(state);
            }
        }
    }

    /// `AddSystemBackdropTarget`: returns `false` when the target is already
    /// one or the controller is closed.
    pub fn add_system_backdrop_target(&self, target: Rc<dyn ICompositionSupportsSystemBackdrop>) -> bool {
        if self.core.closed.get() {
            return false;
        }
        {
            let mut targets = self.core.targets.borrow_mut();
            if targets.iter().any(|entry| Rc::ptr_eq(&entry.target, &target)) {
                return false;
            }
            targets.push(TargetEntry { target, state: SystemBackdropState::Hidden });
        }
        self.update();
        true
    }

    /// `RemoveSystemBackdropTarget`: the target shows nothing of the
    /// controller any more.
    pub fn remove_system_backdrop_target(&self, target: &Rc<dyn ICompositionSupportsSystemBackdrop>) -> bool {
        let removed = {
            let mut targets = self.core.targets.borrow_mut();
            let index = targets.iter().position(|entry| Rc::ptr_eq(&entry.target, target));
            index.map(|index| targets.remove(index))
        };
        let Some(entry) = removed else { return false };
        clear(&entry.target);
        self.update();
        true
    }

    /// `RemoveAllSystemBackdropTargets`.
    pub fn remove_all_system_backdrop_targets(&self) {
        let targets = std::mem::take(&mut *self.core.targets.borrow_mut());
        for entry in targets {
            clear(&entry.target);
        }
        self.update();
    }

    /// `SetSystemBackdropConfiguration`: the policy the controller follows
    /// (`None`: active, the theme of the platform).
    pub fn set_system_backdrop_configuration(&self, configuration: Option<Rc<SystemBackdropConfiguration>>) {
        let subscription = configuration.as_ref().map(|configuration| {
            let weak = self.core.this.clone();
            configuration.changed(move || {
                if let Some(controller) = weak.upgrade() {
                    controller.update();
                }
            })
        });
        let previous = std::mem::replace(
            &mut *self.core.configuration.borrow_mut(),
            configuration.zip(subscription),
        );
        if let Some((_, subscription)) = previous {
            subscription.dispose();
        }
        self.update();
    }

    /// The state of the controller (`State`, Windows App SDK 1.3).
    pub fn state(&self) -> SystemBackdropState {
        self.core.state.get()
    }

    /// `StateChanged`. Disposing the handle unsubscribes.
    pub fn state_changed(&self, handler: impl Fn(SystemBackdropState) + 'static) -> Rc<dyn IDisposable> {
        let token = self.core.state_changed.add(Rc::new(handler));
        let list = Rc::downgrade(&self.core.state_changed);
        Disposable::create(move || {
            if let Some(list) = list.upgrade() {
                list.remove(token);
            }
        })
    }

    /// The state the controller is in for `target`.
    pub fn target_state(&self, target: &Rc<dyn ICompositionSupportsSystemBackdrop>) -> SystemBackdropState {
        self.core
            .targets
            .borrow()
            .iter()
            .find(|entry| Rc::ptr_eq(&entry.target, target))
            .map_or(SystemBackdropState::Hidden, |entry| entry.state)
    }

    /// `TintColor`.
    pub fn tint_color(&self) -> Color {
        self.colors().tint_color
    }

    pub fn set_tint_color(&self, value: Color) {
        self.core.tint_color.set(Some(value));
        self.update();
    }

    /// `TintOpacity`.
    pub fn tint_opacity(&self) -> f32 {
        self.colors().tint_opacity
    }

    pub fn set_tint_opacity(&self, value: f32) {
        self.core.tint_opacity.set(Some(value.clamp(0.0, 1.0)));
        self.update();
    }

    /// `LuminosityOpacity`.
    pub fn luminosity_opacity(&self) -> f32 {
        self.colors().luminosity_opacity
    }

    pub fn set_luminosity_opacity(&self, value: f32) {
        self.core.luminosity_opacity.set(Some(value.clamp(0.0, 1.0)));
        self.update();
    }

    /// `FallbackColor`.
    pub fn fallback_color(&self) -> Color {
        self.colors().fallback_color
    }

    pub fn set_fallback_color(&self, value: Color) {
        self.core.fallback_color.set(Some(value));
        self.update();
    }

    /// `ResetProperties`: the colours go back to the defaults of the kind
    /// and theme.
    pub fn reset_properties(&self) {
        self.core.tint_color.set(None);
        self.core.tint_opacity.set(None);
        self.core.luminosity_opacity.set(None);
        self.core.fallback_color.set(None);
        self.update();
    }

    /// `Close`: the targets are released and the controller does nothing
    /// more.
    pub fn close(&self) {
        if self.core.closed.get() {
            return;
        }
        self.remove_all_system_backdrop_targets();
        self.core.closed.set(true);
        if let Some(subscription) = self.core.policy_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some((_, subscription)) = self.core.configuration.borrow_mut().take() {
            subscription.dispose();
        }
        self.core.state.set(SystemBackdropState::Hidden);
    }

    /// Whether the controller is closed.
    pub fn is_closed(&self) -> bool {
        self.core.closed.get()
    }

    fn set_kind(&self, kind: PlatformSystemBackdropKind) {
        if self.core.kind.replace(kind) != kind {
            self.update();
        }
    }
}

/// Removes what a controller put on `target`.
fn clear(target: &Rc<dyn ICompositionSupportsSystemBackdrop>) {
    target.try_set_platform_system_backdrop(None);
    target.set_system_backdrop(None);
}

/// Applies a decision to a target; returns the state of the target.
fn apply(target: &Rc<dyn ICompositionSupportsSystemBackdrop>, decision: Decision) -> SystemBackdropState {
    let Some(compositor) = target.compositor() else { return SystemBackdropState::Hidden };
    let fallback = |color: Color| {
        target.try_set_platform_system_backdrop(None);
        target.set_system_backdrop(Some((*compositor.create_color_brush_with_color(color)).clone()));
        SystemBackdropState::Fallback
    };
    match decision {
        Decision::Fallback(color) => fallback(color),
        Decision::Active(backdrop) => match target.try_set_platform_system_backdrop(Some(&backdrop)) {
            SystemBackdropSupport::Native => {
                target.set_system_backdrop(None);
                SystemBackdropState::Active
            }
            SystemBackdropSupport::BlurBehind => {
                target.set_system_backdrop(Some(blur_behind_brush(&compositor, &backdrop)));
                SystemBackdropState::Active
            }
            SystemBackdropSupport::InApp => {
                target.set_system_backdrop(Some(in_app_brush(&compositor, &backdrop)));
                SystemBackdropState::Active
            }
            SystemBackdropSupport::None => fallback(backdrop.fallback_color),
        },
    }
}

/// The brush of a material over what the platform blurred behind the
/// window: the luminosity colour and the tint over the transparent window,
/// with noise. The compositor cannot read what is behind the window, so the
/// luminosity is a normal layer (winui-materials.md, deviations).
pub fn blur_behind_brush(compositor: &Rc<Compositor>, backdrop: &PlatformSystemBackdrop) -> Rc<CompositionBrush> {
    let tint = Color { a: (backdrop.tint_opacity.clamp(0.0, 1.0) * 255.0).round() as u8, ..backdrop.tint_color };
    let luminosity = luminosity_color(
        Color { a: 255, ..backdrop.tint_color },
        Some(backdrop.luminosity_opacity as f64),
    );
    let effect = CompositeEffect {
        name: String::new(),
        mode: CanvasComposite::SourceOver,
        sources: vec![
            ColorSourceEffect { name: "LuminosityColor".into(), color: luminosity }.into(),
            ColorSourceEffect { name: "TintColor".into(), color: tint }.into(),
            OpacityEffect {
                name: "NoiseOpacity".into(),
                opacity: ACRYLIC_NOISE_OPACITY,
                source: Some(
                    BorderEffect {
                        name: String::new(),
                        extend_x: CanvasEdgeBehavior::Wrap,
                        extend_y: CanvasEdgeBehavior::Wrap,
                        source: Some(CompositionEffectSourceParameter::new("Noise").into()),
                    }
                    .into(),
                ),
            }
            .into(),
        ],
    };
    let brush = compositor.create_effect_factory(effect).create_brush();
    brush.set_source_parameter("Noise", Some((*MaterialPolicy::noise_brush(compositor)).clone()));
    brush.brush().clone()
}

/// The in-app acrylic of a material: the recipe of `AcrylicBrush` over what
/// is behind the target in its window (a backdrop brush), with the colours
/// of the material.
pub fn in_app_brush(compositor: &Rc<Compositor>, backdrop: &PlatformSystemBackdrop) -> Rc<CompositionBrush> {
    let tint = Color { a: (backdrop.tint_opacity.clamp(0.0, 1.0) * 255.0).round() as u8, ..backdrop.tint_color };
    let luminosity = luminosity_color(Color { a: 255, ..backdrop.tint_color }, Some(backdrop.luminosity_opacity as f64));
    let opaque = tint.a == 255;
    let (effect, animatable) = acrylic_effect(opaque, false, tint, luminosity, backdrop.fallback_color);
    let brush = compositor.create_effect_factory_with_properties(effect, &animatable).create_brush();
    brush.set_source_parameter("Noise", Some((*MaterialPolicy::noise_brush(compositor)).clone()));
    if !opaque {
        brush.set_source_parameter("Backdrop", Some((*compositor.create_backdrop_brush()).clone()));
    }
    brush.brush().clone()
}

/// The Mica controller (WinUI `Microsoft.UI.Composition.SystemBackdrops.MicaController`).
#[derive(Clone)]
pub struct MicaController(Rc<SystemBackdropController>);

impl std::ops::Deref for MicaController {
    type Target = SystemBackdropController;

    fn deref(&self) -> &SystemBackdropController {
        &self.0
    }
}

impl Default for MicaController {
    fn default() -> Self {
        Self::new()
    }
}

impl MicaController {
    /// A controller of Mica of kind `Base`.
    pub fn new() -> MicaController {
        MicaController(SystemBackdropController::new(PlatformSystemBackdropKind::Mica(MicaKind::Base)))
    }

    /// `IsSupported`: whether the platform can draw Mica (Windows 11 and
    /// macOS); elsewhere a controller shows the fallback colour.
    pub fn is_supported() -> bool {
        cfg!(any(target_os = "windows", target_os = "macos"))
    }

    /// `Kind`.
    pub fn kind(&self) -> MicaKind {
        match self.0.core.kind.get() {
            PlatformSystemBackdropKind::Mica(kind) => kind,
            PlatformSystemBackdropKind::DesktopAcrylic(_) => MicaKind::Base,
        }
    }

    pub fn set_kind(&self, kind: MicaKind) {
        self.0.set_kind(PlatformSystemBackdropKind::Mica(kind));
    }

    /// The shared controller.
    pub fn controller(&self) -> &Rc<SystemBackdropController> {
        &self.0
    }
}

/// The desktop acrylic controller (WinUI
/// `Microsoft.UI.Composition.SystemBackdrops.DesktopAcrylicController`).
#[derive(Clone)]
pub struct DesktopAcrylicController(Rc<SystemBackdropController>);

impl std::ops::Deref for DesktopAcrylicController {
    type Target = SystemBackdropController;

    fn deref(&self) -> &SystemBackdropController {
        &self.0
    }
}

impl Default for DesktopAcrylicController {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopAcrylicController {
    /// A controller of desktop acrylic of kind `Default`.
    pub fn new() -> DesktopAcrylicController {
        DesktopAcrylicController(SystemBackdropController::new(PlatformSystemBackdropKind::DesktopAcrylic(
            DesktopAcrylicKind::Default,
        )))
    }

    /// `IsSupported`: whether the platform can draw desktop acrylic
    /// (Windows and macOS, and where the platform blurs what is behind a
    /// transparent window); elsewhere a controller shows the fallback
    /// colour.
    pub fn is_supported() -> bool {
        cfg!(any(target_os = "windows", target_os = "macos", target_os = "linux"))
    }

    /// `Kind`.
    pub fn kind(&self) -> DesktopAcrylicKind {
        match self.0.core.kind.get() {
            PlatformSystemBackdropKind::DesktopAcrylic(kind) => kind,
            PlatformSystemBackdropKind::Mica(_) => DesktopAcrylicKind::Default,
        }
    }

    pub fn set_kind(&self, kind: DesktopAcrylicKind) {
        self.0.set_kind(PlatformSystemBackdropKind::DesktopAcrylic(kind));
    }

    /// The shared controller.
    pub fn controller(&self) -> &Rc<SystemBackdropController> {
        &self.0
    }
}
