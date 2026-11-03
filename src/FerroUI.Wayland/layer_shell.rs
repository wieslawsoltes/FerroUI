//! Desktop components on compositors advertising zwlr_layer_shell_v1.
//! The role is chosen before the first wl_surface commit and reused on remap.
use crate::{wayland_top_level_factory::WaylandTopLevelFactory, window_impl::WindowImpl};
use bitflags::bitflags;
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::{Window, WindowDecorations};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1 as protocol, zwlr_layer_surface_v1 as surface,
};

pub use protocol::Layer;
pub use surface::KeyboardInteractivity;

bitflags! {
    /// Output edges in surface-local logical coordinates.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct Anchors: u32 {
        const TOP = 1;
        const BOTTOM = 2;
        const LEFT = 4;
        const RIGHT = 8;
    }
}

/// Layer-shell's double-buffered placement state. A zero dimension lets the compositor
/// size that axis and requires anchors at both ends. Margins are top/right/bottom/left.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayerConfig {
    pub layer: Layer,
    pub anchors: Anchors,
    pub width: u32,
    pub height: u32,
    pub exclusive_zone: i32,
    pub keyboard: KeyboardInteractivity,
    pub margins: [i32; 4],
}

impl LayerConfig {
    /// A full-output wallpaper, including beneath exclusive panels.
    pub fn background() -> Self {
        Self {
            layer: Layer::Background,
            anchors: Anchors::all(),
            width: 0,
            height: 0,
            exclusive_zone: -1,
            keyboard: KeyboardInteractivity::None,
            margins: [0; 4],
        }
    }
    /// A full-output, keyboard-interactive overlay for a launcher or overview.
    pub fn overlay() -> Self {
        Self {
            layer: Layer::Overlay,
            keyboard: KeyboardInteractivity::Exclusive,
            ..Self::background()
        }
    }
    /// A full-width edge panel reserving its own height, in logical pixels.
    pub fn panel(top: bool, height: u32) -> Self {
        let edge = if top { Anchors::TOP } else { Anchors::BOTTOM };
        Self {
            layer: Layer::Top,
            anchors: Anchors::LEFT | Anchors::RIGHT | edge,
            height,
            exclusive_zone: height.min(i32::MAX as u32) as i32,
            ..Self::background()
        }
    }
    /// A nonactivating top-right notification respecting the compositor's usable area.
    pub fn notification(width: u32, height: u32, top_margin: i32, right_margin: i32) -> Self {
        Self {
            layer: Layer::Overlay,
            anchors: Anchors::TOP | Anchors::RIGHT,
            width,
            height,
            exclusive_zone: 0,
            margins: [top_margin, right_margin, 0, 0],
            ..Self::background()
        }
    }

    pub(crate) fn configured_size(
        self,
        width: u32,
        height: u32,
        output: ferroui_base::PixelSize,
    ) -> ferroui_base::PixelSize {
        let axis = |configured: u32, requested: u32, available: i32| {
            if configured > 0 {
                configured.min(i32::MAX as u32) as i32
            } else if requested > 0 {
                requested.min(i32::MAX as u32) as i32
            } else {
                available.max(1)
            }
        };
        ferroui_base::PixelSize::new(
            axis(width, self.width, output.width),
            axis(height, self.height, output.height),
        )
    }

    pub(crate) fn validate(self) -> Result<(), String> {
        if self.width > i32::MAX as u32 || self.height > i32::MAX as u32 {
            return Err("Layer dimensions exceed the logical coordinate range".into());
        }
        if (self.width == 0 && !self.anchors.contains(Anchors::LEFT | Anchors::RIGHT))
            || (self.height == 0 && !self.anchors.contains(Anchors::TOP | Anchors::BOTTOM))
        {
            return Err("A compositor-sized layer axis needs anchors at both ends".into());
        }
        if self.exclusive_zone < -1 {
            return Err("Layer exclusive zones must be -1 or nonnegative".into());
        }
        if self.exclusive_zone > 0 {
            let a = self.anchors;
            let horizontal = a.contains(Anchors::LEFT) != a.contains(Anchors::RIGHT);
            let vertical = a.contains(Anchors::TOP) != a.contains(Anchors::BOTTOM);
            if horizontal == vertical {
                return Err("A positive exclusive zone needs one unambiguous edge".into());
            }
        }
        Ok(())
    }

    pub(crate) fn check_version(self, version: u32) -> Result<(), String> {
        self.validate()?;
        if self.keyboard == KeyboardInteractivity::OnDemand && version < 4 {
            return Err("On-demand keyboard focus needs layer-shell v4".into());
        }
        Ok(())
    }

    pub(crate) fn apply(self, surface: &surface::ZwlrLayerSurfaceV1) {
        surface.set_size(self.width, self.height);
        surface.set_anchor(surface::Anchor::from_bits_truncate(self.anchors.bits()));
        surface.set_margin(self.margins[0], self.margins[1], self.margins[2], self.margins[3]);
        surface.set_exclusive_zone(self.exclusive_zone);
        surface.set_keyboard_interactivity(self.keyboard);
    }
}

#[derive(Clone)]
pub(crate) struct LayerRole {
    pub namespace: String,
    pub output: Option<String>,
    pub config: LayerConfig,
}

/// Create and update a FerroUI Window with a native desktop-shell role.
pub struct LayerWindow;

impl LayerWindow {
    /// Complete, configured outputs, including their names, scale and logical bounds.
    /// This query reads the registry on its owner thread and works before the first UI
    /// notification has been dispatched. Window::screens().changed signals later changes.
    pub fn outputs() -> Result<Vec<crate::screens::wayland_output_snapshot::WaylandOutputSnapshot>, String> {
        let factory = FerroLocator::current()
            .get_service::<WaylandTopLevelFactory>()
            .ok_or("The native Wayland windowing backend is not active")?;
        factory
            .client()
            .invoke_oob(|worker| {
                worker.state.globals.as_ref().map(|globals| {
                    globals
                        .outputs
                        .outputs()
                        .iter()
                        .map(|output| output.to_snapshot())
                        .collect()
                })
            })
            .recv()
            .map_err(|_| "The Wayland worker stopped")?
            .ok_or_else(|| "Wayland display is disconnected".into())
    }

    /// Select an output by the name reported by wl_output or xdg-output. With None, the
    /// compositor chooses. An unavailable protocol or named output is a recoverable error.
    pub fn new(namespace: &str, output: Option<&str>, config: LayerConfig) -> Result<Ref<Window>, String> {
        if namespace.is_empty() || namespace.as_bytes().contains(&0) {
            return Err("Layer windows require a nonempty namespace without NUL".into());
        }
        config.validate()?;
        let factory = FerroLocator::current()
            .get_service::<WaylandTopLevelFactory>()
            .ok_or("The native Wayland windowing backend is not active")?;
        let role = LayerRole {
            namespace: namespace.to_owned(),
            output: output.map(str::to_owned),
            config,
        };
        let window = Window::with_impl(factory.create_layer_window(role)?);
        window.set_window_decorations(WindowDecorations::None);
        Ok(window)
    }

    /// Apply all placement and keyboard changes with one surface commit. Namespace and
    /// output identity belong to the role and are fixed for the life of the window.
    pub fn configure(window: &Window, config: LayerConfig) -> Result<(), String> {
        config.validate()?;
        let platform = window.platform_impl().ok_or("The layer window is closed")?;
        let native = platform
            .as_any()
            .downcast_ref::<WindowImpl>()
            .ok_or("This window is not a Wayland window")?;
        native.configure_layer(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_configure_fills_each_unspecified_axis_independently() {
        use ferroui_base::PixelSize;
        let panel = LayerConfig::panel(true, 40);
        assert_eq!(
            panel.configured_size(0, 0, PixelSize::new(800, 600)),
            PixelSize::new(800, 40)
        );
        assert_eq!(
            panel.configured_size(701, 38, PixelSize::new(800, 600)),
            PixelSize::new(701, 38)
        );
    }

    #[test]
    fn anchors_and_version_are_checked_before_sending_requests() {
        assert!(LayerConfig::background().validate().is_ok());
        assert!(LayerConfig::panel(false, 48).validate().is_ok());
        assert!(LayerConfig::panel(true, 0).validate().is_err());
        assert!(LayerConfig::notification(0, 40, 0, 0).validate().is_err());
        assert!(LayerConfig {
            keyboard: KeyboardInteractivity::OnDemand,
            ..LayerConfig::overlay()
        }
        .check_version(3)
        .is_err());
        assert!(LayerConfig {
            exclusive_zone: 15,
            ..LayerConfig::overlay()
        }
        .validate()
        .is_err());
    }
}
