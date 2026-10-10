//! The sessions that read the clipboard (the port of
//! `ClipboardReadSessionFactory.cs`).

use crate::selections::clipboard::event_stream_window::EventStreamWindow;
use crate::selections::selection_helper::XlibSelectionConnection;
use crate::selections::selection_read_session::SelectionReadSession;
use crate::x11_enums::XEventMask;
use crate::x11_platform::FerroX11Platform;
use crate::xlib::{self, Atom};
use std::ffi::c_long;
use std::rc::Rc;

/// A session that reads `selection` into a window of its own. Whoever
/// asks for the session disposes it, which destroys the window.
pub fn create_session(platform: &Rc<FerroX11Platform>, selection: Atom) -> SelectionReadSession {
    let window = EventStreamWindow::new(platform, None);
    xlib::x_select_input(platform.display(), window.handle(), XEventMask::PROPERTY_CHANGE_MASK.bits() as c_long);

    SelectionReadSession::new(
        XlibSelectionConnection::new(platform.display()),
        window.handle(),
        selection,
        window,
        platform.info().atoms(),
    )
}
