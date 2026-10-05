use super::IApplicationLifetime;
use crate::Control;
use ferroui_base::Ref;

/// The lifetime of an application that shows a single view (mobile, browser
/// and embedded platforms).
pub trait ISingleViewApplicationLifetime: IApplicationLifetime {
    /// The main view of the application.
    fn main_view(&self) -> Option<Ref<Control>>;

    /// Sets the main view of the application.
    fn set_main_view(&self, value: Option<Ref<Control>>);
}
