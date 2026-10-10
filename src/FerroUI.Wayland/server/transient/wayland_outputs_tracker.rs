//! The outputs of a connection (the port of `WaylandOutputsTracker.cs`).

use ferroui_base::{PixelPoint, PixelSize};

/// The logical position and size of an output.
///
/// With `xdg_output` its values are taken, except where they are a known lie:
/// https://gitlab.gnome.org/GNOME/mutter/-/issues/2631
/// If integer scale > 1 and xdg_output reports the same size
/// as wl_output.mode (which is in physical pixels), mutter
/// is lying. Fall through to the mode/scale derivation.
///
/// Fallback: derive logical pixels from wl_output.mode divided
/// by wl_output.scale (geometry top-left is already in
/// compositor surface coords, no division needed). Matches
/// GTK/Qt fallback when xdg_output is absent.
pub fn compute_logical(
    xdg: Option<(PixelPoint, PixelSize)>,
    mode_position: PixelPoint,
    mode_size: PixelSize,
    scale: i32,
) -> (PixelPoint, PixelSize) {
    if let Some((xdg_position, xdg_size)) = xdg {
        let mutterpt = scale > 1 && xdg_size == mode_size;
        if !mutterpt {
            return (xdg_position, xdg_size);
        }
    }

    (mode_position, PixelSize::new((mode_size.width / scale.max(1)).max(1), (mode_size.height / scale.max(1)).max(1)))
}

#[cfg(target_os = "linux")]
pub use imp::*;

#[cfg(target_os = "linux")]
mod imp {
    use super::compute_logical;
    use crate::screens::i_wayland_outputs_sink::WaylandOutputsSinkProxy;
    use crate::screens::wayland_output_snapshot::{
        WaylandOutputId, WaylandOutputSnapshot, WaylandOutputSubpixel, WaylandOutputTransform, WaylandOutputsSnapshot,
    };
    use crate::server::wayland_worker::WaylandWorkerState;
    use ferroui_base::{PixelPoint, PixelSize};
    use wayland_client::protocol::wl_output::{self, WlOutput};
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
    use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;
    use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};

    /// Worker-side tracker of `wl_output` globals. Maintains the list of visible
    /// outputs (those that have completed their initial event batch) and
    /// pushes immutable snapshots to the UI-thread sink whenever the list or any
    /// output's state changes.
    pub struct WaylandOutputsTracker {
        // Track outputs that haven't got their first done event
        pending_outputs: Vec<Output>,
        outputs: Vec<Output>,
        sink: Option<WaylandOutputsSinkProxy>,
        xdg_output_manager: Option<ZxdgOutputManagerV1>,
    }

    impl WaylandOutputsTracker {
        pub fn new(sink: Option<WaylandOutputsSinkProxy>) -> Self {
            Self { pending_outputs: Vec::new(), outputs: Vec::new(), sink, xdg_output_manager: None }
        }

        /// The visible outputs.
        pub fn outputs(&self) -> &[Output] {
            &self.outputs
        }

        /// The visible output of a registry name.
        pub fn find(&self, name: u32) -> Option<&Output> {
            self.outputs.iter().find(|output| output.name == name)
        }

        /// The visible output of a protocol object.
        pub fn find_by_proxy(&self, output: &WlOutput) -> Option<&Output> {
            self.outputs.iter().find(|candidate| candidate.output == *output)
        }

        /// Binds zxdg_output_manager_v1 and attaches it to every output seen so far
        /// (including those still pending their initial wl_output.done).
        pub fn attach_xdg_output_manager(&mut self, manager: ZxdgOutputManagerV1, queue_handle: &QueueHandle<WaylandWorkerState>) {
            if self.xdg_output_manager.is_some() {
                return;
            }
            for output in self.pending_outputs.iter_mut().chain(self.outputs.iter_mut()) {
                output.attach_xdg_output(&manager, queue_handle);
            }
            self.xdg_output_manager = Some(manager);
        }

        pub fn add_global(&mut self, registry: &WlRegistry, name: u32, version: u32, queue_handle: &QueueHandle<WaylandWorkerState>) {
            if version < 2 {
                return;
            }

            let mut output = Output::new(registry, name, version, queue_handle);
            if let Some(manager) = &self.xdg_output_manager {
                output.attach_xdg_output(manager, queue_handle);
            }
            self.pending_outputs.push(output);
        }

        /// A global is gone: when it was an output, the output is released, and the sink is
        /// told when the output was a visible one. Returns whether it was a visible output.
        pub fn on_global_removed(&mut self, removed_name: u32) -> bool {
            if let Some(index) = self.pending_outputs.iter().position(|output| output.name == removed_name) {
                self.pending_outputs.remove(index).dispose();
                return false;
            }
            if let Some(index) = self.outputs.iter().position(|output| output.name == removed_name) {
                self.outputs.remove(index).dispose();
                self.push_snapshot();
                return true;
            }
            false
        }

        fn output_mut(&mut self, name: u32) -> Option<&mut Output> {
            self.pending_outputs.iter_mut().chain(self.outputs.iter_mut()).find(|output| output.name == name)
        }

        fn on_output_done(&mut self, name: u32) {
            if let Some(index) = self.pending_outputs.iter().position(|output| output.name == name) {
                let output = self.pending_outputs.remove(index);
                self.outputs.push(output);
            }
            self.push_snapshot();
        }

        fn push_snapshot(&self) {
            let Some(sink) = &self.sink else {
                return;
            };
            let list = self.outputs.iter().map(Output::to_snapshot).collect();
            sink.on_outputs_changed(WaylandOutputsSnapshot::new(list));
        }

        /// Releases every output: the connection is going away.
        pub fn dispose(&mut self) {
            for output in self.pending_outputs.drain(..).chain(self.outputs.drain(..)) {
                output.dispose();
            }
        }
    }

    /// An output of the compositor.
    pub struct Output {
        /// The registry name of the global.
        pub name: u32,
        output: WlOutput,
        xdg_output: Option<ZxdgOutputV1>,

        // wl_output state
        pub mode_position: PixelPoint,
        pub mode_size: PixelSize,
        pub scale: i32,
        pub refresh_milli_hz: i32,
        pub subpixel: WaylandOutputSubpixel,
        pub transform: WaylandOutputTransform,
        pub physical_size_mm: PixelSize,
        pub manufacturer: Option<String>,
        pub model: Option<String>,
        pub output_name: Option<String>,
        pub output_description: Option<String>,

        // xdg_output state
        pub has_xdg_output: bool,
        pub xdg_position: PixelPoint,
        pub xdg_size: PixelSize,
        pub xdg_name: Option<String>,
        pub xdg_description: Option<String>,

        pub id: WaylandOutputId,
    }

    impl Output {
        fn new(registry: &WlRegistry, name: u32, version: u32, queue_handle: &QueueHandle<WaylandWorkerState>) -> Self {
            let output: WlOutput = registry.bind(name, version.min(4), queue_handle, name);
            Self {
                name,
                output,
                xdg_output: None,
                mode_position: PixelPoint::default(),
                mode_size: PixelSize::default(),
                scale: 1,
                refresh_milli_hz: 0,
                subpixel: WaylandOutputSubpixel::Unknown,
                transform: WaylandOutputTransform::Normal,
                physical_size_mm: PixelSize::default(),
                manufacturer: None,
                model: None,
                output_name: None,
                output_description: None,
                has_xdg_output: false,
                xdg_position: PixelPoint::default(),
                xdg_size: PixelSize::default(),
                xdg_name: None,
                xdg_description: None,
                id: WaylandOutputId::new(),
            }
        }

        pub fn wl_output(&self) -> &WlOutput {
            &self.output
        }

        fn attach_xdg_output(&mut self, manager: &ZxdgOutputManagerV1, queue_handle: &QueueHandle<WaylandWorkerState>) {
            if self.xdg_output.is_some() {
                return;
            }
            // Manager bind is gated to v3+ in WaylandGlobals, so
            // wl_output.done is the unified terminator for both wl_output
            // and zxdg_output_v1 events.
            self.xdg_output = Some(manager.get_xdg_output(&self.output, queue_handle, self.name));
        }

        pub fn to_snapshot(&self) -> WaylandOutputSnapshot {
            let (position, size) = self.compute_logical();
            WaylandOutputSnapshot {
                id: self.id,
                name: self.xdg_name.clone().or_else(|| self.output_name.clone()),
                description: self.xdg_description.clone().or_else(|| self.output_description.clone()),
                manufacturer: self.manufacturer.clone(),
                model: self.model.clone(),
                logical_position: position,
                logical_size: size,
                integer_scale: self.scale,
                refresh_rate_hz: f64::from(self.refresh_milli_hz) / 1000.0,
                subpixel: self.subpixel,
                transform: self.transform,
                physical_size_mm: self.physical_size_mm,
            }
        }

        /// Logical position/size in compositor logical pixels (same derivation as the snapshot).
        pub fn logical_position(&self) -> PixelPoint {
            self.compute_logical().0
        }

        pub fn logical_size(&self) -> PixelSize {
            self.compute_logical().1
        }

        fn compute_logical(&self) -> (PixelPoint, PixelSize) {
            let xdg = self.has_xdg_output.then_some((self.xdg_position, self.xdg_size));
            compute_logical(xdg, self.mode_position, self.mode_size, self.scale)
        }

        fn dispose(self) {
            if let Some(xdg_output) = self.xdg_output {
                xdg_output.destroy();
            }
            // `wl_output.release` exists from version 3.
            if self.output.version() >= 3 {
                self.output.release();
            }
        }
    }

    fn subpixel_of(value: WEnum<wl_output::Subpixel>) -> WaylandOutputSubpixel {
        match value {
            WEnum::Value(wl_output::Subpixel::None) => WaylandOutputSubpixel::None,
            WEnum::Value(wl_output::Subpixel::HorizontalRgb) => WaylandOutputSubpixel::HorizontalRgb,
            WEnum::Value(wl_output::Subpixel::HorizontalBgr) => WaylandOutputSubpixel::HorizontalBgr,
            WEnum::Value(wl_output::Subpixel::VerticalRgb) => WaylandOutputSubpixel::VerticalRgb,
            WEnum::Value(wl_output::Subpixel::VerticalBgr) => WaylandOutputSubpixel::VerticalBgr,
            _ => WaylandOutputSubpixel::Unknown,
        }
    }

    fn transform_of(value: WEnum<wl_output::Transform>) -> WaylandOutputTransform {
        match value {
            WEnum::Value(wl_output::Transform::_90) => WaylandOutputTransform::Rotated90,
            WEnum::Value(wl_output::Transform::_180) => WaylandOutputTransform::Rotated180,
            WEnum::Value(wl_output::Transform::_270) => WaylandOutputTransform::Rotated270,
            WEnum::Value(wl_output::Transform::Flipped) => WaylandOutputTransform::Flipped,
            WEnum::Value(wl_output::Transform::Flipped90) => WaylandOutputTransform::Flipped90,
            WEnum::Value(wl_output::Transform::Flipped180) => WaylandOutputTransform::Flipped180,
            WEnum::Value(wl_output::Transform::Flipped270) => WaylandOutputTransform::Flipped270,
            _ => WaylandOutputTransform::Normal,
        }
    }

    /// The listener of a `wl_output`; the user data is the registry name of the output.
    impl Dispatch<WlOutput, u32> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &WlOutput,
            event: wl_output::Event,
            data: &u32,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            let Some(globals) = state.globals.as_mut() else {
                return;
            };
            if let wl_output::Event::Done = event {
                globals.outputs.on_output_done(*data);
                return;
            }
            let Some(output) = globals.outputs.output_mut(*data) else {
                return;
            };
            match event {
                wl_output::Event::Name { name } => output.output_name = Some(name),
                wl_output::Event::Description { description } => output.output_description = Some(description),
                wl_output::Event::Geometry { x, y, physical_width, physical_height, subpixel, make, model, transform } => {
                    output.mode_position = PixelPoint::new(x, y);
                    output.physical_size_mm = PixelSize::new(physical_width, physical_height);
                    output.subpixel = subpixel_of(subpixel);
                    output.manufacturer = Some(make);
                    output.model = Some(model);
                    output.transform = transform_of(transform);
                }
                wl_output::Event::Mode { flags, width, height, refresh } => {
                    let current = match flags {
                        WEnum::Value(flags) => flags.contains(wl_output::Mode::Current),
                        WEnum::Unknown(bits) => bits & 1 != 0,
                    };
                    if !current {
                        return;
                    }
                    output.mode_size = PixelSize::new(width, height);
                    output.refresh_milli_hz = refresh;
                }
                wl_output::Event::Scale { factor } => output.scale = factor,
                _ => {}
            }
        }
    }

    /// The listener of a `zxdg_output_v1`; the user data is the registry name of the output.
    impl Dispatch<ZxdgOutputV1, u32> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &ZxdgOutputV1,
            event: zxdg_output_v1::Event,
            data: &u32,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            let Some(output) = state.globals.as_mut().and_then(|globals| globals.outputs.output_mut(*data)) else {
                return;
            };
            match event {
                zxdg_output_v1::Event::LogicalPosition { x, y } => {
                    output.has_xdg_output = true;
                    output.xdg_position = PixelPoint::new(x, y);
                }
                zxdg_output_v1::Event::LogicalSize { width, height } => {
                    output.has_xdg_output = true;
                    output.xdg_size = PixelSize::new(width, height);
                }
                zxdg_output_v1::Event::Name { name } => output.xdg_name = Some(name),
                zxdg_output_v1::Event::Description { description } => output.xdg_description = Some(description),
                // zxdg_output_v1.done is deprecated since v3 (we only bind
                // v3+); wl_output.done is the unified terminator.
                _ => {}
            }
        }
    }

    impl Dispatch<ZxdgOutputManagerV1, ()> for WaylandWorkerState {
        fn event(_: &mut Self, _: &ZxdgOutputManagerV1, _: <ZxdgOutputManagerV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_logical_geometry_is_that_of_xdg_output_or_the_mode_over_the_scale() {
        let mode_position = PixelPoint::new(1920, 0);
        let mode_size = PixelSize::new(3840, 2160);
        // Without xdg_output.
        assert_eq!(compute_logical(None, mode_position, mode_size, 2), (mode_position, PixelSize::new(1920, 1080)));
        assert_eq!(compute_logical(None, mode_position, mode_size, 1), (mode_position, mode_size));
        // A scale of zero is taken as one; a size never falls below one.
        assert_eq!(compute_logical(None, mode_position, PixelSize::new(0, 0), 0).1, PixelSize::new(1, 1));
        // With xdg_output, also with a fractional scale behind it.
        let xdg = (PixelPoint::new(1536, 0), PixelSize::new(3072, 1728));
        assert_eq!(compute_logical(Some(xdg), mode_position, mode_size, 2), xdg);
        // The size of the mode with a scale above one is not believed.
        let lying = (PixelPoint::new(1920, 0), mode_size);
        assert_eq!(compute_logical(Some(lying), mode_position, mode_size, 2), (mode_position, PixelSize::new(1920, 1080)));
        assert_eq!(compute_logical(Some(lying), mode_position, mode_size, 1), lying);
    }
}
