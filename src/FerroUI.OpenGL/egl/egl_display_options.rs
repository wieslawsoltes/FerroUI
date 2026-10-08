use super::{EglContext, EglInterface, EglSurface};
use crate::GlVersion;
use ferroui_base::reactive::IDisposable;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

/// Chooses the configuration of a display among `configs`; `None` when none is usable.
pub type EglConfigProbeCallback = Rc<dyn Fn(&EglInterface, isize, &[isize]) -> Option<isize>>;

/// The options of a display.
#[derive(Clone, Default)]
pub struct EglDisplayOptions {
    pub egl: Option<Rc<EglInterface>>,
    pub supports_context_sharing: bool,
    pub supports_multiple_contexts: bool,
    pub allow_pbuffer_only_configs: bool,
    pub context_loss_is_display_loss: bool,
    pub device_lost_check_callback: Option<Rc<dyn Fn() -> bool>>,
    pub dispose_callback: Option<Rc<dyn Fn()>>,
    pub gl_versions: Option<Vec<GlVersion>>,
    pub probe_config: Option<EglConfigProbeCallback>,
}

/// A feature of a context: the value the context answers a query for the feature with, and
/// what disposes it when the context is disposed.
///
/// The value follows the convention of the optional features: for a feature that is a
/// contract `dyn IFoo` it holds an `Rc<dyn IFoo>`. The original tests the feature object for
/// being disposable; here the feature states it.
#[derive(Clone)]
pub struct EglContextFeature {
    pub feature: Rc<dyn Any>,
    pub disposable: Option<Rc<dyn IDisposable>>,
}

/// Creates a feature for a context that has just been created and is current.
pub type EglContextFeatureFactory = Rc<dyn Fn(&Rc<EglContext>) -> EglContextFeature>;

/// The options of a context.
#[derive(Clone, Default)]
pub struct EglContextOptions {
    pub share_with: Option<Rc<EglContext>>,
    pub offscreen_surface: Option<Rc<EglSurface>>,
    pub dispose_callback: Option<Rc<dyn Fn()>>,
    /// The features of the context besides its own, by the type that is asked for.
    pub extra_features: Option<HashMap<TypeId, EglContextFeatureFactory>>,
}

/// The options of a display that is created for a platform display.
#[derive(Clone, Default)]
pub struct EglDisplayCreationOptions {
    /// The options of the display (the base class of the original).
    pub base: EglDisplayOptions,
    pub platform_type: Option<i32>,
    pub platform_display: isize,
    pub platform_display_attrs: Option<Vec<i32>>,
}
