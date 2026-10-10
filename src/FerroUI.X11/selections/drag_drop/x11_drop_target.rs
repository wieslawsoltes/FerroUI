//! The target side of the protocol for one window (the port of
//! `X11DropTarget.cs`).

use crate::selections::data_format_helper;
use crate::selections::drag_drop::drag_drop_data_reader::{DragDropDataReader, IDragDropItemsSource};
use crate::selections::drag_drop::drag_drop_data_transfer::DragDropDataTransfer;
use crate::selections::drag_drop::i_xdnd_window::IXdndWindow;
use crate::selections::drag_drop::xdnd_action_helper::XdndActions;
use crate::selections::drag_drop::xdnd_constants::{MIN_XDND_VERSION, XDND_VERSION};
use crate::x11_enum_extensions::X11EnumExtensions;
use crate::x11_enums::XModifierMask;
use crate::x11_info::X11Info;
use crate::x11_structs::{EventMask, XEventName};
use crate::xlib::{self, Atom, PropertyMode, XClientMessageEvent, XDisplay, XID};
use ferroui_base::input::raw::{IDragDropDevice, RawDragEvent, RawDragEventType};
use ferroui_base::input::{DataFormat, DragDropEffects, RawInputModifiers};
use ferroui_base::{PixelPoint, Point};
use std::cell::RefCell;
use std::ffi::{c_long, c_ulong};
use std::rc::{Rc, Weak};

/// What the drop target does on the connection. The reference calls Xlib
/// directly; the port goes through this so that the protocol can run
/// against a connection that is not a server (the tests).
pub trait IXdndTargetConnection {
    /// Announces that the window takes part in the protocol
    /// (`XdndAware`, with the version).
    fn set_xdnd_aware(&self, window: XID, version: u8);

    /// The formats a source with more than three of them lists on its
    /// window (`XdndTypeList`).
    fn get_type_list(&self, source_window: XID) -> Option<Vec<Atom>>;

    /// The formats of the framework for format atoms, and the reader of
    /// their values for a target window.
    fn create_reader(&self, format_atoms: &[Atom], target_window: XID)
        -> (Rc<dyn IDragDropItemsSource>, Vec<DataFormat>);

    /// Sends a client message of format 32 to a window.
    fn send_client_message(&self, window: XID, message_type: Atom, data: [c_long; 5]);

    /// The modifiers and buttons that are down.
    fn query_modifiers(&self, window: XID) -> RawInputModifiers;
}

/// The drop target on the connection of the platform.
pub struct XlibXdndTargetConnection {
    info: Rc<X11Info>,
    display: XDisplay,
}

impl XlibXdndTargetConnection {
    pub fn new(info: Rc<X11Info>) -> Rc<Self> {
        let display = info.display();
        Rc::new(Self { info, display })
    }
}

/// Sends a client message of the protocol (`SendXdndMessage` of the
/// reference, on both sides of it).
pub(crate) fn send_xdnd_message(display: XDisplay, window: XID, message_type: Atom, data: [c_long; 5]) {
    let mut evt = xlib::new_event();
    {
        let message = xlib::client_message_event_mut(&mut evt);
        message.type_ = XEventName::ClientMessage as i32;
        message.window = window;
        message.message_type = message_type;
        message.format = 32;
        for (index, value) in data.into_iter().enumerate() {
            message.data.set_long(index, value);
        }
    }
    xlib::x_send_event(display, window, false, EventMask::NO_EVENT_MASK.bits() as c_long, &mut evt);
    xlib::x_flush(display);
}

impl IXdndTargetConnection for XlibXdndTargetConnection {
    fn set_xdnd_aware(&self, window: XID, version: u8) {
        let atoms = self.info.atoms();
        xlib::x_change_property_longs(
            self.display,
            window,
            atoms.XdndAware,
            atoms.ATOM,
            PropertyMode::Replace,
            &[c_ulong::from(version)],
        );
    }

    fn get_type_list(&self, source_window: XID) -> Option<Vec<Atom>> {
        let atoms = self.info.atoms();
        xlib::x_get_window_property_as_int_ptr_array(self.display, source_window, atoms.XdndTypeList, atoms.ATOM)
    }

    fn create_reader(
        &self,
        format_atoms: &[Atom],
        target_window: XID,
    ) -> (Rc<dyn IDragDropItemsSource>, Vec<DataFormat>) {
        let (data_formats, text_formats) = data_format_helper::to_data_formats(format_atoms, self.info.atoms());
        let reader =
            DragDropDataReader::new(self.info.clone(), text_formats, data_formats.clone(), self.display, target_window);
        (reader, data_formats)
    }

    fn send_client_message(&self, window: XID, message_type: Atom, data: [c_long; 5]) {
        send_xdnd_message(self.display, window, message_type, data);
    }

    fn query_modifiers(&self, window: XID) -> RawInputModifiers {
        let pointer = xlib::x_query_pointer(self.display, window);
        if pointer.same_screen {
            XModifierMask::from_bits_retain(pointer.mask as i32).to_raw_input_modifiers()
        } else {
            RawInputModifiers::empty()
        }
    }
}

/// The message atoms the target sends, and the actions.
#[derive(Clone, Copy, Debug)]
pub struct XdndTargetAtoms {
    pub status: Atom,
    pub finished: Atom,
    pub actions: XdndActions,
}

/// The fields of an `XdndEnter` message (`OnXdndEnter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct XdndEnter {
    pub source_window: XID,
    pub version: u8,
    pub has_extra_formats: bool,
    /// The up to three formats the message carries itself.
    pub formats: Vec<Atom>,
}

impl XdndEnter {
    pub(crate) fn parse(data: [c_long; 5]) -> Self {
        Self {
            source_window: data[0] as XID,
            version: ((data[1] >> 24) & 0xFF) as u8,
            has_extra_formats: (data[1] & 1) == 1,
            formats: data[2..5].iter().filter(|format| **format != 0).map(|format| *format as Atom).collect(),
        }
    }
}

/// The point of the screen an `XdndPosition` message carries in one item.
pub(crate) fn position_from_message(packed: c_long) -> PixelPoint {
    let screen_x = ((packed >> 16) & 0xFFFF) as u16;
    let screen_y = (packed & 0xFFFF) as u16;
    PixelPoint::new(i32::from(screen_x), i32::from(screen_y))
}

fn message_data(message: &XClientMessageEvent) -> [c_long; 5] {
    std::array::from_fn(|index| message.data.get_long(index))
}

/// Manages an XDND target for a given X11 window.
/// Specs: <https://www.freedesktop.org/wiki/Specifications/XDND/>
pub struct X11DropTarget {
    drag_drop_device: Rc<dyn IDragDropDevice>,
    window: Weak<dyn IXdndWindow>,
    window_handle: XID,
    connection: Rc<dyn IXdndTargetConnection>,
    atoms: XdndTargetAtoms,
    current_drag: RefCell<Option<Rc<DragDropDataTransfer>>>,
}

impl X11DropTarget {
    /// The drop target of `window`, whose handle is `window_handle`. The
    /// window is announced as taking part in the protocol.
    pub fn new(
        drag_drop_device: Rc<dyn IDragDropDevice>,
        window: Weak<dyn IXdndWindow>,
        window_handle: XID,
        connection: Rc<dyn IXdndTargetConnection>,
        atoms: XdndTargetAtoms,
    ) -> Self {
        connection.set_xdnd_aware(window_handle, XDND_VERSION);

        Self { drag_drop_device, window, window_handle, connection, atoms, current_drag: RefCell::new(None) }
    }

    fn current_drag_of(&self, source_window: XID) -> Option<Rc<DragDropDataTransfer>> {
        self.current_drag.borrow().clone().filter(|drag| drag.source_window() == source_window)
    }

    pub fn on_xdnd_enter(&self, message: &XClientMessageEvent) {
        let Some(input_root) = self.window.upgrade().and_then(|window| window.input_root()) else {
            return;
        };

        // Spec: If the version number in the XdndEnter message is higher than what the target can support,
        // the target should ignore the source.
        let enter = XdndEnter::parse(message_data(message));
        if enter.version < MIN_XDND_VERSION || enter.version > XDND_VERSION {
            return;
        }

        // If we ever receive a new XdndEnter message while a drag is in progress, it means something went wrong.
        // In this case, assume the old drag is stale.
        self.dispose_current_drag();

        let mut formats: Vec<Atom> = Vec::new();
        let mut add = |format: Atom| {
            if format != 0 && !formats.contains(&format) {
                formats.push(format);
            }
        };

        match enter.has_extra_formats.then(|| self.connection.get_type_list(enter.source_window)).flatten() {
            Some(format_list) => format_list.into_iter().for_each(&mut add),
            None => enter.formats.iter().copied().for_each(&mut add),
        }

        let (reader, data_formats) = self.connection.create_reader(&formats, self.window_handle);
        *self.current_drag.borrow_mut() = Some(Rc::new(DragDropDataTransfer::new(
            reader,
            data_formats,
            enter.source_window,
            self.window_handle,
            input_root,
        )));
    }

    pub fn on_xdnd_position(&self, message: &XClientMessageEvent) {
        let data = message_data(message);
        let Some(drag) = self.current_drag_of(data[0] as XID) else {
            return;
        };
        let Some(window) = self.window.upgrade() else {
            return;
        };

        let position = window.point_to_client(position_from_message(data[2]));
        let requested_effects = self.atoms.actions.action_to_effects(data[4] as Atom);
        let event_type =
            if drag.last_position().is_none() { RawDragEventType::DragEnter } else { RawDragEventType::DragOver };
        let modifiers = self.get_modifiers();

        drag.set_last_position(Some(position));
        drag.set_last_timestamp(data[3] as xlib::Time);

        let drag_event = RawDragEvent::new(
            self.drag_drop_device.clone(),
            event_type,
            drag.input_root().clone(),
            position,
            drag.data_transfer(),
            requested_effects,
            modifiers,
        );

        self.drag_drop_device.process_raw_event(&drag_event);

        drag.set_result_effects(drag_event.effects());
        let result_action = self.atoms.actions.effects_to_action(drag_event.effects());
        self.send_xdnd_message(
            self.atoms.status,
            &drag,
            if result_action == 0 { 0 } else { 1 },
            0,
            0,
            result_action as c_long,
        );
    }

    pub fn on_xdnd_leave(&self, message: &XClientMessageEvent) {
        let Some(drag) = self.current_drag_of(message_data(message)[0] as XID) else {
            return;
        };

        let modifiers = self.get_modifiers();

        let drag_leave = RawDragEvent::new(
            self.drag_drop_device.clone(),
            RawDragEventType::DragLeave,
            drag.input_root().clone(),
            Point::default(),
            drag.data_transfer(),
            DragDropEffects::NONE,
            modifiers,
        );

        self.drag_drop_device.process_raw_event(&drag_leave);

        self.dispose_current_drag();
    }

    pub fn on_xdnd_drop(&self, message: &XClientMessageEvent) {
        let Some(drag) = self.current_drag_of(message_data(message)[0] as XID) else {
            return;
        };

        let modifiers = self.get_modifiers();

        let drop = RawDragEvent::new(
            self.drag_drop_device.clone(),
            RawDragEventType::Drop,
            drag.input_root().clone(),
            drag.last_position().unwrap_or_default(),
            drag.data_transfer(),
            drag.result_effects(),
            modifiers,
        );

        self.drag_drop_device.process_raw_event(&drop);

        drag.set_result_effects(drop.effects());
        drag.set_dropped(true);

        self.dispose_current_drag();
    }

    fn send_xdnd_message(
        &self,
        message_type: Atom,
        drag: &DragDropDataTransfer,
        ptr2: c_long,
        ptr3: c_long,
        ptr4: c_long,
        ptr5: c_long,
    ) {
        self.connection.send_client_message(
            drag.source_window(),
            message_type,
            [drag.target_window() as c_long, ptr2, ptr3, ptr4, ptr5],
        );
    }

    fn get_modifiers(&self) -> RawInputModifiers {
        self.connection.query_modifiers(self.window_handle)
    }

    fn dispose_current_drag(&self) {
        let Some(drag) = self.current_drag.borrow_mut().take() else {
            return;
        };

        if drag.dropped() {
            let result_action = self.atoms.actions.effects_to_action(drag.result_effects());
            self.send_xdnd_message(
                self.atoms.finished,
                &drag,
                if result_action == 0 { 0 } else { 1 },
                result_action as c_long,
                0,
                0,
            );
        }

        drag.dispose();
    }
}

#[cfg(test)]
mod tests;
