// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::helpers::debug_overlay_helper::DebugOverlayHelper;
use ferrodock_ferro::controls::DockControl;
use ferroui_base::{reactive::IDisposable, Ref, Visual};
use ferroui_controls::{Control, TopLevel};
use ferroui_system::collections::generic::Dictionary;
use std::{cell::RefCell, rc::Rc};

pub(crate) struct DockDebugOverlayManager {
    top_level: Ref<TopLevel>,
    helpers: RefCell<Dictionary<Ref<DockControl>, Rc<DebugOverlayHelper>>>,
    dock_subscriptions: RefCell<Dictionary<Ref<DockControl>, Rc<dyn IDisposable>>>,
    root_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}
impl DockDebugOverlayManager {
    pub(crate) fn new(top_level: Ref<TopLevel>) -> Rc<Self> {
        let this = Rc::new(Self {
            top_level,
            helpers: RefCell::new(Dictionary::new()),
            dock_subscriptions: RefCell::new(Dictionary::new()),
            root_subscription: RefCell::new(None),
        });
        this.attach_existing(&this.top_level);
        let owner = this.clone();
        this.root_subscription.replace(Some(
            this.top_level
                .layout_updated(move || owner.on_root_layout_updated()),
        ));
        this
    }
    fn attach_existing(self: &Rc<Self>, root: &Visual) {
        let docks: Vec<_> = root
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<DockControl>())
            .collect();
        for dock in docks {
            self.attach_overlay(dock);
        }
    }
    fn on_root_layout_updated(self: &Rc<Self>) {
        let current: Vec<_> = self
            .top_level
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<DockControl>())
            .collect();
        for dock in &current {
            if !self.helpers.borrow().contains_key(dock) {
                self.attach_overlay(dock.clone());
            }
        }
        let previous: Vec<_> = self.helpers.borrow().keys().cloned().collect();
        for dock in previous {
            if !current.contains(&dock) {
                self.remove_overlay(&dock);
            }
        }
    }
    fn attach_overlay(self: &Rc<Self>, dock: Ref<DockControl>) {
        if self.helpers.borrow().contains_key(&dock) {
            return;
        }
        let helper = DebugOverlayHelper::new();
        self.helpers
            .borrow_mut()
            .insert(dock.clone(), helper.clone());
        helper.add_overlay(dock.clone().upcast::<Control>());
        let owner = self.clone();
        let sender = dock.clone();
        let subscription = dock.layout_updated(move || owner.on_dock_layout_updated(&sender));
        self.dock_subscriptions
            .borrow_mut()
            .insert(dock, subscription);
    }
    fn remove_overlay(&self, dock: &Ref<DockControl>) {
        let helper = self.helpers.borrow().get(dock).cloned();
        if let Some(helper) = helper {
            let subscription = self.dock_subscriptions.borrow_mut().remove(dock);
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            helper.remove_overlay(dock);
            self.helpers.borrow_mut().remove(dock);
        }
    }
    fn on_dock_layout_updated(&self, dock: &Ref<DockControl>) {
        let helper = self.helpers.borrow().get(dock).cloned();
        if let Some(helper) = helper {
            helper.invalidate();
        }
    }
}
impl IDisposable for DockDebugOverlayManager {
    fn dispose(&self) {
        let subscription = self.root_subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let pairs: Vec<_> = self
            .helpers
            .borrow()
            .iter()
            .map(|(dock, helper)| (dock.clone(), helper.clone()))
            .collect();
        for (dock, helper) in pairs {
            let subscription = self.dock_subscriptions.borrow_mut().remove(&dock);
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            helper.remove_overlay(&dock);
        }
        self.helpers.borrow_mut().clear();
    }
}
