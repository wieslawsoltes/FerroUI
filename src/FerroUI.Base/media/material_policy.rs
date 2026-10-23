//! The material policy of WinUI's `MaterialHelper` (winui-materials.md):
//! whether acrylic (and Reveal) are drawn or fall back to a colour, and the
//! noise texture the materials share.

use crate::platform::{IPlatformSettings, PlatformMaterialSettings};
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::{
    CompositionStretch, CompositionSurfaceBrush, Compositor, ICompositionSurface, LoadedImageSurface,
};
use crate::{FerroLocator, LocatorExtensions};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The noise texture of the materials of WinUI (`NoiseAsset_256X256_PNG`),
/// 256 by 256 pixels.
pub const NOISE_ASSET_PNG: &[u8] = include_bytes!("assets/noise_asset_256x256_png.png");

type PolicyHandler = Rc<dyn Fn(bool)>;

struct PolicyState {
    initialized: bool,
    disabled: bool,
    simulate_disabled: bool,
    ignore_effects_fast: bool,
    effects_fast: bool,
    settings: Option<PlatformMaterialSettings>,
    listeners: Vec<(u64, PolicyHandler)>,
    next_token: u64,
    settings_subscription: Option<Rc<dyn IDisposable>>,
    noise: Vec<(Weak<Compositor>, CompositionSurfaceBrush)>,
}

impl PolicyState {
    const fn new() -> Self {
        Self {
            initialized: false,
            disabled: false,
            simulate_disabled: false,
            ignore_effects_fast: false,
            effects_fast: true,
            settings: None,
            listeners: Vec::new(),
            next_token: 0,
            settings_subscription: None,
            noise: Vec::new(),
        }
    }

    fn compute_disabled(&self) -> bool {
        let settings = self.settings.unwrap_or_default();
        self.simulate_disabled
            || settings.energy_saver_on
            || !settings.transparency_effects_enabled
            || !(self.effects_fast || self.ignore_effects_fast)
    }
}

crate::ferro_thread_local! {
    static POLICY: RefCell<PolicyState> = const { RefCell::new(PolicyState::new()) };
}

/// The material policy (WinUI's `MaterialHelper`): materials are disabled,
/// and acrylic draws its fallback colour, while the energy saver is on, the
/// user turned transparency effects off, the effects are not fast enough,
/// or a test simulates it.
///
/// The settings come from the platform
/// ([`IPlatformSettings::get_material_settings`]), read when the policy is
/// first asked and whenever the platform reports a change. The policy is
/// per thread, as the objects it serves.
pub struct MaterialPolicy;

impl MaterialPolicy {
    fn ensure_initialized() {
        let initialize = POLICY.with(|state| !state.borrow().initialized);
        if !initialize {
            return;
        }
        let settings = FerroLocator::current().get_service::<dyn IPlatformSettings>();
        let subscription = settings.as_ref().map(|settings| {
            settings.material_settings_changed(Rc::new(|| MaterialPolicy::update_policy_status()))
        });
        POLICY.with(|state| {
            let mut state = state.borrow_mut();
            state.initialized = true;
            state.settings = settings.map(|settings| settings.get_material_settings());
            state.settings_subscription = subscription;
            state.disabled = state.compute_disabled();
        });
    }

    /// Whether materials are disabled by the policy.
    pub fn is_disabled_by_policy() -> bool {
        Self::ensure_initialized();
        POLICY.with(|state| state.borrow().disabled)
    }

    /// `PolicyChanged`: `handler` is called now with the current state, and
    /// whenever the state changes. Disposing the returned handle
    /// unsubscribes.
    pub fn policy_changed(handler: impl Fn(bool) + 'static) -> Rc<dyn IDisposable> {
        Self::ensure_initialized();
        let handler: PolicyHandler = Rc::new(handler);
        let (token, disabled) = POLICY.with(|state| {
            let mut state = state.borrow_mut();
            let token = state.next_token;
            state.next_token += 1;
            state.listeners.push((token, handler.clone()));
            (token, state.disabled)
        });
        handler(disabled);
        Disposable::create(move || {
            let _ = POLICY.try_with(|state| state.borrow_mut().listeners.retain(|(existing, _)| *existing != token));
        })
    }

    /// Reads the settings of the platform again and tells the listeners
    /// when the state changes.
    pub fn update_policy_status() {
        let settings = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .map(|settings| settings.get_material_settings());
        POLICY.with(|state| state.borrow_mut().settings = settings);
        Self::recompute();
    }

    fn recompute() {
        Self::ensure_initialized();
        let changed = POLICY.with(|state| {
            let mut state = state.borrow_mut();
            let disabled = state.compute_disabled();
            if disabled == state.disabled {
                return None;
            }
            state.disabled = disabled;
            Some((disabled, state.listeners.iter().map(|(_, handler)| handler.clone()).collect::<Vec<_>>()))
        });
        if let Some((disabled, listeners)) = changed {
            for handler in listeners {
                handler(disabled);
            }
        }
    }

    /// `SimulateDisabledByPolicy`: whether a test makes the policy disable
    /// materials.
    pub fn simulate_disabled_by_policy() -> bool {
        POLICY.with(|state| state.borrow().simulate_disabled)
    }

    pub fn set_simulate_disabled_by_policy(value: bool) {
        POLICY.with(|state| state.borrow_mut().simulate_disabled = value);
        Self::recompute();
    }

    /// `IgnoreAreEffectsFast`: whether slow effects are ignored.
    pub fn ignore_are_effects_fast() -> bool {
        POLICY.with(|state| state.borrow().ignore_effects_fast)
    }

    pub fn set_ignore_are_effects_fast(value: bool) {
        POLICY.with(|state| state.borrow_mut().ignore_effects_fast = value);
        Self::recompute();
    }

    /// Whether the effects of the renderer are fast enough for materials
    /// (WinUI's `CompositionCapabilities.AreEffectsFast`); `true` unless a
    /// platform or the application says otherwise.
    pub fn are_effects_fast() -> bool {
        POLICY.with(|state| state.borrow().effects_fast)
    }

    pub fn set_are_effects_fast(value: bool) {
        POLICY.with(|state| state.borrow_mut().effects_fast = value);
        Self::recompute();
    }

    /// The settings of the platform the policy last read.
    pub fn platform_settings() -> PlatformMaterialSettings {
        Self::ensure_initialized();
        POLICY.with(|state| state.borrow().settings.unwrap_or_default())
    }

    /// The noise brush the materials share on `compositor` (WinUI's
    /// `MaterialHelper.GetNoiseBrush`): a surface brush of the noise texture
    /// that does not stretch and sits at the top left corner, which a border
    /// effect tiles.
    pub fn noise_brush(compositor: &Rc<Compositor>) -> CompositionSurfaceBrush {
        let existing = POLICY.with(|state| {
            let mut state = state.borrow_mut();
            state.noise.retain(|(owner, _)| owner.strong_count() > 0);
            state
                .noise
                .iter()
                .find(|(owner, _)| owner.upgrade().is_some_and(|owner| Rc::ptr_eq(&owner, compositor)))
                .map(|(_, brush)| brush.clone())
        });
        if let Some(existing) = existing {
            return existing;
        }
        let surface = LoadedImageSurface::start_load_from_stream(compositor, &mut std::io::Cursor::new(NOISE_ASSET_PNG));
        let brush = compositor.create_surface_brush_with_surface(surface as Rc<dyn ICompositionSurface>);
        brush.set_stretch(CompositionStretch::None);
        brush.set_horizontal_alignment_ratio(0.0);
        brush.set_vertical_alignment_ratio(0.0);
        POLICY.with(|state| state.borrow_mut().noise.push((Rc::downgrade(compositor), brush.clone())));
        brush
    }
}
