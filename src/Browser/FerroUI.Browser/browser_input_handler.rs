use crate::browser_data_transfer_helper::{IReadableDataItems, JsReadableDataItems};
use crate::browser_drag_data_transfer::BrowserDragDataTransfer;
use crate::browser_input_pane::BrowserInputPane;
use crate::browser_mouse_device::{BrowserMouseDevice, ContainerPointerCapture, IPointerCapture};
use crate::browser_text_input_method::BrowserTextInputMethod;
use crate::interop::{input_helper, JsObject};
use crate::key_interop;
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::input::raw::{
    IDragDropDevice, IRawInputEventArgs, RawDragEvent, RawDragEventType, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
    RawPointerEventType, RawPointerPoint, RawTextInputEventArgs, RawTouchEventArgs,
};
use ferroui_base::input::{
    DragDropEffects, IDataTransfer, IInputDevice, IInputRoot, IntermediatePoints, KeyDeviceType, MouseDevice, PenDevice, RawInputModifiers,
    TouchDevice,
};
use ferroui_base::{FerroLocator, LocatorExtensions, Point, Size, Vector};
use std::cell::{Cell, LazyCell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Instant;

/// What the input handler needs from the top-level it belongs to; replaced
/// by a recorder in the tests.
pub(crate) trait IInputTopLevel {
    /// Calls the input callback of the top-level, if it has one.
    fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>);

    /// The client size of the top-level: what the wheel scrolls by when
    /// the page reports its deltas in pages.
    fn page_size(&self) -> Size;
}

/// The calls the input handler makes into the page; replaced by a recorder
/// in the tests.
pub(crate) trait IInputPage {
    fn subscribe_input_events(&self, top_level_id: i32);
    fn unsubscribe_input_events(&self);
    fn get_coalesced_events(&self, pointer_event: &JsObject) -> Vec<f64>;
}

struct InputPage {
    container: JsObject,
    subscription: RefCell<Option<JsObject>>,
}

impl IInputPage for InputPage {
    fn subscribe_input_events(&self, top_level_id: i32) {
        let subscription = input_helper::subscribe_input_events(&self.container, top_level_id);
        let old = self.subscription.replace(Some(subscription));
        drop(old);
    }

    fn unsubscribe_input_events(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            input_helper::unsubscribe_input_events(&subscription);
        }
    }

    fn get_coalesced_events(&self, pointer_event: &JsObject) -> Vec<f64> {
        input_helper::get_coalesced_events(pointer_event)
    }
}

/// `deltaMode` of a wheel event: the deltas are pixels (the unit of every
/// mode that is not one of the two below).
#[cfg(test)]
const DOM_DELTA_PIXEL: i32 = 0;
/// `deltaMode` of a wheel event: the deltas are lines.
const DOM_DELTA_LINE: i32 = 1;
/// `deltaMode` of a wheel event: the deltas are pages.
const DOM_DELTA_PAGE: i32 = 2;

/// The pixels the wheel delta of the framework stands for: the scroll
/// content presenter scrolls 50 units per delta of 1, and the reference
/// implementation of this backend divides the pixel deltas by 50.
const PIXELS_PER_WHEEL_DELTA: f64 = 50.0;

/// The lines of a wheel event that make one unit of the wheel delta of the
/// framework.
///
/// A delta of 1 is one notch of a wheel: the Windows backend of the
/// reference implementation divides the wheel message by 120
/// (`WHEEL_DELTA`, one notch), the X11 backend reports 1 per wheel click.
/// One notch scrolls three lines, the default of the systems that count
/// wheel scrolling in lines (and what browsers that use this unit report
/// per notch), so three lines are one delta.
const LINES_PER_WHEEL_DELTA: f64 = 3.0;

/// The numbers the page sends per coalesced point.
const ITEMS_PER_POINT: usize = 6;

/// Turns the events of the element of a top-level into raw input events
/// and delivers them to the top-level.
pub struct BrowserInputHandler {
    weak_self: Weak<BrowserInputHandler>,
    top_level_impl: Weak<dyn IInputTopLevel>,
    page: Box<dyn IInputPage>,
    container: Rc<dyn IPointerCapture>,
    sw: Instant,
    touch_device: Rc<TouchDevice>,
    pen_device: Rc<PenDevice>,
    wheel_mouse_device: Rc<MouseDevice>,
    mouse_devices: RefCell<Vec<BrowserMouseDevice>>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    text_input_method: Rc<BrowserTextInputMethod>,
    input_pane: Rc<BrowserInputPane>,
}

impl BrowserInputHandler {
    /// Creates the input handler of a top-level and subscribes to the input
    /// events of `container`.
    ///
    /// # Panics
    /// Panics when `container` or `input_element` is null or undefined.
    pub(crate) fn new(
        top_level_impl: Weak<dyn IInputTopLevel>,
        container: JsObject,
        input_element: JsObject,
        top_level_id: i32,
    ) -> Rc<Self> {
        if container.is_null() || container.is_undefined() {
            panic!("Value cannot be null. (Parameter 'container')");
        }

        let page = InputPage { container: container.clone(), subscription: RefCell::new(None) };
        let capture = Rc::new(ContainerPointerCapture::new(container.clone()));

        Self::create(top_level_impl, Box::new(page), capture, top_level_id, move |this| {
            BrowserTextInputMethod::new(this, container, input_element)
        })
    }

    fn create(
        top_level_impl: Weak<dyn IInputTopLevel>,
        page: Box<dyn IInputPage>,
        container: Rc<dyn IPointerCapture>,
        top_level_id: i32,
        text_input_method: impl FnOnce(Weak<BrowserInputHandler>) -> Rc<BrowserTextInputMethod>,
    ) -> Rc<Self> {
        // Input is dispatched synchronously on the UI thread: the original groups raw events
        // (`RawEventGrouper`) only when it runs with a managed dispatcher on another thread.
        let this = Rc::new_cyclic(|weak_self: &Weak<BrowserInputHandler>| Self {
            weak_self: weak_self.clone(),
            top_level_impl,
            page,
            container,
            sw: Instant::now(),
            touch_device: TouchDevice::new(),
            pen_device: PenDevice::new(false),
            wheel_mouse_device: MouseDevice::new(),
            mouse_devices: RefCell::new(Vec::new()),
            input_root: RefCell::new(None),
            text_input_method: text_input_method(weak_self.clone()),
            input_pane: BrowserInputPane::new(),
        });

        this.page.subscribe_input_events(top_level_id);

        this
    }

    #[cfg(test)]
    fn with_page(
        top_level_impl: Weak<dyn IInputTopLevel>,
        page: Box<dyn IInputPage>,
        container: Rc<dyn IPointerCapture>,
        text_input_page: Box<dyn crate::browser_text_input_method::ITextInputPage>,
        top_level_id: i32,
    ) -> Rc<Self> {
        Self::create(top_level_impl, page, container, top_level_id, move |this| {
            BrowserTextInputMethod::with_page(this, text_input_page)
        })
    }

    /// The text input method of the top-level.
    pub fn text_input_method(&self) -> &Rc<BrowserTextInputMethod> {
        &self.text_input_method
    }

    /// The input pane of the top-level.
    pub fn input_pane(&self) -> &Rc<BrowserInputPane> {
        &self.input_pane
    }

    /// The milliseconds since the handler was created: the time stamp of
    /// the raw input events.
    pub fn timestamp(&self) -> u64 {
        self.sw.elapsed().as_millis() as u64
    }

    pub(crate) fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        let old = self.input_root.replace(Some(input_root));
        drop(old);
    }

    /// The input root the events are raised for.
    pub(crate) fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    /// Ends the subscription to the input events of the element.
    // Not in the original, which never unsubscribes.
    pub(crate) fn dispose(&self) {
        self.page.unsubscribe_input_events();
    }

    fn create_raw_pointer(
        offset_x: f64,
        offset_y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        twist: f64,
    ) -> RawPointerPoint {
        let mut point = RawPointerPoint::new();
        point.position = Point::new(offset_x, offset_y);
        point.pressure = pressure as f32;
        point.x_tilt = tilt_x as f32;
        point.y_tilt = tilt_y as f32;
        point.twist = twist as f32;
        point
    }

    /// Whether the modifiers of a key event make it a command rather than a
    /// character: Meta, or Control without Alt.
    fn is_command_chord(modifier: i32) -> bool {
        let modifiers = Self::to_raw_input_modifiers(modifier);
        modifiers.contains(RawInputModifiers::META)
            || (modifiers.contains(RawInputModifiers::CONTROL) && !modifiers.contains(RawInputModifiers::ALT))
    }

    fn to_raw_input_modifiers(modifier: i32) -> RawInputModifiers {
        RawInputModifiers::from_bits_retain(modifier)
    }

    fn pointer_move_type(pointer_type: &str) -> RawPointerEventType {
        match pointer_type {
            "touch" => RawPointerEventType::TouchUpdate,
            _ => RawPointerEventType::Move,
        }
    }

    fn pointer_down_type(pointer_type: &str, buttons: i32) -> RawPointerEventType {
        match pointer_type {
            "touch" => RawPointerEventType::TouchBegin,
            _ => match buttons {
                0 => RawPointerEventType::LeftButtonDown,
                1 => RawPointerEventType::MiddleButtonDown,
                2 => RawPointerEventType::RightButtonDown,
                3 => RawPointerEventType::XButton1Down,
                4 => RawPointerEventType::XButton2Down,
                5 => RawPointerEventType::XButton1Down, // should be pen eraser button,
                _ => RawPointerEventType::Move,
            },
        }
    }

    fn pointer_up_type(pointer_type: &str, buttons: i32) -> RawPointerEventType {
        match pointer_type {
            "touch" => RawPointerEventType::TouchEnd,
            _ => match buttons {
                0 => RawPointerEventType::LeftButtonUp,
                1 => RawPointerEventType::MiddleButtonUp,
                2 => RawPointerEventType::RightButtonUp,
                3 => RawPointerEventType::XButton1Up,
                4 => RawPointerEventType::XButton2Up,
                5 => RawPointerEventType::XButton1Up, // should be pen eraser button,
                _ => RawPointerEventType::Move,
            },
        }
    }

    /// The points of the numbers the page sends for the coalesced events of
    /// a pointer move, without the last one.
    // Differs from the original, whose loop steps the index by the numbers per point while it
    // bounds it by the count of points and so decodes only about one point in six: here every
    // point but the last is decoded.
    fn decode_coalesced_events(points_props: &[f64]) -> Vec<RawPointerPoint> {
        let points_count = points_props.len() / ITEMS_PER_POINT;

        // Skip the last one, as it is already processed point.
        points_props
            .chunks_exact(ITEMS_PER_POINT)
            .take(points_count.saturating_sub(1))
            .map(|props| Self::create_raw_pointer(props[0], props[1], props[2], props[3], props[4], props[5]))
            .collect()
    }

    /// The wheel delta of the framework for the deltas of a wheel event of
    /// the page.
    // Differs from the original, which divides by 50 whatever the unit of the deltas is.
    fn wheel_delta(delta_x: f64, delta_y: f64, delta_mode: i32, page_size: Size) -> Vector {
        match delta_mode {
            // Lines are counted in notches (see `LINES_PER_WHEEL_DELTA`).
            DOM_DELTA_LINE => Vector::new(-(delta_x / LINES_PER_WHEEL_DELTA), -(delta_y / LINES_PER_WHEEL_DELTA)),
            // A page is the size of the view, in the pixels a delta stands for.
            DOM_DELTA_PAGE => Vector::new(
                -(delta_x * page_size.width / PIXELS_PER_WHEEL_DELTA),
                -(delta_y * page_size.height / PIXELS_PER_WHEEL_DELTA),
            ),
            // Pixels, as the original; anything else a browser might invent is taken for pixels.
            _ => Vector::new(-(delta_x / PIXELS_PER_WHEEL_DELTA), -(delta_y / PIXELS_PER_WHEEL_DELTA)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn on_pointer_move(
        &self,
        pointer_type: &str,
        pointer_id: i64,
        offset_x: f64,
        offset_y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        twist: f64,
        modifier: i32,
        args_obj: JsObject,
    ) -> bool {
        let point = Self::create_raw_pointer(offset_x, offset_y, pressure, tilt_x, tilt_y, twist);
        let type_ = Self::pointer_move_type(pointer_type);

        // The event of the page is shared with the lazily computed points, so that it can be
        // released after the event has been processed whether or not the points were asked for.
        let args_obj = Rc::new(Cell::new(Some(args_obj)));

        let coalesced_events: IntermediatePoints = {
            let args_obj = args_obj.clone();
            let this = self.weak_self.clone();
            let get_points: Box<dyn FnOnce() -> Option<Vec<RawPointerPoint>>> = Box::new(move || {
                let args_obj = args_obj.take();
                let (Some(this), Some(args_obj)) = (this.upgrade(), args_obj) else {
                    return Some(Vec::new());
                };

                // To minimize interop usage, we resolve all points properties in a single call.
                let points_props = this.page.get_coalesced_events(&args_obj);
                Some(Self::decode_coalesced_events(&points_props))
            });
            Rc::new(LazyCell::new(get_points))
        };

        let handled = self.raw_pointer_event(
            type_,
            pointer_type,
            point,
            Self::to_raw_input_modifiers(modifier),
            pointer_id,
            Some(coalesced_events),
        );

        // Release the handle of the event after processing it.
        // The intermediate points are only expected to be accessed synchronously during event processing.
        let args_obj = args_obj.take();
        drop(args_obj);

        handled
    }

    #[allow(clippy::too_many_arguments)]
    pub fn on_pointer_down(
        &self,
        pointer_type: &str,
        pointer_id: i64,
        buttons: i32,
        offset_x: f64,
        offset_y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        twist: f64,
        modifier: i32,
    ) -> bool {
        let type_ = Self::pointer_down_type(pointer_type, buttons);

        let point = Self::create_raw_pointer(offset_x, offset_y, pressure, tilt_x, tilt_y, twist);
        self.raw_pointer_event(type_, pointer_type, point, Self::to_raw_input_modifiers(modifier), pointer_id, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn on_pointer_up(
        &self,
        pointer_type: &str,
        pointer_id: i64,
        buttons: i32,
        offset_x: f64,
        offset_y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        twist: f64,
        modifier: i32,
    ) -> bool {
        let type_ = Self::pointer_up_type(pointer_type, buttons);

        let point = Self::create_raw_pointer(offset_x, offset_y, pressure, tilt_x, tilt_y, twist);
        self.raw_pointer_event(type_, pointer_type, point, Self::to_raw_input_modifiers(modifier), pointer_id, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn on_pointer_cancel(
        &self,
        pointer_type: &str,
        pointer_id: i64,
        offset_x: f64,
        offset_y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        twist: f64,
        modifier: i32,
    ) -> bool {
        if pointer_type == "touch" {
            let point = Self::create_raw_pointer(offset_x, offset_y, pressure, tilt_x, tilt_y, twist);
            self.raw_pointer_event(
                RawPointerEventType::TouchCancel,
                pointer_type,
                point,
                Self::to_raw_input_modifiers(modifier),
                pointer_id,
                None,
            );
        }

        false
    }

    /// The wheel turned; `delta_mode` is the unit of the deltas
    /// (`WheelEvent.deltaMode`).
    pub fn on_wheel(
        &self,
        offset_x: f64,
        offset_y: f64,
        delta_x: f64,
        delta_y: f64,
        delta_mode: i32,
        modifier: i32,
    ) -> bool {
        let page_size = if delta_mode == DOM_DELTA_PAGE {
            self.top_level_impl.upgrade().map_or(Size::new(0.0, 0.0), |top_level| top_level.page_size())
        } else {
            Size::new(0.0, 0.0)
        };

        self.raw_mouse_wheel_event(
            Point::new(offset_x, offset_y),
            Self::wheel_delta(delta_x, delta_y, delta_mode, page_size),
            Self::to_raw_input_modifiers(modifier),
        )
    }

    /// A drag operation of the page entered, moved over, left or dropped on
    /// the element of the top-level. `data_transfer` is the data transfer of
    /// the event and `items` its items as readable data items.
    pub fn on_drag_event(
        &self,
        type_: &str,
        offset_x: f64,
        offset_y: f64,
        modifiers: i32,
        data_transfer: &JsObject,
        items: JsObject,
    ) -> bool {
        let effect_allowed = input_helper::get_effect_allowed(data_transfer);
        let (handled, drop_effect) = self.on_drag_event_core(
            type_,
            offset_x,
            offset_y,
            modifiers,
            effect_allowed.as_deref().unwrap_or("none"),
            JsReadableDataItems::new(items),
        );
        if let Some(drop_effect) = drop_effect {
            input_helper::set_drop_effect(data_transfer, &Self::drop_effect_name(drop_effect));
        }
        handled
    }

    /// See [`on_drag_event`](Self::on_drag_event): whether the event was
    /// handled, and the effect to show when the event was raised.
    pub(crate) fn on_drag_event_core(
        &self,
        type_: &str,
        offset_x: f64,
        offset_y: f64,
        modifiers: i32,
        effect_allowed_str: &str,
        items: Rc<dyn IReadableDataItems>,
    ) -> (bool, Option<DragDropEffects>) {
        let event_type = match type_ {
            "dragenter" => RawDragEventType::DragEnter,
            "dragover" => RawDragEventType::DragOver,
            "dragleave" => RawDragEventType::DragLeave,
            "drop" => RawDragEventType::Drop,
            _ => return (false, None),
        };

        // The original imports its storage module here, so that a dropped file can be read. The
        // storage module and the storage files are not ported yet; a dropped file has no value
        // (see BrowserDataTransferHelper).

        let position = Point::new(offset_x, offset_y);

        let effect_allowed_str = effect_allowed_str.to_ascii_lowercase();
        let mut effect_allowed = DragDropEffects::NONE;

        if effect_allowed_str.contains("copy") {
            effect_allowed |= DragDropEffects::COPY;
        }

        if effect_allowed_str.contains("link") {
            effect_allowed |= DragDropEffects::LINK;
        }

        if effect_allowed_str.contains("move") {
            effect_allowed |= DragDropEffects::MOVE;
        }

        if effect_allowed_str == "all" || effect_allowed_str == "uninitialized" {
            effect_allowed |= DragDropEffects::MOVE | DragDropEffects::COPY | DragDropEffects::LINK;
        }

        if effect_allowed == DragDropEffects::NONE {
            return (false, None);
        }

        let data_transfer: Rc<dyn IDataTransfer> = BrowserDragDataTransfer::new(items);
        let Some(drop_effect) = self.raw_drag_event(
            event_type,
            position,
            Self::to_raw_input_modifiers(modifiers),
            data_transfer,
            effect_allowed,
        ) else {
            return (false, None);
        };

        // Note, due to complications of JS interop, we ignore this return value.
        // And instead assume, that event is handled for any "drop" and "drag-over" stages.
        let handled = matches!(event_type, RawDragEventType::Drop | RawDragEventType::DragOver)
            && drop_effect != DragDropEffects::NONE;
        (handled, Some(drop_effect))
    }

    /// The name of drag effects as the original writes them to the page:
    /// the name of the enumeration value in lower case, so that a
    /// combination of effects (`"copy, move"`) is not a value the page
    /// accepts and leaves the effect unchanged.
    fn drop_effect_name(effects: DragDropEffects) -> String {
        if effects == DragDropEffects::NONE {
            return "none".to_string();
        }

        let mut names = Vec::new();
        for (flag, name) in
            [(DragDropEffects::COPY, "copy"), (DragDropEffects::MOVE, "move"), (DragDropEffects::LINK, "link")]
        {
            if effects.contains(flag) {
                names.push(name.to_string());
            }
        }
        let unknown = effects.bits() & !(DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK).bits();
        if unknown != 0 {
            // The original writes the number of a value with bits it has no name for.
            return effects.bits().to_string();
        }
        names.join(", ")
    }

    /// A key went down. `code` and `key` are the values of the event of the
    /// page; a missing one (some virtual keyboards and autofill send key
    /// events without them) is an unknown key.
    pub fn on_key_down(&self, code: Option<&str>, key: Option<&str>, modifier: i32) -> bool {
        let (code, key) = (code.unwrap_or_default(), key.unwrap_or_default());
        let mut handled =
            self.raw_keyboard_event(RawKeyEventType::KeyDown, code, key, Self::to_raw_input_modifiers(modifier));

        // One UTF-16 unit, as the original counts.
        // Differs from the original, which turns every such key into text: with Meta, or with
        // Control alone, the key is a shortcut of the application, the browser or the system,
        // not a character (Control with Alt is AltGr and does produce one). Otherwise an
        // unhandled Ctrl+R would type "r" and, being handled as text, keep the browser from
        // reloading.
        if !handled && key.encode_utf16().count() == 1 && !Self::is_command_chord(modifier) {
            handled = self.raw_text_event(key);
        }

        handled
    }

    /// A key went up; see [`on_key_down`](Self::on_key_down).
    pub fn on_key_up(&self, code: Option<&str>, key: Option<&str>, modifier: i32) -> bool {
        let (code, key) = (code.unwrap_or_default(), key.unwrap_or_default());
        self.raw_keyboard_event(RawKeyEventType::KeyUp, code, key, Self::to_raw_input_modifiers(modifier))
    }

    fn raw_pointer_event(
        &self,
        event_type: RawPointerEventType,
        pointer_type: &str,
        p: RawPointerPoint,
        modifiers: RawInputModifiers,
        touch_point_id: i64,
        intermediate_points: Option<IntermediatePoints>,
    ) -> bool {
        if let Some(input_root) = self.input_root() {
            let device = self.get_pointer_device(pointer_type, touch_point_id);
            let timestamp = self.timestamp();

            if device.as_any().is::<TouchDevice>() {
                let args = Rc::new(RawTouchEventArgs::with_point(
                    device,
                    timestamp,
                    input_root,
                    event_type,
                    p,
                    modifiers,
                    touch_point_id,
                ));
                args.set_intermediate_points(intermediate_points);

                self.schedule_input(args.clone());

                return args.handled();
            }

            let args =
                Rc::new(RawPointerEventArgs::with_point(device, timestamp, input_root, event_type, p, modifiers));
            args.set_raw_pointer_id(touch_point_id);
            args.set_intermediate_points(intermediate_points);

            self.schedule_input(args.clone());

            return args.handled();
        }

        false
    }

    fn get_pointer_device(&self, pointer_type: &str, pointer_id: i64) -> Rc<dyn IInputDevice> {
        if pointer_type == "touch" {
            return self.touch_device.clone();
        } else if pointer_type == "pen" {
            return self.pen_device.clone();
        }

        // TODO: refactor pointer devices, so we can reuse single instance here.
        let existing = self
            .mouse_devices
            .borrow()
            .iter()
            .find(|mouse_device| mouse_device.pointer_id() == pointer_id)
            .map(|mouse_device| mouse_device.device().clone());
        if let Some(mouse_device) = existing {
            return mouse_device;
        }

        let new_mouse_device = BrowserMouseDevice::new(pointer_id, self.container.clone());
        let device = new_mouse_device.device().clone();
        self.mouse_devices.borrow_mut().push(new_mouse_device);
        device
    }

    fn raw_mouse_wheel_event(&self, p: Point, v: Vector, modifiers: RawInputModifiers) -> bool {
        if let Some(input_root) = self.input_root() {
            let device: Rc<dyn IInputDevice> = self.wheel_mouse_device.clone();
            let args = Rc::new(RawMouseWheelEventArgs::new(device, self.timestamp(), input_root, p, v, modifiers));

            self.schedule_input(args.clone());

            return args.handled();
        }

        false
    }

    /// Raises a raw drag event; the effects the target accepted, or `None`
    /// when there is no input root to raise it for.
    fn raw_drag_event(
        &self,
        event_type: RawDragEventType,
        position: Point,
        modifiers: RawInputModifiers,
        data_transfer: Rc<dyn IDataTransfer>,
        drop_effect: DragDropEffects,
    ) -> Option<DragDropEffects> {
        let device = FerroLocator::current().get_required_service::<dyn IDragDropDevice>();
        let input_root = self.input_root()?;
        let event_args =
            Rc::new(RawDragEvent::new(device, event_type, input_root, position, data_transfer, drop_effect, modifiers));
        self.schedule_input(event_args.clone());
        Some(event_args.effects())
    }

    fn raw_keyboard_event(
        &self,
        type_: RawKeyEventType,
        dom_code: &str,
        dom_key: &str,
        modifiers: RawInputModifiers,
    ) -> bool {
        let Some(input_root) = self.input_root() else {
            return false;
        };

        let physical_key = key_interop::physical_key_from_dom_code(Some(dom_code));
        let key = key_interop::key_from_dom_key(Some(dom_key), physical_key);
        let key_symbol = key_interop::key_symbol_from_dom_key(Some(dom_key));

        let device: Rc<dyn IInputDevice> = BrowserWindowingPlatform::keyboard();
        let args = Rc::new(RawKeyEventArgs::new(
            device,
            self.timestamp(),
            input_root,
            type_,
            key,
            modifiers,
            physical_key,
            key_symbol,
            KeyDeviceType::Keyboard,
        ));

        self.schedule_input(args.clone());

        args.handled()
    }

    pub(crate) fn raw_text_event(&self, text: &str) -> bool {
        if let Some(input_root) = self.input_root() {
            let device: Rc<dyn IInputDevice> = BrowserWindowingPlatform::keyboard();
            let args = Rc::new(RawTextInputEventArgs::new(device, self.timestamp(), input_root, text));
            self.schedule_input(args.clone());

            return args.handled();
        }

        false
    }

    fn schedule_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        // The original hands the event to its event grouper when it runs with a managed
        // dispatcher; here there is none and the event is dispatched right away.
        self.dispatch_input(args);
    }

    fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        if self.input_root.borrow().is_none() {
            return;
        }

        if let Some(top_level_impl) = self.top_level_impl.upgrade() {
            top_level_impl.dispatch_input(args);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_text_input_method::ITextInputPage;
    use ferroui_base::input::text_input::{
        ITextInputMethodImpl, TextInputMethodClient, TextInputMethodClientEvents, TextSelection,
    };
    use ferroui_base::input::{FocusManager, InputElement, Key, KeyboardDevice, PhysicalKey};
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{Rect, Ref, Visual};

    struct TestRoot {
        element: Ref<InputElement>,
    }

    impl IInputRoot for TestRoot {
        fn focus_manager(&self) -> Option<Rc<FocusManager>> {
            None
        }

        fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
            None
        }

        fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}

        fn cursor_element(&self) -> Option<Ref<InputElement>> {
            None
        }

        fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}

        fn root_element(&self) -> Ref<InputElement> {
            self.element.clone()
        }

        fn focus_root(&self) -> Ref<InputElement> {
            self.element.clone()
        }

        fn pointer_over_invalidated(&self) {}
    }

    /// Records the events the handler dispatches, as the top-level would
    /// receive them.
    struct TestTopLevel {
        events: RefCell<Vec<Rc<dyn IRawInputEventArgs>>>,
        /// The points the events carried, read while the event was being dispatched.
        points: RefCell<Vec<Option<Vec<RawPointerPoint>>>>,
        handle: Cell<bool>,
        read_points: Cell<bool>,
        /// The effects a drop target accepts, set on the drag events.
        drag_effects: Cell<Option<DragDropEffects>>,
        page_size: Size,
    }

    impl IInputTopLevel for TestTopLevel {
        fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>) {
            if self.handle.get() {
                args.set_handled(true);
            }
            if let (Some(effects), Some(drag)) = (self.drag_effects.get(), args.downcast_ref::<RawDragEvent>()) {
                drag.set_effects(effects);
            }
            if self.read_points.get() {
                let points = args
                    .downcast_ref::<RawPointerEventArgs>()
                    .and_then(|args| args.intermediate_points())
                    .and_then(|points| (**points).clone());
                self.points.borrow_mut().push(points);
            }
            self.events.borrow_mut().push(args);
        }

        fn page_size(&self) -> Size {
            self.page_size
        }
    }

    #[derive(Default)]
    struct PageLog {
        calls: RefCell<Vec<String>>,
        coalesced: RefCell<Vec<f64>>,
    }

    struct RecordingPage(Rc<PageLog>);

    impl IInputPage for RecordingPage {
        fn subscribe_input_events(&self, top_level_id: i32) {
            self.0.calls.borrow_mut().push(format!("subscribe {top_level_id}"));
        }

        fn unsubscribe_input_events(&self) {
            self.0.calls.borrow_mut().push("unsubscribe".to_string());
        }

        fn get_coalesced_events(&self, _pointer_event: &JsObject) -> Vec<f64> {
            self.0.calls.borrow_mut().push("coalesced".to_string());
            self.0.coalesced.borrow().clone()
        }
    }

    struct RecordingCapture(Rc<PageLog>);

    impl IPointerCapture for RecordingCapture {
        fn set_pointer_capture(&self, pointer_id: i64) {
            self.0.calls.borrow_mut().push(format!("capture {pointer_id}"));
        }

        fn release_pointer_capture(&self, pointer_id: i64) {
            self.0.calls.borrow_mut().push(format!("release {pointer_id}"));
        }
    }

    struct SilentTextInputPage;

    impl ITextInputPage for SilentTextInputPage {
        fn hide_input_element(&self) {}
        fn show_input_element(&self) {}
        fn focus_input_element(&self) {}
        fn focus_container_element(&self) {}
        fn clear_input_element(&self) {}
        fn set_surrounding_text(&self, _text: &str, _start: i32, _end: i32) {}
        fn set_bounds(&self, _x: i32, _y: i32, _width: i32, _height: i32, _caret: i32) {}

        fn selection_start(&self) -> i32 {
            0
        }

        fn selection_end(&self) -> i32 {
            0
        }
    }

    struct Fixture {
        handler: Rc<BrowserInputHandler>,
        top_level: Rc<TestTopLevel>,
        page: Rc<PageLog>,
    }

    impl Fixture {
        /// A handler without an input root.
        fn without_root() -> Self {
            let top_level = Rc::new(TestTopLevel {
                events: RefCell::new(Vec::new()),
                points: RefCell::new(Vec::new()),
                handle: Cell::new(false),
                read_points: Cell::new(false),
                drag_effects: Cell::new(None),
                page_size: Size::new(800.0, 600.0),
            });
            let page = Rc::new(PageLog::default());
            let weak: Weak<TestTopLevel> = Rc::downgrade(&top_level);
            let handler = BrowserInputHandler::with_page(
                weak,
                Box::new(RecordingPage(page.clone())),
                Rc::new(RecordingCapture(page.clone())),
                Box::new(SilentTextInputPage),
                42,
            );
            Self { handler, top_level, page }
        }

        fn new() -> Self {
            let fixture = Self::without_root();
            fixture.handler.set_input_root(Rc::new(TestRoot { element: InputElement::new() }));
            fixture
        }

        fn event_count(&self) -> usize {
            self.top_level.events.borrow().len()
        }

        fn event(&self, index: usize) -> Rc<dyn IRawInputEventArgs> {
            self.top_level.events.borrow()[index].clone()
        }

        fn pointer_event_type(&self, index: usize) -> RawPointerEventType {
            let event = self.event(index);
            event.downcast_ref::<RawPointerEventArgs>().expect("a pointer event").type_()
        }
    }

    fn point(x: f64, y: f64, pressure: f32, x_tilt: f32, y_tilt: f32, twist: f32) -> RawPointerPoint {
        let mut point = RawPointerPoint::new();
        point.position = Point::new(x, y);
        point.pressure = pressure;
        point.x_tilt = x_tilt;
        point.y_tilt = y_tilt;
        point.twist = twist;
        point
    }

    fn assert_close(expected: f64, actual: f64) {
        assert!((expected - actual).abs() < 1e-9, "expected {expected}, got {actual}");
    }

    // --- what does not need a handler -------------------------------------

    #[test]
    fn the_modifiers_of_the_page_are_the_raw_input_modifiers() {
        for (bits, expected) in [
            (0, RawInputModifiers::NONE),
            (1, RawInputModifiers::ALT),
            (2, RawInputModifiers::CONTROL),
            (4, RawInputModifiers::SHIFT),
            (8, RawInputModifiers::META),
            (16, RawInputModifiers::LEFT_MOUSE_BUTTON),
            (32, RawInputModifiers::RIGHT_MOUSE_BUTTON),
            (64, RawInputModifiers::MIDDLE_MOUSE_BUTTON),
            (128, RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON),
            (256, RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON),
            (512, RawInputModifiers::PEN_INVERTED),
            (1024, RawInputModifiers::PEN_ERASER),
            (2048, RawInputModifiers::PEN_BARREL_BUTTON),
        ] {
            assert_eq!(expected, BrowserInputHandler::to_raw_input_modifiers(bits), "{bits}");
        }

        assert_eq!(
            RawInputModifiers::CONTROL | RawInputModifiers::SHIFT | RawInputModifiers::LEFT_MOUSE_BUTTON,
            BrowserInputHandler::to_raw_input_modifiers(2 | 4 | 16)
        );
        assert_eq!(RawInputModifiers::KEYBOARD_MASK, BrowserInputHandler::to_raw_input_modifiers(15));
    }

    #[test]
    fn a_move_of_a_touch_is_a_touch_update_and_any_other_move_a_move() {
        assert_eq!(RawPointerEventType::TouchUpdate, BrowserInputHandler::pointer_move_type("touch"));
        assert_eq!(RawPointerEventType::Move, BrowserInputHandler::pointer_move_type("mouse"));
        assert_eq!(RawPointerEventType::Move, BrowserInputHandler::pointer_move_type("pen"));
        assert_eq!(RawPointerEventType::Move, BrowserInputHandler::pointer_move_type(""));
    }

    #[test]
    fn the_button_of_a_pointer_down_selects_the_event() {
        for pointer_type in ["mouse", "pen", ""] {
            for (button, expected) in [
                (0, RawPointerEventType::LeftButtonDown),
                (1, RawPointerEventType::MiddleButtonDown),
                (2, RawPointerEventType::RightButtonDown),
                (3, RawPointerEventType::XButton1Down),
                (4, RawPointerEventType::XButton2Down),
                (5, RawPointerEventType::XButton1Down),
                (6, RawPointerEventType::Move),
                (-1, RawPointerEventType::Move),
            ] {
                assert_eq!(expected, BrowserInputHandler::pointer_down_type(pointer_type, button), "{button}");
            }
        }

        for button in [-1, 0, 1, 2, 5] {
            assert_eq!(RawPointerEventType::TouchBegin, BrowserInputHandler::pointer_down_type("touch", button));
        }
    }

    #[test]
    fn the_button_of_a_pointer_up_selects_the_event() {
        for pointer_type in ["mouse", "pen", ""] {
            for (button, expected) in [
                (0, RawPointerEventType::LeftButtonUp),
                (1, RawPointerEventType::MiddleButtonUp),
                (2, RawPointerEventType::RightButtonUp),
                (3, RawPointerEventType::XButton1Up),
                (4, RawPointerEventType::XButton2Up),
                (5, RawPointerEventType::XButton1Up),
                (6, RawPointerEventType::Move),
                (-1, RawPointerEventType::Move),
            ] {
                assert_eq!(expected, BrowserInputHandler::pointer_up_type(pointer_type, button), "{button}");
            }
        }

        for button in [-1, 0, 1, 2, 5] {
            assert_eq!(RawPointerEventType::TouchEnd, BrowserInputHandler::pointer_up_type("touch", button));
        }
    }

    #[test]
    fn a_raw_pointer_point_carries_the_position_and_the_state_of_the_pen() {
        let raw = BrowserInputHandler::create_raw_pointer(10.5, 20.25, 0.75, 30.0, -45.0, 90.0);

        assert_eq!(point(10.5, 20.25, 0.75, 30.0, -45.0, 90.0), raw);
    }

    #[test]
    fn every_coalesced_point_but_the_last_is_decoded() {
        let props = [
            1.0, 2.0, 0.1, 10.0, 11.0, 12.0, //
            3.0, 4.0, 0.2, 20.0, 21.0, 22.0, //
            5.0, 6.0, 0.3, 30.0, 31.0, 32.0, //
            7.0, 8.0, 0.4, 40.0, 41.0, 42.0,
        ];

        let points = BrowserInputHandler::decode_coalesced_events(&props);

        assert_eq!(
            vec![
                point(1.0, 2.0, 0.1, 10.0, 11.0, 12.0),
                point(3.0, 4.0, 0.2, 20.0, 21.0, 22.0),
                point(5.0, 6.0, 0.3, 30.0, 31.0, 32.0),
            ],
            points
        );
    }

    #[test]
    fn more_points_than_numbers_per_point_are_all_decoded() {
        // Eight points: a loop that steps by the numbers per point while it counts points
        // would decode two of the seven.
        let props: Vec<f64> = (0..8 * ITEMS_PER_POINT).map(|i| i as f64).collect();

        let points = BrowserInputHandler::decode_coalesced_events(&props);

        assert_eq!(7, points.len());
        for (index, decoded) in points.iter().enumerate() {
            let first = (index * ITEMS_PER_POINT) as f64;
            assert_eq!(Point::new(first, first + 1.0), decoded.position);
            assert_eq!((first + 2.0) as f32, decoded.pressure);
            assert_eq!((first + 3.0) as f32, decoded.x_tilt);
            assert_eq!((first + 4.0) as f32, decoded.y_tilt);
            assert_eq!((first + 5.0) as f32, decoded.twist);
        }
    }

    #[test]
    fn a_move_of_one_point_or_none_has_no_coalesced_points() {
        assert!(BrowserInputHandler::decode_coalesced_events(&[]).is_empty());
        assert!(BrowserInputHandler::decode_coalesced_events(&[1.0, 2.0, 0.5, 0.0, 0.0, 0.0]).is_empty());
        // Numbers that do not make a whole point are ignored.
        assert!(BrowserInputHandler::decode_coalesced_events(&[1.0, 2.0, 0.5]).is_empty());
        // Two points and a number left over: the first point.
        let props = [1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 3.0, 4.0, 0.5, 0.0, 0.0, 0.0, 9.0];
        assert_eq!(vec![point(1.0, 2.0, 0.5, 0.0, 0.0, 0.0)], BrowserInputHandler::decode_coalesced_events(&props));
    }

    #[test]
    fn pixels_of_the_wheel_are_divided_by_fifty_and_inverted() {
        let page = Size::new(800.0, 600.0);

        let delta = BrowserInputHandler::wheel_delta(-50.0, 100.0, DOM_DELTA_PIXEL, page);
        assert_close(1.0, delta.x);
        assert_close(-2.0, delta.y);

        let none = BrowserInputHandler::wheel_delta(0.0, 0.0, DOM_DELTA_PIXEL, page);
        assert_close(0.0, none.x);
        assert_close(0.0, none.y);
    }

    #[test]
    fn three_lines_of_the_wheel_are_one_notch() {
        let page = Size::new(800.0, 600.0);

        let delta = BrowserInputHandler::wheel_delta(-3.0, 3.0, DOM_DELTA_LINE, page);

        assert_close(1.0, delta.x);
        assert_close(-1.0, delta.y);

        let one_line = BrowserInputHandler::wheel_delta(0.0, 1.0, DOM_DELTA_LINE, page);
        assert_close(-1.0 / 3.0, one_line.y);
    }

    #[test]
    fn a_page_of_the_wheel_is_the_size_of_the_view() {
        let page = Size::new(800.0, 600.0);

        let delta = BrowserInputHandler::wheel_delta(1.0, -1.0, DOM_DELTA_PAGE, page);

        assert_close(-16.0, delta.x);
        assert_close(12.0, delta.y);
    }

    #[test]
    fn an_unknown_unit_of_the_wheel_is_taken_for_pixels() {
        let page = Size::new(800.0, 600.0);

        let delta = BrowserInputHandler::wheel_delta(50.0, 50.0, 7, page);

        assert_close(-1.0, delta.x);
        assert_close(-1.0, delta.y);
    }

    // --- the handler ------------------------------------------------------

    #[test]
    fn the_handler_subscribes_to_the_events_of_its_top_level_and_unsubscribes_when_disposed() {
        let fixture = Fixture::without_root();
        assert_eq!(vec!["subscribe 42"], *fixture.page.calls.borrow());

        fixture.handler.dispose();
        assert_eq!(vec!["subscribe 42", "unsubscribe"], *fixture.page.calls.borrow());
    }

    #[test]
    fn without_an_input_root_nothing_is_dispatched() {
        let fixture = Fixture::without_root();
        fixture.top_level.handle.set(true);

        assert!(!fixture.handler.on_pointer_down("mouse", 1, 0, 1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 16));
        assert!(!fixture.handler.on_pointer_up("mouse", 1, 0, 1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 0));
        assert!(!fixture.handler.on_pointer_move("mouse", 1, 1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 0, JsObject::UNDEFINED));
        assert!(!fixture.handler.on_pointer_cancel("touch", 1, 1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 0));
        assert!(!fixture.handler.on_wheel(1.0, 2.0, 0.0, 100.0, DOM_DELTA_PIXEL, 0));
        assert!(!fixture.handler.on_key_down(Some("KeyA"), Some("a"), 0));
        assert!(!fixture.handler.on_key_up(Some("KeyA"), Some("a"), 0));
        assert!(!fixture.handler.raw_text_event("a"));

        assert_eq!(0, fixture.event_count());
    }

    #[test]
    fn a_mouse_button_raises_a_pointer_event_of_the_mouse_device_of_its_pointer() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();

        assert!(!fixture.handler.on_pointer_down("mouse", 7, 2, 10.0, 20.0, 0.5, 1.0, 2.0, 3.0, 32 | 4));
        fixture.top_level.handle.set(true);
        assert!(fixture.handler.on_pointer_up("mouse", 7, 2, 11.0, 21.0, 0.0, 0.0, 0.0, 0.0, 4));

        assert_eq!(2, fixture.event_count());
        let down = fixture.event(0);
        let down = down.downcast_ref::<RawPointerEventArgs>().expect("a pointer event");
        assert!(fixture.event(0).downcast_ref::<RawTouchEventArgs>().is_none());
        assert_eq!(RawPointerEventType::RightButtonDown, down.type_());
        assert_eq!(point(10.0, 20.0, 0.5, 1.0, 2.0, 3.0), down.point());
        assert_eq!(RawInputModifiers::RIGHT_MOUSE_BUTTON | RawInputModifiers::SHIFT, down.input_modifiers());
        assert_eq!(7, down.raw_pointer_id());
        assert!(down.intermediate_points().is_none());
        assert!(down.device().as_any().is::<MouseDevice>());

        assert_eq!(RawPointerEventType::RightButtonUp, fixture.pointer_event_type(1));
    }

    #[test]
    fn every_pointer_of_the_mouse_has_a_device_of_its_own_that_is_reused() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();

        fixture.handler.on_pointer_down("mouse", 1, 0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 16);
        fixture.handler.on_pointer_up("mouse", 1, 0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0);
        fixture.handler.on_pointer_down("mouse", 2, 0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 16);
        // An unknown kind of pointer is a mouse.
        fixture.handler.on_pointer_down("", 1, 0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 16);

        let device = |index: usize| Rc::as_ptr(fixture.event(index).device()) as *const ();
        assert_eq!(device(0), device(1));
        assert_ne!(device(0), device(2));
        assert_eq!(device(0), device(3));
        assert_eq!(2, fixture.handler.mouse_devices.borrow().len());
    }

    #[test]
    fn a_touch_raises_touch_events_of_the_touch_device() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();

        fixture.handler.on_pointer_down("touch", 11, 0, 1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 0);
        fixture.handler.on_pointer_move("touch", 11, 3.0, 4.0, 0.5, 0.0, 0.0, 0.0, 0, JsObject::UNDEFINED);
        fixture.handler.on_pointer_up("touch", 11, 0, 5.0, 6.0, 0.5, 0.0, 0.0, 0.0, 0);
        assert!(!fixture.handler.on_pointer_cancel("touch", 12, 7.0, 8.0, 0.5, 0.0, 0.0, 0.0, 0));

        assert_eq!(4, fixture.event_count());
        for (index, expected) in [
            RawPointerEventType::TouchBegin,
            RawPointerEventType::TouchUpdate,
            RawPointerEventType::TouchEnd,
            RawPointerEventType::TouchCancel,
        ]
        .into_iter()
        .enumerate()
        {
            let event = fixture.event(index);
            let touch = event.downcast_ref::<RawTouchEventArgs>().expect("a touch event");
            assert_eq!(expected, touch.type_());
            assert!(touch.device().as_any().is::<TouchDevice>());
        }

        let begin = fixture.event(0);
        assert_eq!(11, begin.downcast_ref::<RawPointerEventArgs>().expect("a pointer event").raw_pointer_id());
        let cancel = fixture.event(3);
        assert_eq!(12, cancel.downcast_ref::<RawPointerEventArgs>().expect("a pointer event").raw_pointer_id());
        assert!(fixture.handler.mouse_devices.borrow().is_empty());
    }

    #[test]
    fn a_cancelled_pointer_that_is_not_a_touch_raises_nothing() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();
        fixture.top_level.handle.set(true);

        assert!(!fixture.handler.on_pointer_cancel("mouse", 1, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0));
        assert!(!fixture.handler.on_pointer_cancel("pen", 1, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0));
        assert_eq!(0, fixture.event_count());

        // A cancelled touch is raised, and still reported as not handled.
        assert!(!fixture.handler.on_pointer_cancel("touch", 1, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0));
        assert_eq!(1, fixture.event_count());
    }

    #[test]
    fn a_pen_raises_pointer_events_of_the_pen_device() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();

        fixture.handler.on_pointer_down("pen", 5, 0, 1.0, 2.0, 0.8, 10.0, 20.0, 30.0, 16);
        fixture.handler.on_pointer_down("pen", 5, 5, 1.0, 2.0, 0.8, 10.0, 20.0, 30.0, 1024);

        let down = fixture.event(0);
        let down = down.downcast_ref::<RawPointerEventArgs>().expect("a pointer event");
        assert!(fixture.event(0).downcast_ref::<RawTouchEventArgs>().is_none());
        assert_eq!(RawPointerEventType::LeftButtonDown, down.type_());
        assert!(down.device().as_any().is::<PenDevice>());
        assert_eq!(5, down.raw_pointer_id());
        assert_eq!(point(1.0, 2.0, 0.8, 10.0, 20.0, 30.0), down.point());
        assert_eq!(RawPointerEventType::XButton1Down, fixture.pointer_event_type(1));
        assert!(fixture.handler.mouse_devices.borrow().is_empty());
    }

    #[test]
    fn the_coalesced_points_of_a_move_are_asked_for_only_when_they_are_read() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();
        *fixture.page.coalesced.borrow_mut() =
            vec![1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 3.0, 4.0, 0.5, 0.0, 0.0, 0.0, 5.0, 6.0, 0.5, 0.0, 0.0, 0.0];
        fixture.page.calls.borrow_mut().clear();

        fixture.handler.on_pointer_move("mouse", 1, 5.0, 6.0, 0.5, 0.0, 0.0, 0.0, 0, JsObject::UNDEFINED);
        assert!(fixture.page.calls.borrow().is_empty());

        fixture.top_level.read_points.set(true);
        fixture.handler.on_pointer_move("mouse", 1, 5.0, 6.0, 0.5, 0.0, 0.0, 0.0, 0, JsObject::UNDEFINED);

        assert_eq!(vec!["coalesced"], *fixture.page.calls.borrow());
        assert_eq!(
            vec![Some(vec![point(1.0, 2.0, 0.5, 0.0, 0.0, 0.0), point(3.0, 4.0, 0.5, 0.0, 0.0, 0.0)])],
            *fixture.top_level.points.borrow()
        );
        assert_eq!(RawPointerEventType::Move, fixture.pointer_event_type(1));
    }

    #[test]
    fn the_coalesced_points_of_a_move_are_empty_once_the_move_has_been_processed() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();
        *fixture.page.coalesced.borrow_mut() = vec![1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 3.0, 4.0, 0.5, 0.0, 0.0, 0.0];
        fixture.page.calls.borrow_mut().clear();

        fixture.handler.on_pointer_move("mouse", 1, 3.0, 4.0, 0.5, 0.0, 0.0, 0.0, 0, JsObject::UNDEFINED);

        let event = fixture.event(0);
        let points = event
            .downcast_ref::<RawPointerEventArgs>()
            .expect("a pointer event")
            .intermediate_points()
            .expect("a move carries its lazily computed points");
        assert_eq!(Some(Vec::new()), (**points).clone());
        // The event of the page was released: the page is not asked.
        assert!(fixture.page.calls.borrow().is_empty());
    }

    #[test]
    fn the_wheel_raises_a_wheel_event_of_the_wheel_device() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();

        assert!(!fixture.handler.on_wheel(10.0, 20.0, 0.0, 100.0, DOM_DELTA_PIXEL, 2));
        fixture.handler.on_wheel(10.0, 20.0, 0.0, 3.0, DOM_DELTA_LINE, 0);
        fixture.handler.on_wheel(10.0, 20.0, 1.0, 1.0, DOM_DELTA_PAGE, 0);
        fixture.top_level.handle.set(true);
        assert!(fixture.handler.on_wheel(10.0, 20.0, 0.0, 100.0, DOM_DELTA_PIXEL, 0));

        let event = fixture.event(0);
        let wheel = event.downcast_ref::<RawMouseWheelEventArgs>().expect("a wheel event");
        assert_eq!(RawPointerEventType::Wheel, wheel.type_());
        assert_eq!(Point::new(10.0, 20.0), wheel.position());
        assert_close(0.0, wheel.delta().x);
        assert_close(-2.0, wheel.delta().y);
        assert_eq!(RawInputModifiers::CONTROL, wheel.input_modifiers());
        assert_wheel_device(&fixture, 0);
        assert_wheel_device(&fixture, 3);
        assert!(fixture.handler.mouse_devices.borrow().is_empty());

        let lines = fixture.event(1);
        assert_close(-1.0, lines.downcast_ref::<RawMouseWheelEventArgs>().expect("a wheel event").delta().y);

        // A page is the size the top-level reports.
        let pages = fixture.event(2);
        let pages = pages.downcast_ref::<RawMouseWheelEventArgs>().expect("a wheel event").delta();
        assert_close(-16.0, pages.x);
        assert_close(-12.0, pages.y);
    }

    /// The device of the event at `index` is the wheel device of the handler.
    fn assert_wheel_device(fixture: &Fixture, index: usize) {
        let event = fixture.event(index);
        assert_eq!(
            Rc::as_ptr(event.device()) as *const (),
            Rc::as_ptr(&fixture.handler.wheel_mouse_device) as *const ()
        );
    }

    /// A text input client that only records its preedit text.
    struct TestClient {
        events: TextInputMethodClientEvents,
        preedit: RefCell<Vec<Option<String>>>,
    }

    impl TextInputMethodClient for TestClient {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }

        fn text_view_visual(&self) -> Ref<Visual> {
            unreachable!("the text input method of the browser does not ask for the visual")
        }

        fn supports_preedit(&self) -> bool {
            true
        }

        fn supports_surrounding_text(&self) -> bool {
            true
        }

        fn surrounding_text(&self) -> String {
            String::new()
        }

        fn cursor_rectangle(&self) -> Rect {
            Rect::default()
        }

        fn selection(&self) -> TextSelection {
            TextSelection::new(0, 0)
        }

        fn set_selection(&self, _value: TextSelection) {}

        fn set_preedit_text(&self, preedit_text: Option<&str>) {
            self.preedit.borrow_mut().push(preedit_text.map(str::to_string));
        }
    }

    fn with_keyboard() -> Rc<KeyboardDevice> {
        let keyboard = KeyboardDevice::new();
        BrowserWindowingPlatform::set_keyboard_for_unit_tests(Some(keyboard.clone()));
        keyboard
    }

    #[test]
    fn a_key_raises_a_key_event_of_the_keyboard_of_the_platform() {
        let _scope = Dispatcher::unit_test_scope();
        let keyboard = with_keyboard();
        let fixture = Fixture::new();
        fixture.top_level.handle.set(true);

        assert!(fixture.handler.on_key_down(Some("ShiftRight"), Some("Shift"), 4));
        assert!(fixture.handler.on_key_up(Some("Numpad1"), Some("1"), 0));

        assert_eq!(2, fixture.event_count());
        let down = fixture.event(0);
        let down = down.downcast_ref::<RawKeyEventArgs>().expect("a key event");
        assert_eq!(RawKeyEventType::KeyDown, down.type_());
        assert_eq!(Key::RightShift, down.key());
        assert_eq!(PhysicalKey::ShiftRight, down.physical_key());
        assert_eq!(None, down.key_symbol());
        assert_eq!(RawInputModifiers::SHIFT, down.modifiers());
        assert_eq!(KeyDeviceType::Keyboard, down.key_device_type());
        assert_eq!(Rc::as_ptr(down.device()) as *const (), Rc::as_ptr(&keyboard) as *const ());

        let up = fixture.event(1);
        let up = up.downcast_ref::<RawKeyEventArgs>().expect("a key event");
        assert_eq!(RawKeyEventType::KeyUp, up.type_());
        assert_eq!(Key::NumPad1, up.key());
        assert_eq!(PhysicalKey::NumPad1, up.physical_key());
        assert_eq!(Some("1".to_string()), up.key_symbol());

        BrowserWindowingPlatform::set_keyboard_for_unit_tests(None);
    }

    #[test]
    fn a_key_down_of_one_character_that_is_not_handled_becomes_text() {
        let _scope = Dispatcher::unit_test_scope();
        let _keyboard = with_keyboard();
        let fixture = Fixture::new();

        // Not handled: the key and then its text.
        assert!(!fixture.handler.on_key_down(Some("KeyA"), Some("a"), 0));
        assert_eq!(2, fixture.event_count());
        assert!(fixture.event(0).downcast_ref::<RawKeyEventArgs>().is_some());
        let text = fixture.event(1);
        assert_eq!("a", text.downcast_ref::<RawTextInputEventArgs>().expect("a text event").text());

        // A named key, and a character outside the basic plane (two UTF-16 units), are no text.
        assert!(!fixture.handler.on_key_down(Some("Enter"), Some("Enter"), 0));
        assert!(!fixture.handler.on_key_down(Some("KeyA"), Some("\u{1f600}"), 0));
        assert_eq!(4, fixture.event_count());

        // A key up is never text.
        assert!(!fixture.handler.on_key_up(Some("KeyA"), Some("a"), 0));
        assert_eq!(5, fixture.event_count());

        // Handled: the key only.
        fixture.top_level.handle.set(true);
        assert!(fixture.handler.on_key_down(Some("KeyB"), Some("b"), 0));
        assert_eq!(6, fixture.event_count());

        BrowserWindowingPlatform::set_keyboard_for_unit_tests(None);
    }

    #[test]
    fn a_character_key_with_a_command_modifier_is_not_text() {
        let _scope = Dispatcher::unit_test_scope();
        let _keyboard = with_keyboard();
        let fixture = Fixture::new();
        const ALT: i32 = 1;
        const CONTROL: i32 = 2;
        const SHIFT: i32 = 4;
        const META: i32 = 8;

        // Control or Meta: the key only.
        assert!(!fixture.handler.on_key_down(Some("KeyR"), Some("r"), CONTROL));
        assert!(!fixture.handler.on_key_down(Some("KeyR"), Some("r"), META));
        assert!(!fixture.handler.on_key_down(Some("KeyR"), Some("R"), CONTROL | SHIFT));
        assert_eq!(3, fixture.event_count());

        // Shift, Alt (Option on a Mac) and AltGr (Control with Alt) produce characters.
        assert!(!fixture.handler.on_key_down(Some("KeyR"), Some("R"), SHIFT));
        assert!(!fixture.handler.on_key_down(Some("KeyR"), Some("\u{ae}"), ALT));
        assert!(!fixture.handler.on_key_down(Some("KeyQ"), Some("@"), CONTROL | ALT));
        assert_eq!(9, fixture.event_count());
        let text = fixture.event(8);
        assert_eq!("@", text.downcast_ref::<RawTextInputEventArgs>().expect("a text event").text());

        BrowserWindowingPlatform::set_keyboard_for_unit_tests(None);
    }

    #[test]
    fn a_key_event_without_code_or_key_is_an_unknown_key_and_no_text() {
        let _scope = Dispatcher::unit_test_scope();
        let _keyboard = with_keyboard();
        let fixture = Fixture::new();

        assert!(!fixture.handler.on_key_down(None, None, 0));
        assert!(!fixture.handler.on_key_up(None, None, 0));

        assert_eq!(2, fixture.event_count());
        let down = fixture.event(0);
        let down = down.downcast_ref::<RawKeyEventArgs>().expect("a key event");
        assert_eq!(Key::None, down.key());
        assert_eq!(PhysicalKey::None, down.physical_key());
        assert_eq!(None, down.key_symbol());

        BrowserWindowingPlatform::set_keyboard_for_unit_tests(None);
    }

    #[test]
    fn the_text_of_a_finished_composition_is_raised_as_text() {
        let _scope = Dispatcher::unit_test_scope();
        let _keyboard = with_keyboard();
        let fixture = Fixture::new();

        // Without a client the text input method ignores the composition.
        fixture.handler.text_input_method().on_composition_end(Some("\u{304b}"));
        assert_eq!(0, fixture.event_count());

        let client =
            Rc::new(TestClient { events: TextInputMethodClientEvents::new(), preedit: RefCell::new(Vec::new()) });
        let text_input_client: Rc<dyn TextInputMethodClient> = client.clone();
        fixture.handler.text_input_method().set_client(Some(text_input_client));

        fixture.handler.text_input_method().on_composition_start();
        fixture.handler.text_input_method().on_composition_update(Some("ka"));
        assert_eq!(0, fixture.event_count());

        fixture.handler.text_input_method().on_composition_end(Some("\u{304b}"));
        assert_eq!(1, fixture.event_count());
        let text = fixture.event(0);
        assert_eq!("\u{304b}", text.downcast_ref::<RawTextInputEventArgs>().expect("a text event").text());
        assert_eq!(vec![None, Some("ka".to_string()), None], *client.preedit.borrow());

        // A composition that ends without text raises nothing.
        fixture.handler.text_input_method().on_composition_end(None);
        assert_eq!(1, fixture.event_count());

        BrowserWindowingPlatform::set_keyboard_for_unit_tests(None);
    }

    #[test]
    fn the_time_stamps_of_the_events_do_not_go_back() {
        let _scope = Dispatcher::unit_test_scope();
        let fixture = Fixture::new();

        fixture.handler.on_pointer_down("mouse", 1, 0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 16);
        fixture.handler.on_pointer_up("mouse", 1, 0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0);

        assert!(fixture.event(0).timestamp() <= fixture.event(1).timestamp());
        assert!(fixture.event(1).timestamp() <= fixture.handler.timestamp());
    }

    #[test]
    fn the_handler_has_a_closed_input_pane_and_a_text_input_method_that_is_not_composing() {
        let fixture = Fixture::without_root();

        assert!(!fixture.handler.text_input_method().is_composing());
        assert!(fixture.handler.input_pane().on_geometry_change(0.0, 0.0, 0.0, 0.0));
    }

    // --- drag and drop ------------------------------------------------------

    /// The drag-and-drop device of the application, bound for the duration
    /// of a test.
    struct DragDropScope(Rc<dyn ferroui_base::reactive::IDisposable>);

    impl DragDropScope {
        fn new() -> Self {
            let scope = FerroLocator::enter_scope();
            FerroLocator::current_mutable()
                .bind::<dyn IDragDropDevice>()
                .to_constant(ferroui_base::input::DragDropDevice::instance() as Rc<dyn IDragDropDevice>);
            Self(scope)
        }
    }

    impl Drop for DragDropScope {
        fn drop(&mut self) {
            self.0.dispose();
        }
    }

    fn text_items(text: &'static str) -> Rc<dyn IReadableDataItems> {
        use crate::browser_data_transfer_helper::tests::{FakeReadableDataItem, FakeReadableDataItems, FakeValue};
        Rc::new(FakeReadableDataItems(vec![FakeReadableDataItem::new(
            &["text/plain"],
            &[("text/plain", FakeValue::String(text))],
        )]))
    }

    fn drag_event(fixture: &Fixture, index: usize) -> Rc<dyn IRawInputEventArgs> {
        let event = fixture.event(index);
        assert!(event.downcast_ref::<RawDragEvent>().is_some(), "a drag event");
        event
    }

    #[test]
    fn the_drag_events_of_the_page_are_raw_drag_events() {
        let _scope = Dispatcher::unit_test_scope();
        let _drag_drop = DragDropScope::new();
        let fixture = Fixture::new();

        for (type_, expected) in [
            ("dragenter", RawDragEventType::DragEnter),
            ("dragover", RawDragEventType::DragOver),
            ("dragleave", RawDragEventType::DragLeave),
            ("drop", RawDragEventType::Drop),
        ] {
            fixture.handler.on_drag_event_core(type_, 10.0, 20.0, 2, "copy", text_items("a"));
            let event = drag_event(&fixture, fixture.event_count() - 1);
            let drag = event.downcast_ref::<RawDragEvent>().unwrap();
            assert_eq!(expected, drag.type_());
            assert_eq!(Point::new(10.0, 20.0), drag.location());
            assert_eq!(ferroui_base::input::KeyModifiers::CONTROL, drag.key_modifiers());
        }
        assert_eq!(4, fixture.event_count());

        assert_eq!((false, None), fixture.handler.on_drag_event_core("drag", 0.0, 0.0, 0, "copy", text_items("a")));
        assert_eq!(4, fixture.event_count());
    }

    #[test]
    fn the_allowed_effects_are_read_from_the_effect_the_page_allows() {
        let _scope = Dispatcher::unit_test_scope();
        let _drag_drop = DragDropScope::new();
        let fixture = Fixture::new();

        let all = DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK;
        for (effect_allowed, expected) in [
            ("copy", DragDropEffects::COPY),
            ("copyLink", DragDropEffects::COPY | DragDropEffects::LINK),
            ("copyMove", DragDropEffects::COPY | DragDropEffects::MOVE),
            ("linkMove", DragDropEffects::LINK | DragDropEffects::MOVE),
            ("move", DragDropEffects::MOVE),
            ("all", all),
            ("uninitialized", all),
            ("ALL", all),
        ] {
            let (_, effect) = fixture.handler.on_drag_event_core("dragenter", 0.0, 0.0, 0, effect_allowed, text_items("a"));
            assert_eq!(Some(expected), effect, "{effect_allowed}");
        }

        let count = fixture.event_count();
        assert_eq!((false, None), fixture.handler.on_drag_event_core("dragover", 0.0, 0.0, 0, "none", text_items("a")));
        assert_eq!(count, fixture.event_count());
    }

    #[test]
    fn a_drag_over_or_a_drop_is_handled_when_the_target_accepts_an_effect() {
        let _scope = Dispatcher::unit_test_scope();
        let _drag_drop = DragDropScope::new();
        let fixture = Fixture::new();

        fixture.top_level.drag_effects.set(Some(DragDropEffects::COPY));
        assert_eq!(
            (true, Some(DragDropEffects::COPY)),
            fixture.handler.on_drag_event_core("dragover", 0.0, 0.0, 0, "all", text_items("a"))
        );
        assert_eq!(
            (true, Some(DragDropEffects::COPY)),
            fixture.handler.on_drag_event_core("drop", 0.0, 0.0, 0, "all", text_items("a"))
        );
        assert_eq!(
            (false, Some(DragDropEffects::COPY)),
            fixture.handler.on_drag_event_core("dragenter", 0.0, 0.0, 0, "all", text_items("a"))
        );

        fixture.top_level.drag_effects.set(Some(DragDropEffects::NONE));
        assert_eq!(
            (false, Some(DragDropEffects::NONE)),
            fixture.handler.on_drag_event_core("drop", 0.0, 0.0, 0, "all", text_items("a"))
        );
    }

    #[test]
    fn the_dropped_text_is_read_from_the_items_of_the_page() {
        use ferroui_base::input::{DataFormat, DataTransferExtensions};

        let _scope = Dispatcher::unit_test_scope();
        let _drag_drop = DragDropScope::new();
        let fixture = Fixture::new();

        fixture.handler.on_drag_event_core("drop", 0.0, 0.0, 0, "copy", text_items("dropped"));
        let event = drag_event(&fixture, 0);
        let data_transfer = event.downcast_ref::<RawDragEvent>().unwrap().data_transfer().clone();
        assert!(data_transfer.formats().iter().any(|format| DataFormat::text() == *format));
        assert_eq!(Some("dropped".to_string()), data_transfer.try_get_text());
    }

    #[test]
    fn without_an_input_root_a_drag_raises_nothing() {
        let _scope = Dispatcher::unit_test_scope();
        let _drag_drop = DragDropScope::new();
        let fixture = Fixture::without_root();

        assert_eq!((false, None), fixture.handler.on_drag_event_core("drop", 0.0, 0.0, 0, "copy", text_items("a")));
        assert_eq!(0, fixture.event_count());
    }

    #[test]
    fn the_drop_effect_is_the_lower_case_name_of_the_effects() {
        assert_eq!("none", BrowserInputHandler::drop_effect_name(DragDropEffects::NONE));
        assert_eq!("copy", BrowserInputHandler::drop_effect_name(DragDropEffects::COPY));
        assert_eq!("move", BrowserInputHandler::drop_effect_name(DragDropEffects::MOVE));
        assert_eq!("link", BrowserInputHandler::drop_effect_name(DragDropEffects::LINK));
        assert_eq!(
            "copy, move, link",
            BrowserInputHandler::drop_effect_name(DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK)
        );
        assert_eq!("9", BrowserInputHandler::drop_effect_name(DragDropEffects::from_bits_retain(9)));
    }
}
