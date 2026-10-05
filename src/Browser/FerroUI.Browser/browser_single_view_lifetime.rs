use crate::ferro_view::FerroView;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{
    IApplicationLifetime, ISingleTopLevelApplicationLifetime, ISingleViewApplicationLifetime,
};
use ferroui_controls::{Control, TopLevel};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The lifetime of an application that shows one view in a web page.
#[derive(Default)]
pub struct BrowserSingleViewLifetime {
    view: RefCell<Option<Rc<FerroView>>>,
}

impl BrowserSingleViewLifetime {
    /// Creates a lifetime without a view; the start-up of the platform
    /// sets it.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The view the main view is shown in.
    pub fn view(&self) -> Option<Rc<FerroView>> {
        self.view.borrow().clone()
    }

    /// Sets the view the main view is shown in.
    pub fn set_view(&self, view: Option<Rc<FerroView>>) {
        *self.view.borrow_mut() = view;
    }

    fn ensure_view(&self) -> Rc<FerroView> {
        match self.view() {
            Some(view) => view,
            None => panic!(
                "Browser lifetime was not initialized. Make sure AppBuilder::start_browser_app was called."
            ),
        }
    }
}

impl IApplicationLifetime for BrowserSingleViewLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }

    fn as_single_top_level_application_lifetime(&self) -> Option<&dyn ISingleTopLevelApplicationLifetime> {
        Some(self)
    }
}

impl ISingleViewApplicationLifetime for BrowserSingleViewLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.ensure_view().content()
    }

    fn set_main_view(&self, value: Option<Ref<Control>>) {
        self.ensure_view().set_content(value);
    }
}

impl ISingleTopLevelApplicationLifetime for BrowserSingleViewLifetime {
    fn top_level(&self) -> Option<Ref<TopLevel>> {
        self.view().map(|view| view.top_level())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lifetime_without_a_view_has_no_top_level() {
        let lifetime = BrowserSingleViewLifetime::new();

        assert!(lifetime.view().is_none());
        assert!(lifetime.top_level().is_none());
    }

    #[test]
    fn the_lifetime_is_a_single_view_and_single_top_level_lifetime() {
        let lifetime = BrowserSingleViewLifetime::new();
        let lifetime: &dyn IApplicationLifetime = &*lifetime;

        assert!(lifetime.as_single_view_application_lifetime().is_some());
        assert!(lifetime.as_single_top_level_application_lifetime().is_some());
        assert!(lifetime.as_controlled_application_lifetime().is_none());
        assert!(lifetime.as_classic_desktop_style_application_lifetime().is_none());
    }

    #[test]
    #[should_panic(expected = "Browser lifetime was not initialized")]
    fn reading_the_main_view_before_the_view_exists_fails() {
        BrowserSingleViewLifetime::new().main_view();
    }

    #[test]
    #[should_panic(expected = "Browser lifetime was not initialized")]
    fn setting_the_main_view_before_the_view_exists_fails() {
        BrowserSingleViewLifetime::new().set_main_view(None);
    }
}
