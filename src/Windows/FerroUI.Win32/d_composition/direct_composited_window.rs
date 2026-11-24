//! The composition tree of a window in the DirectComposition mode: a
//! target for the window with one visual, whose content is the surface the
//! window is rendered to.

use super::{DirectCompositionShared, IDCompositionDevice2, IDCompositionSurface, IDCompositionTarget, IDCompositionVisual};
use crate::sync_root::SharedCom;
use ferroui_microcom::HResult;
use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
use std::sync::{Arc, Mutex, PoisonError};

struct Objects {
    container: SharedCom<IDCompositionVisual>,
    /// The target of the window: held for as long as the tree is shown.
    _target: SharedCom<IDCompositionTarget>,
    device: SharedCom<IDCompositionDevice2>,
}

pub(crate) struct DirectCompositedWindow {
    shared: Arc<DirectCompositionShared>,
    window_info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    /// `None` once disposed.
    objects: Mutex<Option<Objects>>,
}

impl DirectCompositedWindow {
    pub fn new(
        info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
        shared: Arc<DirectCompositionShared>,
    ) -> Result<Arc<DirectCompositedWindow>, HResult> {
        let device = shared.device().cast::<IDCompositionDevice2>()?;
        let desktop_target = shared.device().create_target_for_hwnd(info.handle(), 0)?.ok_or(HResult::POINTER)?;
        let target = desktop_target.cast::<IDCompositionTarget>()?;
        let container = shared.device().create_visual()?.ok_or(HResult::POINTER)?;
        target.set_root(Some(&*container))?;

        // SAFETY (the three): objects of DirectComposition, which are
        // free-threaded; they are used under the lock of the shared state.
        let objects = unsafe {
            Objects { container: SharedCom::new(container), _target: SharedCom::new(target), device: SharedCom::new(device) }
        };
        Ok(Arc::new(DirectCompositedWindow { shared, window_info: info, objects: Mutex::new(Some(objects)) }))
    }

    pub fn window_info(&self) -> &Arc<dyn IEglWindowGlPlatformSurfaceInfo> {
        &self.window_info
    }

    /// Releases the tree, inside the lock of the shared state.
    pub fn dispose(&self) {
        let _lock = self.shared.sync_root().lock();
        let objects = self.objects.lock().unwrap_or_else(PoisonError::into_inner).take();
        drop(objects);
    }

    /// Makes `surface` the content of the visual of the window.
    ///
    /// # Panics
    /// Panics on a window that was disposed (the reference then calls a
    /// released object).
    pub fn set_surface(&self, surface: &IDCompositionSurface) -> Result<(), HResult> {
        let objects = self.objects.lock().unwrap_or_else(PoisonError::into_inner);
        let objects = objects.as_ref().expect("the composited window was disposed");
        let content: &ferroui_microcom::IUnknown = surface;
        objects.container.set_content(Some(content))
    }

    /// Enters the lock of the shared state; the transaction commits the
    /// batch of the device and leaves the lock when it is dropped.
    pub fn begin_transaction(self: &Arc<Self>) -> Transaction {
        self.shared.sync_root().enter();
        Transaction { window: self.clone() }
    }

    /// Commits the batch of the device and leaves the lock of the shared
    /// state: the end of a transaction.
    fn end_transaction(&self) {
        {
            let objects = self.objects.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(objects) = objects.as_ref() {
                // The reference ignores nothing here: a failing commit
                // throws out of the disposal. A failure cannot leave a
                // destructor, and the next frame commits again.
                let _ = objects.device.commit();
            }
        }
        self.shared.sync_root().exit();
    }
}

/// The window shows a swap chain for composition as the content of its
/// visual (`Win32PlatformOptions::use_composition_swap_chain`).
#[cfg(windows)]
impl crate::direct_x::ISwapchainVisualHost for DirectCompositedWindow {
    fn window_size(&self) -> ferroui_base::PixelSize {
        self.window_info.size()
    }

    fn window_scaling(&self) -> f64 {
        self.window_info.scaling()
    }

    fn begin_transaction(&self) {
        self.shared.sync_root().enter();
    }

    fn end_transaction(&self) {
        DirectCompositedWindow::end_transaction(self);
    }

    fn set_swapchain_content(
        &self,
        swap_chain: &ferroui_microcom::ComPtr<crate::direct_x::IDXGISwapChain1>,
    ) -> Result<(), HResult> {
        let objects = self.objects.lock().unwrap_or_else(PoisonError::into_inner);
        let objects = objects.as_ref().ok_or(HResult::OBJECTDISPOSED)?;
        let content: &ferroui_microcom::IUnknown = swap_chain;
        objects.container.set_content(Some(content))
    }

    /// A visual of DirectComposition shows its content at its own size.
    fn resize_if_needed(&self, _size: ferroui_base::PixelSize) -> Result<(), HResult> {
        Ok(())
    }

    /// The mode has no composition effects.
    fn apply_effects(
        &self,
        _transparency_level: ferroui_base::rendering::composition::CompositionTransparencyLevel,
        _theme_variant: ferroui_base::platform::PlatformThemeVariant,
    ) -> Result<(), HResult> {
        Ok(())
    }
}

impl Drop for DirectCompositedWindow {
    fn drop(&mut self) {
        // A window that is still alive when the process ends is destroyed
        // with the values of the main thread, after the thread that
        // renders was ended, possibly inside the lock of a frame: nothing
        // is released then, as in the reference, and nothing is waited for.
        #[cfg(windows)]
        if crate::interop::unmanaged_methods::process_is_shutting_down() {
            if let Some(objects) = self.objects.lock().unwrap_or_else(PoisonError::into_inner).take() {
                std::mem::forget(objects);
            }
            return;
        }
        self.dispose();
    }
}

/// The changes of one frame: committed, and the lock left, when dropped.
pub(crate) struct Transaction {
    window: Arc<DirectCompositedWindow>,
}

impl Drop for Transaction {
    fn drop(&mut self) {
        self.window.end_transaction();
    }
}
