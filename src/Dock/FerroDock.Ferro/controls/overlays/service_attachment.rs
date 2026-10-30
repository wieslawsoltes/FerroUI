// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// The source AttachService/DetachService helpers. Never hold a borrow across
// service accessors or notifications: those may reenter a control's properties.
pub(super) struct ServiceAttachment<T: INotifyPropertyChanged + ?Sized> {
    service: RefCell<Option<Rc<T>>>,
    token: Cell<Option<u64>>,
}
impl<T: INotifyPropertyChanged + ?Sized> ServiceAttachment<T> {
    pub(super) fn new() -> Self {
        Self {
            service: RefCell::new(None),
            token: Cell::new(None),
        }
    }
    pub(super) fn current(&self) -> Option<Rc<T>> {
        self.service.borrow().clone()
    }
    pub(super) fn attach(&self, service: Option<Rc<T>>, handler: Rc<dyn Fn(&str)>) {
        let old = self.current();
        if match (&old, &service) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        } {
            return;
        }
        if let Some(old) = old {
            if let Some(token) = self.token.take() {
                old.property_changed().remove(token);
            }
        }
        self.service.replace(service);
        if let Some(service) = self.current() {
            self.token
                .set(Some(service.property_changed().add(handler)));
        }
    }
    pub(super) fn detach(&self) {
        if let Some(service) = self.current() {
            if let Some(token) = self.token.take() {
                service.property_changed().remove(token);
            }
            self.service.replace(None);
        }
    }
}
impl<T: INotifyPropertyChanged + ?Sized> Drop for ServiceAttachment<T> {
    fn drop(&mut self) {
        self.detach();
    }
}
