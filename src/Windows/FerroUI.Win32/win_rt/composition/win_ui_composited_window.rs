//! The composition tree of a window in the Windows.UI.Composition mode: a
//! target for the window with a container, in it the visuals of the blur
//! effects (mica for the light and for the dark theme, acrylic) and above
//! them the visual whose brush is the surface the window is rendered to.

use crate::i_blur_host::BlurEffect;
use ferroui_base::platform::PlatformThemeVariant;
use ferroui_base::rendering::composition::CompositionTransparencyLevel;

/// The blur effect of a transparency level for the theme of the frame.
pub(crate) fn blur_effect_of(transparency_level: CompositionTransparencyLevel, theme_variant: PlatformThemeVariant) -> BlurEffect {
    match transparency_level {
        CompositionTransparencyLevel::AcrylicBlur => BlurEffect::Acrylic,
        CompositionTransparencyLevel::Mica if theme_variant == PlatformThemeVariant::Dark => BlurEffect::MicaDark,
        CompositionTransparencyLevel::Mica => BlurEffect::MicaLight,
        _ => BlurEffect::None,
    }
}

/// Which of the three blur visuals of a window are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlurVisibility {
    pub blur: bool,
    pub mica_light: bool,
    pub mica_dark: bool,
}

/// The visuals an effect shows: its own, or the acrylic one in place of a
/// mica visual the system has no brush for (before Windows 11).
pub(crate) fn blur_visibility(blur_effect: BlurEffect, has_mica_light: bool, has_mica_dark: bool) -> BlurVisibility {
    BlurVisibility {
        blur: blur_effect == BlurEffect::Acrylic
            || (blur_effect == BlurEffect::MicaLight && !has_mica_light)
            || (blur_effect == BlurEffect::MicaDark && !has_mica_dark),
        mica_light: blur_effect == BlurEffect::MicaLight,
        mica_dark: blur_effect == BlurEffect::MicaDark,
    }
}

#[cfg(windows)]
pub(crate) use imp::{Transaction, WinUiCompositedWindow};

#[cfg(windows)]
mod imp {
    use super::super::win_ui_composition_shared::WinUiCompositionShared;
    use super::super::win_ui_composition_utils::WinUiCompositionUtils;
    use super::super::Required;
    use super::{blur_effect_of, blur_visibility};
    use crate::i_blur_host::BlurEffect;
    use crate::sync_root::SharedCom;
    use crate::win_rt::numerics::Vector2;
    use crate::win_rt::{
        ICompositionBrush, ICompositionRoundedRectangleGeometry, ICompositionSurface, ICompositionSurfaceBrush,
        ICompositionTarget, IVisual, IVisual2,
    };
    use ferroui_base::platform::PlatformThemeVariant;
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use ferroui_base::PixelSize;
    use ferroui_microcom::HResult;
    use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
    use std::sync::{Arc, Mutex, PoisonError};

    struct Objects {
        composition_rounded_rectangle_geometry: Option<SharedCom<ICompositionRoundedRectangleGeometry>>,
        mica_light: Option<SharedCom<IVisual>>,
        mica_dark: Option<SharedCom<IVisual>>,
        blur: SharedCom<IVisual>,
        visual: SharedCom<IVisual>,
        surface_brush: SharedCom<ICompositionSurfaceBrush>,
        /// The target of the window: held for as long as the tree is shown.
        _target: SharedCom<ICompositionTarget>,
    }

    #[derive(Default)]
    struct State {
        size: PixelSize,
        applied_blur_effect: Option<BlurEffect>,
    }

    pub(crate) struct WinUiCompositedWindow {
        window_info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
        shared: Arc<WinUiCompositionShared>,
        /// `None` once disposed.
        objects: Mutex<Option<Objects>>,
        state: Mutex<State>,
    }

    impl WinUiCompositedWindow {
        pub fn new(
            info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
            shared: Arc<WinUiCompositionShared>,
            backdrop_corner_radius: Option<f32>,
        ) -> Result<Arc<WinUiCompositedWindow>, HResult> {
            let compositor = shared.compositor();
            let desktop_target = shared.desktop_interop().create_desktop_window_target(info.handle(), 0).required()?;
            let target = desktop_target.cast::<ICompositionTarget>()?;

            let container = compositor.create_container_visual().required()?;
            let container_visual = container.cast::<IVisual>()?;
            let container_visual2 = container.cast::<IVisual2>()?;
            container_visual2.set_relative_size_adjustment(Vector2 { x: 1.0, y: 1.0 })?;
            let container_children = container.get_children().required()?;

            target.set_root(Some(&container_visual))?;

            let blur = WinUiCompositionUtils::create_blur_visual(compositor, shared.blur_brush())?;
            let mut mica_light = None;
            if let Some(mica_brush_light) = shared.mica_brush_light() {
                let visual = WinUiCompositionUtils::create_blur_visual(compositor, mica_brush_light)?;
                container_children.insert_at_top(Some(&visual))?;
                mica_light = Some(visual);
            }

            let mut mica_dark = None;
            if let Some(mica_brush_dark) = shared.mica_brush_dark() {
                let visual = WinUiCompositionUtils::create_blur_visual(compositor, mica_brush_dark)?;
                container_children.insert_at_top(Some(&visual))?;
                mica_dark = Some(visual);
            }

            let composition_rounded_rectangle_geometry = WinUiCompositionUtils::clip_visual(
                compositor,
                backdrop_corner_radius,
                &[Some(&*blur), mica_light.as_deref(), mica_dark.as_deref()],
            )?;

            container_children.insert_at_top(Some(&blur))?;
            let sprite_visual = compositor.create_sprite_visual().required()?;
            let visual = sprite_visual.cast::<IVisual>()?;
            container_children.insert_at_top(Some(&visual))?;

            let surface_brush = compositor.create_surface_brush().required()?;
            let composition_brush = surface_brush.cast::<ICompositionBrush>()?;
            sprite_visual.set_set_brush(Some(&composition_brush))?;
            target.set_root(Some(&container_visual))?;

            // SAFETY (all of them): objects of the Windows Runtime
            // composition, which are agile; they are used under the lock of
            // the shared state.
            let objects = unsafe {
                Objects {
                    composition_rounded_rectangle_geometry: composition_rounded_rectangle_geometry
                        .map(|geometry| SharedCom::new(geometry)),
                    mica_light: mica_light.map(|visual| SharedCom::new(visual)),
                    mica_dark: mica_dark.map(|visual| SharedCom::new(visual)),
                    blur: SharedCom::new(blur),
                    visual: SharedCom::new(visual),
                    surface_brush: SharedCom::new(surface_brush),
                    _target: SharedCom::new(target),
                }
            };
            Ok(Arc::new(WinUiCompositedWindow {
                window_info: info,
                shared,
                objects: Mutex::new(Some(objects)),
                state: Mutex::new(State::default()),
            }))
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

        /// Calls `f` with the objects of the tree.
        ///
        /// # Panics
        /// Panics on a window that was disposed (the reference then calls a
        /// released object).
        fn with_objects<R>(&self, f: impl FnOnce(&Objects) -> R) -> R {
            let objects = self.objects.lock().unwrap_or_else(PoisonError::into_inner);
            f(objects.as_ref().expect("the composited window was disposed"))
        }

        pub fn set_surface(&self, surface: &ICompositionSurface) -> Result<(), HResult> {
            self.with_objects(|objects| objects.surface_brush.set_surface(Some(surface)))
        }

        /// Called from the render thread with the transaction (the lock of
        /// the shared state) held, so effect changes are applied within the
        /// same composition batch as the frame they belong to.
        pub fn apply_effects(
            &self,
            transparency_level: CompositionTransparencyLevel,
            theme_variant: PlatformThemeVariant,
        ) -> Result<(), HResult> {
            debug_assert!(self.shared.sync_root().is_entered());

            let blur_effect = blur_effect_of(transparency_level, theme_variant);
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if state.applied_blur_effect == Some(blur_effect) {
                return Ok(());
            }

            self.with_objects(|objects| {
                let visibility = blur_visibility(blur_effect, objects.mica_light.is_some(), objects.mica_dark.is_some());
                objects.blur.set_is_visible(i32::from(visibility.blur))?;
                if let Some(mica_light) = &objects.mica_light {
                    mica_light.set_is_visible(i32::from(visibility.mica_light))?;
                }
                if let Some(mica_dark) = &objects.mica_dark {
                    mica_dark.set_is_visible(i32::from(visibility.mica_dark))?;
                }
                Ok::<(), HResult>(())
            })?;
            state.applied_blur_effect = Some(blur_effect);
            Ok(())
        }

        /// Enters the lock of the shared state, which the transaction
        /// leaves when it is dropped.
        pub fn begin_transaction(self: &Arc<Self>) -> Transaction {
            self.shared.sync_root().enter();
            Transaction { shared: self.shared.clone() }
        }

        pub fn resize_if_needed(&self, size: PixelSize) -> Result<(), HResult> {
            debug_assert!(self.shared.sync_root().is_entered());

            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if state.size != size {
                self.with_objects(|objects| {
                    let size = Vector2 { x: size.width as f32, y: size.height as f32 };
                    objects.visual.set_size(size)?;
                    if let Some(geometry) = &objects.composition_rounded_rectangle_geometry {
                        geometry.set_size(size)?;
                    }
                    Ok::<(), HResult>(())
                })?;
                state.size = size;
            }
            Ok(())
        }
    }

    impl Drop for WinUiCompositedWindow {
        fn drop(&mut self) {
            // A window that is still alive when the process ends is
            // destroyed with the values of the main thread, after the
            // thread that renders was ended, possibly inside the lock of a
            // frame: nothing is released then, as in the reference, and
            // nothing is waited for (docs/porting/win32-platform.md,
            // section 11.3).
            if crate::interop::unmanaged_methods::process_is_shutting_down() {
                // The lock of the objects is not waited for either: the
                // thread that renders may have been ended inside it.
                if let Ok(mut objects) = self.objects.try_lock() {
                    if let Some(objects) = objects.take() {
                        std::mem::forget(objects);
                    }
                }
                return;
            }
            self.dispose();
        }
    }

    /// The changes of one frame: the lock is left when it is dropped. The
    /// compositor commits by itself (the connection asks it for the next
    /// commit).
    pub(crate) struct Transaction {
        shared: Arc<WinUiCompositionShared>,
    }

    impl Drop for Transaction {
        fn drop(&mut self) {
            self.shared.sync_root().exit();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the window.
    use super::*;

    #[test]
    fn a_transparency_level_has_the_blur_effect_of_the_theme() {
        use CompositionTransparencyLevel as Level;
        use PlatformThemeVariant::{Dark, Light};
        assert_eq!(BlurEffect::None, blur_effect_of(Level::None, Light));
        assert_eq!(BlurEffect::None, blur_effect_of(Level::Transparent, Dark));
        // The blur level has no effect of its own in this mode.
        assert_eq!(BlurEffect::None, blur_effect_of(Level::Blur, Light));
        assert_eq!(BlurEffect::Acrylic, blur_effect_of(Level::AcrylicBlur, Light));
        assert_eq!(BlurEffect::Acrylic, blur_effect_of(Level::AcrylicBlur, Dark));
        assert_eq!(BlurEffect::MicaLight, blur_effect_of(Level::Mica, Light));
        assert_eq!(BlurEffect::MicaDark, blur_effect_of(Level::Mica, Dark));
    }

    #[test]
    fn an_effect_shows_its_visual_and_acrylic_stands_in_for_a_missing_mica_brush() {
        let none = BlurVisibility { blur: false, mica_light: false, mica_dark: false };
        assert_eq!(none, blur_visibility(BlurEffect::None, true, true));
        assert_eq!(none, blur_visibility(BlurEffect::GaussianBlur, true, true));
        assert_eq!(BlurVisibility { blur: true, ..none }, blur_visibility(BlurEffect::Acrylic, true, true));
        assert_eq!(BlurVisibility { mica_light: true, ..none }, blur_visibility(BlurEffect::MicaLight, true, true));
        assert_eq!(BlurVisibility { mica_dark: true, ..none }, blur_visibility(BlurEffect::MicaDark, true, true));
        // Before Windows 11 there is no mica brush: the acrylic visual is
        // shown (the mica flags are of visuals that do not exist then).
        assert_eq!(
            BlurVisibility { blur: true, mica_light: true, mica_dark: false },
            blur_visibility(BlurEffect::MicaLight, false, false)
        );
        assert_eq!(
            BlurVisibility { blur: true, mica_light: false, mica_dark: true },
            blur_visibility(BlurEffect::MicaDark, false, false)
        );
    }
}
