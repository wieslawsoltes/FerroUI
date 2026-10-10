//! The worker's surfaces (the port of `WSurface.cs`): a `wl_surface`, the
//! `xdg_surface` over it and the `xdg_toplevel` over that, each with the
//! state that is sent again to a compositor that restarted.
//!
//! The reference has three classes derived from one another, and listener
//! classes that hold their surface. Here a top-level holds its shell surface,
//! which holds its surface, and what a base class calls on a derived one
//! (`OnScaleChanged`, `OnOutputsChanged`, `OnConfigureBatchComplete`) is done
//! by the handler of the event after the base returned what happened. The
//! listener classes are the `Dispatch` implementations at the end of this
//! file: the user data of an object is the number of its surface.
//!
//! A popup (`WXdgPopup`) holds its shell surface likewise and names its parent
//! by number, because the parent is another entry of the worker's maps: what
//! the reference reads of the parent object is handed in as [`imp::PopupParent`],
//! and what it calls on the parent (`RegisterPendingChildPopup`) is done by
//! the worker's state (`WaylandWorkerState::try_attach_popup_to_parent`).
//!
//! The fractional scale object and the viewport of a surface, the members of
//! text input and the export of a top-level belong to later parts of stage 2
//! of `docs/porting/wayland-platform.md`.

use ferroui_base::{PixelSize, Point, Rect, Size, Thickness};
use std::sync::atomic::{AtomicU64, Ordering};

/// The number of a surface of the worker: what the UI thread holds of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WSurfaceId(u64);

impl WSurfaceId {
    /// A number no other surface has.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for WSurfaceId {
    fn default() -> Self {
        Self::new()
    }
}

const SCALE_EPSILON: f64 = 1e-6;

/// The scale a surface should render at. Priority: fractional preferred_scale (when fractional path
/// is active) > preferred_buffer_scale (wl_surface v6+) > max scale of currently-entered outputs.
pub fn compute_scale(
    preferred_fractional_scale: Option<f64>,
    last_preferred_buffer_scale: Option<i32>,
    output_scales: impl IntoIterator<Item = i32>,
) -> f64 {
    if let Some(fractional) = preferred_fractional_scale {
        return fractional;
    }
    if let Some(preferred) = last_preferred_buffer_scale {
        return f64::from(preferred);
    }
    let mut new_scale = 1.0;
    for scale in output_scales {
        if f64::from(scale) > new_scale {
            new_scale = f64::from(scale);
        }
    }
    new_scale
}

/// Whether a computed scale differs from the current one.
pub fn scale_differs(new_scale: f64, current_scale: f64) -> bool {
    (new_scale - current_scale).abs() > SCALE_EPSILON
}

/// The integer scale of the buffers of a surface without fractional scaling.
pub fn buffer_scale(scaling: f64) -> i32 {
    (scaling.ceil() as i32).max(1)
}

/// The window geometry of a surface with shadow extents: left, top, width and height in
/// surface-local logical pixels, excluding the shadow margins. `None` for a surface without
/// a size.
///
/// Geometry must be >= 1x1 and lie within the buffer's logical size (protocol requirement).
/// On the fractional-scale path the renderer passes the authoritative logical size from
/// the latest configure; for the integer-scale path we fall back to ceiling the buffer
/// pixel size by the integer scale.
pub fn compute_window_geometry(
    shadow_extents: Thickness,
    buffer_pixel_size: PixelSize,
    scaling: f64,
    logical_size: Size,
    has_fractional_scaling: bool,
) -> Option<(i32, i32, i32, i32)> {
    let (surface_width, surface_height) =
        if has_fractional_scaling && logical_size.width > 0.0 && logical_size.height > 0.0 {
            ((logical_size.width.round_ties_even() as i32).max(1), (logical_size.height.round_ties_even() as i32).max(1))
        } else {
            let int_scale = buffer_scale(scaling);
            ((buffer_pixel_size.width + int_scale - 1) / int_scale, (buffer_pixel_size.height + int_scale - 1) / int_scale)
        };

    if surface_width <= 0 || surface_height <= 0 {
        return None;
    }

    let left = (shadow_extents.left as i32).clamp(0, surface_width - 1);
    let top = (shadow_extents.top as i32).clamp(0, surface_height - 1);
    let right = ((f64::from(surface_width) - shadow_extents.right).ceil() as i32).clamp(left + 1, surface_width);
    let bottom = ((f64::from(surface_height) - shadow_extents.bottom).ceil() as i32).clamp(top + 1, surface_height);

    Some((left, top, right - left, bottom - top))
}

/// The minimum and the maximum size as `xdg_toplevel` takes them: `None` is "no constraint",
/// which is sent as 0.
///
/// Spec: "Requesting a minimum size to be larger than the maximum
/// size of a surface is illegal and will result in an invalid_size
/// error." The minimum is clamped down to the maximum; 0 on max means
/// "unconstrained" and lets any min through.
pub fn min_max_size_request(min_size: Option<Size>, max_size: Option<Size>) -> ((i32, i32), (i32, i32)) {
    let (mut min_w, mut min_h) = match min_size {
        Some(min) => ((min.width.ceil() as i32).max(1), (min.height.ceil() as i32).max(1)),
        None => (0, 0),
    };
    let (max_w, max_h) = match max_size {
        Some(max) => ((max.width.floor() as i32).max(1), (max.height.floor() as i32).max(1)),
        None => (0, 0),
    };

    if max_w > 0 && min_w > max_w {
        min_w = max_w;
    }
    if max_h > 0 && min_h > max_h {
        min_h = max_h;
    }

    ((min_w, min_h), (max_w, max_h))
}

/// The largest size a top-level sized to its content may take: the bounds the compositor
/// gave, else the smallest logical size of the outputs, else 800 by 600.
pub fn calculate_max_size(bounds: Option<PixelSize>, output_logical_sizes: impl IntoIterator<Item = PixelSize>) -> Size {
    // Prefer compositor-provided bounds
    if let Some(bounds) = bounds {
        if bounds.width > 0 && bounds.height > 0 {
            return Size::new(f64::from(bounds.width), f64::from(bounds.height));
        }
    }

    // Fallback: compute from output geometry
    let mut max_size: Option<Size> = None;
    for logical_size in output_logical_sizes {
        let size = logical_size.to_size(1.0);
        max_size = Some(match max_size {
            None => size,
            Some(max) => Size::new(max.width.min(size.width), max.height.min(size.height)),
        });
    }

    max_size.unwrap_or_else(|| Size::new(800.0, 600.0))
}

/// What an `xdg_positioner` is given for a popup: the size, the anchor rectangle (x, y, width,
/// height in the window geometry of the parent) and the offset, when there is one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionerGeometry {
    pub size: (i32, i32),
    pub anchor_rect: (i32, i32, i32, i32),
    pub offset: Option<(i32, i32)>,
}

/// The numbers of the positioner of a popup (the computation of `BuildPositioner`).
///
/// `anchor_rect` is given in the parent's *buffer-relative* logical frame (true top-left,
/// includes shadow). Wayland's xdg_positioner wants coords in the parent's *window-geometry*
/// frame (excludes shadow): `parent_geometry`, the parent's most recent window geometry
/// (left, top, width, height), is the single source of truth for the shift.
pub fn compute_positioner_geometry(
    size: Size,
    deflate: Thickness,
    anchor_rect: Rect,
    offset: Point,
    parent_geometry: Option<(i32, i32, i32, i32)>,
) -> PositionerGeometry {
    // Size corresponds to the popup's window geometry (see the
    // xdg_positioner.set_size contract), which excludes the child margin.
    // The protocol requires positive integers.
    let geometry_size = size.deflate(deflate);
    let width = (geometry_size.width.ceil() as i32).max(1);
    let height = (geometry_size.height.ceil() as i32).max(1);

    // If the parent hasn't reported geometry yet, that's a contract
    // violation (we shouldn't be building a positioner before the
    // parent is mapped) — fall back to assuming origin (0,0) and skip
    // clamping rather than crashing.
    let (origin_x, origin_y) = parent_geometry.map_or((0, 0), |(left, top, _, _)| (left, top));

    let mut anchor_x = anchor_rect.x.round_ties_even() as i32 - origin_x;
    let mut anchor_y = anchor_rect.y.round_ties_even() as i32 - origin_y;
    let mut anchor_w = (anchor_rect.width.round_ties_even() as i32).max(0);
    let mut anchor_h = (anchor_rect.height.round_ties_even() as i32).max(0);

    // Clamp into the parent's window geometry. The protocol forbids the
    // anchor rect from extending outside the parent's geometry; rather
    // than failing the call (which would protocol-error the connection)
    // we clip it. UI-side may produce slightly off-edge anchor rects
    // when, e.g., a context menu opens near the corner of a window.
    //
    // The clamp must produce a strictly positive (>= 1×1) rectangle:
    // xdg_positioner accepts a zero-sized anchor rect via set_anchor_rect,
    // but the spec considers such a positioner *incomplete* and the
    // subsequent get_popup/reposition raises xdg_wm_base.invalid_positioner
    // — a fatal error that disconnects every window. So we
    // shift the floor back by one when the clamp would collapse to zero.
    match parent_geometry {
        Some((_, _, g_width, g_height)) if g_width > 0 && g_height > 0 => {
            let max_x0 = (g_width - 1).max(0);
            let max_y0 = (g_height - 1).max(0);
            let x0 = anchor_x.clamp(0, max_x0);
            let y0 = anchor_y.clamp(0, max_y0);
            let x1 = anchor_x.saturating_add(anchor_w.max(1)).clamp(x0 + 1, g_width);
            let y1 = anchor_y.saturating_add(anchor_h.max(1)).clamp(y0 + 1, g_height);
            anchor_x = x0;
            anchor_y = y0;
            anchor_w = x1 - x0;
            anchor_h = y1 - y0;
        }
        _ => {
            // No parent geometry to clamp against — still ensure the
            // anchor rect is at least 1×1 to keep the positioner valid.
            anchor_w = anchor_w.max(1);
            anchor_h = anchor_h.max(1);
        }
    }

    // Offset is post-resolution (popup-relative); no parent/shadow shift.
    let offset = (offset.x != 0.0 || offset.y != 0.0)
        .then(|| (offset.x.round_ties_even() as i32, offset.y.round_ties_even() as i32));

    PositionerGeometry { size: (width, height), anchor_rect: (anchor_x, anchor_y, anchor_w, anchor_h), offset }
}

/// The first version of `xdg_wm_base` with `xdg_popup.reposition`.
pub const XDG_POPUP_REPOSITION_SINCE: u32 = 3;

#[cfg(target_os = "linux")]
pub use imp::*;

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use crate::screens::wayland_output_snapshot::WaylandOutputId;
    use crate::server::persistent::decoration_mode::DecorationMode;
    use crate::server::persistent::i_persistent_object::{ConnectionContext, IPersistentWaylandObject};
    use crate::server::persistent::i_w_surface_event_sink::{
        PlatformInputEventCookie, WSurfaceEventSinkProxy, WXdgPopupEventSinkProxy, WXdgTopLevelEventSinkProxy,
    };
    use crate::server::persistent::xdg_popup_configure_batch::XdgPopupConfigureBatch;
    use crate::server::persistent::xdg_popup_positioner_params::XdgPopupPositionerParams;
    use crate::server::persistent::wayland_cursor::WaylandCursorId;
    use crate::server::persistent::wayland_input_event_cookie::WaylandInputEventCookie;
    use crate::server::persistent::xdg_configure_batch::XdgConfigureBatch;
    use crate::server::transient::rendering::i_wayland_framebuffer_surface::{
        IWaylandFramebufferSurface, IWaylandSurfaceRenderTarget,
    };
    use crate::server::transient::wayland_globals::WaylandGlobals;
    use crate::server::wayland_worker::{WaylandWorker, WaylandWorkerState};
    use crate::ferro_wayland_exception::FerroWaylandException;
    use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
    use std::rc::{Rc, Weak};
    use std::sync::mpsc::Sender;
    use std::sync::Arc;
    use wayland_client::protocol::wl_callback::{self, WlCallback};
    use wayland_client::protocol::wl_region::WlRegion;
    use wayland_client::protocol::wl_surface::{self, WlSurface};
    use wayland_client::{Connection, Dispatch, QueueHandle};
    use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::{
        self, ZxdgToplevelDecorationV1,
    };
    use ferroui_base::logging::{LogEventLevel, Logger};
    use std::collections::HashMap;
    use wayland_client::Proxy;
    use wayland_protocols::xdg::shell::client::xdg_popup::{self, XdgPopup};
    use wayland_protocols::xdg::shell::client::xdg_positioner::XdgPositioner;
    use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};
    use wayland_protocols::xdg::shell::client::xdg_toplevel::{self, ResizeEdge, XdgToplevel};

    /// The user data of a `wl_surface`: what the surface is. The reference tags the surfaces
    /// of shell surfaces (`WlSurface.Tags`) and finds the shell surface of an input event by
    /// the tag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum WlSurfaceData {
        /// The surface of a shell surface (a top-level or a popup).
        Shell(WSurfaceId),
        /// A surface without events of its own: a cursor.
        Cursor,
    }

    /// What happened to a surface when one of its events was handled, for the classes
    /// derived from `WSurface` in the reference.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct SurfaceChange {
        /// The computed scale changed to this value (`OnScaleChanged`).
        pub scale: Option<f64>,
        /// The outputs the surface is on changed (`OnOutputsChanged`).
        pub outputs: bool,
    }

    /// A `wl_surface` of the worker.
    pub struct WSurface {
        id: WSurfaceId,
        worker: Arc<WaylandWorker>,
        connection_id: Option<u64>,
        wl_surface: Option<WlSurface>,
        last_preferred_buffer_scale: Option<i32>,
        preferred_fractional_scale: Option<f64>,
        /// The outputs the surface is on, in the order it entered them: the registry names.
        outputs: Vec<u32>,
        frame_callback: Option<WlCallback>,
        hit_test_visible: bool,
        current_scale: f64,
        active_render_targets: Vec<Weak<dyn IWaylandSurfaceRenderTarget>>,
        /// Cursor currently requested for this surface, or `None` for the default arrow. Lives on
        /// the worker thread — updated via `set_cursor`, consumed by the input dispatcher when
        /// refreshing the pointer cursor.
        current_cursor: Option<WaylandCursorId>,
    }

    impl WSurface {
        pub fn new(id: WSurfaceId, worker: Arc<WaylandWorker>) -> Self {
            Self {
                id,
                worker,
                connection_id: None,
                wl_surface: None,
                last_preferred_buffer_scale: None,
                preferred_fractional_scale: None,
                outputs: Vec::new(),
                frame_callback: None,
                hit_test_visible: true,
                current_scale: 1.0,
                active_render_targets: Vec::new(),
                current_cursor: None,
            }
        }

        pub fn id(&self) -> WSurfaceId {
            self.id
        }

        pub fn worker(&self) -> &Arc<WaylandWorker> {
            &self.worker
        }

        pub fn wl_surface(&self) -> Option<&WlSurface> {
            self.wl_surface.as_ref()
        }

        /// Fractional scaling needs the fractional scale object and the viewport of the
        /// surface: stage 2. Until then a surface has neither.
        pub fn has_fractional_scaling(&self) -> bool {
            false
        }

        /// Whether this surface currently has an active text input client, which gates
        /// compose processing. A client is set through the text input members of the
        /// surface, which belong to stage 2: until then there is none.
        pub fn has_text_input_client(&self) -> bool {
            false
        }

        /// The number of the connection the surface is on (`CurrentDisplay` of the reference).
        pub fn current_connection_id(&self) -> Option<u64> {
            self.connection_id
        }

        pub fn current_scale(&self) -> f64 {
            self.current_scale
        }

        pub fn current_cursor(&self) -> Option<WaylandCursorId> {
            self.current_cursor
        }

        pub fn outputs(&self) -> &[u32] {
            &self.outputs
        }

        /// Sets the cursor of the surface. The caller refreshes the pointers that are over it
        /// (`WaylandWorkerState::notify_cursor_changed`).
        pub fn set_cursor(&mut self, cursor: Option<WaylandCursorId>) {
            self.current_cursor = cursor;
        }

        pub fn set_hit_test_visible(&mut self, value: bool, globals: Option<&WaylandGlobals>, can_commit_out_of_band: bool) {
            if self.hit_test_visible == value {
                return;
            }
            self.hit_test_visible = value;
            // No-op while disconnected; OnConnected replays the cached value.
            let (Some(_), Some(globals)) = (&self.wl_surface, globals) else {
                return;
            };
            self.apply_input_region(globals);
            // Toggling hit-test visibility changes nothing about what's drawn, so an
            // idle surface may never attach another buffer. Commit out of band to
            // promote the region — a commit without an attach is well-defined.
            if can_commit_out_of_band {
                if let Some(wl_surface) = &self.wl_surface {
                    wl_surface.commit();
                }
            }
        }

        /// Installs the input region matching the cached hit-test state. An empty
        /// region makes the compositor route pointer/touch input to whatever is
        /// behind this surface.
        fn apply_input_region(&self, globals: &WaylandGlobals) {
            let Some(wl_surface) = &self.wl_surface else {
                return;
            };

            if self.hit_test_visible {
                // null is the protocol default: an infinite input region.
                wl_surface.set_input_region(None);
                return;
            }

            let region = globals.wl_compositor.create_region(globals.queue_handle(), ());
            wl_surface.set_input_region(Some(&region));
            region.destroy();
        }

        pub fn on_connected(&mut self, cx: &ConnectionContext<'_>) {
            self.connection_id = Some(cx.connection_id);
            self.wl_surface = Some(cx.globals.wl_compositor.create_surface(cx.queue_handle, WlSurfaceData::Shell(self.id)));

            // Re-apply the cached input region on (re)connect. It's double-buffered
            // state, promoted by the next commit — which happens before the surface
            // can receive any input.
            self.apply_input_region(cx.globals);
        }

        pub fn is_frame_callback_pending(&self) -> bool {
            self.frame_callback.is_some()
        }

        pub fn state(&self) -> PlatformRenderTargetState {
            if self.connection_id.is_some() && self.wl_surface.is_some() && !self.is_frame_callback_pending() {
                PlatformRenderTargetState::READY
            } else {
                PlatformRenderTargetState::default()
            }
        }

        pub fn register_render_target(&mut self, render_target: Weak<dyn IWaylandSurfaceRenderTarget>) {
            self.active_render_targets.push(render_target);
        }

        pub fn unregister_render_target(&mut self, render_target: &Rc<dyn IWaylandSurfaceRenderTarget>) {
            self.active_render_targets.retain(|target| match target.upgrade() {
                Some(target) => !Rc::ptr_eq(&target, render_target),
                None => false,
            });
        }

        /// Recomputes the desired scale and says what it changed to. Called on the Wayland
        /// thread.
        fn recompute_scale(&mut self, globals: Option<&WaylandGlobals>) -> Option<f64> {
            let output_scales: Vec<i32> = match globals {
                Some(globals) => {
                    self.outputs.iter().filter_map(|name| globals.outputs.find(*name)).map(|output| output.scale).collect()
                }
                None => Vec::new(),
            };
            let new_scale = compute_scale(self.preferred_fractional_scale, self.last_preferred_buffer_scale, output_scales);
            if scale_differs(new_scale, self.current_scale) {
                self.current_scale = new_scale;
                return Some(new_scale);
            }
            None
        }

        /// Invoked by the platform surface before attaching a buffer and
        /// calling wl_surface::commit. EGL/Vulkan drivers want to make commits
        /// themselves for some reason, so we have this kind of callback logic.
        ///
        /// Stages double-buffered per-frame state — frame callback,
        /// ack_configure, set_window_geometry, viewport / buffer scale, min/max
        /// constraints — into the next commit so they apply atomically with
        /// the new buffer.
        pub fn on_before_new_buffer_attached(&mut self, globals: &WaylandGlobals, scene_info: &RenderTargetSceneInfo) {
            let Some(wl_surface) = &self.wl_surface else {
                return;
            };
            wl_surface.set_buffer_scale(buffer_scale(scene_info.scaling));
            self.frame_callback = Some(wl_surface.frame(globals.queue_handle(), self.id));
        }

        fn on_frame_done(&mut self, callback: &WlCallback) {
            self.worker.wakeup_render_loop();
            // A popup that was created again has dropped the callback of its last frame: the
            // answer to that one must not clear the callback of a newer frame.
            if self.frame_callback.as_ref().is_some_and(|pending| pending == callback) {
                self.frame_callback = None;
            }
        }

        /// Forgets the frame callback of a surface that is unmapped, which the compositor
        /// answers late or never.
        fn forget_frame_callback(&mut self) {
            self.frame_callback = None;
        }

        fn on_preferred_buffer_scale(&mut self, factor: i32, globals: Option<&WaylandGlobals>) -> SurfaceChange {
            self.last_preferred_buffer_scale = Some(factor);
            SurfaceChange { scale: self.recompute_scale(globals), outputs: false }
        }

        fn on_enter(&mut self, output: Option<u32>, globals: Option<&WaylandGlobals>) -> SurfaceChange {
            if let Some(output) = output {
                self.outputs.push(output);
            }
            SurfaceChange { scale: self.recompute_scale(globals), outputs: true }
        }

        fn on_leave(&mut self, output: Option<u32>, globals: Option<&WaylandGlobals>) -> SurfaceChange {
            if let Some(output) = output {
                if let Some(index) = self.outputs.iter().position(|name| *name == output) {
                    self.outputs.remove(index);
                }
            }
            SurfaceChange { scale: self.recompute_scale(globals), outputs: true }
        }

        /// An output of the list is gone: the compositor removed its global.
        pub fn forget_output(&mut self, name: u32) {
            self.outputs.retain(|output| *output != name);
        }

        pub fn on_disconnected(&mut self) {
            self.outputs.clear();
            self.last_preferred_buffer_scale = None;
            self.preferred_fractional_scale = None;
            self.frame_callback = None;
            // Dispose render targets before destroying the wl_surface so EGL
            // resources are torn down while the underlying wl_surface is still alive.
            for render_target in std::mem::take(&mut self.active_render_targets) {
                if let Some(render_target) = render_target.upgrade() {
                    render_target.dispose_from_surface();
                }
            }
            if let Some(wl_surface) = self.wl_surface.take() {
                wl_surface.destroy();
            }
            self.connection_id = None;
        }
    }

    /// An `xdg_surface` of the worker.
    pub struct WXdgShellSurface {
        surface: WSurface,
        xdg_surface: Option<XdgSurface>,
        pending_ack_serial: Option<u32>,
        initial_configure_acknowledged: bool,
        // True once we've attached a buffer + acked the initial configure
        // following the most recent OnConnected — i.e. the surface is now
        // *mapped* in xdg-shell terms.
        mapped: bool,
        /// Children popups whose parents weren't yet mapped when the popup
        /// went through OnConnected (or when the child itself reconnected
        /// before the parent finished mapping). Drained when this surface
        /// transitions to mapped (in `on_before_new_buffer_attached`).
        /// Cleared on disconnect — every child's OnDisconnected runs
        /// independently, so on reconnect they will re-register if still pending.
        pending_child_popups: Vec<WSurfaceId>,
        /// The children the transition to mapped drained, until the worker's state attaches
        /// them (the loop of `OnBeforeNewBufferAttached` in the reference).
        children_to_attach: Vec<WSurfaceId>,
        /// The UI-thread event sink associated with this surface.
        event_sink: WSurfaceEventSinkProxy,
        shadow_extents: Option<Thickness>,
        /// The window-geometry rectangle (left, top, width, height in
        /// surface-local logical pixels, *excluding* shadow margins) that we
        /// last sent to the compositor via `xdg_surface.set_window_geometry`.
        /// `None` until the first buffer is attached on the current connection.
        last_window_geometry: Option<(i32, i32, i32, i32)>,
    }

    impl WXdgShellSurface {
        pub fn new(id: WSurfaceId, worker: Arc<WaylandWorker>, event_sink: WSurfaceEventSinkProxy) -> Self {
            Self {
                surface: WSurface::new(id, worker),
                xdg_surface: None,
                pending_ack_serial: None,
                initial_configure_acknowledged: false,
                mapped: false,
                pending_child_popups: Vec::new(),
                children_to_attach: Vec::new(),
                event_sink,
                shadow_extents: None,
                last_window_geometry: None,
            }
        }

        pub fn surface(&self) -> &WSurface {
            &self.surface
        }

        pub fn surface_mut(&mut self) -> &mut WSurface {
            &mut self.surface
        }

        pub fn xdg_surface(&self) -> Option<&XdgSurface> {
            self.xdg_surface.as_ref()
        }

        /// True iff this surface is currently mapped (xdg-shell sense).
        pub fn is_mapped(&self) -> bool {
            self.mapped
        }

        // Committing an xdg_surface before it has a role, or before its initial
        // configure has been acked, is a protocol error. Until we're mapped, staged
        // state rides along on the commit that assigns the role
        // or on the first buffer attach.
        pub fn can_commit_out_of_band(&self) -> bool {
            self.mapped
        }

        pub fn register_pending_child_popup(&mut self, popup: WSurfaceId) {
            self.pending_child_popups.push(popup);
        }

        pub fn unregister_pending_child_popup(&mut self, popup: WSurfaceId) {
            if let Some(index) = self.pending_child_popups.iter().position(|pending| *pending == popup) {
                self.pending_child_popups.remove(index);
            }
        }

        /// The popups that waited for this surface to be mapped and may attach now.
        pub fn take_children_to_attach(&mut self) -> Vec<WSurfaceId> {
            std::mem::take(&mut self.children_to_attach)
        }

        pub fn event_sink(&self) -> &WSurfaceEventSinkProxy {
            &self.event_sink
        }

        pub fn shadow_extents(&self) -> Thickness {
            self.shadow_extents.unwrap_or_default()
        }

        pub fn last_window_geometry(&self) -> Option<(i32, i32, i32, i32)> {
            self.last_window_geometry
        }

        pub fn set_shadow_extents(&mut self, extents: Thickness) {
            self.shadow_extents = Some(extents);
        }

        /// What a derived class of the reference gets through `OnScaleChanged` and
        /// `OnOutputsChanged`: both go to the sink.
        fn apply_change(&self, change: SurfaceChange, globals: Option<&WaylandGlobals>) {
            if let Some(scale) = change.scale {
                self.event_sink.on_scale_changed(scale);
            }
            if change.outputs {
                // Snapshot the per-surface entered-outputs identity list and post
                // it to the UI thread so the screens can map
                // the window onto a Screen. Identity matches WaylandOutputSnapshot.Id.
                let ids: Vec<WaylandOutputId> = match globals {
                    Some(globals) => self
                        .surface
                        .outputs
                        .iter()
                        .filter_map(|name| globals.outputs.find(*name))
                        .map(|output| output.id)
                        .collect(),
                    None => Vec::new(),
                };
                self.event_sink.on_surface_outputs_changed(ids);
            }
        }

        pub fn on_connected(&mut self, cx: &ConnectionContext<'_>) {
            self.surface.on_connected(cx);
            self.pending_ack_serial = None;
            self.initial_configure_acknowledged = false;
            self.mapped = false;

            let wl_surface = self.surface.wl_surface.as_ref().expect("the surface was just created");
            self.xdg_surface = Some(cx.globals.xdg_wm_base.get_xdg_surface(wl_surface, cx.queue_handle, self.surface.id));
        }

        pub fn on_disconnected(&mut self) {
            if let Some(xdg_surface) = self.xdg_surface.take() {
                xdg_surface.destroy();
            }
            self.pending_ack_serial = None;
            self.initial_configure_acknowledged = false;
            self.mapped = false;
            self.pending_child_popups.clear();
            self.children_to_attach.clear();
            self.last_window_geometry = None;
            self.event_sink.on_surface_outputs_changed(Vec::new());
            self.surface.on_disconnected();
        }

        /// Called from UI thread (via Post) to set the serial that should be acked on next commit.
        pub fn set_pending_ack_serial(&mut self, serial: u32) {
            self.initial_configure_acknowledged = true;
            self.pending_ack_serial = Some(serial);
        }

        pub fn state(&self) -> PlatformRenderTargetState {
            if self.initial_configure_acknowledged {
                self.surface.state()
            } else {
                PlatformRenderTargetState::NOT_READY_WILL_WAKEUP_RENDER_LOOP
            }
        }

        pub fn on_before_new_buffer_attached(&mut self, globals: &WaylandGlobals, scene_info: &RenderTargetSceneInfo) {
            if let (Some(serial), Some(xdg_surface)) = (self.pending_ack_serial.take(), &self.xdg_surface) {
                xdg_surface.ack_configure(serial);
            }

            self.maybe_emit_window_geometry(scene_info.size, scene_info.scaling, scene_info.logical_size);

            self.surface.on_before_new_buffer_attached(globals, scene_info);

            // First buffer attach + configure-ack since the most recent
            // (re-)connect → we are now mapped. Drain any popups that were
            // queued while we were unmapped; they can now safely call
            // xdg_surface.get_popup against us.
            if !self.mapped {
                self.mapped = true;
                self.children_to_attach.append(&mut self.pending_child_popups);
            }
        }

        /// Computes and emits the surface's window geometry (the rectangle
        /// inside the buffer that the compositor should treat as the visible
        /// window, excluding decoration-area shadows).
        ///
        /// Skipped entirely until the surface receives its first
        /// `set_shadow_extents` call — the compositor then treats
        /// the whole buffer as the visible window. Once extents have
        /// been set we keep emitting geometry even if extents go back to all
        /// zero (e.g. when a window is maximized): the compositor still needs
        /// the explicit "geometry == buffer" call to discard the previous
        /// shadow margins.
        fn maybe_emit_window_geometry(&mut self, buffer_pixel_size: PixelSize, scaling: f64, logical_size: Size) {
            let Some(shadow_extents) = self.shadow_extents else {
                return;
            };

            let geometry = compute_window_geometry(
                shadow_extents,
                buffer_pixel_size,
                scaling,
                logical_size,
                self.surface.has_fractional_scaling(),
            );
            let Some((left, top, width, height)) = geometry else {
                self.last_window_geometry = None;
                return;
            };

            if let Some(xdg_surface) = &self.xdg_surface {
                xdg_surface.set_window_geometry(left, top, width, height);
            }
            self.last_window_geometry = Some((left, top, width, height));
        }
    }

    /// An `xdg_toplevel` of the worker.
    pub struct WXdgTopLevel {
        shell: WXdgShellSurface,
        basic_init_completed: Option<Sender<Arc<XdgConfigureBatch>>>,
        basic_init_is_completed: bool,
        xdg_top_level: Option<XdgToplevel>,
        pending_batch: XdgConfigureBatch,
        // Typed event sink for xdg_toplevel-specific events.
        top_level_event_sink: WXdgTopLevelEventSinkProxy,
        min_size: Option<Size>,
        max_size: Option<Size>,
        title: Option<String>,
        decoration: Option<ZxdgToplevelDecorationV1>,
        // Disable SSD support completely and don't allow re-enabling it because we can't
        csd_sticky: bool,
    }

    impl WXdgTopLevel {
        /// A top-level that is not registered with the worker yet. `basic_init_completed`
        /// receives the first sealed configure (`BasicInitCompleted` of the reference).
        pub fn new(
            id: WSurfaceId,
            worker: Arc<WaylandWorker>,
            event_sink: WXdgTopLevelEventSinkProxy,
            basic_init_completed: Sender<Arc<XdgConfigureBatch>>,
        ) -> Self {
            Self {
                shell: WXdgShellSurface::new(id, worker, event_sink.as_surface_sink().clone()),
                basic_init_completed: Some(basic_init_completed),
                basic_init_is_completed: false,
                xdg_top_level: None,
                pending_batch: XdgConfigureBatch::default(),
                top_level_event_sink: event_sink,
                min_size: None,
                max_size: None,
                title: None,
                decoration: None,
                csd_sticky: false,
            }
        }

        pub fn shell(&self) -> &WXdgShellSurface {
            &self.shell
        }

        pub fn shell_mut(&mut self) -> &mut WXdgShellSurface {
            &mut self.shell
        }

        pub fn xdg_top_level(&self) -> Option<&XdgToplevel> {
            self.xdg_top_level.as_ref()
        }

        fn calculate_max_size(&self, globals: Option<&WaylandGlobals>) -> Size {
            let logical_sizes: Vec<PixelSize> = match globals {
                Some(globals) => {
                    let entered: Vec<PixelSize> = self
                        .shell
                        .surface
                        .outputs
                        .iter()
                        .filter_map(|name| globals.outputs.find(*name))
                        .map(|output| output.logical_size())
                        .collect();
                    if entered.is_empty() {
                        globals.outputs.outputs().iter().map(|output| output.logical_size()).collect()
                    } else {
                        entered
                    }
                }
                None => Vec::new(),
            };
            calculate_max_size(self.pending_batch.bounds, logical_sizes)
        }

        pub fn set_maximized(&self) {
            if let Some(top_level) = &self.xdg_top_level {
                top_level.set_maximized();
            }
        }

        pub fn unset_maximized(&self) {
            if let Some(top_level) = &self.xdg_top_level {
                top_level.unset_maximized();
            }
        }

        pub fn set_fullscreen(&self) {
            if let Some(top_level) = &self.xdg_top_level {
                top_level.set_fullscreen(None);
            }
        }

        pub fn unset_fullscreen(&self) {
            if let Some(top_level) = &self.xdg_top_level {
                top_level.unset_fullscreen();
            }
        }

        pub fn set_minimized(&self) {
            if let Some(top_level) = &self.xdg_top_level {
                top_level.set_minimized();
            }
        }

        pub fn set_title(&mut self, title: Option<String>) {
            if let Some(top_level) = &self.xdg_top_level {
                top_level.set_title(title.clone().unwrap_or_default());
            }
            self.title = title;
        }

        pub fn set_min_max_size(&mut self, min_size: Option<Size>, max_size: Option<Size>) {
            if min_size == self.min_size && max_size == self.max_size {
                return;
            }
            self.min_size = min_size;
            self.max_size = max_size;
            // No-op if we aren't currently connected; OnConnected replays the
            // cached values when a connection is (re)established.
            if self.xdg_top_level.is_none() {
                return;
            }
            self.emit_min_max_size();
            // Push immediately so an idle window with no pending buffer still
            // sees the new constraints take effect. A commit without an
            // attach is well-defined: it promotes the double-buffered values
            // without redrawing.
            if let Some(wl_surface) = self.shell.surface.wl_surface() {
                wl_surface.commit();
            }
        }

        fn emit_min_max_size(&self) {
            let Some(top_level) = &self.xdg_top_level else {
                return;
            };
            let ((min_w, min_h), (max_w, max_h)) = min_max_size_request(self.min_size, self.max_size);
            top_level.set_min_size(min_w, min_h);
            top_level.set_max_size(max_w, max_h);
        }

        /// Makes the top-level the child of another one, or of none.
        pub fn set_parent(&self, parent: Option<&XdgToplevel>) {
            if let Some(top_level) = &self.xdg_top_level {
                top_level.set_parent(parent);
            }
        }

        fn consume_cookie(&self, platform_cookie: Option<&PlatformInputEventCookie>) -> Option<(wayland_client::protocol::wl_seat::WlSeat, u32)> {
            let cookie = platform_cookie?.downcast_ref::<WaylandInputEventCookie>()?;
            let connection_id = self.shell.surface.current_connection_id()?;
            cookie.try_consume(connection_id)
        }

        /// Starts an interactive move with the serial of the input event of the cookie.
        pub fn move_(&self, platform_cookie: Option<&PlatformInputEventCookie>) {
            let Some(top_level) = &self.xdg_top_level else {
                return;
            };
            if let Some((seat, serial)) = self.consume_cookie(platform_cookie) {
                top_level._move(&seat, serial);
            }
        }

        /// Starts an interactive resize with the serial of the input event of the cookie.
        pub fn resize(&self, platform_cookie: Option<&PlatformInputEventCookie>, edge: ResizeEdge) {
            let Some(top_level) = &self.xdg_top_level else {
                return;
            };
            if let Some((seat, serial)) = self.consume_cookie(platform_cookie) {
                top_level.resize(&seat, serial, edge);
            }
        }

        /// `xdg_surface.configure`: the batch is complete.
        fn on_configure_batch_complete(&mut self, serial: u32, globals: Option<&WaylandGlobals>) {
            let mut batch = std::mem::take(&mut self.pending_batch);
            batch.serial = serial;
            // The bounds of the batch are read by the computation of the largest size.
            self.pending_batch.bounds = batch.bounds;
            batch.max_size = self.calculate_max_size(globals);
            self.pending_batch = XdgConfigureBatch::default();

            let batch = Arc::new(batch);
            self.top_level_event_sink.on_configure(batch.clone());
            if let Some(basic_init_completed) = self.basic_init_completed.take() {
                // The UI thread waits for this one; a window that is gone already does not.
                let _ = basic_init_completed.send(batch);
            }
            self.basic_init_is_completed = true;

            self.shell.surface.worker.wakeup_render_loop();
        }

        fn on_decoration_configure(&mut self, mode: DecorationMode) {
            if self.basic_init_is_completed {
                self.top_level_event_sink.on_decoration_mode_changed(mode);
            } else {
                self.pending_batch.initial_decoration_mode = Some(mode);
            }
        }

        pub fn destroy_decoration(&mut self) {
            self.csd_sticky = true;
            if let Some(decoration) = self.decoration.take() {
                decoration.destroy();
            }
        }

    }

    impl IPersistentWaylandObject for WXdgTopLevel {
        fn on_connected(&mut self, cx: &ConnectionContext<'_>) {
            self.shell.on_connected(cx);
            if cx.globals.outputs.outputs().is_empty() {
                FerroWaylandException::new("Expected at least one wl_output at this point").throw();
            }
            self.pending_batch = XdgConfigureBatch::default();
            let xdg_surface = self.shell.xdg_surface.as_ref().expect("the shell surface was just connected");
            let top_level = xdg_surface.get_toplevel(cx.queue_handle, self.shell.surface.id);
            if !self.csd_sticky {
                if let Some(decoration_manager) = &cx.globals.xdg_decoration_manager {
                    let decoration =
                        decoration_manager.get_toplevel_decoration(&top_level, cx.queue_handle, self.shell.surface.id);
                    decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide);
                    self.decoration = Some(decoration);
                }
            }

            // Re-apply cached title on reconnect.
            if let Some(title) = &self.title {
                top_level.set_title(title.clone());
            }

            // The application identifier of the options: an addition (see the options).
            if let Some(app_id) = &cx.globals.app_id {
                top_level.set_app_id(app_id.clone());
            }

            self.xdg_top_level = Some(top_level);

            // Re-apply cached min/max if they were ever set on a previous
            // (now-dead) connection. The commit below will
            // promote the queued double-buffered state.
            if self.min_size.is_some() || self.max_size.is_some() {
                self.emit_min_max_size();
            }
            if let Some(wl_surface) = self.shell.surface.wl_surface() {
                wl_surface.commit();
            }
        }

        fn on_disconnected(&mut self) {
            self.shell.event_sink.on_keyboard_leave();
            if let Some(decoration) = self.decoration.take() {
                decoration.destroy();
            }
            if let Some(top_level) = self.xdg_top_level.take() {
                top_level.destroy();
            }
            self.pending_batch = XdgConfigureBatch::default();
            self.shell.on_disconnected();
        }
    }

    impl IWaylandFramebufferSurface for WXdgTopLevel {
        fn wl_surface(&self) -> Option<&WlSurface> {
            self.shell.surface.wl_surface()
        }

        fn state(&self) -> PlatformRenderTargetState {
            self.shell.state()
        }

        fn register_render_target(&mut self, render_target: Weak<dyn IWaylandSurfaceRenderTarget>) {
            self.shell.surface.register_render_target(render_target);
        }

        fn unregister_render_target(&mut self, render_target: &Rc<dyn IWaylandSurfaceRenderTarget>) {
            self.shell.surface.unregister_render_target(render_target);
        }

        fn on_before_new_buffer_attached(&mut self, globals: &WaylandGlobals, scene_info: &RenderTargetSceneInfo) {
            self.shell.on_before_new_buffer_attached(globals, scene_info);
        }

        // Toplevels/popups attach one buffer per frame inside the throttled render loop, which flushes
        // naturally, so no eager roundtrip is needed (it would stall the render loop).
        fn enforce_buffer_creation_roundtrip(&self) -> bool {
            false
        }
    }

    /// The shell surface of a top-level or of a popup, by its number.
    pub fn shell_surface_of<'a>(
        top_levels: &'a HashMap<WSurfaceId, WXdgTopLevel>,
        popups: &'a HashMap<WSurfaceId, WXdgPopup>,
        id: WSurfaceId,
    ) -> Option<&'a WXdgShellSurface> {
        match top_levels.get(&id) {
            Some(top_level) => Some(&top_level.shell),
            None => popups.get(&id).map(|popup| &popup.shell),
        }
    }

    /// The shell surface of a top-level or of a popup, by its number.
    pub fn shell_surface_of_mut<'a>(
        top_levels: &'a mut HashMap<WSurfaceId, WXdgTopLevel>,
        popups: &'a mut HashMap<WSurfaceId, WXdgPopup>,
        id: WSurfaceId,
    ) -> Option<&'a mut WXdgShellSurface> {
        match top_levels.get_mut(&id) {
            Some(top_level) => Some(&mut top_level.shell),
            None => popups.get_mut(&id).map(|popup| &mut popup.shell),
        }
    }

    /// What a popup reads of its parent surface (`_parent.IsMapped`, `_parent.XdgSurface`,
    /// `_parent.LastWindowGeometry` of the reference).
    #[derive(Clone, Debug)]
    pub struct PopupParent {
        pub is_mapped: bool,
        pub xdg_surface: Option<XdgSurface>,
        pub last_window_geometry: Option<(i32, i32, i32, i32)>,
    }

    impl PopupParent {
        pub fn of(shell: &WXdgShellSurface) -> Self {
            Self {
                is_mapped: shell.mapped,
                xdg_surface: shell.xdg_surface.clone(),
                last_window_geometry: shell.last_window_geometry,
            }
        }
    }

    /// What became of an attempt to attach a popup to its parent.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PopupAttach {
        /// Nothing to do, or the popup is attached.
        Done,
        /// Parent not yet mapped — it has to register the popup and call back
        /// when its first buffer commits.
        ParentNotMapped,
    }

    /// Worker-side xdg_popup surface. The UI thread keeps a
    /// `WXdgPopupProxy` handle and feeds positioner parameters via
    /// `update_positioner`; the worker rebuilds the protocol-level
    /// `xdg_positioner` and calls `xdg_surface.get_popup` on every
    /// (re-)connect (so the popup survives compositor reconnects). Popup
    /// configure events are accumulated in `pending_batch` and
    /// flushed to `IWXdgPopupEventSink::on_popup_configure` when the
    /// wrapping `xdg_surface.configure(serial)` seals the batch.
    pub struct WXdgPopup {
        shell: WXdgShellSurface,
        popup_event_sink: WXdgPopupEventSinkProxy,
        parent: WSurfaceId,
        xdg_popup: Option<XdgPopup>,
        pending_batch: XdgPopupConfigureBatch,
        positioner: Option<XdgPopupPositionerParams>,
        // 0 is reserved (means "no token"); start at 1 and bump on every reposition.
        next_reposition_token: u32,
        /// The popup was created again in place of a reposition (a compositor whose
        /// `xdg_wm_base` is older than version 3): its next configure asks for a frame.
        recreated: bool,
    }

    impl WXdgPopup {
        pub fn new(id: WSurfaceId, worker: Arc<WaylandWorker>, event_sink: WXdgPopupEventSinkProxy, parent: WSurfaceId) -> Self {
            Self {
                shell: WXdgShellSurface::new(id, worker, event_sink.as_surface_sink().clone()),
                popup_event_sink: event_sink,
                parent,
                xdg_popup: None,
                pending_batch: XdgPopupConfigureBatch::default(),
                positioner: None,
                next_reposition_token: 1,
                recreated: false,
            }
        }

        pub fn shell(&self) -> &WXdgShellSurface {
            &self.shell
        }

        pub fn shell_mut(&mut self) -> &mut WXdgShellSurface {
            &mut self.shell
        }

        /// The number of the parent surface: a top-level or another popup.
        pub fn parent(&self) -> WSurfaceId {
            self.parent
        }

        /// Whether the popup has its role object on the compositor.
        pub fn is_attached(&self) -> bool {
            self.xdg_popup.is_some()
        }

        /// Takes new positioner parameters. `has_attached_children` says whether a popup
        /// whose parent this one is has its role object: such a popup cannot be created
        /// again (the topmost popup has to be destroyed first).
        pub fn update_positioner(
            &mut self,
            positioner: XdgPopupPositionerParams,
            parent: Option<&PopupParent>,
            globals: Option<&WaylandGlobals>,
            has_attached_children: bool,
        ) -> PopupAttach {
            let previous = self.positioner.replace(positioner);

            // The popup child's margin (Deflate) must be carved out of the surface's window geometry
            // so the compositor positions and constrains against the child content.
            if positioner.deflate != Thickness::default() {
                self.shell.set_shadow_extents(positioner.deflate);
            }

            let Some(previous) = previous else {
                // First positioner — attempt to attach now (may still defer
                // if the parent isn't mapped or we're not yet connected;
                // OnConnected / parent's drain will retry).
                return self.try_attach_to_parent(parent, globals);
            };

            let (Some(xdg_popup), Some(globals)) = (&self.xdg_popup, globals) else {
                return PopupAttach::Done;
            };

            if globals.xdg_wm_base.version() < XDG_POPUP_REPOSITION_SINCE {
                // `xdg_popup.reposition` does not exist before version 3 of the shell. The
                // popup is given its new place the way toolkits did before the request
                // existed: its role objects are destroyed and created again from the new
                // positioner. Parameters that did not change leave the popup alone.
                if previous == positioner {
                    return PopupAttach::Done;
                }
                if has_attached_children {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "Wayland") {
                        logger.log(
                            None,
                            "A popup with an open child popup cannot be repositioned on xdg_wm_base before version 3",
                        );
                    }
                    return PopupAttach::Done;
                }
                return self.recreate(parent, globals);
            }

            // Already mapped — issue a reposition. The compositor will reply
            // with a fresh configure + repositioned(token) sequence; the
            // pending batch is reset so the next OnPopupConfigure carries the
            // post-reposition geometry.
            let p = Self::build_positioner(&positioner, parent, globals);
            self.pending_batch = XdgPopupConfigureBatch::default();
            xdg_popup.reposition(&p, self.next_reposition_token);
            self.next_reposition_token += 1;
            p.destroy();
            PopupAttach::Done
        }

        /// The equivalent of a reposition on a compositor without the request: the popup is
        /// unmapped, its `xdg_popup` and `xdg_surface` are destroyed, and both are created
        /// again on the same `wl_surface` with a positioner of the new parameters. The
        /// surface is then as it is after a connect: it waits for a configure, and its next
        /// buffer maps it.
        fn recreate(&mut self, parent: Option<&PopupParent>, globals: &WaylandGlobals) -> PopupAttach {
            let Some(wl_surface) = self.shell.surface.wl_surface.clone() else {
                return PopupAttach::Done;
            };
            if let Some(xdg_popup) = self.xdg_popup.take() {
                xdg_popup.destroy();
            }
            if let Some(xdg_surface) = self.shell.xdg_surface.take() {
                xdg_surface.destroy();
            }
            // An `xdg_surface` cannot be made for a surface that has a buffer.
            wl_surface.attach(None, 0, 0);
            wl_surface.commit();

            self.shell.pending_ack_serial = None;
            self.shell.initial_configure_acknowledged = false;
            self.shell.mapped = false;
            self.shell.last_window_geometry = None;
            self.shell.surface.forget_frame_callback();
            self.shell.xdg_surface =
                Some(globals.xdg_wm_base.get_xdg_surface(&wl_surface, globals.queue_handle(), self.shell.surface.id));
            self.recreated = true;

            self.try_attach_to_parent(parent, Some(globals))
        }

        /// Called either from our own `on_connected`, or from the
        /// parent's `on_before_new_buffer_attached` when
        /// finalising pending child popups whose creation was deferred because
        /// the parent wasn't mapped yet. xdg_surface.get_popup against an
        /// unmapped parent is a protocol error, so we wait for the parent's
        /// first frame.
        pub fn try_attach_to_parent(&mut self, parent: Option<&PopupParent>, globals: Option<&WaylandGlobals>) -> PopupAttach {
            if self.xdg_popup.is_some() {
                return PopupAttach::Done; // already attached
            }
            let (Some(xdg_surface), Some(globals)) = (&self.shell.xdg_surface, globals) else {
                return PopupAttach::Done; // we're not connected
            };
            let Some(pos) = &self.positioner else {
                return PopupAttach::Done; // UI hasn't supplied positioner yet
            };
            let Some(parent) = parent else {
                // The parent surface is gone: the popup has nothing to attach to.
                return PopupAttach::Done;
            };
            let parent_xdg_surface = match &parent.xdg_surface {
                Some(parent_xdg_surface) if parent.is_mapped => parent_xdg_surface,
                // Parent not yet mapped — register; parent will call us back
                // when its first buffer commits.
                _ => return PopupAttach::ParentNotMapped,
            };

            let positioner = Self::build_positioner(pos, Some(parent), globals);
            self.pending_batch = XdgPopupConfigureBatch::default();
            self.xdg_popup =
                Some(xdg_surface.get_popup(Some(parent_xdg_surface), &positioner, globals.queue_handle(), self.shell.surface.id));
            positioner.destroy();
            if let Some(wl_surface) = self.shell.surface.wl_surface() {
                wl_surface.commit();
            }
            PopupAttach::Done
        }

        /// Translates the cached `XdgPopupPositionerParams` into a
        /// fresh `xdg_positioner` protocol object. The caller owns the
        /// returned object and must destroy it after use.
        /// Performs the buffer→geometry origin shift on the anchor rect (using
        /// the parent's most recent `set_window_geometry` as the
        /// authoritative source) and clamps it into the parent's window-geometry
        /// rectangle as required by the xdg_positioner spec.
        fn build_positioner(p: &XdgPopupPositionerParams, parent: Option<&PopupParent>, globals: &WaylandGlobals) -> XdgPositioner {
            let positioner = globals.xdg_wm_base.create_positioner(globals.queue_handle(), ());

            let geometry = compute_positioner_geometry(
                p.size,
                p.deflate,
                p.anchor_rect,
                p.offset,
                parent.and_then(|parent| parent.last_window_geometry),
            );
            positioner.set_size(geometry.size.0, geometry.size.1);
            let (anchor_x, anchor_y, anchor_w, anchor_h) = geometry.anchor_rect;
            positioner.set_anchor_rect(anchor_x, anchor_y, anchor_w, anchor_h);
            positioner.set_anchor(p.anchor);
            positioner.set_gravity(p.gravity);
            positioner.set_constraint_adjustment(p.constraint_adjustment);

            if let Some((x, y)) = geometry.offset {
                positioner.set_offset(x, y);
            }

            positioner
        }

        /// `xdg_surface.configure`: the batch is complete.
        fn on_configure_batch_complete(&mut self, serial: u32) {
            let mut batch = std::mem::take(&mut self.pending_batch);
            batch.serial = serial;
            batch.recreated = std::mem::take(&mut self.recreated);

            self.popup_event_sink.on_popup_configure(batch);

            self.shell.surface.worker.wakeup_render_loop();
        }

        /// Tells the popup that the connection is lost, or that the popup is destroyed. The
        /// caller unregisters it from the pending list of its parent
        /// (`_parent.UnregisterPendingChildPopup`), which is another object of the worker.
        pub fn on_disconnected(&mut self) {
            self.shell.event_sink.on_keyboard_leave();
            if let Some(xdg_popup) = self.xdg_popup.take() {
                xdg_popup.destroy();
            }
            self.pending_batch = XdgPopupConfigureBatch::default();
            self.recreated = false;
            self.shell.on_disconnected();
        }

        /// Always create wl_surface + xdg_surface (so any pending children of
        /// _ours_ can attach), even if our own popup creation is deferred
        /// because the parent isn't ready yet. The caller then tries to attach the popup
        /// to its parent.
        pub fn on_connected(&mut self, cx: &ConnectionContext<'_>) {
            self.shell.on_connected(cx);
        }
    }

    impl IWaylandFramebufferSurface for WXdgPopup {
        fn wl_surface(&self) -> Option<&WlSurface> {
            self.shell.surface.wl_surface()
        }

        fn state(&self) -> PlatformRenderTargetState {
            self.shell.state()
        }

        fn register_render_target(&mut self, render_target: Weak<dyn IWaylandSurfaceRenderTarget>) {
            self.shell.surface.register_render_target(render_target);
        }

        fn unregister_render_target(&mut self, render_target: &Rc<dyn IWaylandSurfaceRenderTarget>) {
            self.shell.surface.unregister_render_target(render_target);
        }

        fn on_before_new_buffer_attached(&mut self, globals: &WaylandGlobals, scene_info: &RenderTargetSceneInfo) {
            self.shell.on_before_new_buffer_attached(globals, scene_info);
        }

        fn enforce_buffer_creation_roundtrip(&self) -> bool {
            false
        }
    }

    impl Dispatch<XdgPopup, WSurfaceId> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            proxy: &XdgPopup,
            event: xdg_popup::Event,
            data: &WSurfaceId,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            let Some(popup) = state.popups.get_mut(data) else {
                return;
            };
            // The events of the role object a popup had before it was created again.
            if popup.xdg_popup.as_ref() != Some(proxy) {
                return;
            }
            match event {
                xdg_popup::Event::Configure { x, y, width, height } => {
                    popup.pending_batch.x = x;
                    popup.pending_batch.y = y;
                    popup.pending_batch.width = width;
                    popup.pending_batch.height = height;
                }
                xdg_popup::Event::PopupDone => popup.popup_event_sink.on_popup_done(),
                // Repositioned(token) acks a previous Reposition request. We don't
                // currently surface the token to the UI side (the UI doesn't track
                // outstanding reposition requests — the geometry already arrives
                // via the matching configure event).
                _ => {}
            }
        }
    }

    impl Dispatch<XdgPositioner, ()> for WaylandWorkerState {
        fn event(_: &mut Self, _: &XdgPositioner, _: <XdgPositioner as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        }
    }

    // The listeners of the reference.

    impl Dispatch<WlSurface, WlSurfaceData> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &WlSurface,
            event: wl_surface::Event,
            data: &WlSurfaceData,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            let WlSurfaceData::Shell(id) = data else {
                return;
            };
            let WaylandWorkerState { globals, top_levels, popups, .. } = state;
            let globals = globals.as_ref();
            let Some(shell) = shell_surface_of_mut(top_levels, popups, *id) else {
                return;
            };
            let change = match event {
                wl_surface::Event::PreferredBufferScale { factor } => shell.surface.on_preferred_buffer_scale(factor, globals),
                wl_surface::Event::Enter { output } => {
                    let name = globals.and_then(|globals| globals.outputs.find_by_proxy(&output)).map(|output| output.name);
                    shell.surface.on_enter(name, globals)
                }
                wl_surface::Event::Leave { output } => {
                    let name = globals.and_then(|globals| globals.outputs.find_by_proxy(&output)).map(|output| output.name);
                    shell.surface.on_leave(name, globals)
                }
                _ => return,
            };
            shell.apply_change(change, globals);
        }
    }

    /// The frame callback of a surface.
    impl Dispatch<WlCallback, WSurfaceId> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            proxy: &WlCallback,
            event: wl_callback::Event,
            data: &WSurfaceId,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            if let wl_callback::Event::Done { .. } = event {
                match shell_surface_of_mut(&mut state.top_levels, &mut state.popups, *data) {
                    Some(shell) => shell.surface.on_frame_done(proxy),
                    // The surface is gone; the loop is woken as the listener of the reference does.
                    None => state.worker.wakeup_render_loop(),
                }
            }
        }
    }

    impl Dispatch<WlRegion, ()> for WaylandWorkerState {
        fn event(_: &mut Self, _: &WlRegion, _: <WlRegion as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        }
    }

    impl Dispatch<XdgSurface, WSurfaceId> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            proxy: &XdgSurface,
            event: xdg_surface::Event,
            data: &WSurfaceId,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            if let xdg_surface::Event::Configure { serial } = event {
                let globals = state.globals.as_ref();
                if let Some(top_level) = state.top_levels.get_mut(data) {
                    top_level.on_configure_batch_complete(serial, globals);
                } else if let Some(popup) = state.popups.get_mut(data) {
                    // A configure of the `xdg_surface` a popup had before it was created
                    // again is of an object that is gone.
                    if popup.shell.xdg_surface.as_ref() == Some(proxy) {
                        popup.on_configure_batch_complete(serial);
                    }
                }
            }
        }
    }

    impl Dispatch<XdgToplevel, WSurfaceId> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &XdgToplevel,
            event: xdg_toplevel::Event,
            data: &WSurfaceId,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            let Some(top_level) = state.top_levels.get_mut(data) else {
                return;
            };
            match event {
                xdg_toplevel::Event::ConfigureBounds { width, height } => {
                    top_level.pending_batch.bounds = Some(PixelSize::new(width, height));
                }
                xdg_toplevel::Event::Configure { width, height, states } => {
                    top_level.pending_batch.size = PixelSize::new(width, height);
                    top_level.pending_batch.states = XdgConfigureBatch::parse_states(&states);
                }
                xdg_toplevel::Event::Close => top_level.top_level_event_sink.on_close(),
                _ => {}
            }
        }
    }

    impl Dispatch<ZxdgToplevelDecorationV1, WSurfaceId> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &ZxdgToplevelDecorationV1,
            event: zxdg_toplevel_decoration_v1::Event,
            data: &WSurfaceId,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            if let zxdg_toplevel_decoration_v1::Event::Configure { mode } = event {
                let translated = match mode {
                    wayland_client::WEnum::Value(zxdg_toplevel_decoration_v1::Mode::ServerSide) => DecorationMode::ServerSide,
                    _ => DecorationMode::ClientSide,
                };
                if let Some(top_level) = state.top_levels.get_mut(data) {
                    top_level.on_decoration_configure(translated);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_scale_prefers_the_fractional_then_the_buffer_scale_then_the_outputs() {
        assert_eq!(compute_scale(None, None, []), 1.0);
        assert_eq!(compute_scale(None, None, [1, 2, 1]), 2.0);
        assert_eq!(compute_scale(None, Some(3), [1, 2]), 3.0);
        assert_eq!(compute_scale(Some(1.25), Some(3), [2]), 1.25);
        // A fractional scale below one is taken as it is.
        assert_eq!(compute_scale(Some(0.75), None, []), 0.75);
        assert!(scale_differs(1.25, 1.0));
        assert!(!scale_differs(1.0 + 1e-9, 1.0));
        assert_eq!(buffer_scale(1.0), 1);
        assert_eq!(buffer_scale(1.25), 2);
        assert_eq!(buffer_scale(0.5), 1);
    }

    #[test]
    fn the_window_geometry_excludes_the_shadow_and_stays_inside_the_surface() {
        let shadow = Thickness::new(10.0, 12.0, 14.0, 16.0);
        // Integer scale 2: a buffer of 800 by 600 is a surface of 400 by 300.
        assert_eq!(
            compute_window_geometry(shadow, PixelSize::new(800, 600), 2.0, Size::new(400.0, 300.0), false),
            Some((10, 12, 376, 272))
        );
        // An odd buffer size rounds the surface up.
        assert_eq!(
            compute_window_geometry(Thickness::default(), PixelSize::new(801, 601), 2.0, Size::new(0.0, 0.0), false),
            Some((0, 0, 401, 301))
        );
        // Fractional scaling takes the logical size.
        assert_eq!(
            compute_window_geometry(shadow, PixelSize::new(500, 375), 1.25, Size::new(400.0, 300.0), true),
            Some((10, 12, 376, 272))
        );
        // A shadow larger than the surface leaves a geometry of one pixel.
        assert_eq!(
            compute_window_geometry(Thickness::new(50.0, 50.0, 50.0, 50.0), PixelSize::new(20, 20), 1.0, Size::default(), false),
            Some((19, 19, 1, 1))
        );
        assert_eq!(
            compute_window_geometry(shadow, PixelSize::new(0, 0), 1.0, Size::default(), false),
            None
        );
    }

    #[test]
    fn size_limits_are_rounded_inwards_and_the_minimum_never_exceeds_the_maximum() {
        assert_eq!(min_max_size_request(None, None), ((0, 0), (0, 0)));
        assert_eq!(
            min_max_size_request(Some(Size::new(100.2, 50.0)), Some(Size::new(300.9, 200.0))),
            ((101, 50), (300, 200))
        );
        assert_eq!(min_max_size_request(Some(Size::new(500.0, 500.0)), Some(Size::new(300.0, 600.0))), ((300, 500), (300, 600)));
        // Without a maximum any minimum goes through.
        assert_eq!(min_max_size_request(Some(Size::new(5000.0, 0.2)), None), ((5000, 1), (0, 0)));
    }

    #[test]
    fn the_largest_automatic_size_is_the_bounds_or_the_smallest_output() {
        assert_eq!(calculate_max_size(Some(PixelSize::new(1000, 700)), [PixelSize::new(1920, 1080)]), Size::new(1000.0, 700.0));
        assert_eq!(
            calculate_max_size(Some(PixelSize::new(0, 0)), [PixelSize::new(1920, 1080), PixelSize::new(1280, 1440)]),
            Size::new(1280.0, 1080.0)
        );
        assert_eq!(calculate_max_size(None, []), Size::new(800.0, 600.0));
    }

    #[test]
    fn the_positioner_of_a_popup_is_in_the_window_geometry_of_its_parent() {
        // A parent without shadow: the anchor rectangle goes through.
        let geometry = compute_positioner_geometry(
            Size::new(200.0, 120.5),
            Thickness::default(),
            Rect::new(40.0, 30.0, 100.0, 24.0),
            Point::new(0.0, 0.0),
            Some((0, 0, 800, 600)),
        );
        assert_eq!(geometry, PositionerGeometry { size: (200, 121), anchor_rect: (40, 30, 100, 24), offset: None });

        // A parent with a shadow of 10 and 12: the origin moves; the margin of the child is not
        // part of the size; the offset is rounded.
        let geometry = compute_positioner_geometry(
            Size::new(220.0, 140.0),
            Thickness::new(10.0, 10.0, 10.0, 10.0),
            Rect::new(50.0, 42.0, 100.0, 24.0),
            Point::new(2.4, -3.6),
            Some((10, 12, 780, 576)),
        );
        assert_eq!(geometry, PositionerGeometry { size: (200, 120), anchor_rect: (40, 30, 100, 24), offset: Some((2, -4)) });
    }

    #[test]
    fn the_anchor_rectangle_is_clamped_into_the_parent_and_never_empty() {
        let parent = Some((0, 0, 800, 600));
        let none = Thickness::default();
        let size = Size::new(100.0, 100.0);
        let zero = Point::new(0.0, 0.0);
        // Over the right and bottom edge.
        assert_eq!(
            compute_positioner_geometry(size, none, Rect::new(780.0, 590.0, 100.0, 24.0), zero, parent).anchor_rect,
            (780, 590, 20, 10)
        );
        // Outside altogether: the last pixel of the parent.
        assert_eq!(
            compute_positioner_geometry(size, none, Rect::new(900.0, 700.0, 10.0, 10.0), zero, parent).anchor_rect,
            (799, 599, 1, 1)
        );
        // Before the origin.
        assert_eq!(
            compute_positioner_geometry(size, none, Rect::new(-20.0, -5.0, 10.0, 10.0), zero, parent).anchor_rect,
            (0, 0, 1, 5)
        );
        // An empty rectangle (a point, as a context menu gives) is one pixel.
        assert_eq!(
            compute_positioner_geometry(size, none, Rect::new(300.0, 200.0, 0.0, 0.0), zero, parent).anchor_rect,
            (300, 200, 1, 1)
        );
        // Without a geometry of the parent nothing is clamped, and the rectangle is not empty.
        assert_eq!(
            compute_positioner_geometry(size, none, Rect::new(900.0, -7.0, 0.0, 12.0), zero, None).anchor_rect,
            (900, -7, 1, 12)
        );
        // A size that the margin of the child eats leaves a size of one.
        assert_eq!(
            compute_positioner_geometry(Size::new(10.0, 10.0), Thickness::new(8.0, 8.0, 8.0, 8.0), Rect::default(), zero, parent).size,
            (1, 1)
        );
        // Halves round to the even number, as the reference rounds.
        assert_eq!(
            compute_positioner_geometry(size, none, Rect::new(2.5, 3.5, 4.5, 5.5), zero, parent).anchor_rect,
            (2, 4, 4, 6)
        );
    }
}
