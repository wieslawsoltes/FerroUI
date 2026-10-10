//! Pointer, touch and scroll input through the X Input extension, version
//! 2 (the port of `XI2Manager.cs`).

use crate::x11_enum_extensions::X11EnumExtensions;
use crate::x11_enums::{XEventMask, XModifierMask};
use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::xi_structs::{
    XiDeviceChangeReason, XiDeviceClass, XiDeviceEventFlags, XiDeviceType, XiEnterLeaveDetail, XiEventType,
    XiPredefinedDeviceId, XiScrollType,
};
use crate::xlib::{
    self, Atom, XGenericEventCookie, XIClassInfo, XIDeviceChangedEventData, XIDeviceEventData, XIDeviceInfo,
    XIEnterLeaveEventData, XIEventData, XIScrollClassInfo, XIValuatorClassInfo, XID,
};
use ferroui_base::input::raw::{
    IRawInputEventArgs, RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType, RawPointerPoint,
    RawTouchEventArgs,
};
use ferroui_base::input::{IInputDevice, IInputRoot, RawInputModifiers};
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Vector};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

const DEFAULT_EVENT_TYPES: [XiEventType; 5] = [
    XiEventType::XI_Motion,
    XiEventType::XI_ButtonPress,
    XiEventType::XI_ButtonRelease,
    XiEventType::XI_Leave,
    XiEventType::XI_Enter,
];

const MULTI_TOUCH_EVENT_TYPES: [XiEventType; 3] =
    [XiEventType::XI_TouchBegin, XiEventType::XI_TouchUpdate, XiEventType::XI_TouchEnd];

/// A window that receives input of the X Input extension (`IXI2Client`).
pub trait IXI2Client {
    fn is_enabled(&self) -> bool;
    /// The input root of the window.
    ///
    /// # Panics
    /// Panics when the window has none yet.
    fn input_root(&self) -> Rc<dyn IInputRoot>;
    fn schedule_xi2_input(&self, args: Rc<dyn IRawInputEventArgs>);
    fn mouse_device(&self) -> Rc<dyn IInputDevice>;
    fn pen_device(&self) -> Rc<dyn IInputDevice>;
    fn touch_device(&self) -> Rc<dyn IInputDevice>;
}

/// The labels of the valuators the backend reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KnownValuatorAtoms {
    pub touch_major: Atom,
    pub touch_minor: Atom,
    pub pressure: Atom,
    pub pressure_pen: Atom,
    pub abs_tilt_x: Atom,
    pub abs_tilt_y: Atom,
}

impl KnownValuatorAtoms {
    fn intern(x11: &X11Info) -> Self {
        let atoms = x11.atoms();
        // ABS_MT_TOUCH_MAJOR ABS_MT_TOUCH_MINOR
        // https://www.kernel.org/doc/html/latest/input/multi-touch-protocol.html
        Self {
            touch_major: atoms.get_atom("Abs MT Touch Major"),
            touch_minor: atoms.get_atom("Abs MT Touch Minor"),
            pressure: atoms.get_atom("Abs MT Pressure"),
            pressure_pen: atoms.get_atom("Abs Pressure"),
            abs_tilt_x: atoms.get_atom("Abs Tilt X"),
            abs_tilt_y: atoms.get_atom("Abs Tilt Y"),
        }
    }
}

/// The master pointer: its valuators and scrollers (`DeviceInfo` and
/// `PointerDeviceInfo` of the reference, which has no other device).
#[derive(Clone, Debug, Default)]
pub(crate) struct PointerDeviceInfo {
    pub id: i32,
    pub valuators: Vec<XIValuatorClassInfo>,
    pub scrollers: Vec<XIScrollClassInfo>,

    current_slave_name: Option<String>,
    current_slave_is_eraser: bool,
    known: KnownValuatorAtoms,

    pub pressure_xi_valuator_class_info: Option<XIValuatorClassInfo>,
    pub touch_major_xi_valuator_class_info: Option<XIValuatorClassInfo>,
    pub touch_minor_xi_valuator_class_info: Option<XIValuatorClassInfo>,
    pub tilt_x_xi_valuator_class_info: Option<XIValuatorClassInfo>,
    pub tilt_y_xi_valuator_class_info: Option<XIValuatorClassInfo>,
}

impl PointerDeviceInfo {
    pub(crate) fn new(id: i32, classes: &[XIClassInfo], known: KnownValuatorAtoms) -> Self {
        let mut this = Self { id, known, ..Self::default() };
        this.update_core(classes);
        this.update_known_valuator();
        this
    }

    fn update_core(&mut self, classes: &[XIClassInfo]) {
        let mut valuators = Vec::new();
        let mut scrollers = Vec::new();
        for class in classes {
            match class {
                XIClassInfo::Valuator(valuator) => valuators.push(*valuator),
                XIClassInfo::Scroll(scroller) => scrollers.push(*scroller),
                XIClassInfo::Other => {}
            }
        }

        self.valuators = valuators;
        self.scrollers = scrollers;
    }

    pub(crate) fn update_valuators(&mut self, valuators: &[(i32, f64)]) {
        for (key, value) in valuators {
            if *key >= 0 && self.valuators.len() > *key as usize {
                self.valuators[*key as usize].value = *value;
            }
        }
    }

    fn update_known_valuator(&mut self) {
        self.pressure_xi_valuator_class_info = None;
        self.touch_major_xi_valuator_class_info = None;
        self.touch_minor_xi_valuator_class_info = None;

        for xi_valuator_class_info in &self.valuators {
            let label = xi_valuator_class_info.label;
            if label == self.known.pressure || label == self.known.pressure_pen {
                self.pressure_xi_valuator_class_info = Some(*xi_valuator_class_info);
            } else if label == self.known.touch_major {
                self.touch_major_xi_valuator_class_info = Some(*xi_valuator_class_info);
            } else if label == self.known.touch_minor {
                self.touch_minor_xi_valuator_class_info = Some(*xi_valuator_class_info);
            } else if label == self.known.abs_tilt_x {
                self.tilt_x_xi_valuator_class_info = Some(*xi_valuator_class_info);
            } else if label == self.known.abs_tilt_y {
                self.tilt_y_xi_valuator_class_info = Some(*xi_valuator_class_info);
            }
        }
    }

    /// Takes the classes of a device changed event. `slave` is the device
    /// the pointer switched to, as `(id, all devices)`, when it switched.
    pub(crate) fn update(&mut self, classes: &[XIClassInfo], slave: Option<(i32, &[XIDeviceInfo])>) {
        self.update_core(classes);

        if let Some((slave_id, devices)) = slave {
            self.current_slave_name = None;
            self.current_slave_is_eraser = false;

            if let Some(device) = devices.iter().find(|device| device.device_id == slave_id) {
                self.current_slave_is_eraser = device.name.to_ascii_lowercase().contains("eraser");
                self.current_slave_name = Some(device.name.clone());
            }
        }

        self.update_known_valuator();
    }

    pub(crate) fn has_pressure_valuator(&self) -> bool {
        self.pressure_xi_valuator_class_info.is_some()
    }

    pub(crate) fn is_eraser(&self) -> bool {
        self.current_slave_is_eraser
    }

    // Not read by the backend, as in the reference.
    #[allow(dead_code)]
    pub(crate) fn name(&self) -> Option<&str> {
        self.current_slave_name.as_deref()
    }

    // Not read by the backend, as in the reference.
    #[allow(dead_code)]
    pub(crate) fn has_scroll(&self, ev: &ParsedDeviceEvent) -> bool {
        ev.valuators.iter().any(|(key, _)| self.scrollers.iter().any(|s| s.number == *key))
    }

    pub(crate) fn has_motion(&self, ev: &ParsedDeviceEvent) -> bool {
        ev.valuators.iter().any(|(key, _)| self.scrollers.iter().all(|s| s.number != *key))
    }
}

/// What a client is told for a device event: the raw input events, in
/// order.
pub(crate) enum XI2Input {
    Touch { type_: RawPointerEventType, point: RawPointerPoint, modifiers: RawInputModifiers, id: i64 },
    Wheel { pen: bool, position: Point, delta: Vector, modifiers: RawInputModifiers },
    Pointer { pen: bool, type_: RawPointerEventType, point: RawPointerPoint, modifiers: RawInputModifiers },
}

/// The X Input extension for the windows of the platform.
pub struct XI2Manager {
    x11: Rc<X11Info>,
    multitouch: bool,
    clients: RefCell<HashMap<XID, Weak<dyn IXI2Client>>>,
    pointer_device: RefCell<PointerDeviceInfo>,
    platform: Weak<FerroX11Platform>,
}

impl XI2Manager {
    pub fn try_create(platform: &Rc<FerroX11Platform>) -> Option<Rc<XI2Manager>> {
        let x11 = platform.info();

        let devices = xlib::xi_query_device(x11.display(), XiPredefinedDeviceId::XIAllMasterDevices as i32);

        let pointer_device = devices
            .iter()
            .find(|device| device.use_ == XiDeviceType::XIMasterPointer as i32)
            .map(|device| PointerDeviceInfo::new(device.device_id, &device.classes, KnownValuatorAtoms::intern(x11)))?;

        let status = xlib::xi_select_events(
            x11.display(),
            x11.root_window(),
            &[(pointer_device.id, vec![XiEventType::XI_DeviceChanged.0])],
        );

        if status != 0 {
            return None;
        }

        Some(Rc::new(XI2Manager {
            x11: x11.clone(),
            multitouch: platform.options().enable_multi_touch.unwrap_or(true),
            clients: RefCell::new(HashMap::new()),
            pointer_device: RefCell::new(pointer_device),
            platform: Rc::downgrade(platform),
        }))
    }

    pub fn add_window(&self, xid: XID, window: Weak<dyn IXI2Client>) -> XEventMask {
        self.clients.borrow_mut().insert(xid, window);

        let mut events: Vec<i32> = DEFAULT_EVENT_TYPES.iter().map(|event| event.0).collect();

        if self.multitouch {
            events.extend(MULTI_TOUCH_EVENT_TYPES.iter().map(|event| event.0));
        }

        let device = self.pointer_device.borrow().id;
        xlib::xi_select_events(self.x11.display(), xid, &[(device, events)]);

        // We are taking over mouse input handling from here
        XEventMask::POINTER_MOTION_MASK
            | XEventMask::BUTTON_MOTION_MASK
            | XEventMask::BUTTON1_MOTION_MASK
            | XEventMask::BUTTON2_MOTION_MASK
            | XEventMask::BUTTON3_MOTION_MASK
            | XEventMask::BUTTON4_MOTION_MASK
            | XEventMask::BUTTON5_MOTION_MASK
            | XEventMask::BUTTON_PRESS_MASK
            | XEventMask::BUTTON_RELEASE_MASK
            | XEventMask::LEAVE_WINDOW_MASK
            | XEventMask::ENTER_WINDOW_MASK
    }

    pub fn on_window_destroyed(&self, xid: XID) {
        self.clients.borrow_mut().remove(&xid);
    }

    fn client(&self, xid: XID) -> Option<Rc<dyn IXI2Client>> {
        self.clients.borrow().get(&xid).and_then(Weak::upgrade)
    }

    /// Handles a generic event of the extension.
    pub fn on_event(&self, cookie: &XGenericEventCookie) {
        let Some(event) = xlib::copy_xi_event(cookie) else {
            return;
        };
        match event {
            XIEventData::DeviceChanged(changed) => self.on_device_changed(&changed),
            XIEventData::Device(dev) => {
                if let Some(client) = self.client(dev.event_window) {
                    self.on_device_event(&*client, &ParsedDeviceEvent::new(&dev));
                }
            }
            XIEventData::EnterLeave(rev) => {
                if let Some(client) = self.client(rev.event_window) {
                    self.on_enter_leave_event(&*client, &rev);
                }
            }
            XIEventData::Other(_) => {}
        }
    }

    fn on_device_changed(&self, changed: &XIDeviceChangedEventData) {
        if changed.reason == XiDeviceChangeReason::XISlaveSwitch as i32 {
            let devices = xlib::xi_query_device(self.x11.display(), XiPredefinedDeviceId::XIAllDevices as i32);
            self.pointer_device.borrow_mut().update(&changed.classes, Some((changed.source_id, &devices)));
        } else {
            self.pointer_device.borrow_mut().update(&changed.classes, None);
        }
    }

    fn on_enter_leave_event(&self, client: &dyn IXI2Client, ev: &XIEnterLeaveEventData) {
        let detail = ev.detail;
        if detail != XiEnterLeaveDetail::XINotifyNonlinearVirtual as i32
            && detail != XiEnterLeaveDetail::XINotifyNonlinear as i32
            && detail != XiEnterLeaveDetail::XINotifyVirtual as i32
            && detail != XiEnterLeaveDetail::XINotifyAncestor as i32
        {
            return;
        }

        let buttons = ParsedDeviceEvent::parse_button_state(&ev.buttons);
        if !buttons.is_empty() {
            return;
        }

        if ev.evtype == XiEventType::XI_Leave.0 {
            {
                let mut pointer_device = self.pointer_device.borrow_mut();
                let numbers: Vec<i32> = pointer_device.scrollers.iter().map(|scroller| scroller.number).collect();
                for number in numbers {
                    // The reference indexes the valuators by the number of
                    // the scroller; a number beyond them is passed over
                    // here, where the reference fails.
                    if let Some(valuator) = usize::try_from(number).ok().and_then(|n| pointer_device.valuators.get_mut(n))
                    {
                        valuator.value = 0.0;
                    }
                }
            }

            client.schedule_xi2_input(Rc::new(RawPointerEventArgs::new(
                client.mouse_device(),
                ev.time as u64,
                client.input_root(),
                RawPointerEventType::LeaveWindow,
                Point::new(ev.event_x, ev.event_y),
                buttons,
            )));
        } else if ev.evtype == XiEventType::XI_Enter.0 {
            client.schedule_xi2_input(Rc::new(RawPointerEventArgs::new(
                client.mouse_device(),
                ev.time as u64,
                client.input_root(),
                RawPointerEventType::Move,
                Point::new(ev.event_x, ev.event_y),
                XModifierMask::from_bits_retain(ev.mods_effective).to_raw_input_modifiers(),
            )));
        }
    }

    fn on_device_event(&self, client: &dyn IXI2Client, ev: &ParsedDeviceEvent) {
        let screen_bounds = |root_position: Point| -> Option<PixelRect> {
            let pixel_point = PixelPoint::new(root_position.x as i32, root_position.y as i32);
            let platform = self.platform.upgrade()?;
            platform.screens().screen_from_point(pixel_point).map(|screen| screen.bounds())
        };
        let inputs = {
            let mut pointer_device = self.pointer_device.borrow_mut();
            translate_device_event(&mut pointer_device, self.multitouch, client.is_enabled(), ev, &screen_bounds)
        };

        for input in inputs {
            let args: Rc<dyn IRawInputEventArgs> = match input {
                XI2Input::Touch { type_, point, modifiers, id } => Rc::new(RawTouchEventArgs::with_point(
                    client.touch_device(),
                    ev.timestamp,
                    client.input_root(),
                    type_,
                    point,
                    modifiers,
                    id,
                )),
                XI2Input::Wheel { pen, position, delta, modifiers } => Rc::new(RawMouseWheelEventArgs::new(
                    if pen { client.pen_device() } else { client.mouse_device() },
                    ev.timestamp,
                    client.input_root(),
                    position,
                    delta,
                    modifiers,
                )),
                XI2Input::Pointer { pen, type_, point, modifiers } => Rc::new(RawPointerEventArgs::with_point(
                    if pen { client.pen_device() } else { client.mouse_device() },
                    ev.timestamp,
                    client.input_root(),
                    type_,
                    point,
                    modifiers,
                )),
            };
            client.schedule_xi2_input(args);
        }
    }
}

/// Translates a device event into raw input (`OnDeviceEvent`), updating
/// the valuators of the pointer device. `screen_bounds` gives the bounds
/// of the screen at a root position.
pub(crate) fn translate_device_event(
    pointer_device: &mut PointerDeviceInfo,
    multitouch: bool,
    client_is_enabled: bool,
    ev: &ParsedDeviceEvent,
    screen_bounds: &dyn Fn(Point) -> Option<PixelRect>,
) -> Vec<XI2Input> {
    let mut inputs = Vec::new();
    let valuator = |number: i32| ev.valuators.iter().find(|(key, _)| *key == number).map(|(_, value)| *value);

    if ev.type_ == XiEventType::XI_TouchBegin
        || ev.type_ == XiEventType::XI_TouchUpdate
        || ev.type_ == XiEventType::XI_TouchEnd
    {
        let type_ = if ev.type_ == XiEventType::XI_TouchBegin {
            RawPointerEventType::TouchBegin
        } else if ev.type_ == XiEventType::XI_TouchUpdate {
            RawPointerEventType::TouchUpdate
        } else {
            RawPointerEventType::TouchEnd
        };

        let mut raw_pointer_point = point_at(ev.position);

        if let Some(valuator_class_info) = pointer_device.pressure_xi_valuator_class_info {
            if let Some(pressure_value) = valuator(valuator_class_info.number) {
                // In our API we use range from 0.0 to 1.0.
                let pressure =
                    (pressure_value - valuator_class_info.min) / (valuator_class_info.max - valuator_class_info.min);
                raw_pointer_point.pressure = pressure as f32;
            }
        }

        if let Some(touch_major_info) = pointer_device.touch_major_xi_valuator_class_info {
            let mut touch_major: Option<f64> = None;
            let mut touch_minor: Option<f64> = None;
            let mut bounds = PixelRect::default();
            if let Some(touch_major_value) = valuator(touch_major_info.number) {
                if let Some(screen_bounds_from_point) = screen_bounds(ev.root_position) {
                    bounds = screen_bounds_from_point;

                    // As https://www.kernel.org/doc/html/latest/input/multi-touch-protocol.html says, using `screenBounds.Width` is not accurate enough.
                    touch_major = Some(
                        (touch_major_value - touch_major_info.min) / (touch_major_info.max - touch_major_info.min)
                            * bounds.width as f64,
                    );
                }
            }

            if let Some(touch_major) = touch_major {
                if let Some(touch_minor_info) = pointer_device.touch_minor_xi_valuator_class_info {
                    if let Some(touch_minor_value) = valuator(touch_minor_info.number) {
                        touch_minor = Some(
                            (touch_minor_value - touch_minor_info.min) / (touch_minor_info.max - touch_minor_info.min)
                                * bounds.height as f64,
                        );
                    }
                }

                let touch_minor = touch_minor.unwrap_or(touch_major);

                let center = ev.position;
                let left_x = center.x - touch_major / 2.0;
                let top_y = center.y - touch_minor / 2.0;

                raw_pointer_point.set_contact_rect(Rect::new(left_x, top_y, touch_major, touch_minor));
            }
        }

        inputs.push(XI2Input::Touch { type_, point: raw_pointer_point, modifiers: ev.modifiers, id: ev.detail as i64 });
        return inputs;
    }

    if !client_is_enabled || (multitouch && ev.emulated) {
        return inputs;
    }

    let mut event_modifiers = ev.modifiers;
    if pointer_device.is_eraser() {
        event_modifiers |= RawInputModifiers::PEN_ERASER;
    }

    let pen = pointer_device.has_pressure_valuator();

    if ev.type_ == XiEventType::XI_Motion {
        let mut scroll_delta = Vector::default();
        let mut raw_pointer_point = point_at(ev.position);

        for v in &ev.valuators {
            for scroller in &pointer_device.scrollers {
                if scroller.number == v.0 {
                    // The reference indexes the valuators by the number
                    // of the scroller; a number beyond them counts as a
                    // value that was reset.
                    let old = usize::try_from(scroller.number)
                        .ok()
                        .and_then(|n| pointer_device.valuators.get(n))
                        .map_or(0.0, |valuator| valuator.value);
                    // Value was zero after reset, ignore the event and use it as a reference next time
                    if old == 0.0 {
                        continue;
                    }
                    let diff = (old - v.1) / scroller.increment;
                    if scroller.scroll_type == XiScrollType::Horizontal as i32 {
                        scroll_delta = scroll_delta.with_x(scroll_delta.x + diff);
                    } else {
                        scroll_delta = scroll_delta.with_y(scroll_delta.y + diff);
                    }
                }
            }

            set_pen_specific_values(pointer_device, *v, &mut raw_pointer_point);
        }

        if scroll_delta != Vector::default() {
            inputs.push(XI2Input::Wheel { pen, position: ev.position, delta: scroll_delta, modifiers: event_modifiers });
        }
        if pointer_device.has_motion(ev) {
            inputs.push(XI2Input::Pointer {
                pen,
                type_: RawPointerEventType::Move,
                point: raw_pointer_point,
                modifiers: event_modifiers,
            });
        }
    }

    if ev.type_ == XiEventType::XI_ButtonPress && ev.button >= 4 && ev.button <= 7 && !ev.emulated {
        let scroll_delta = match ev.button {
            4 => Some(Vector::new(0.0, 1.0)),
            5 => Some(Vector::new(0.0, -1.0)),
            6 => Some(Vector::new(1.0, 0.0)),
            7 => Some(Vector::new(-1.0, 0.0)),
            _ => None,
        };

        if let Some(scroll_delta) = scroll_delta {
            inputs.push(XI2Input::Wheel {
                pen: false,
                position: ev.position,
                delta: scroll_delta,
                modifiers: event_modifiers,
            });
        }
    }

    if ev.type_ == XiEventType::XI_ButtonPress || ev.type_ == XiEventType::XI_ButtonRelease {
        let down = ev.type_ == XiEventType::XI_ButtonPress;
        let type_ = match ev.button {
            1 => Some(if down { RawPointerEventType::LeftButtonDown } else { RawPointerEventType::LeftButtonUp }),
            2 => Some(if down { RawPointerEventType::MiddleButtonDown } else { RawPointerEventType::MiddleButtonUp }),
            3 => Some(if down { RawPointerEventType::RightButtonDown } else { RawPointerEventType::RightButtonUp }),
            8 => Some(if down { RawPointerEventType::XButton1Down } else { RawPointerEventType::XButton1Up }),
            9 => Some(if down { RawPointerEventType::XButton2Down } else { RawPointerEventType::XButton2Up }),
            _ => None,
        };

        if let Some(type_) = type_ {
            let mut pointer_point = point_at(ev.position);

            for ev_valuator in &ev.valuators {
                set_pen_specific_values(pointer_device, *ev_valuator, &mut pointer_point);
            }

            inputs.push(XI2Input::Pointer { pen, type_, point: pointer_point, modifiers: event_modifiers });
        }
    }

    pointer_device.update_valuators(&ev.valuators);
    inputs
}

/// A pointer point at a position, with the defaults of the other values.
fn point_at(position: Point) -> RawPointerPoint {
    let mut point = RawPointerPoint::new();
    point.position = position;
    point
}

fn set_pen_specific_values(pointer_device: &PointerDeviceInfo, item: (i32, f64), raw_pointer_point: &mut RawPointerPoint) {
    if let Some(valuator_class_info) = pointer_device.pressure_xi_valuator_class_info {
        if item.0 == valuator_class_info.number {
            let pressure = (item.1 - valuator_class_info.min) / (valuator_class_info.max - valuator_class_info.min);
            raw_pointer_point.pressure = pressure as f32;
        }
    }

    if let Some(tilt_x_valuator_class_info) = pointer_device.tilt_x_xi_valuator_class_info {
        if item.0 == tilt_x_valuator_class_info.number {
            raw_pointer_point.x_tilt = item.1 as f32;
        }
    }

    if let Some(tilt_y_valuator_class_info) = pointer_device.tilt_y_xi_valuator_class_info {
        if item.0 == tilt_y_valuator_class_info.number {
            raw_pointer_point.y_tilt = item.1 as f32;
        }
    }
}

/// A device event with its state read (`ParsedDeviceEvent`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedDeviceEvent {
    pub type_: XiEventType,
    pub modifiers: RawInputModifiers,
    pub timestamp: u64,
    pub position: Point,
    pub root_position: Point,
    pub button: i32,
    pub detail: i32,
    pub emulated: bool,
    pub valuators: Vec<(i32, f64)>,
}

impl ParsedDeviceEvent {
    /// The pressed buttons of a button mask.
    pub(crate) fn parse_button_state(buttons: &[u8]) -> RawInputModifiers {
        let len = buttons.len();
        let mut rv = RawInputModifiers::empty();
        if len > 0 {
            if xlib::xi_mask_is_set(buttons, 1) {
                rv |= RawInputModifiers::LEFT_MOUSE_BUTTON;
            }
            if xlib::xi_mask_is_set(buttons, 2) {
                rv |= RawInputModifiers::MIDDLE_MOUSE_BUTTON;
            }
            if xlib::xi_mask_is_set(buttons, 3) {
                rv |= RawInputModifiers::RIGHT_MOUSE_BUTTON;
            }
            if len > 1 {
                if xlib::xi_mask_is_set(buttons, 8) {
                    rv |= RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON;
                }
                if xlib::xi_mask_is_set(buttons, 9) {
                    rv |= RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON;
                }
            }
        }

        rv
    }

    pub(crate) fn new(ev: &XIDeviceEventData) -> Self {
        let type_ = XiEventType(ev.evtype);
        let state = XModifierMask::from_bits_retain(ev.mods_effective);
        let mut modifiers = RawInputModifiers::empty();
        if state.contains(XModifierMask::SHIFT_MASK) {
            modifiers |= RawInputModifiers::SHIFT;
        }
        if state.contains(XModifierMask::CONTROL_MASK) {
            modifiers |= RawInputModifiers::CONTROL;
        }
        if state.contains(XModifierMask::MOD1_MASK) {
            modifiers |= RawInputModifiers::ALT;
        }
        if state.contains(XModifierMask::MOD4_MASK) {
            modifiers |= RawInputModifiers::META;
        }

        modifiers |= Self::parse_button_state(&ev.buttons);

        let button = if type_ == XiEventType::XI_ButtonPress || type_ == XiEventType::XI_ButtonRelease {
            ev.detail
        } else {
            0
        };
        Self {
            type_,
            modifiers,
            timestamp: ev.time as u64,
            position: Point::new(ev.event_x, ev.event_y),
            root_position: Point::new(ev.root_x, ev.root_y),
            button,
            detail: ev.detail,
            emulated: XiDeviceEventFlags::from_bits_retain(ev.flags).contains(XiDeviceEventFlags::XI_POINTER_EMULATED),
            valuators: ev.valuators.clone(),
        }
    }
}

// The class kinds the reference names when it reads the classes of a
// device; the copies of `crate::xlib` are made by the same values.
const _: () = {
    assert!(XiDeviceClass::XIValuatorClass as i32 == xlib::xi2::XIValuatorClass);
    assert!(XiDeviceClass::XIScrollClass as i32 == xlib::xi2::XIScrollClass);
};

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. The
    // device events are built as the X Input library delivers them (the C
    // structure with its masks and values), from the values of events
    // recorded with `xinput test-xi2` on an X.Org server.
    use super::*;
    use crate::xlib::xi2;

    const PRESSURE: Atom = 501;
    const TILT_X: Atom = 502;
    const TILT_Y: Atom = 503;
    const TOUCH_MAJOR: Atom = 504;
    const TOUCH_MINOR: Atom = 505;

    fn known() -> KnownValuatorAtoms {
        KnownValuatorAtoms {
            touch_major: TOUCH_MAJOR,
            touch_minor: TOUCH_MINOR,
            pressure: 506,
            pressure_pen: PRESSURE,
            abs_tilt_x: TILT_X,
            abs_tilt_y: TILT_Y,
        }
    }

    fn valuator(number: i32, label: Atom, min: f64, max: f64) -> XIClassInfo {
        XIClassInfo::Valuator(XIValuatorClassInfo { number, label, min, max, ..XIValuatorClassInfo::default() })
    }

    fn scroller(number: i32, scroll_type: i32, increment: f64) -> XIClassInfo {
        XIClassInfo::Scroll(XIScrollClassInfo { number, scroll_type, increment, ..XIScrollClassInfo::default() })
    }

    /// A mouse of the server: x, y, horizontal and vertical scroll.
    fn mouse() -> PointerDeviceInfo {
        PointerDeviceInfo::new(
            2,
            &[
                valuator(0, 0, -1.0, -1.0),
                valuator(1, 0, -1.0, -1.0),
                valuator(2, 0, -1.0, -1.0),
                valuator(3, 0, -1.0, -1.0),
                scroller(2, XiScrollType::Horizontal as i32, 120.0),
                scroller(3, XiScrollType::Vertical as i32, 120.0),
                XIClassInfo::Other,
            ],
            known(),
        )
    }

    /// Builds the C structure of a device event and copies it as the
    /// event dispatcher does.
    fn device_event(
        evtype: i32,
        detail: i32,
        flags: i32,
        mods: i32,
        buttons: &[u8],
        valuator_mask: &[u8],
        values: &[f64],
    ) -> XIDeviceEventData {
        let mut buttons = buttons.to_vec();
        let mut valuator_mask = valuator_mask.to_vec();
        let mut values = values.to_vec();
        // SAFETY: an all-zero event is a valid value of the C structure.
        let mut ev: xi2::XIDeviceEvent = unsafe { std::mem::zeroed() };
        ev.evtype = evtype;
        ev.time = 123_456;
        ev.detail = detail;
        ev.event = 0x40_0001;
        ev.root_x = 310.5;
        ev.root_y = 220.25;
        ev.event_x = 110.5;
        ev.event_y = 20.25;
        ev.flags = flags;
        ev.buttons.mask_len = buttons.len() as i32;
        ev.buttons.mask = buttons.as_mut_ptr();
        ev.valuators.mask_len = valuator_mask.len() as i32;
        ev.valuators.mask = valuator_mask.as_mut_ptr();
        ev.valuators.values = values.as_mut_ptr();
        ev.mods.effective = mods;
        // SAFETY: the masks and the values point at the vectors above,
        // with the lengths the event states and a value per set bit.
        unsafe { xlib::copy_xi_device_event(&ev) }
    }

    fn parsed(
        evtype: i32,
        detail: i32,
        flags: i32,
        mods: i32,
        buttons: &[u8],
        valuator_mask: &[u8],
        values: &[f64],
    ) -> ParsedDeviceEvent {
        ParsedDeviceEvent::new(&device_event(evtype, detail, flags, mods, buttons, valuator_mask, values))
    }

    fn no_screen(_: Point) -> Option<PixelRect> {
        None
    }

    #[test]
    fn a_device_event_is_copied_with_its_masks_and_values() {
        // Motion with x and y (bits 0 and 1), button one down (bit 1 of the button mask),
        // shift and control (1 | 4).
        let data = device_event(xi2::XI_Motion, 0, 0, 5, &[0b0000_0010, 0, 0, 0], &[0b0000_0011, 0], &[1500.25, 800.75]);
        assert_eq!(data.evtype, xi2::XI_Motion);
        assert_eq!(data.time, 123_456);
        assert_eq!(data.event_window, 0x40_0001);
        assert_eq!(data.buttons, vec![2, 0, 0, 0]);
        assert_eq!(data.valuators, vec![(0, 1500.25), (1, 800.75)]);
        assert_eq!((data.event_x, data.event_y, data.root_x, data.root_y), (110.5, 20.25, 310.5, 220.25));

        let ev = ParsedDeviceEvent::new(&data);
        assert_eq!(ev.type_, XiEventType::XI_Motion);
        assert_eq!(
            ev.modifiers,
            RawInputModifiers::SHIFT | RawInputModifiers::CONTROL | RawInputModifiers::LEFT_MOUSE_BUTTON
        );
        assert_eq!(ev.timestamp, 123_456);
        assert_eq!(ev.position, Point::new(110.5, 20.25));
        assert_eq!(ev.root_position, Point::new(310.5, 220.25));
        assert_eq!(ev.button, 0);
        assert!(!ev.emulated);
    }

    #[test]
    fn valuators_are_numbered_by_the_bits_of_the_mask() {
        // Bits 2 and 9 are set: two values, for the valuators 2 and 9.
        assert_eq!(xlib::xi_valuators(&[0b0000_0100, 0b0000_0010], &[7.0, 9.0]), vec![(2, 7.0), (9, 9.0)]);
        // No mask, no valuators; fewer values than bits ends the list.
        assert!(xlib::xi_valuators(&[], &[1.0]).is_empty());
        assert_eq!(xlib::xi_valuators(&[0b0000_0111], &[1.0, 2.0]), vec![(0, 1.0), (1, 2.0)]);
        assert!(xlib::xi_mask_is_set(&[0, 1], 8));
        assert!(!xlib::xi_mask_is_set(&[0, 1], 16));
        assert!(!xlib::xi_mask_is_set(&[0xff], -1));
    }

    #[test]
    fn the_button_state_reads_the_first_two_bytes() {
        let state = ParsedDeviceEvent::parse_button_state;
        assert_eq!(state(&[]), RawInputModifiers::empty());
        assert_eq!(state(&[0b0000_1110]), RawInputModifiers::LEFT_MOUSE_BUTTON
            | RawInputModifiers::MIDDLE_MOUSE_BUTTON
            | RawInputModifiers::RIGHT_MOUSE_BUTTON);
        // The extended buttons are bits 8 and 9: only with a second byte.
        assert_eq!(
            state(&[0, 0b0000_0011]),
            RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON | RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON
        );
        // The scroll buttons (4 to 7) are not a state.
        assert_eq!(state(&[0b1111_0000]), RawInputModifiers::empty());
    }

    #[test]
    fn a_button_press_and_release_are_pointer_events() {
        let mut device = mouse();
        let press = parsed(xi2::XI_ButtonPress, 1, 0, 0, &[0], &[0], &[]);
        assert_eq!(press.button, 1);
        let inputs = translate_device_event(&mut device, true, true, &press, &no_screen);
        assert_eq!(inputs.len(), 1);
        match &inputs[0] {
            XI2Input::Pointer { pen, type_, point, modifiers } => {
                assert!(!pen);
                assert_eq!(*type_, RawPointerEventType::LeftButtonDown);
                assert_eq!(point.position, Point::new(110.5, 20.25));
                assert_eq!(*modifiers, RawInputModifiers::empty());
            }
            _ => panic!("a pointer event was expected"),
        }

        for (button, down, up) in [
            (2, RawPointerEventType::MiddleButtonDown, RawPointerEventType::MiddleButtonUp),
            (3, RawPointerEventType::RightButtonDown, RawPointerEventType::RightButtonUp),
            (8, RawPointerEventType::XButton1Down, RawPointerEventType::XButton1Up),
            (9, RawPointerEventType::XButton2Down, RawPointerEventType::XButton2Up),
        ] {
            for (evtype, expected) in [(xi2::XI_ButtonPress, down), (xi2::XI_ButtonRelease, up)] {
                let ev = parsed(evtype, button, 0, 0, &[0], &[0], &[]);
                let inputs = translate_device_event(&mut device, true, true, &ev, &no_screen);
                assert!(matches!(&inputs[..], [XI2Input::Pointer { type_, .. }] if *type_ == expected));
            }
        }

        // A button the framework has no event for.
        let ev = parsed(xi2::XI_ButtonPress, 10, 0, 0, &[0], &[0], &[]);
        assert!(translate_device_event(&mut device, true, true, &ev, &no_screen).is_empty());
    }

    #[test]
    fn the_scroll_buttons_are_wheel_events_unless_emulated() {
        let mut device = mouse();
        for (button, delta) in [
            (4, Vector::new(0.0, 1.0)),
            (5, Vector::new(0.0, -1.0)),
            (6, Vector::new(1.0, 0.0)),
            (7, Vector::new(-1.0, 0.0)),
        ] {
            let ev = parsed(xi2::XI_ButtonPress, button, 0, 0, &[0], &[0], &[]);
            let inputs = translate_device_event(&mut device, false, true, &ev, &no_screen);
            assert!(matches!(&inputs[..], [XI2Input::Wheel { delta: d, pen: false, .. }] if *d == delta));
        }

        // The server emulates the buttons from the scroll valuators: the valuators are used.
        let ev = parsed(xi2::XI_ButtonPress, 5, xi2::XIPointerEmulated, 0, &[0], &[0], &[]);
        assert!(ev.emulated);
        assert!(translate_device_event(&mut device, false, true, &ev, &no_screen).is_empty());
        // With multi-touch every emulated event is dropped.
        assert!(translate_device_event(&mut device, true, true, &ev, &no_screen).is_empty());
    }

    #[test]
    fn scroll_valuators_give_deltas_after_a_reference_value() {
        let mut device = mouse();
        // The first value after a reset is only remembered.
        let first = parsed(xi2::XI_Motion, 0, 0, 0, &[0], &[0b0000_1000], &[1200.0]);
        assert!(translate_device_event(&mut device, true, true, &first, &no_screen).is_empty());
        assert_eq!(device.valuators[3].value, 1200.0);

        // One detent down: the value grows by the increment.
        let second = parsed(xi2::XI_Motion, 0, 0, 0, &[0], &[0b0000_1000], &[1320.0]);
        let inputs = translate_device_event(&mut device, true, true, &second, &no_screen);
        assert!(matches!(&inputs[..], [XI2Input::Wheel { delta, .. }] if *delta == Vector::new(0.0, -1.0)));

        // Half a detent to the right on the horizontal scroller, with a pointer motion in
        // the same event (x and the horizontal scroller: bits 0 and 2).
        device.valuators[2].value = 600.0;
        let third = parsed(xi2::XI_Motion, 0, 0, 0, &[0], &[0b0000_0101], &[10.0, 540.0]);
        let inputs = translate_device_event(&mut device, true, true, &third, &no_screen);
        assert_eq!(inputs.len(), 2);
        assert!(matches!(&inputs[0], XI2Input::Wheel { delta, .. } if *delta == Vector::new(0.5, 0.0)));
        assert!(matches!(&inputs[1], XI2Input::Pointer { type_: RawPointerEventType::Move, .. }));
        assert_eq!(device.valuators[0].value, 10.0);
        assert_eq!(device.valuators[2].value, 540.0);
    }

    #[test]
    fn a_plain_motion_is_a_move_and_a_disabled_client_gets_nothing() {
        let mut device = mouse();
        let ev = parsed(xi2::XI_Motion, 0, 0, 8, &[0], &[0b0000_0011], &[5.0, 6.0]);
        assert!(device.has_motion(&ev) && !device.has_scroll(&ev));
        let inputs = translate_device_event(&mut device, true, true, &ev, &no_screen);
        assert!(matches!(
            &inputs[..],
            [XI2Input::Pointer { type_: RawPointerEventType::Move, modifiers, pen: false, .. }]
                if *modifiers == RawInputModifiers::ALT
        ));
        assert!(translate_device_event(&mut device, true, false, &ev, &no_screen).is_empty());
    }

    #[test]
    fn a_pen_reports_pressure_and_tilt() {
        let mut device = PointerDeviceInfo::new(
            2,
            &[
                valuator(0, 0, 0.0, 30000.0),
                valuator(1, 0, 0.0, 20000.0),
                valuator(2, PRESSURE, 0.0, 2048.0),
                valuator(3, TILT_X, -64.0, 63.0),
                valuator(4, TILT_Y, -64.0, 63.0),
            ],
            known(),
        );
        assert!(device.has_pressure_valuator());
        let ev = parsed(xi2::XI_Motion, 0, 0, 0, &[0], &[0b0001_1111], &[100.0, 200.0, 512.0, 12.0, -7.0]);
        let inputs = translate_device_event(&mut device, true, true, &ev, &no_screen);
        match &inputs[..] {
            [XI2Input::Pointer { pen: true, type_: RawPointerEventType::Move, point, .. }] => {
                assert_eq!(point.pressure, 0.25);
                assert_eq!(point.x_tilt, 12.0);
                assert_eq!(point.y_tilt, -7.0);
            }
            _ => panic!("a pen move was expected"),
        }

        // The pointer switches to the eraser end of the pen.
        let devices = [
            XIDeviceInfo { device_id: 9, name: "Wacom Pen stylus".into(), use_: 3, attachment: 2, enabled: true, classes: vec![] },
            XIDeviceInfo { device_id: 10, name: "Wacom Pen Eraser".into(), use_: 3, attachment: 2, enabled: true, classes: vec![] },
        ];
        let classes = [valuator(0, 0, 0.0, 1.0), valuator(1, 0, 0.0, 1.0), valuator(2, PRESSURE, 0.0, 1024.0)];
        device.update(&classes, Some((10, &devices)));
        assert!(device.is_eraser());
        assert_eq!(device.name(), Some("Wacom Pen Eraser"));
        let press = parsed(xi2::XI_ButtonPress, 1, 0, 0, &[0], &[0b0000_0100], &[1024.0]);
        let inputs = translate_device_event(&mut device, true, true, &press, &no_screen);
        match &inputs[..] {
            [XI2Input::Pointer { pen: true, type_: RawPointerEventType::LeftButtonDown, point, modifiers }] => {
                assert_eq!(point.pressure, 1.0);
                assert!(modifiers.contains(RawInputModifiers::PEN_ERASER));
            }
            _ => panic!("a pen press was expected"),
        }

        // Back to the tip; a switch to a device that is not listed clears the name.
        device.update(&classes, Some((9, &devices)));
        assert!(!device.is_eraser());
        device.update(&classes, Some((77, &devices)));
        assert_eq!(device.name(), None);
        // The tilt valuators of before are kept, as in the reference, which only resets
        // the pressure and the touch sizes.
        assert!(device.tilt_x_xi_valuator_class_info.is_some());
    }

    #[test]
    fn touches_carry_their_identifier_pressure_and_contact() {
        let mut device = PointerDeviceInfo::new(
            2,
            &[
                valuator(0, 0, 0.0, 4095.0),
                valuator(1, 0, 0.0, 4095.0),
                valuator(2, TOUCH_MAJOR, 0.0, 255.0),
                valuator(3, TOUCH_MINOR, 0.0, 255.0),
                valuator(4, 506, 0.0, 255.0),
            ],
            known(),
        );
        let screen = |_: Point| Some(PixelRect::new(0, 0, 1920, 1080));
        let begin = parsed(xi2::XI_TouchBegin, 41, 0, 0, &[0], &[0b0001_1100], &[25.5, 51.0, 127.5]);
        let inputs = translate_device_event(&mut device, true, false, &begin, &screen);
        match &inputs[..] {
            [XI2Input::Touch { type_: RawPointerEventType::TouchBegin, point, id: 41, .. }] => {
                assert_eq!(point.pressure, 0.5);
                // A tenth of the width and a fifth of the height of the screen, centred.
                assert_eq!(point.contact_rect(), Rect::new(110.5 - 96.0, 20.25 - 108.0, 192.0, 216.0));
            }
            _ => panic!("a touch was expected"),
        }

        // Without the minor axis the contact is as high as it is wide; without a screen
        // there is no contact at all.
        let update = parsed(xi2::XI_TouchUpdate, 41, 0, 0, &[0], &[0b0000_0100], &[25.5]);
        let inputs = translate_device_event(&mut device, true, true, &update, &screen);
        assert!(matches!(&inputs[..], [XI2Input::Touch { type_: RawPointerEventType::TouchUpdate, point, .. }]
            if point.contact_rect().size() == ferroui_base::Size::new(192.0, 192.0)));
        let inputs = translate_device_event(&mut device, true, true, &update, &no_screen);
        assert!(matches!(&inputs[..], [XI2Input::Touch { point, .. }]
            if point.contact_rect().size() == ferroui_base::Size::default()));

        let end = parsed(xi2::XI_TouchEnd, 41, 0, 0, &[0], &[0], &[]);
        let inputs = translate_device_event(&mut device, true, true, &end, &no_screen);
        assert!(matches!(&inputs[..], [XI2Input::Touch { type_: RawPointerEventType::TouchEnd, id: 41, .. }]));
    }

    #[test]
    fn the_event_types_are_those_of_the_extension() {
        assert_eq!(XiEventType::XI_Motion.0, xi2::XI_Motion);
        assert_eq!(XiEventType::XI_ButtonPress.0, xi2::XI_ButtonPress);
        assert_eq!(XiEventType::XI_ButtonRelease.0, xi2::XI_ButtonRelease);
        assert_eq!(XiEventType::XI_Enter.0, xi2::XI_Enter);
        assert_eq!(XiEventType::XI_Leave.0, xi2::XI_Leave);
        assert_eq!(XiEventType::XI_TouchBegin.0, xi2::XI_TouchBegin);
        assert_eq!(XiEventType::XI_TouchEnd.0, xi2::XI_TouchEnd);
        assert_eq!(XiEventType::XI_DeviceChanged.0, xi2::XI_DeviceChanged);
        assert_eq!(XiDeviceEventFlags::XI_POINTER_EMULATED.bits(), xi2::XIPointerEmulated);
        assert_eq!(XiDeviceType::XIMasterPointer as i32, xi2::XIMasterPointer);
        assert_eq!(XiPredefinedDeviceId::XIAllMasterDevices as i32, xi2::XIAllMasterDevices);
        assert_eq!(XiDeviceChangeReason::XISlaveSwitch as i32, xi2::XISlaveSwitch);
        assert_eq!(XiScrollType::Horizontal as i32, xi2::XIScrollTypeHorizontal);
        assert_eq!(XiEnterLeaveDetail::XINotifyAncestor as i32, xi2::XINotifyAncestor);
        assert_eq!(XiEnterLeaveDetail::XINotifyNonlinearVirtual as i32, xi2::XINotifyNonlinearVirtual);
    }
}
