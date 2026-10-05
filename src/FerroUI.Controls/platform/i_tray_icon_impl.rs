use super::{INativeMenuExporter, IWindowIconImpl};
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// The platform implementation of a tray icon.
pub trait ITrayIconImpl: IDisposable {
    /// Sets the icon of this tray icon.
    fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>);

    /// Sets the tooltip text of this tray icon.
    fn set_tool_tip_text(&self, text: Option<&str>);

    /// Sets if the tray icon is visible or not.
    fn set_is_visible(&self, visible: bool);

    /// Gets the menu exporter of this tray icon.
    fn menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>>;

    /// Gets the action that is called when the tray icon is clicked.
    fn on_clicked(&self) -> Option<Rc<dyn Fn()>>;

    /// Sets the action that is called when the tray icon is clicked.
    fn set_on_clicked(&self, value: Option<Rc<dyn Fn()>>);

    /// The tray icon viewed as one whose icon can be marked as a template
    /// icon, when the implementation supports that.
    fn as_tray_icon_with_is_template_impl(&self) -> Option<&dyn ITrayIconWithIsTemplateImpl> {
        None
    }
}

/// A tray icon whose icon can be marked as a template icon.
pub trait ITrayIconWithIsTemplateImpl: ITrayIconImpl {
    /// Sets if the tray icon has a template/monochrome icon or not.
    fn set_is_template_icon(&self, is_template_icon: bool);
}
