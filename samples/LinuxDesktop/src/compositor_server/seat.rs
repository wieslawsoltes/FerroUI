//! Route FerroUI input to the native seat in logical output coordinates.
use super::{state::State, Input};
use smithay::input::keyboard::{KeyboardTarget, Layout, SerializedMods};
use smithay::{
    backend::input::{Axis, AxisSource, ButtonState, KeyState},
    delegate_data_device, delegate_seat,
    desktop::{find_popup_root_surface, PopupKeyboardGrab, PopupKind, PopupPointerGrab},
    input::{
        keyboard::FilterResult,
        pointer::{AxisFrame, ButtonEvent, CursorImageStatus, Focus, MotionEvent},
        Seat, SeatHandler, SeatState,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
            Resource,
        },
    },
    utils::{Logical, Point, Rectangle, Serial, SERIAL_COUNTER},
    wayland::{
        compositor::{with_states, SurfaceAttributes},
        selection::{
            data_device::{
                set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
                ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::{PopupSurface, SurfaceCachedState, ToplevelSurface},
    },
};
use std::{os::fd::OwnedFd, sync::Arc};

#[derive(Default)]
pub(super) struct HostKeyboard {
    keymap: Option<Arc<str>>,
    valid: bool,
    last_report: Option<(WlSurface, SerializedMods)>,
}

pub(super) struct Drag {
    id: u64,
    origin: Point<f64, Logical>,
    frame: Rectangle<i32, Logical>,
    edge: Option<ResizeEdge>,
    button: u32,
}

impl State {
    fn hit(&self, point: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>, u64)> {
        self.hits.iter().rev().find_map(|hit| {
            let v = &hit.view;
            let loc = Point::from((v.x as f64, v.y as f64));
            let local = point - loc;
            if local.x < 0.
                || local.y < 0.
                || local.x >= v.width as f64
                || local.y >= v.height as f64
            {
                return None;
            }
            let accepts = with_states(&hit.wl, |s| {
                s.cached_state
                    .get::<SurfaceAttributes>()
                    .current()
                    .input_region
                    .as_ref()
                    .is_none_or(|region| region.contains(local.to_i32_floor()))
            });
            accepts.then(|| (hit.wl.clone(), loc, v.window))
        })
    }

    /// Retarget an unmoving pointer when a new surface covers it, a popup is
    /// dismissed, or a workspace disappears. Keep ordinary commits input-silent.
    pub(super) fn refresh_pointer(&mut self, time: u32) {
        let Some(at) = self.pointer_at else { return };
        let next = self.hit(at).map(|(wl, origin, _)| (wl, origin));
        if next == self.pointer_target {
            return;
        }
        self.pointer_target = next.clone();
        if let Some(pointer) = self.seat.get_pointer() {
            pointer.motion(
                self,
                next,
                &MotionEvent {
                    location: at,
                    serial: SERIAL_COUNTER.next_serial(),
                    time,
                },
            );
            pointer.frame(self);
        }
    }

    pub(super) fn seat_event(&mut self, event: Input) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        match event {
            Input::Motion { x, y, time } if x.is_finite() && y.is_finite() => {
                let point: Point<f64, Logical> = (x, y).into();
                self.pointer_at = Some(point);
                if self.drag.is_some() {
                    self.apply_drag(point);
                }
                let target = self.hit(point).map(|(wl, origin, _)| (wl, origin));
                self.pointer_target = target.clone();
                pointer.motion(
                    self,
                    target,
                    &MotionEvent {
                        location: point,
                        serial: SERIAL_COUNTER.next_serial(),
                        time,
                    },
                );
                pointer.frame(self);
            }
            Input::Leave { time } => {
                let point = self.pointer_at.take().unwrap_or_default();
                self.pointer_target = None;
                pointer.motion(
                    self,
                    None,
                    &MotionEvent {
                        location: point,
                        serial: SERIAL_COUNTER.next_serial(),
                        time,
                    },
                );
                pointer.frame(self);
            }
            Input::Button {
                code,
                pressed,
                time,
            } => {
                self.refresh_pointer(time);
                if pressed && !pointer.is_grabbed() {
                    if let Some((_, _, id)) = self.pointer_at.and_then(|p| self.hit(p)) {
                        self.focus(Some(id));
                    }
                }
                pointer.button(
                    self,
                    &ButtonEvent {
                        serial: SERIAL_COUNTER.next_serial(),
                        time,
                        button: code,
                        state: if pressed {
                            ButtonState::Pressed
                        } else {
                            ButtonState::Released
                        },
                    },
                );
                pointer.frame(self);
                if !pressed && self.drag.as_ref().is_some_and(|d| d.button == code) {
                    self.drag = None;
                }
            }
            Input::Axis {
                horizontal,
                vertical,
                time,
            } => {
                let mut frame = AxisFrame::new(time).source(AxisSource::Wheel);
                if horizontal != 0. {
                    frame = frame.value(Axis::Horizontal, horizontal);
                }
                if vertical != 0. {
                    frame = frame.value(Axis::Vertical, vertical);
                }
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            Input::WaylandKey {
                event,
                pressed,
                time,
            } => {
                if event.repeat || event.code > 0x2ff {
                    return;
                }
                if let Some(map) = event.keymap {
                    if self
                        .host_keyboard
                        .keymap
                        .as_ref()
                        .is_none_or(|last| !Arc::ptr_eq(last, &map))
                    {
                        self.host_keyboard.valid = keyboard
                            .set_keymap_from_string(self, map.to_string())
                            .is_ok();
                        self.host_keyboard.keymap = Some(map);
                        self.host_keyboard.last_report = None;
                    }
                    if !self.host_keyboard.valid {
                        return;
                    }
                }
                let mods = event.modifiers;
                let wire = SerializedMods {
                    depressed: mods.depressed,
                    latched: mods.latched,
                    locked: mods.locked,
                    layout_effective: mods.group,
                };
                if keyboard.modifier_state().serialized.layout_effective != mods.group {
                    keyboard.with_xkb_state(self, |mut ctx| ctx.set_layout(Layout(mods.group)));
                }
                // The host may already hold a modifier when keyboard focus enters this
                // output. Its mask is authoritative at this wire event; those earlier
                // presses need not have passed through the nested keyboard device.
                if let Some(target) = keyboard.current_focus() {
                    if self
                        .host_keyboard
                        .last_report
                        .as_ref()
                        .is_none_or(|(last, state)| last != &target || *state != wire)
                    {
                        let mut report = keyboard.modifier_state();
                        report.serialized = wire;
                        let seat = self.seat.clone();
                        target.modifiers(&seat, self, report, SERIAL_COUNTER.next_serial());
                        self.host_keyboard.last_report = Some((target, wire));
                    }
                }
                let before = keyboard.modifier_state().serialized;
                keyboard.input::<(), _>(
                    self,
                    (event.code + 8).into(),
                    if pressed {
                        KeyState::Pressed
                    } else {
                        KeyState::Released
                    },
                    SERIAL_COUNTER.next_serial(),
                    time,
                    |_, _, _| FilterResult::Forward,
                );
                if before != keyboard.modifier_state().serialized {
                    self.host_keyboard.last_report = None;
                }
            }
            Input::Key {
                code,
                pressed,
                time,
            } => {
                if code > 0 && code <= 0x2ff {
                    keyboard.input::<(), _>(
                        self,
                        (code + 8).into(),
                        if pressed {
                            KeyState::Pressed
                        } else {
                            KeyState::Released
                        },
                        SERIAL_COUNTER.next_serial(),
                        time,
                        |_, _, _| FilterResult::Forward,
                    );
                }
            }
            Input::KeyboardLeave { time } => {
                self.host_keyboard.last_report = None;
                for code in keyboard.pressed_keys() {
                    keyboard.input::<(), _>(
                        self,
                        code,
                        KeyState::Released,
                        SERIAL_COUNTER.next_serial(),
                        time,
                        |_, _, _| FilterResult::Forward,
                    );
                }
                self.focus(None);
            }
            _ => {}
        }
    }

    pub(super) fn grab_popup(&mut self, popup: PopupSurface, resource: WlSeat, serial: Serial) {
        let Some(seat) = Seat::from_resource(&resource) else {
            return;
        };
        let popup = PopupKind::Xdg(popup);
        let Ok(root) = find_popup_root_surface(&popup) else {
            return;
        };
        if self
            .focused
            .and_then(|id| self.windows.iter().find(|w| w.id == id))
            .is_none_or(|w| w.role.wl_surface() != &root)
        {
            return;
        }
        let Ok(grab) = self.popups.grab_popup(root, popup, &seat, serial) else {
            return;
        };
        if let Some(pointer) = seat.get_pointer() {
            pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
        }
        if let Some(keyboard) = seat.get_keyboard() {
            keyboard.set_focus(self, grab.current_grab(), serial);
            keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
        }
    }

    pub(super) fn begin_drag(
        &mut self,
        role: ToplevelSurface,
        resource: WlSeat,
        serial: Serial,
        edge: Option<ResizeEdge>,
    ) {
        let Some(seat) = Seat::<Self>::from_resource(&resource) else {
            return;
        };
        let Some(pointer) = seat.get_pointer() else {
            return;
        };
        let Some(start) = pointer
            .grab_start_data()
            .filter(|_| pointer.has_grab(serial))
        else {
            return;
        };
        let Some((target, _)) = start.focus else {
            return;
        };
        let Some(window) = self.windows.iter().find(|w| w.role == role) else {
            return;
        };
        if !self
            .hits
            .iter()
            .any(|h| h.wl == target && h.view.window == window.id)
        {
            return;
        }
        let id = window.id;
        let initial = Rectangle::new(window.location, self.geometry(role.wl_surface()).size);
        let mut frame = window.saved.unwrap_or(initial);
        if window.maximized || window.fullscreen {
            if edge.is_none() {
                let relative =
                    (start.location - initial.loc.to_f64()).x / f64::from(initial.size.w.max(1));
                frame.loc.x = (start.location.x - relative.clamp(0., 1.) * f64::from(frame.size.w))
                    .round() as i32;
                frame.loc.y = start.location.y.round() as i32 - 12;
            }
            self.normal_frame(id, Some(frame.loc), Some(frame.size));
        }
        self.drag = Some(Drag {
            id,
            origin: start.location,
            frame,
            edge,
            button: start.button,
        });
    }
    fn apply_drag(&mut self, at: Point<f64, Logical>) {
        let Some(d) = &self.drag else { return };
        let Some(w) = self.windows.iter_mut().find(|w| w.id == d.id) else {
            return;
        };
        let delta: Point<i32, Logical> = (at - d.origin).to_i32_round();
        let mut rect = d.frame;
        if let Some(edge) = d.edge {
            let left = matches!(
                edge,
                ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft
            );
            let right = matches!(
                edge,
                ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight
            );
            let top = matches!(
                edge,
                ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight
            );
            let bottom = matches!(
                edge,
                ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight
            );
            let (min, max) = with_states(w.role.wl_surface(), |s| {
                let mut cache = s.cached_state.get::<SurfaceCachedState>();
                let st = cache.current();
                (st.min_size, st.max_size)
            });
            let limit = |size: i32, min: i32, max: i32| {
                size.clamp(
                    min.max(1),
                    if max > 0 {
                        max.max(min).max(1)
                    } else {
                        i32::MAX
                    },
                )
            };
            if left || right {
                rect.size.w = limit(
                    rect.size.w + if left { -delta.x } else { delta.x },
                    min.w,
                    max.w,
                );
            }
            if top || bottom {
                rect.size.h = limit(
                    rect.size.h + if top { -delta.y } else { delta.y },
                    min.h,
                    max.h,
                );
            }
            if left {
                rect.loc.x += d.frame.size.w - rect.size.w;
            }
            if top {
                rect.loc.y += d.frame.size.h - rect.size.h;
            }
            w.role.with_pending_state(|s| s.size = Some(rect.size));
            w.role.send_pending_configure();
        } else {
            rect.loc += delta;
        }
        w.location = rect.loc;
        self.dirty = true;
    }
}
impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seats
    }
    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        set_data_device_focus(&self.display, seat, focused.and_then(Resource::client));
    }
    fn cursor_image(&mut self, _: &Seat<Self>, _: CursorImageStatus) {}
}
impl SelectionHandler for State {
    type SelectionUserData = ();
}
impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.selection
    }
}
impl ClientDndGrabHandler for State {}
impl ServerDndGrabHandler for State {
    fn send(&mut self, _: String, _: OwnedFd, _: Seat<Self>) {}
}
delegate_seat!(State);
delegate_data_device!(State);
