// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{data::BindingOperations, Ref};
use ferroui_controls::{primitives::FlyoutBase, Button};
use std::cell::RefCell;
use std::rc::Rc;
/// Original chrome-unload flyout retention and binding-aware reattachment.
pub(crate) struct ToolMenuLifetime {
    button: RefCell<Option<Ref<Button>>>,
    detached_flyout: RefCell<Option<Ref<FlyoutBase>>>,
}
impl ToolMenuLifetime {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            button: RefCell::new(None),
            detached_flyout: RefCell::new(None),
        })
    }
    pub(crate) fn set_button(&self, button: Option<Ref<Button>>) {
        self.detach();
        let previous = self.button.replace(button);
        drop(previous);
        let previous = self.detached_flyout.replace(None);
        drop(previous);
    }
    pub(crate) fn attach(&self) {
        let button = self.button.borrow().clone();
        if let Some(button) = button {
            if button.flyout().is_none() {
                let flyout = self.detached_flyout.borrow().clone();
                if let Some(flyout) = flyout {
                    if let Some(binding) = BindingOperations::get_binding_expression_base(
                        &button,
                        Button::flyout_property().as_property(),
                    ) {
                        binding.update_target();
                    } else {
                        button.set_current_value(Button::flyout_property(), Some(flyout));
                    }
                }
            }
        }
        let previous = self.detached_flyout.replace(None);
        drop(previous);
    }
    pub(crate) fn detach(&self) {
        let button = self.button.borrow().clone();
        let Some(button) = button else {
            return;
        };
        let Some(flyout) = button.flyout() else {
            return;
        };
        let previous = self.detached_flyout.replace(Some(flyout));
        drop(previous);
        button.set_current_value(Button::flyout_property(), None);
    }
}
