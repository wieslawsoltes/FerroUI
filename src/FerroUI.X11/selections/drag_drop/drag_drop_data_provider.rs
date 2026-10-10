//! The data of a drag, offered to other clients as the selection of the
//! protocol (the port of `DragDropDataProvider.cs`).

use crate::selections::selection_data_provider::SelectionDataProvider;
use crate::x11_platform::FerroX11Platform;
use crate::xlib::{XEvent, XID};
use ferroui_base::input::IAsyncDataTransfer;
use std::rc::Rc;

pub struct DragDropDataProvider {
    base: SelectionDataProvider,
}

impl DragDropDataProvider {
    pub fn new(platform: &Rc<FerroX11Platform>, data_transfer: Rc<dyn IAsyncDataTransfer>) -> Self {
        let base = SelectionDataProvider::new(platform, platform.info().atoms().XdndSelection);
        base.set_data_transfer(Some(data_transfer));
        Self { base }
    }

    /// What is called after something was sent to the other client
    /// (`Activity`).
    pub fn set_activity(&self, activity: Option<Rc<dyn Fn()>>) {
        self.base.set_on_activity(activity);
    }

    pub fn get_owner(&self) -> XID {
        self.base.get_owner()
    }

    pub fn set_owner(&self, window: XID) {
        self.base.set_owner(window);
    }

    /// Answers the request of the drop target for the data. `evt` is the
    /// `SelectionRequest` event.
    pub fn on_selection_request(&self, evt: &XEvent) {
        self.base.on_selection_request(evt);
    }

    pub fn dispose(&self) {
        self.set_activity(None);
        if let Some(data_transfer) = self.base.data_transfer() {
            data_transfer.dispose();
        }
        self.base.set_data_transfer(None);
    }
}
