// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original inspector logging/filtering state, independent of UI types.
use ferrodock_model::core::IFactory;
use ferrodock_model::source_string::ordinal_ignore_case_contains;
use ferroui_reactiveui::{
    reactive_object, ICommand, IReactiveObject, ReactiveCommand, ReactiveObject,
};
use ferroui_system::{collections::object_model::ObservableCollection, IDisposable};
use std::{cell::RefCell, rc::Rc};
/// Local clock contract used by the original event timestamp format.
pub trait IEventLogClock {
    /// Gets the source local HH:mm:ss.fff timestamp.
    fn local_timestamp(&self) -> String;
}
/// Reactive presentation state extracted from the original inspector.
pub struct RootDockDebugViewModel {
    base: ReactiveObject,
    events: ObservableCollection<String>,
    events_identity: Rc<()>,
    filter: RefCell<Option<String>>,
    clock: Rc<dyn IEventLogClock>,
    subscriptions: RefCell<Option<Vec<Rc<dyn IDisposable>>>>,
    clear: Rc<dyn ICommand>,
}
reactive_object!(RootDockDebugViewModel, base);
impl RootDockDebugViewModel {
    /// Creates source state with an injected local clock.
    pub fn new(clock: Rc<dyn IEventLogClock>) -> Rc<Self> {
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let weak = weak.clone();
            Self {
                base: ReactiveObject::new(),
                events: ObservableCollection::new(),
                events_identity: Rc::new(()),
                filter: RefCell::new(None),
                clock,
                subscriptions: RefCell::new(None),
                clear: Rc::new(ReactiveCommand::create(move || {
                    if let Some(model) = weak.upgrade() {
                        model.clear_events();
                    }
                })),
            }
        })
    }
    /// Gets the original shared, externally mutable event collection.
    pub fn events(&self) -> ObservableCollection<String> {
        self.events.clone()
    }
    /// Gets the original nullable filter.
    pub fn filter(&self) -> Option<String> {
        self.filter.borrow().clone()
    }
    /// Sets the filter and raises Filter then FilteredEvents only when changed.
    pub fn set_filter(&self, value: Option<String>) {
        if *self.filter.borrow() == value {
            return;
        }
        self.filter.replace(value);
        self.raise_property_changed("Filter");
        self.raise_property_changed("FilteredEvents");
    }
    /// Gets the original lazy enumerable over the same event collection.
    pub fn filtered_events(self: &Rc<Self>) -> EventLogView {
        if self.filter.borrow().as_ref().is_none_or(String::is_empty) {
            EventLogView {
                events: self.events.clone(),
                owner: None,
                identity: self.events_identity.clone(),
            }
        } else {
            EventLogView {
                events: self.events.clone(),
                owner: Some(self.clone()),
                identity: Rc::new(()),
            }
        }
    }
    /// Gets the command replacing the source view's clear-event handler.
    pub fn clear_events_command(&self) -> Rc<dyn ICommand> {
        self.clear.clone()
    }
    /// Clears the source log and invalidates its filtered enumerable.
    pub fn clear_events(&self) {
        self.events.clear();
        self.raise_property_changed("FilteredEvents");
    }
    /// Adds the original local timestamp and raises the source filtered-property event.
    pub fn add_event(&self, message: &str) {
        self.events
            .add(format!("{} {message}", self.clock.local_timestamp()));
        self.raise_property_changed("FilteredEvents");
    }
    /// Replaces all original factory subscriptions on a DataContext transition.
    pub fn set_factory(self: &Rc<Self>, factory: Option<Rc<dyn IFactory>>) {
        let previous = self.subscriptions.borrow().clone();
        if let Some(previous) = previous {
            for subscription in previous {
                subscription.dispose();
            }
            self.subscriptions.replace(None);
        }
        let Some(factory) = factory else {
            return;
        };
        let mut subscriptions = Vec::with_capacity(26);
        let model = self.clone();
        subscriptions.push(factory.active_dockable_changed(Rc::new(move |event| {
            model.add_event(&format!(
                "ActiveDockableChanged {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.focused_dockable_changed(Rc::new(move |event| {
            model.add_event(&format!(
                "FocusedDockableChanged {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_init(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableInit {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_added(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableAdded {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_removed(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableRemoved {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_closed(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableClosed {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_moved(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableMoved {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_docked(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableDocked {} {:?}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default(),
                event.operation()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_undocked(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableUndocked {} {:?}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default(),
                event.operation()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_swapped(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableSwapped {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_pinned(Rc::new(move |event| {
            model.add_event(&format!(
                "DockablePinned {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_unpinned(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableUnpinned {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_hidden(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableHidden {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_restored(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableRestored {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_opened(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowOpened {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_closing(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowClosing {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_closed(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowClosed {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_added(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowAdded {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_removed(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowRemoved {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_move_drag_begin(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowMoveDragBegin {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_move_drag(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowMoveDrag {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_move_drag_end(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowMoveDragEnd {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_activated(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowActivated {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_activated(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableActivated {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.window_deactivated(Rc::new(move |event| {
            model.add_event(&format!(
                "WindowDeactivated {}",
                event
                    .window()
                    .map(|window| window.title())
                    .unwrap_or_default()
            ))
        })));
        let model = self.clone();
        subscriptions.push(factory.dockable_deactivated(Rc::new(move |event| {
            model.add_event(&format!(
                "DockableDeactivated {}",
                event
                    .dockable()
                    .map(|dockable| dockable.title())
                    .unwrap_or_default()
            ))
        })));
        self.subscriptions.replace(Some(subscriptions));
    }
}
/// Native enumerable projection; filtered views retain the original captured owner.
#[derive(Clone)]
pub struct EventLogView {
    events: ObservableCollection<String>,
    owner: Option<Rc<RootDockDebugViewModel>>,
    identity: Rc<()>,
}
impl PartialEq for EventLogView {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
}
impl EventLogView {
    pub(crate) fn empty() -> Self {
        Self {
            events: ObservableCollection::new(),
            owner: None,
            identity: Rc::new(()),
        }
    }

    /// Gets the original backing collection, without a copied log.
    pub fn events(&self) -> ObservableCollection<String> {
        self.events.clone()
    }
    /// Identifies an unfiltered observable collection rather than a LINQ query.
    pub fn is_unfiltered(&self) -> bool {
        self.owner.is_none()
    }
    /// Gets native item-source identity corresponding to the original enumerable object.
    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.identity) as usize
    }
    fn matches(&self, value: &str) -> bool {
        match &self.owner {
            None => true,
            Some(owner) => ordinal_ignore_case_contains(
                value,
                owner
                    .filter()
                    .as_deref()
                    .expect("Value cannot be null. (Parameter 'value')"),
            ),
        }
    }
    /// Enumerates matching source elements lazily in their original order.
    pub fn iter(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.events.count()).filter_map(|index| {
            let value = self.events.get(index);
            self.matches(&value).then_some(value)
        })
    }
    /// Enumerates the current matching count, as an enumerable consumer does.
    pub fn count(&self) -> usize {
        if self.owner.is_none() {
            return self.events.count();
        }
        let mut count = 0;
        for index in 0..self.events.count() {
            if self.matches(&self.events.get(index)) {
                count += 1;
            }
        }
        count
    }
    /// Gets a matching element in original order without materializing a second log.
    pub fn get(&self, index: usize) -> String {
        if self.owner.is_none() {
            return self.events.get(index);
        }
        let mut current = 0;
        for position in 0..self.events.count() {
            let value = self.events.get(position);
            if self.matches(&value) {
                if current == index {
                    return value;
                }
                current += 1;
            }
        }
        panic!("Source enumerable index out of range")
    }
}
