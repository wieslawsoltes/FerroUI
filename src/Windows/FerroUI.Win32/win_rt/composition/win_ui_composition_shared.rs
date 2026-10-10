//! What the windows of the Windows.UI.Composition mode share: the
//! compositor, the backdrop brushes and the lock of a frame.

use crate::platform_constants::Version;

/// The versions of Windows the mode and its effects need.
pub(crate) struct WinUiCompositionVersions;

impl WinUiCompositionVersions {
    pub const MIN_WIN_COMPOSITION_VERSION: Version = Version { major: 10, minor: 0, build: 17134 };
    pub const MIN_ACRYLIC_VERSION: Version = Version { major: 10, minor: 0, build: 15063 };
    pub const MIN_HOST_BACKDROP_VERSION: Version = Version { major: 10, minor: 0, build: 22000 };
}

#[cfg(windows)]
pub(crate) use imp::WinUiCompositionShared;

#[cfg(windows)]
mod imp {
    use super::super::win_ui_composition_utils::WinUiCompositionUtils;
    use super::WinUiCompositionVersions;
    use crate::platform_constants::Version;
    use crate::sync_root::{SharedCom, SyncRoot};
    use crate::win_rt::{ICompositionBrush, ICompositor, ICompositor5, ICompositorDesktopInterop};
    use ferroui_microcom::{ComPtr, HResult};
    use std::sync::Arc;

    /// The shared state. The objects of the compositor are agile: the
    /// reference calls them from the thread of the connection (which
    /// renders) and releases the tree of a window on the UI thread, with
    /// the calls serialized by the lock; so they are held as pointers
    /// that are shared between threads. Dropping the value releases them
    /// (`Dispose` of the reference).
    pub(crate) struct WinUiCompositionShared {
        compositor: SharedCom<ICompositor>,
        compositor5: SharedCom<ICompositor5>,
        desktop_interop: SharedCom<ICompositorDesktopInterop>,
        blur_brush: SharedCom<ICompositionBrush>,
        mica_brush_light: Option<SharedCom<ICompositionBrush>>,
        mica_brush_dark: Option<SharedCom<ICompositionBrush>>,
        sync_root: Arc<SyncRoot>,
    }

    #[allow(dead_code)] // The versions are asked through `WinUiCompositionVersions`, which every host has.
    impl WinUiCompositionShared {
        pub const MIN_WIN_COMPOSITION_VERSION: Version = WinUiCompositionVersions::MIN_WIN_COMPOSITION_VERSION;
        pub const MIN_ACRYLIC_VERSION: Version = WinUiCompositionVersions::MIN_ACRYLIC_VERSION;
        pub const MIN_HOST_BACKDROP_VERSION: Version = WinUiCompositionVersions::MIN_HOST_BACKDROP_VERSION;

        /// Holds a reference of its own to the compositor and creates the
        /// brushes of the blur effects.
        pub fn new(compositor: &ComPtr<ICompositor>) -> Result<Arc<WinUiCompositionShared>, HResult> {
            let compositor5 = compositor.cast::<ICompositor5>()?;
            let blur_brush = WinUiCompositionUtils::create_acrylic_blur_backdrop_brush(compositor)?;
            let mica_brush_light = WinUiCompositionUtils::create_mica_backdrop_brush(compositor, 242.0, 0.6)?;
            let mica_brush_dark = WinUiCompositionUtils::create_mica_backdrop_brush(compositor, 32.0, 0.8)?;
            let desktop_interop = compositor.cast::<ICompositorDesktopInterop>()?;
            // SAFETY (all of them): objects of the Windows Runtime
            // composition, which are agile; the changes of a frame are
            // serialized by the lock beside them.
            unsafe {
                Ok(Arc::new(WinUiCompositionShared {
                    compositor: SharedCom::new(compositor.clone()),
                    compositor5: SharedCom::new(compositor5),
                    desktop_interop: SharedCom::new(desktop_interop),
                    blur_brush: SharedCom::new(blur_brush),
                    mica_brush_light: mica_brush_light.map(|brush| SharedCom::new(brush)),
                    mica_brush_dark: mica_brush_dark.map(|brush| SharedCom::new(brush)),
                    sync_root: SyncRoot::new(),
                }))
            }
        }

        pub fn compositor(&self) -> &ComPtr<ICompositor> {
            &self.compositor
        }

        pub fn compositor5(&self) -> &ComPtr<ICompositor5> {
            &self.compositor5
        }

        pub fn desktop_interop(&self) -> &ComPtr<ICompositorDesktopInterop> {
            &self.desktop_interop
        }

        pub fn blur_brush(&self) -> &ComPtr<ICompositionBrush> {
            &self.blur_brush
        }

        pub fn mica_brush_light(&self) -> Option<&ComPtr<ICompositionBrush>> {
            self.mica_brush_light.as_deref()
        }

        pub fn mica_brush_dark(&self) -> Option<&ComPtr<ICompositionBrush>> {
            self.mica_brush_dark.as_deref()
        }

        /// The lock around the changes of a frame.
        pub fn sync_root(&self) -> &Arc<SyncRoot> {
            &self.sync_root
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use crate::platform_constants::PlatformConstants;

    #[test]
    fn the_versions_of_the_mode_and_its_effects() {
        // Windows 10 1803 for the mode, 1703 for acrylic, Windows 11 for
        // the host backdrop and mica.
        assert!(WinUiCompositionVersions::MIN_WIN_COMPOSITION_VERSION > PlatformConstants::WINDOWS10);
        assert!(WinUiCompositionVersions::MIN_ACRYLIC_VERSION < WinUiCompositionVersions::MIN_WIN_COMPOSITION_VERSION);
        assert!(WinUiCompositionVersions::MIN_HOST_BACKDROP_VERSION > WinUiCompositionVersions::MIN_WIN_COMPOSITION_VERSION);
        assert_eq!("10.0.17134", WinUiCompositionVersions::MIN_WIN_COMPOSITION_VERSION.to_string());
    }
}
