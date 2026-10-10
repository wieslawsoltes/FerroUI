//! Creates the platform graphics of ANGLE for the first of the allowed
//! Direct3D APIs that works.

use super::{D3D11AngleWin32PlatformGraphics, D3D9AngleWin32PlatformGraphics, Win32AngleEglInterface};
use crate::angle_options::{AngleOptions, PlatformApi};
use crate::win32_platform_options::GraphicsAdapterSelectionCallback;
use ferroui_base::logging::{LogEventLevel, Logger};
use std::rc::Rc;

/// The platform graphics the factory created: the reference returns them as
/// the platform graphics contract and its callers test the type; here the
/// two types are the two cases.
pub enum AnglePlatformGraphics {
    D3D11(D3D11AngleWin32PlatformGraphics),
    D3D9(D3D9AngleWin32PlatformGraphics),
}

/// The allowed APIs without repetitions, in order (`Distinct`).
pub(crate) fn distinct_platform_apis(options: Option<&AngleOptions>) -> Vec<PlatformApi> {
    let allowed_platform_apis =
        options.and_then(|options| options.allowed_platform_apis.clone()).unwrap_or_else(|| vec![PlatformApi::DirectX11]);
    let mut distinct = Vec::new();
    for api in allowed_platform_apis {
        if !distinct.contains(&api) {
            distinct.push(api);
        }
    }
    distinct
}

pub struct AngleWin32PlatformGraphicsFactory;

impl AngleWin32PlatformGraphicsFactory {
    /// The platform graphics of ANGLE, or `None` when ANGLE cannot be
    /// loaded or none of the allowed APIs works; the reasons are logged.
    ///
    /// `selection_callback` is the adapter selection of the platform
    /// options, which the reference reads from its services where the
    /// device is created.
    pub fn try_create(
        options: Option<&AngleOptions>,
        selection_callback: Option<GraphicsAdapterSelectionCallback>,
    ) -> Option<AnglePlatformGraphics> {
        let egl = match Win32AngleEglInterface::new() {
            Ok(egl) => Rc::new(egl),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to load ANGLE: {0}", &[&e]);
                }
                return None;
            }
        };

        let gl_versions = options.map(AngleOptions::open_gl_es_profiles);

        for api in distinct_platform_apis(options) {
            match api {
                PlatformApi::DirectX11 => {
                    if let Some(platform_graphics) = Self::try_create_d3d11(&egl, gl_versions.clone(), selection_callback.clone()) {
                        return Some(AnglePlatformGraphics::D3D11(platform_graphics));
                    }
                }
                PlatformApi::DirectX9 => {
                    if let Some(platform_graphics) = D3D9AngleWin32PlatformGraphics::try_create(&egl) {
                        return Some(AnglePlatformGraphics::D3D9(platform_graphics));
                    }
                }
            }
            // The reference falls into its default branch for an API that
            // did not work, and logs this.
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                logger.log_with_values(None, "Unknown requested PlatformApi {0}", &[&format!("{api:?}")]);
            }
        }

        None
    }

    #[cfg(windows)]
    fn try_create_d3d11(
        egl: &Rc<Win32AngleEglInterface>,
        gl_versions: Option<Vec<ferroui_opengl::GlVersion>>,
        selection_callback: Option<GraphicsAdapterSelectionCallback>,
    ) -> Option<D3D11AngleWin32PlatformGraphics> {
        D3D11AngleWin32PlatformGraphics::try_create(egl, gl_versions, selection_callback)
    }

    #[cfg(not(windows))]
    fn try_create_d3d11(
        _egl: &Rc<Win32AngleEglInterface>,
        _gl_versions: Option<Vec<ferroui_opengl::GlVersion>>,
        _selection_callback: Option<GraphicsAdapterSelectionCallback>,
    ) -> Option<D3D11AngleWin32PlatformGraphics> {
        None
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn direct3d_11_alone_is_allowed_without_options() {
        assert_eq!(vec![PlatformApi::DirectX11], distinct_platform_apis(None));
        assert_eq!(vec![PlatformApi::DirectX11], distinct_platform_apis(Some(&AngleOptions::default())));
    }

    #[test]
    fn the_allowed_apis_are_tried_once_each_in_order() {
        let options = AngleOptions {
            allowed_platform_apis: Some(vec![PlatformApi::DirectX9, PlatformApi::DirectX11, PlatformApi::DirectX9]),
            ..Default::default()
        };

        assert_eq!(vec![PlatformApi::DirectX9, PlatformApi::DirectX11], distinct_platform_apis(Some(&options)));
    }

    #[cfg(not(all(windows, feature = "angle")))]
    #[test]
    fn a_build_without_angle_has_no_platform_graphics() {
        assert!(AngleWin32PlatformGraphicsFactory::try_create(Some(&AngleOptions::default()), None).is_none());
    }
}
