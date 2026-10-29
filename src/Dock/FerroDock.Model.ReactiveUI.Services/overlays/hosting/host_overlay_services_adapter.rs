// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::services::*;
use std::rc::Rc;
/// Adapts six existing service references without creating or replacing services.
pub struct HostOverlayServicesAdapter {
    busy: Rc<dyn IDockBusyService>,
    dialogs: Rc<dyn IDockDialogService>,
    confirmations: Rc<dyn IDockConfirmationService>,
    global_busy: Rc<dyn IDockGlobalBusyService>,
    global_dialog: Rc<dyn IDockGlobalDialogService>,
    global_confirmation: Rc<dyn IDockGlobalConfirmationService>,
}
impl HostOverlayServicesAdapter {
    /// Retains the supplied nonnull service references exactly as in the source constructor.
    pub fn new(
        busy: Rc<dyn IDockBusyService>,
        dialogs: Rc<dyn IDockDialogService>,
        confirmations: Rc<dyn IDockConfirmationService>,
        global_busy: Rc<dyn IDockGlobalBusyService>,
        global_dialog: Rc<dyn IDockGlobalDialogService>,
        global_confirmation: Rc<dyn IDockGlobalConfirmationService>,
    ) -> Rc<Self> {
        Rc::new(Self {
            busy,
            dialogs,
            confirmations,
            global_busy,
            global_dialog,
            global_confirmation,
        })
    }
}
impl IHostOverlayServices for HostOverlayServicesAdapter {
    fn busy(&self) -> Rc<dyn IDockBusyService> {
        self.busy.clone()
    }
    fn dialogs(&self) -> Rc<dyn IDockDialogService> {
        self.dialogs.clone()
    }
    fn confirmations(&self) -> Rc<dyn IDockConfirmationService> {
        self.confirmations.clone()
    }
    fn global_busy_service(&self) -> Rc<dyn IDockGlobalBusyService> {
        self.global_busy.clone()
    }
    fn global_dialog_service(&self) -> Rc<dyn IDockGlobalDialogService> {
        self.global_dialog.clone()
    }
    fn global_confirmation_service(&self) -> Rc<dyn IDockGlobalConfirmationService> {
        self.global_confirmation.clone()
    }
}
