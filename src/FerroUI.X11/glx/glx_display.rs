//! The frame buffer configuration and the contexts of GLX on a connection
//! (the port of `Glx/GlxDisplay.cs`).

use super::glx::{GlxFbConfig, GlxInterface};
use super::glx_consts::*;
use super::glx_context::GlxContext;
use crate::x11_info::X11Info;
use crate::x11_platform::X11PlatformOptions;
use crate::xlib::{self, VisualInfo, XDisplay, XID};
use ferroui_base::utilities::ThreadBound;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_opengl::{GlProfileType, GlVersion, IGlContext, OpenGlException};
use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};

/// The environment variable that lets a renderer of the blacklist be used
/// (`1`).
pub const IGNORE_RENDERER_BLACKLIST_VARIABLE: &str = "FERROUI_GLX_IGNORE_RENDERER_BLACKLIST";

/// The attributes of the frame buffer configuration that is asked for.
pub(crate) const BASE_ATTRIBS: [i32; 20] = [
    GLX_X_RENDERABLE,
    1,
    GLX_RENDER_TYPE,
    GLX_RGBA_BIT,
    GLX_DRAWABLE_TYPE,
    GLX_WINDOW_BIT | GLX_PBUFFER_BIT,
    GLX_DOUBLEBUFFER,
    1,
    GLX_RED_SIZE,
    8,
    GLX_GREEN_SIZE,
    8,
    GLX_BLUE_SIZE,
    8,
    GLX_ALPHA_SIZE,
    8,
    GLX_DEPTH_SIZE,
    1,
    GLX_STENCIL_SIZE,
    8,
];

/// Which of the configurations the library offers is used: the first, or
/// the first whose visual has a depth of 32 bits when there is one
/// (the loop of the constructor of the reference). `depths` has the depth
/// of the visual of each configuration, `None` for a configuration
/// without a visual.
pub(crate) fn choose_configuration(depths: &[Option<i32>]) -> Option<usize> {
    let mut chosen = None;
    for (c, depth) in depths.iter().enumerate() {
        // We prefer 32 bit visuals
        if chosen.is_none() || *depth == Some(32) {
            chosen = Some(c);
            if *depth == Some(32) {
                break;
            }
        }
    }
    chosen
}

/// The profile mask of the context attributes for a version (`Create`).
pub(crate) fn profile_mask(profile: GlVersion) -> i32 {
    let mut profile_mask = GLX_CONTEXT_CORE_PROFILE_BIT_ARB;
    if profile.type_() == GlProfileType::OpenGL
        && profile.is_compatibility_profile()
        && (profile.major() > 3 || profile.major() == 3 && profile.minor() >= 2)
    {
        profile_mask = GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB;
    } else if profile.type_() == GlProfileType::OpenGLES {
        profile_mask = GLX_CONTEXT_ES2_PROFILE_BIT_EXT;
    }
    profile_mask
}

/// The attributes a context of a version is created with.
pub(crate) fn context_attribs(profile: GlVersion) -> [i32; 7] {
    [
        GLX_CONTEXT_MAJOR_VERSION_ARB,
        profile.major(),
        GLX_CONTEXT_MINOR_VERSION_ARB,
        profile.minor(),
        GLX_CONTEXT_PROFILE_MASK_ARB,
        profile_mask(profile),
        0,
    ]
}

/// The item of the blacklist a renderer is refused for, when there is one.
pub(crate) fn blacklisted_by<'a>(renderer: &str, blacklist: &'a [String]) -> Option<&'a str> {
    blacklist.iter().map(String::as_str).find(|item| renderer.contains(item))
}

/// The frame buffer configuration of the connection that renders, its
/// visual, and the contexts created with it.
///
/// The display is shared by the UI thread, which creates it and reads the
/// visual for its windows, and the thread that renders, which creates the
/// contexts: what it holds is the connection, identifiers and values. The
/// context the constructor probes with (`DeferredContext`) is an object of
/// the thread that created the display and is only reached there.
pub struct GlxDisplay {
    this: std::sync::Weak<GlxDisplay>,
    deferred_display: XDisplay,
    probe_profiles: Vec<GlVersion>,
    fbconfig: GlxFbConfig,
    visual: VisualInfo,
    display_extensions: Vec<String>,
    version: Mutex<Option<GlVersion>>,
    sample_count: i32,
    stencil_size: i32,
    glx: GlxInterface,
    deferred_context: ThreadBound<OnceCell<Rc<GlxContext>>>,
}

// SAFETY: apart from the probe context, which `ThreadBound` keeps to its
// thread, the display holds values and one pointer, the visual: it belongs
// to the connection, which is never closed, and is only handed to Xlib,
// which reads its identifier.
unsafe impl Send for GlxDisplay {}
// SAFETY: see `Send`; the version is behind a lock.
unsafe impl Sync for GlxDisplay {}

impl GlxDisplay {
    pub fn visual_info(&self) -> VisualInfo {
        self.visual
    }

    /// The context the display was probed with.
    ///
    /// # Panics
    /// Panics on a thread other than the one that created the display.
    pub fn deferred_context(&self) -> Rc<GlxContext> {
        self.deferred_context.get().get().expect("the context is set by the constructor").clone()
    }

    pub fn glx(&self) -> &GlxInterface {
        &self.glx
    }

    /// The connection the contexts are created and made current on
    /// (`X11Info.DeferredDisplay`).
    pub fn deferred_display(&self) -> XDisplay {
        self.deferred_display
    }

    pub fn fb_config(&self) -> GlxFbConfig {
        self.fbconfig
    }

    pub fn new(x11: &X11Info, probe_profiles: &[GlVersion]) -> Result<Arc<GlxDisplay>, OpenGlException> {
        let glx = GlxInterface::new()?;
        let deferred_display = x11.deferred_display();
        let display_extensions = glx.get_extensions(deferred_display);

        let mut attribs = BASE_ATTRIBS.to_vec();
        attribs.push(0);
        let configs = glx.choose_fb_config(deferred_display, x11.default_screen(), &attribs);
        let visuals: Vec<Option<VisualInfo>> =
            configs.iter().map(|config| glx.get_visual_from_fb_config(deferred_display, *config)).collect();
        let depths: Vec<Option<i32>> = visuals.iter().map(|visual| visual.map(|visual| visual.depth)).collect();

        let Some(chosen) = choose_configuration(&depths) else {
            return Err(OpenGlException::new("Unable to choose FBConfig"));
        };
        let fbconfig = configs[chosen];

        let Some(visual) = visuals[chosen] else {
            return Err(OpenGlException::new("Unable to get visual info from FBConfig"));
        };
        let sample_count = glx.get_fb_config_attrib(deferred_display, fbconfig, GLX_SAMPLES).unwrap_or(0);
        let stencil_size = glx.get_fb_config_attrib(deferred_display, fbconfig, GLX_STENCIL_SIZE).unwrap_or(0);

        let attributes = [GLX_PBUFFER_WIDTH, 1, GLX_PBUFFER_HEIGHT, 1, 0];

        // As the reference, which creates two pixel buffers here that it does not use.
        glx.create_pbuffer(deferred_display, fbconfig, &attributes);
        glx.create_pbuffer(deferred_display, fbconfig, &attributes);

        xlib::x_flush(deferred_display);

        let display = Arc::new_cyclic(|this| GlxDisplay {
            this: this.clone(),
            deferred_display,
            probe_profiles: probe_profiles.to_vec(),
            fbconfig,
            visual,
            display_extensions,
            version: Mutex::new(None),
            sample_count,
            stencil_size,
            glx,
            deferred_context: ThreadBound::new(OnceCell::new()),
        });

        let deferred_context = display.create_context_core(display.create_p_buffer(), None, sample_count, stencil_size, true)?;
        let _ = display.deferred_context.get().set(deferred_context.clone());
        {
            let current = deferred_context.make_current();
            let gl_interface = deferred_context.gl_interface();
            let check = (|| {
                if gl_interface.version().is_none() {
                    return Err(OpenGlException::new("GL version string is null, aborting"));
                }
                let Some(renderer) = gl_interface.renderer() else {
                    return Err(OpenGlException::new("GL renderer string is null, aborting"));
                };

                if std::env::var(IGNORE_RENDERER_BLACKLIST_VARIABLE).ok().as_deref() != Some("1") {
                    let opts = FerroLocator::current()
                        .get_service::<X11PlatformOptions>()
                        .map(|options| (*options).clone())
                        .unwrap_or_default();
                    if let Some(item) = blacklisted_by(renderer, &opts.glx_renderer_blacklist) {
                        return Err(OpenGlException::new(format!("Renderer '{renderer}' is blacklisted by '{item}'")));
                    }
                }
                Ok(())
            })();
            current.dispose();
            check?;
        }

        Ok(display)
    }

    fn create_p_buffer(&self) -> XID {
        self.glx.create_pbuffer(self.deferred_display, self.fbconfig, &[GLX_PBUFFER_WIDTH, 1, GLX_PBUFFER_HEIGHT, 1, 0])
    }

    /// A context with a pixel buffer of its own as its default drawable
    /// (`CreateContext()`).
    pub fn create_context(&self) -> Result<Rc<GlxContext>, OpenGlException> {
        self.create_context_core(self.create_p_buffer(), None, self.sample_count, self.stencil_size, true)
    }

    /// A context that shares its objects with `share`
    /// (`CreateContext(IGlContext share)`).
    pub fn create_context_shared_with(&self, share: &Rc<GlxContext>) -> Result<Rc<GlxContext>, OpenGlException> {
        self.create_context_core(
            self.create_p_buffer(),
            Some(share.clone()),
            share.sample_count(),
            share.stencil_size(),
            true,
        )
    }

    fn create_context_core(
        &self,
        default_xid: XID,
        share: Option<Rc<GlxContext>>,
        sample_count: i32,
        stencil_size: i32,
        owns_p_buffer: bool,
    ) -> Result<Rc<GlxContext>, OpenGlException> {
        let sharelist = share.as_ref().map_or(0, |share| share.handle());
        let this = self.this.upgrade().expect("the display is alive");

        let create = |profile: GlVersion| -> Option<Rc<GlxContext>> {
            let attrs = context_attribs(profile);

            let handle = self.glx.create_context_attribs_arb(self.deferred_display, self.fbconfig, sharelist, true, &attrs);
            if handle != 0 {
                *self.version.lock().unwrap_or_else(PoisonError::into_inner) = Some(profile);
                // A context that cannot be made current is passed over like one that
                // could not be created (the `catch` of the reference).
                return GlxContext::new(
                    self.glx,
                    handle,
                    &this,
                    share.clone(),
                    profile,
                    sample_count,
                    stencil_size,
                    default_xid,
                    owns_p_buffer,
                )
                .ok();
            }

            None
        };

        let mut rv = None;
        let version = *self.version.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(version) = version {
            rv = create(version);
        }

        if rv.is_none() {
            for v in &self.probe_profiles {
                if v.type_() == GlProfileType::OpenGLES
                    && !self.display_extensions.iter().any(|e| e == "GLX_EXT_create_context_es2_profile")
                {
                    continue;
                }
                rv = create(*v);
                if rv.is_some() {
                    *self.version.lock().unwrap_or_else(PoisonError::into_inner) = Some(*v);
                    break;
                }
            }
        }

        rv.ok_or_else(|| OpenGlException::new("Unable to create direct GLX context"))
    }

    pub fn swap_buffers(&self, xid: XID) {
        self.glx.swap_buffers(self.deferred_display, xid);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_first_configuration_is_used_unless_one_has_a_visual_of_depth_32() {
        assert_eq!(choose_configuration(&[]), None);
        assert_eq!(choose_configuration(&[Some(24), Some(24)]), Some(0));
        assert_eq!(choose_configuration(&[Some(24), Some(32), Some(32)]), Some(1));
        assert_eq!(choose_configuration(&[Some(32), Some(24)]), Some(0));
        // A configuration without a visual is taken when it is the first, as in the reference,
        // which then fails for the missing visual.
        assert_eq!(choose_configuration(&[None, Some(24)]), Some(0));
        assert_eq!(choose_configuration(&[None, Some(24), Some(32)]), Some(2));
    }

    #[test]
    fn the_configuration_asks_for_a_double_buffered_rgba_window_with_stencil() {
        let value = |name: i32| BASE_ATTRIBS.chunks(2).find(|pair| pair[0] == name).map(|pair| pair[1]);
        assert_eq!(value(GLX_X_RENDERABLE), Some(1));
        assert_eq!(value(GLX_RENDER_TYPE), Some(GLX_RGBA_BIT));
        assert_eq!(value(GLX_DRAWABLE_TYPE), Some(GLX_WINDOW_BIT | GLX_PBUFFER_BIT));
        assert_eq!(value(GLX_DOUBLEBUFFER), Some(1));
        assert_eq!(value(GLX_ALPHA_SIZE), Some(8));
        assert_eq!(value(GLX_STENCIL_SIZE), Some(8));
    }

    #[test]
    fn the_profile_of_a_context_follows_its_version() {
        let core = GlVersion::new(GlProfileType::OpenGL, 4, 0);
        assert_eq!(profile_mask(core), GLX_CONTEXT_CORE_PROFILE_BIT_ARB);
        let compatibility = GlVersion::with_compatibility_profile(GlProfileType::OpenGL, 3, 2, true);
        assert_eq!(profile_mask(compatibility), GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB);
        // Before 3.2 there are no profiles: the bit of the core profile is sent, as in the reference.
        let old = GlVersion::with_compatibility_profile(GlProfileType::OpenGL, 3, 0, true);
        assert_eq!(profile_mask(old), GLX_CONTEXT_CORE_PROFILE_BIT_ARB);
        let es = GlVersion::new(GlProfileType::OpenGLES, 3, 0);
        assert_eq!(profile_mask(es), GLX_CONTEXT_ES2_PROFILE_BIT_EXT);

        assert_eq!(
            context_attribs(GlVersion::new(GlProfileType::OpenGLES, 3, 2)),
            [
                GLX_CONTEXT_MAJOR_VERSION_ARB,
                3,
                GLX_CONTEXT_MINOR_VERSION_ARB,
                2,
                GLX_CONTEXT_PROFILE_MASK_ARB,
                GLX_CONTEXT_ES2_PROFILE_BIT_EXT,
                0
            ]
        );
    }

    #[test]
    fn a_renderer_is_refused_for_the_first_item_it_contains() {
        let blacklist = X11PlatformOptions::new().glx_renderer_blacklist;
        assert_eq!(blacklisted_by("llvmpipe (LLVM 20.1.2, 128 bits)", &blacklist), Some("llvmpipe"));
        assert_eq!(blacklisted_by("SVGA3D; build: RELEASE;  LLVM;", &blacklist), Some("SVGA3D"));
        assert_eq!(blacklisted_by("Mesa Intel(R) UHD Graphics 620 (KBL GT2)", &blacklist), None);
        assert_eq!(blacklisted_by("llvmpipe", &[]), None);
    }
}
