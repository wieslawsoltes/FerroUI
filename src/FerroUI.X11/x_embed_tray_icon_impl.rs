//! The tray icon without a session bus (the port of
//! `XEmbedTrayIconImpl.cs`). The reference has no tray icon over the
//! XEmbed system tray protocol: this class logs that once and does
//! nothing else.

use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_controls::platform::{INativeMenuExporter, ITrayIconImpl, IWindowIconImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
pub struct XEmbedTrayIconImpl {
    is_called: Cell<bool>,
    on_clicked: RefCell<Option<Rc<dyn Fn()>>>,
}

impl XEmbedTrayIconImpl {
    pub fn new() -> Self {
        Self::default()
    }

    fn not_implemented(&self) {
        if self.is_called.get() {
            return;
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::X11_PLATFORM) {
            logger.log(
                None,
                "TODO: XEmbed System Tray Icons is not implemented yet. Tray icons won't be available on this system.",
            );
        }

        self.is_called.set(true);
    }

    /// Whether the message was logged.
    pub fn is_called(&self) -> bool {
        self.is_called.get()
    }
}

impl IDisposable for XEmbedTrayIconImpl {
    fn dispose(&self) {
        self.not_implemented();
    }
}

impl ITrayIconImpl for XEmbedTrayIconImpl {
    fn set_icon(&self, _icon: Option<Rc<dyn IWindowIconImpl>>) {
        self.not_implemented();
    }

    fn set_tool_tip_text(&self, _text: Option<&str>) {
        self.not_implemented();
    }

    fn set_is_visible(&self, _visible: bool) {
        self.not_implemented();
    }

    fn menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
        None
    }

    fn on_clicked(&self) -> Option<Rc<dyn Fn()>> {
        self.on_clicked.borrow().clone()
    }

    fn set_on_clicked(&self, value: Option<Rc<dyn Fn()>>) {
        *self.on_clicked.borrow_mut() = value;
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;

    #[test]
    fn every_member_does_nothing_but_remember_that_it_was_called() {
        let tray_icon = XEmbedTrayIconImpl::new();
        assert!(!tray_icon.is_called());
        assert!(tray_icon.menu_exporter().is_none());
        // The click action is a property like any other.
        tray_icon.set_on_clicked(Some(Rc::new(|| {})));
        assert!(tray_icon.on_clicked().is_some());
        assert!(!tray_icon.is_called());

        tray_icon.set_is_visible(true);
        assert!(tray_icon.is_called());
        tray_icon.set_icon(None);
        tray_icon.set_tool_tip_text(Some("text"));
        tray_icon.dispose();
        assert!(tray_icon.is_called());
    }
}
