//! Port of `ViewModels/NetworkViewModel.cs` of the reference sample.

use super::{Command, ViewModelBase};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_reactiveui::reactive_object;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The view model of the network page: the address of its request, the response, and whether the
/// network is available.
pub struct NetworkViewModel {
    base: ViewModelBase,
    url: RefCell<Option<String>>,
    response_content: RefCell<Option<String>>,
    response_status_code: RefCell<i32>,
    is_network_available: RefCell<bool>,
    update_network_status_command: RefCell<Option<Rc<dyn ICommand>>>,
}
reactive_object!(NetworkViewModel, base);
super::reactive_accessors!(NetworkViewModel {
    url, set_url, "Url": Option<String>;
    response_content, set_response_content, "ResponseContent": Option<String>;
    response_status_code, set_response_status_code, "ResponseStatusCode": i32;
    is_network_available, set_is_network_available, "IsNetworkAvailable": bool;
});

impl PartialEq for NetworkViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl NetworkViewModel {
    /// `NetworkViewModel()`.
    pub fn new() -> Rc<Self> {
        let view_model = Rc::new(Self {
            base: ViewModelBase::new(),
            url: RefCell::new(Some("https://httpbin.org/get".to_string())),
            response_content: RefCell::new(None),
            response_status_code: RefCell::new(0),
            is_network_available: RefCell::new(false),
            update_network_status_command: RefCell::new(None),
        });
        let weak: Weak<Self> = Rc::downgrade(&view_model);
        *view_model.update_network_status_command.borrow_mut() =
            Some(Command::create_with(move |parameter| {
                let is_available = parameter
                    .and_then(|parameter| parameter.downcast_ref::<bool>())
                    .copied()
                    .unwrap_or_default();
                if let Some(view_model) = weak.upgrade() {
                    view_model.update_network_status(is_available);
                }
            }));
        view_model
    }

    /// `UpdateNetworkStatusCommand`.
    pub fn update_network_status_command(&self) -> Option<Rc<dyn ICommand>> {
        self.update_network_status_command.borrow().clone()
    }

    /// `UpdateNetworkStatus(bool)`.
    fn update_network_status(&self, is_available: bool) {
        self.set_is_network_available(is_available);
    }
}

type Vm = Rc<NetworkViewModel>;

ferro_markup_type!(class NetworkViewModel {
    this: Rc<NetworkViewModel>,
    handles: [NetworkViewModel, Rc<NetworkViewModel>, Option<Rc<NetworkViewModel>>],
    constructors: [() => NetworkViewModel::new],
    properties: [
        Url: Option<String> { get: |this: &Vm| this.url(), set: |this: &Vm, value: Option<String>| this.set_url(value) },
        ResponseContent: Option<String> { get: |this: &Vm| this.response_content(), set: |this: &Vm, value: Option<String>| this.set_response_content(value) },
        ResponseStatusCode: i32 { get: |this: &Vm| this.response_status_code(), set: |this: &Vm, value: i32| this.set_response_status_code(value) },
        IsNetworkAvailable: bool { get: |this: &Vm| this.is_network_available(), set: |this: &Vm, value: bool| this.set_is_network_available(value) },
        UpdateNetworkStatusCommand: Option<Rc<dyn ICommand>> { get: |this: &Vm| this.update_network_status_command() },
    ],
    notify_property_changed: NetworkViewModel,
});
