use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{
    IActivityApplicationLifetime, IApplicationLifetime, ISingleViewApplicationLifetime,
};
use ferroui_controls::Control;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The lifetime of an Android application: every main activity asks the
/// factory for its main view.
#[derive(Default)]
pub struct ApplicationLifetime {
    main_view: RefCell<Option<Ref<Control>>>,
    main_view_factory: RefCell<Option<Rc<dyn Fn() -> Ref<Control>>>>,
}

impl ApplicationLifetime {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

impl IApplicationLifetime for ApplicationLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }

    fn as_activity_application_lifetime(&self) -> Option<&dyn IActivityApplicationLifetime> {
        Some(self)
    }
}

impl IActivityApplicationLifetime for ApplicationLifetime {
    fn main_view_factory(&self) -> Option<Rc<dyn Fn() -> Ref<Control>>> {
        self.main_view_factory.borrow().clone()
    }

    fn set_main_view_factory(&self, value: Option<Rc<dyn Fn() -> Ref<Control>>>) {
        let previous = self.main_view_factory.replace(value);
        drop(previous);
    }
}

impl ISingleViewApplicationLifetime for ApplicationLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.main_view.borrow().clone()
    }

    fn set_main_view(&self, value: Option<Ref<Control>>) {
        let previous = self.main_view.replace(value.clone());
        drop(previous);

        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::ANDROID_PLATFORM) {
            logger.log(
                None,
                "ISingleViewApplicationLifetime.MainView is not fully supported on Android. \
                 Consider setting IActivityApplicationLifetime.MainViewFactory.",
            );
        }
        match value {
            Some(main_view) => self.set_main_view_factory(Some(Rc::new(move || main_view.clone()))),
            None => self.set_main_view_factory(None),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the lifetime.
    use super::*;

    #[test]
    fn the_lifetime_is_a_single_view_and_an_activity_lifetime() {
        let lifetime = ApplicationLifetime::new();
        let lifetime: &dyn IApplicationLifetime = &*lifetime;

        assert!(lifetime.as_single_view_application_lifetime().is_some());
        assert!(lifetime.as_activity_application_lifetime().is_some());
        assert!(lifetime.as_controlled_application_lifetime().is_none());
        assert!(lifetime.as_classic_desktop_style_application_lifetime().is_none());
    }

    #[test]
    fn a_lifetime_starts_without_a_view_and_without_a_factory() {
        let lifetime = ApplicationLifetime::new();

        assert!(lifetime.main_view().is_none());
        assert!(lifetime.main_view_factory().is_none());
    }

    #[test]
    fn clearing_the_main_view_clears_the_factory() {
        let lifetime = ApplicationLifetime::new();
        lifetime.set_main_view_factory(Some(Rc::new(|| panic!("not called"))));

        lifetime.set_main_view(None);

        assert!(lifetime.main_view_factory().is_none());
    }
}
