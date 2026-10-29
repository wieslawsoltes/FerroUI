// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::collections::ListHandle;
use crate::core::events::*;
use crate::core::*;
use crate::services::IWindowLifecycleService;
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};

#[path = "factory_base_close_range.rs"]
mod close_range;
#[path = "factory_base_close.rs"]
mod closing;
#[path = "factory_base_collapse.rs"]
mod collapse;
#[path = "factory_base_collection.rs"]
mod collection;
#[path = "factory_base_create_window.rs"]
mod create_window;
#[path = "factory_base_document_layout.rs"]
mod document_layout;
#[path = "factory_base_empty.rs"]
mod empty;
#[path = "factory_base_events.rs"]
pub(crate) mod events;
#[path = "factory_base_float.rs"]
mod floating;
#[path = "factory_base_focus.rs"]
mod focus;
#[path = "factory_base_hide.rs"]
mod hide;
#[path = "factory_base_hooks.rs"]
mod hooks;
#[path = "factory_base_init.rs"]
mod init;
#[path = "factory_base_items_source.rs"]
mod items_source;
#[path = "factory_base_lifecycle.rs"]
mod lifecycle;
#[path = "factory_base_locator.rs"]
mod locator;
#[path = "factory_base_metadata.rs"]
mod metadata;
#[path = "factory_base_move.rs"]
mod movement;
#[path = "factory_base_ownership.rs"]
mod ownership;
#[path = "factory_base_pin.rs"]
mod pin;
#[path = "factory_base_preview.rs"]
mod preview;
#[path = "factory_query.rs"]
mod query;
#[path = "factory_base_remove.rs"]
mod removal;
#[path = "factory_base_split.rs"]
mod split;
#[path = "factory_base_state.rs"]
mod state;
#[path = "factory_base_state_sync.rs"]
mod state_sync;
#[path = "factory_base_tracking.rs"]
mod tracking;
#[path = "factory_base_window.rs"]
mod window;
#[path = "factory_base_window_options.rs"]
mod window_options;
pub use hooks::FactoryBaseImpl;
pub use query::{DockableQuery, DockableQueryEnumerator};

type DockableListCreator =
    Box<dyn Fn(Vec<Option<Rc<dyn IDockable>>>) -> ListHandle<Option<Rc<dyn IDockable>>>>;
type WindowListCreator =
    Box<dyn Fn(Vec<Option<Rc<dyn IDockWindow>>>) -> ListHandle<Option<Rc<dyn IDockWindow>>>>;

/// The shared state and implemented algorithms of the source abstract factory.
/// Compose it into a model implementing the complete IFactory contract. Pending
/// source members are not exposed as replacement implementations or stubs.
pub struct FactoryBase {
    runtime: Weak<dyn IFactory>,
    hooks: Weak<dyn FactoryBaseImpl>,
    state_sync_depth: Cell<usize>,
    state_coalescing: RefCell<HashSet<usize>>,
    global_tracking: RefCell<GlobalDockTrackingState>,
    tracking_changed: Rc<Event<GlobalDockTrackingChangedEventArgs>>,
    dockable_initialized: Rc<Event<DockableInitEventArgs>>,
    active_changed: Rc<Event<ActiveDockableChangedEventArgs>>,
    focused_changed: Rc<Event<FocusedDockableChangedEventArgs>>,
    activated: Rc<Event<DockableActivatedEventArgs>>,
    deactivated: Rc<Event<DockableDeactivatedEventArgs>>,
    window_activated: Rc<Event<WindowActivatedEventArgs>>,
    window_deactivated: Rc<Event<WindowDeactivatedEventArgs>>,
    lifecycle_services: ListHandle<Option<Rc<dyn IWindowLifecycleService>>>,
    dockable_added: Rc<Event<DockableAddedEventArgs>>,
    dockable_removed: Rc<Event<DockableRemovedEventArgs>>,
    dockable_closing: Rc<Event<DockableClosingEventArgs>>,
    dockable_closed: Rc<Event<DockableClosedEventArgs>>,
    dockable_moved: Rc<Event<DockableMovedEventArgs>>,
    dockable_docked: Rc<Event<DockableDockedEventArgs>>,
    dockable_undocked: Rc<Event<DockableUndockedEventArgs>>,
    dockable_swapped: Rc<Event<DockableSwappedEventArgs>>,
    dockable_pinned: Rc<Event<DockablePinnedEventArgs>>,
    dockable_unpinned: Rc<Event<DockableUnpinnedEventArgs>>,
    dockable_hidden: Rc<Event<DockableHiddenEventArgs>>,
    dockable_restored: Rc<Event<DockableRestoredEventArgs>>,
    window_opened: Rc<Event<WindowOpenedEventArgs>>,
    window_closing: Rc<Event<WindowClosingEventArgs>>,
    window_closed: Rc<Event<WindowClosedEventArgs>>,
    window_added: Rc<Event<WindowAddedEventArgs>>,
    window_removed: Rc<Event<WindowRemovedEventArgs>>,
    window_move_drag_begin: Rc<Event<WindowMoveDragBeginEventArgs>>,
    window_move_drag: Rc<Event<WindowMoveDragEventArgs>>,
    window_move_drag_end: Rc<Event<WindowMoveDragEndEventArgs>>,
    create_dockable_list: DockableListCreator,
    create_window_list: WindowListCreator,
    default_context_locator: RefCell<Option<ContextFactory>>,
    default_host_window_locator: RefCell<Option<HostWindowFactory>>,
    context_locator: RefCell<Option<ContextLocatorMap>>,
    host_window_locator: RefCell<Option<HostWindowLocatorMap>>,
    dockable_locator: RefCell<Option<DockableLocatorMap>>,
    hide_tools_on_close: Cell<bool>,
    hide_documents_on_close: Cell<bool>,
    items_source: items_source::ItemsSourceTracking,
}

impl FactoryBase {
    /// Constructs the abstract component with the full enclosing factory identity.
    /// The typed thunk preserves a concrete override of generic CreateList when
    /// source initialization needs a dockable list through the erased factory.
    pub fn construct<T: FactoryBaseImpl + 'static>(runtime: Weak<T>) -> Self {
        let creator = runtime.clone();
        let window_creator = runtime.clone();
        let hooks: Weak<dyn FactoryBaseImpl> = runtime.clone();
        let runtime: Weak<dyn IFactory> = runtime;
        Self {
            runtime,
            hooks,
            state_sync_depth: Cell::new(0),
            state_coalescing: RefCell::new(HashSet::new()),
            global_tracking: RefCell::new(GlobalDockTrackingState::empty()),
            tracking_changed: Rc::new(Event::new()),
            dockable_initialized: Rc::new(Event::new()),
            active_changed: Rc::new(Event::new()),
            focused_changed: Rc::new(Event::new()),
            activated: Rc::new(Event::new()),
            deactivated: Rc::new(Event::new()),
            window_activated: Rc::new(Event::new()),
            window_deactivated: Rc::new(Event::new()),
            lifecycle_services: ListHandle::from_plain(Vec::new()),
            dockable_added: Rc::new(Event::new()),
            dockable_removed: Rc::new(Event::new()),
            dockable_closing: Rc::new(Event::new()),
            dockable_closed: Rc::new(Event::new()),
            dockable_moved: Rc::new(Event::new()),
            dockable_docked: Rc::new(Event::new()),
            dockable_undocked: Rc::new(Event::new()),
            dockable_swapped: Rc::new(Event::new()),
            dockable_pinned: Rc::new(Event::new()),
            dockable_unpinned: Rc::new(Event::new()),
            dockable_hidden: Rc::new(Event::new()),
            dockable_restored: Rc::new(Event::new()),
            window_opened: Rc::new(Event::new()),
            window_closing: Rc::new(Event::new()),
            window_closed: Rc::new(Event::new()),
            window_added: Rc::new(Event::new()),
            window_removed: Rc::new(Event::new()),
            window_move_drag_begin: Rc::new(Event::new()),
            window_move_drag: Rc::new(Event::new()),
            window_move_drag_end: Rc::new(Event::new()),
            create_dockable_list: Box::new(move |items| {
                creator
                    .upgrade()
                    .expect("the enclosing factory is alive")
                    .create_list(items)
            }),
            create_window_list: Box::new(move |items| {
                window_creator
                    .upgrade()
                    .expect("the enclosing factory is alive")
                    .create_list(items)
            }),
            default_context_locator: RefCell::new(None),
            default_host_window_locator: RefCell::new(None),
            context_locator: RefCell::new(None),
            host_window_locator: RefCell::new(None),
            dockable_locator: RefCell::new(None),
            hide_tools_on_close: Cell::new(false),
            hide_documents_on_close: Cell::new(false),
            items_source: items_source::ItemsSourceTracking::default(),
        }
    }

    fn runtime(&self) -> Rc<dyn IFactory> {
        self.runtime
            .upgrade()
            .expect("the enclosing factory is alive while its component is borrowed")
    }

    fn hooks(&self) -> Rc<dyn FactoryBaseImpl> {
        self.hooks
            .upgrade()
            .expect("the enclosing factory is alive")
    }
}

#[cfg(test)]
#[path = "factory_base_tests.rs"]
mod tests;
