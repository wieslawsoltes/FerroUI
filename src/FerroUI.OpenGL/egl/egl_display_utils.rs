use super::egl_consts::*;
use super::{EglConfigProbeCallback, EglDisplayCreationOptions, EglInterface};
use crate::{GlProfileType, GlVersion, OpenGlException};
use std::rc::Rc;

pub(crate) struct EglDisplayUtils;

/// The interface the options of a display name.
///
/// # Panics
/// Panics when the options name none: the constructor of the interface that loads the EGL
/// library of the system by name (`new EglInterface()`) is not ported, a platform resolves
/// the entry points of its library and passes the interface.
pub(crate) fn required_egl(egl: &Option<Rc<EglInterface>>) -> Rc<EglInterface> {
    match egl {
        Some(egl) => egl.clone(),
        None => panic!("The options of an EGL display must name the EGL interface"),
    }
}

/// What a version of the API asks of a configuration and of a context.
struct VersionConfig {
    attributes: Vec<i32>,
    api: i32,
    renderable_type_bit: i32,
    version: GlVersion,
}

impl EglDisplayUtils {
    pub fn create_display(options: &EglDisplayCreationOptions) -> Result<isize, OpenGlException> {
        let egl = required_egl(&options.base.egl);
        let display = match options.platform_type {
            None => egl.get_display(0),
            Some(platform_type) => {
                if !egl.is_get_platform_display_ext_available() {
                    return Err(OpenGlException::new("eglGetPlatformDisplayEXT is not supported by libegl"));
                }

                egl.get_platform_display_ext(
                    platform_type,
                    options.platform_display,
                    options.platform_display_attrs.as_deref(),
                )
            }
        };

        if display == 0 {
            return Err(OpenGlException::get_formatted_exception_for_egl("eglGetDisplay", &egl));
        }
        Ok(display)
    }

    // Enumerates every config matching the attribute list and lets the probe callback pick one (or simply
    // takes the first one if no probe is supplied).
    fn choose_config_with_probe(
        egl: &EglInterface,
        display: isize,
        attribs: &[i32],
        probe: Option<&EglConfigProbeCallback>,
    ) -> Option<isize> {
        let mut num_configs = 0;
        if !egl.choose_configs(display, attribs, None, 0, &mut num_configs) || num_configs == 0 {
            return None;
        }

        let mut configs = vec![0isize; num_configs as usize];
        let config_size = configs.len() as i32;
        if !egl.choose_configs(display, attribs, Some(configs.as_mut_slice()), config_size, &mut num_configs) || num_configs == 0 {
            return None;
        }
        if num_configs as usize != configs.len() {
            configs.resize(num_configs as usize, 0);
        }

        match probe {
            None => Some(configs[0]),
            Some(probe) => probe(egl, display, &configs),
        }
    }

    pub fn initialize_and_get_config(
        egl: &EglInterface,
        display: isize,
        versions: Option<&[GlVersion]>,
        probe_config: Option<&EglConfigProbeCallback>,
        allow_pbuffer_only_configs: bool,
    ) -> Result<EglConfigInfo, OpenGlException> {
        let (mut major, mut minor) = (0, 0);
        if !egl.initialize(display, &mut major, &mut minor) {
            return Err(OpenGlException::get_formatted_exception_for_egl("eglInitialize", egl));
        }

        let default_versions = [GlVersion::new(GlProfileType::OpenGLES, 3, 0), GlVersion::new(GlProfileType::OpenGLES, 2, 0)];
        let versions = versions.unwrap_or(&default_versions);

        let cfgs = versions.iter().map(|x| {
            if x.type_() == GlProfileType::OpenGLES {
                let type_bit = match x.major() {
                    2 => EGL_OPENGL_ES2_BIT,
                    1 => EGL_OPENGL_ES_BIT,
                    _ => EGL_OPENGL_ES3_BIT,
                };

                VersionConfig {
                    attributes: vec![
                        EGL_CONTEXT_MAJOR_VERSION,
                        x.major(),
                        EGL_CONTEXT_MINOR_VERSION,
                        x.minor(),
                        EGL_NONE,
                    ],
                    api: EGL_OPENGL_ES_API,
                    renderable_type_bit: type_bit,
                    version: *x,
                }
            } else {
                let attrs = if x.major() > 3 || (x.major() == 3 && x.minor() >= 2) {
                    vec![
                        EGL_CONTEXT_MAJOR_VERSION,
                        x.major(),
                        EGL_CONTEXT_MINOR_VERSION,
                        x.minor(),
                        EGL_CONTEXT_OPENGL_PROFILE_MASK,
                        if x.is_compatibility_profile() {
                            EGL_CONTEXT_OPENGL_COMPATIBILITY_PROFILE_BIT
                        } else {
                            EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT
                        },
                        EGL_NONE,
                    ]
                } else {
                    vec![EGL_CONTEXT_MAJOR_VERSION, x.major(), EGL_CONTEXT_MINOR_VERSION, x.minor(), EGL_NONE]
                };

                VersionConfig { attributes: attrs, api: EGL_OPENGL_API, renderable_type_bit: EGL_OPENGL_BIT, version: *x }
            }
        });

        let surface_types: &[i32] = if allow_pbuffer_only_configs {
            &[EGL_PBUFFER_BIT | EGL_WINDOW_BIT, EGL_WINDOW_BIT, EGL_PBUFFER_BIT]
        } else {
            &[EGL_PBUFFER_BIT | EGL_WINDOW_BIT, EGL_WINDOW_BIT]
        };

        for cfg in cfgs {
            if !egl.bind_api(cfg.api) {
                continue;
            }
            for &surface_type in surface_types {
                for stencil_size in [8, 1, 0] {
                    for depth_size in [8, 1, 0] {
                        let attribs = [
                            EGL_SURFACE_TYPE,
                            surface_type,
                            EGL_RENDERABLE_TYPE,
                            cfg.renderable_type_bit,
                            EGL_RED_SIZE,
                            8,
                            EGL_GREEN_SIZE,
                            8,
                            EGL_BLUE_SIZE,
                            8,
                            EGL_ALPHA_SIZE,
                            8,
                            EGL_STENCIL_SIZE,
                            stencil_size,
                            EGL_DEPTH_SIZE,
                            depth_size,
                            EGL_NONE,
                        ];
                        let Some(config) = Self::choose_config_with_probe(egl, display, &attribs, probe_config) else {
                            continue;
                        };

                        let mut sample_count = 0;
                        egl.get_config_attrib(display, config, EGL_SAMPLES, &mut sample_count);
                        let mut returned_stencil_size = 0;
                        egl.get_config_attrib(display, config, EGL_STENCIL_SIZE, &mut returned_stencil_size);
                        return Ok(EglConfigInfo::new(
                            config,
                            cfg.version,
                            surface_type,
                            cfg.attributes,
                            sample_count,
                            returned_stencil_size,
                            cfg.api,
                        ));
                    }
                }
            }
        }

        Err(OpenGlException::new("No suitable EGL config was found"))
    }
}

pub(crate) struct EglConfigInfo {
    config: isize,
    version: GlVersion,
    surface_type: i32,
    attributes: Vec<i32>,
    sample_count: i32,
    stencil_size: i32,
    api: i32,
}

impl EglConfigInfo {
    pub fn new(
        config: isize,
        version: GlVersion,
        surface_type: i32,
        attributes: Vec<i32>,
        sample_count: i32,
        stencil_size: i32,
        api: i32,
    ) -> Self {
        Self { config, version, surface_type, attributes, sample_count, stencil_size, api }
    }

    pub fn config(&self) -> isize {
        self.config
    }

    pub fn version(&self) -> GlVersion {
        self.version
    }

    pub fn surface_type(&self) -> i32 {
        self.surface_type
    }

    pub fn attributes(&self) -> &[i32] {
        &self.attributes
    }

    pub fn sample_count(&self) -> i32 {
        self.sample_count
    }

    pub fn stencil_size(&self) -> i32 {
        self.stencil_size
    }

    pub fn api(&self) -> i32 {
        self.api
    }
}
