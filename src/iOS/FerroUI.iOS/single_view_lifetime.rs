//! The lifetime of an application that shows one view.

use crate::ferro_view::FerroView;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{
    IApplicationLifetime, ISingleTopLevelApplicationLifetime, ISingleViewApplicationLifetime,
};
use ferroui_controls::{Control, TopLevel};
use objc2::rc::Retained;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The lifetime of an application whose main view is shown in the view of
/// its scene.
#[derive(Default)]
pub struct SingleViewLifetime {
    main_view: RefCell<Option<Ref<Control>>>,
    view: RefCell<Option<Retained<FerroView>>>,
}

impl SingleViewLifetime {
    /// Creates a lifetime without a view; the scene delegate sets it.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The view the main view is shown in.
    pub fn view(&self) -> Option<Retained<FerroView>> {
        self.view.borrow().clone()
    }

    /// Sets the view the main view is shown in; the view that was set
    /// before loses its content and is disposed.
    pub(crate) fn set_view(&self, value: Option<Retained<FerroView>>) {
        let old = self.view.borrow_mut().take();
        if let Some(old) = old {
            old.set_content(None);
            old.dispose();
        }
        *self.view.borrow_mut() = value.clone();
        if let Some(view) = value {
            let main_view = self.main_view.borrow().clone();
            view.set_content(main_view);
        }
    }
}

impl IApplicationLifetime for SingleViewLifetime {
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

impl ISingleViewApplicationLifetime for SingleViewLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.main_view.borrow().clone()
    }

    fn set_main_view(&self, value: Option<Ref<Control>>) {
        if *self.main_view.borrow() != value {
            *self.main_view.borrow_mut() = value.clone();
            if let Some(view) = self.view() {
                view.set_content(value);
            }
        }
    }
}

impl ISingleTopLevelApplicationLifetime for SingleViewLifetime {
    fn top_level(&self) -> Option<Ref<TopLevel>> {
        self.view().map(|view| view.top_level())
    }
}
