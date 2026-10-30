// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::DebugOverlayAdorner;
use ferrodock_settings::DockProperties;
use ferroui_base::{
    input::{InputElement, PointerEventArgs},
    interactivity::RoutingStrategies,
    reactive::IDisposable,
    Ref, StyledElement, WeakRef,
};
use ferroui_controls::{primitives::AdornerLayer, Control};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubscriptionKind {
    Moved,
    Leave,
    Layout,
}
struct Subscription {
    control: WeakRef<Control>,
    kind: SubscriptionKind,
    disposable: Rc<dyn IDisposable>,
}
pub(crate) struct DebugOverlayHelper {
    adorner: RefCell<Option<Ref<DebugOverlayAdorner>>>,
    control: RefCell<Option<Ref<Control>>>,
    subscriptions: RefCell<Vec<Subscription>>,
}
impl DebugOverlayHelper {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            adorner: RefCell::new(None),
            control: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        })
    }
    pub(crate) fn add_overlay(self: &Rc<Self>, control: Ref<Control>) {
        let Some(layer) = AdornerLayer::get_adorner_layer(&control) else {
            return;
        };
        self.control.replace(Some(control.clone()));
        let adorner = DebugOverlayAdorner::new();
        AdornerLayer::set_adorned_element(&adorner, control.clone());
        self.adorner.replace(Some(adorner.clone()));
        adorner.set_parent(control.clone().upcast::<StyledElement>());
        layer.children().add(adorner.upcast::<Control>());
        let routes =
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE | RoutingStrategies::DIRECT;
        let owner = self.clone();
        let moved = control.add_disposable_handler_with(
            InputElement::pointer_moved_event(),
            move |_, event| owner.on_pointer_moved(event),
            routes,
            false,
        );
        self.remember(&control, SubscriptionKind::Moved, moved);
        let owner = self.clone();
        let leave = control.add_disposable_handler_with(
            InputElement::pointer_exited_event(),
            move |_, _| owner.on_pointer_leave(),
            routes,
            false,
        );
        self.remember(&control, SubscriptionKind::Leave, leave);
        let owner = self.clone();
        let layout = control.layout_updated(move || owner.on_layout_updated());
        self.remember(&control, SubscriptionKind::Layout, layout);
    }
    pub(crate) fn remove_overlay(&self, control: &Control) {
        if self.adorner.borrow().is_none() {
            return;
        }
        if let Some(layer) = AdornerLayer::get_adorner_layer(control) {
            let adorner = self
                .adorner
                .borrow()
                .clone()
                .expect("Object reference not set to an instance of an object.");
            layer.children().remove(adorner.upcast::<Control>());
        }
        let adorner = self
            .adorner
            .borrow()
            .clone()
            .expect("Object reference not set to an instance of an object.");
        adorner.set_parent(None::<Ref<StyledElement>>);
        self.remove_subscriptions(control, SubscriptionKind::Moved, true);
        self.remove_subscriptions(control, SubscriptionKind::Leave, true);
        self.remove_subscriptions(control, SubscriptionKind::Layout, false);
        self.control.replace(None);
        self.adorner.replace(None);
    }
    fn on_pointer_moved(&self, event: &PointerEventArgs) {
        let control = self.control.borrow().clone();
        if self.adorner.borrow().is_none() {
            return;
        }
        let Some(control) = control else {
            return;
        };
        let position = event.get_position(Some(&control));
        let hit = control
            .get_visuals_at(position)
            .into_iter()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| Self::is_debug_target(control));
        let adorner = self
            .adorner
            .borrow()
            .clone()
            .expect("Object reference not set to an instance of an object.");
        adorner.set_pointer_over(hit);
    }
    fn on_pointer_leave(&self) {
        let adorner = self.adorner.borrow().clone();
        if let Some(adorner) = adorner {
            adorner.set_pointer_over(None);
        }
    }
    fn on_layout_updated(&self) {
        self.invalidate();
    }
    pub(crate) fn invalidate(&self) {
        let adorner = self.adorner.borrow().clone();
        if let Some(adorner) = adorner {
            adorner.invalidate_visual();
        }
    }
    fn is_debug_target(control: &Control) -> bool {
        DockProperties::get_is_dock_target(control)
            || DockProperties::get_is_drag_area(control)
            || DockProperties::get_is_drop_area(control)
    }
    fn remember(
        &self,
        control: &Ref<Control>,
        kind: SubscriptionKind,
        disposable: Rc<dyn IDisposable>,
    ) {
        self.subscriptions.borrow_mut().push(Subscription {
            control: control.downgrade(),
            kind,
            disposable,
        });
    }
    fn remove_subscriptions(&self, control: &Control, kind: SubscriptionKind, all: bool) {
        // Routed RemoveHandler removes all equal delegates; CLR event -= removes
        // only the last match. Weak publisher handles are undo tokens, not owners.
        loop {
            let subscription = {
                let mut subscriptions = self.subscriptions.borrow_mut();
                let index = subscriptions.iter().rposition(|item| {
                    item.kind == kind
                        && item
                            .control
                            .upgrade()
                            .is_some_and(|owner| std::ptr::eq(&*owner, control))
                });
                index.map(|index| subscriptions.remove(index))
            };
            let Some(subscription) = subscription else {
                break;
            };
            subscription.disposable.dispose();
            if !all {
                break;
            }
        }
    }
}
