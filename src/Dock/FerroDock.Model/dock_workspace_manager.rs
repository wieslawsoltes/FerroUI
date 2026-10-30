// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::IRootDock;
use crate::core::*;
use crate::dock_workspace::is_blank;
use crate::factory_base::events::subscribe;
use crate::workspace_ordinal_data::ORDINAL_UPPER;
use crate::{
    DockState, DockWorkspace, DockWorkspaceDirtyChangedEventArgs, DockWorkspaceTrackingOptions,
};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

fn key(id: &str) -> String {
    id.chars()
        .map(|ch| {
            let code = ch as u32;
            match ORDINAL_UPPER.binary_search_by_key(&code, |&(lower, _)| lower) {
                Ok(index) => {
                    char::from_u32(ORDINAL_UPPER[index].1).expect("a captured Unicode scalar")
                }
                Err(_) => ch,
            }
        })
        .collect()
}
#[derive(Default)]
struct WorkspaceStore {
    values: Dictionary<String, Rc<DockWorkspace>>,
    version: u32,
}
/// The live, read-only dictionary values view returned by Workspaces.
#[derive(Clone)]
pub struct WorkspaceCollection {
    store: Rc<RefCell<WorkspaceStore>>,
}
impl WorkspaceCollection {
    /// Gets the current workspace count.
    pub fn count(&self) -> usize {
        self.store.borrow().values.len()
    }
    /// Gets whether the current collection is empty.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
    /// Enumerates current values in System Dictionary slot order. Adding a new key
    /// invalidates an enumerator; removing or clearing keys does not, as in .NET.
    pub fn iter(&self) -> WorkspaceIterator {
        let store = self.store.borrow();
        WorkspaceIterator {
            store: self.store.clone(),
            keys: store.values.keys().cloned().collect(),
            index: 0,
            version: store.version,
        }
    }
}
/// An independent enumerator for a live workspace values collection.
/// Keys are retained for the source slot order; values are read when advanced.
// The published System dictionary has no shareable enumerator/slot accessor.
pub struct WorkspaceIterator {
    store: Rc<RefCell<WorkspaceStore>>,
    keys: Vec<String>,
    index: usize,
    version: u32,
}
impl Iterator for WorkspaceIterator {
    type Item = Rc<DockWorkspace>;
    fn next(&mut self) -> Option<Self::Item> {
        let store = self.store.borrow();
        assert_eq!(
            store.version, self.version,
            "Collection was modified; enumeration operation may not execute."
        );
        while let Some(id) = self.keys.get(self.index) {
            self.index += 1;
            if let Some(value) = store.values.get(id) {
                return Some(value.clone());
            }
        }
        None
    }
}

/// Manages named snapshots and source factory-event dirty tracking.
/// The serializer type remains concrete to preserve its generic virtual calls
/// without restricting IDockSerializer to layout-only operations.
pub struct DockWorkspaceManager<S: IDockSerializer> {
    serializer: Rc<S>,
    workspaces: Rc<RefCell<WorkspaceStore>>,
    active: RefCell<Option<Rc<DockWorkspace>>>,
    tracking_factory: RefCell<Option<Rc<dyn IFactory>>>,
    tracked_root: RefCell<Option<Rc<dyn IRootDock>>>,
    tracking_options: RefCell<Rc<DockWorkspaceTrackingOptions>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    layout_dirty: Cell<bool>,
    last_reported_dirty: Cell<bool>,
    changed: Rc<Event<DockWorkspaceDirtyChangedEventArgs>>,
    runtime: Weak<Self>,
}
impl<S: IDockSerializer> DockWorkspaceManager<S> {
    /// Creates an empty manager with default tracking options.
    pub fn new(serializer: Rc<S>) -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            serializer,
            workspaces: Rc::new(RefCell::new(WorkspaceStore::default())),
            active: RefCell::new(None),
            tracking_factory: RefCell::new(None),
            tracked_root: RefCell::new(None),
            tracking_options: RefCell::new(DockWorkspaceTrackingOptions::new()),
            subscriptions: RefCell::new(Vec::new()),
            layout_dirty: Cell::new(false),
            last_reported_dirty: Cell::new(false),
            changed: Rc::new(Event::new()),
            runtime: runtime.clone(),
        })
    }
    /// Gets a live read-only view of known workspaces.
    pub fn workspaces(&self) -> WorkspaceCollection {
        WorkspaceCollection {
            store: self.workspaces.clone(),
        }
    }
    /// Gets the most recently captured or restored workspace.
    pub fn active_workspace(&self) -> Option<Rc<DockWorkspace>> {
        self.active.borrow().clone()
    }
    /// Gets the active workspace's dirty flag, or the unassigned layout's dirty flag.
    pub fn is_dirty(&self) -> bool {
        self.active_workspace()
            .map_or_else(|| self.layout_dirty.get(), |w| w.is_dirty())
    }
    /// Gets whether a factory is being observed.
    pub fn is_tracking(&self) -> bool {
        self.tracking_factory.borrow().is_some()
    }
    /// Gets the current shared tracking options.
    pub fn tracking_options(&self) -> Rc<DockWorkspaceTrackingOptions> {
        self.tracking_options.borrow().clone()
    }
    /// Subscribes to changes in the effective dirty flag; dispose to unsubscribe.
    pub fn workspace_dirty_changed(
        &self,
        handler: Rc<dyn Fn(&DockWorkspaceDirtyChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.changed, handler)
    }
    /// Captures a workspace using all source optional defaults: includes state,
    /// uses the ID as its display name, and supplies no description.
    pub fn capture_default(
        &self,
        id: &str,
        layout: Rc<dyn IDock>,
    ) -> SerializationResult<Rc<DockWorkspace>> {
        self.capture(id, layout, true, None, None)
    }
    /// Captures a snapshot, retaining existing workspace identity for a matching ID.
    /// include_state is the source optional parameter (default true).
    pub fn capture(
        &self,
        id: &str,
        layout: Rc<dyn IDock>,
        include_state: bool,
        name: Option<String>,
        description: Option<String>,
    ) -> SerializationResult<Rc<DockWorkspace>> {
        assert!(!is_blank(id), "Workspace id cannot be null or whitespace.");
        let state = if include_state {
            let state = DockState::new();
            state.save(layout.clone());
            Some(state)
        } else {
            None
        };
        let payload = self.serializer.serialize(layout)?;
        let existing = self.workspaces.borrow().values.get(&key(id)).cloned();
        if let Some(existing) = existing {
            existing.update(payload, state, name, description);
            self.active.replace(Some(existing.clone()));
            self.mark_clean();
            return Ok(existing);
        }
        let workspace = DockWorkspace::new(id.to_owned(), payload, state);
        workspace.set_name(Some(
            name.filter(|s| !is_blank(s))
                .unwrap_or_else(|| id.to_owned()),
        ));
        workspace.set_description(description);
        {
            let mut store = self.workspaces.borrow_mut();
            store.values.insert(key(id), workspace.clone());
            store.version = store.version.wrapping_add(1);
        }
        self.active.replace(Some(workspace.clone()));
        self.mark_clean();
        Ok(workspace)
    }
    /// Looks up a workspace using ordinal case-insensitive ID comparison.
    pub fn get(&self, id: &str) -> Option<Rc<DockWorkspace>> {
        if is_blank(id) {
            return None;
        }
        self.workspaces.borrow().values.get(&key(id)).cloned()
    }
    /// Removes the ID. The source active-ID equality is case-sensitive, even though
    /// dictionary removal is case-insensitive; preserve that distinction.
    pub fn remove(&self, id: &str) -> bool {
        if is_blank(id) {
            return false;
        }
        let removed = self.workspaces.borrow_mut().values.remove(&key(id));
        let was_removed = removed.is_some();
        drop(removed);
        if was_removed && self.active_workspace().is_some_and(|w| w.id() == id) {
            self.active.replace(None);
            self.mark_clean();
        }
        was_removed
    }
    /// Clears snapshots, active selection and dirty state.
    pub fn clear(&self) {
        // Retain values while clearing so user content is released outside the borrow.
        let retained: Vec<_> = self.workspaces.borrow().values.values().cloned().collect();
        self.workspaces.borrow_mut().values.clear();
        drop(retained);
        self.active.replace(None);
        self.mark_clean();
    }
    /// Deserializes and restores cached content. A null result leaves active/dirty state intact.
    pub fn restore(
        &self,
        workspace: Rc<DockWorkspace>,
    ) -> SerializationResult<Option<Rc<dyn IDock>>> {
        let Some(layout) = self
            .serializer
            .deserialize::<Rc<dyn IDock>>(Some(&workspace.layout()))?
        else {
            return Ok(None);
        };
        if let Some(state) = workspace.state() {
            state.restore(layout.clone());
        }
        self.active.replace(Some(workspace));
        self.mark_clean();
        Ok(Some(layout))
    }
    /// Restores a known ID; None corresponds to the source false/out-null result.
    pub fn try_restore(&self, id: &str) -> SerializationResult<Option<Rc<dyn IDock>>> {
        let Some(workspace) = self.get(id) else {
            return Ok(None);
        };
        self.restore(workspace)
    }
    /// Starts observing a factory. Reusing the factory replaces options and clears
    /// the root scope without changing subscriptions or dirty state.
    pub fn track_factory(
        &self,
        factory: Rc<dyn IFactory>,
        options: Option<Rc<DockWorkspaceTrackingOptions>>,
    ) {
        let same = self
            .tracking_factory
            .borrow()
            .as_ref()
            .is_some_and(|f| Rc::ptr_eq(f, &factory));
        if same {
            self.tracking_options.replace(options.unwrap_or_default());
            self.tracked_root.replace(None);
            return;
        }
        self.stop_tracking();
        self.tracking_factory.replace(Some(factory.clone()));
        self.tracking_options.replace(options.unwrap_or_default());
        self.subscribe_factory(factory);
    }
    /// Observes a layout and its factory, limiting changes to its root.
    /// Panics if the layout has no factory, before altering existing tracking.
    pub fn track_layout(
        &self,
        layout: Rc<dyn IDock>,
        options: Option<Rc<DockWorkspaceTrackingOptions>>,
    ) {
        assert!(layout.factory().is_some(), "Layout factory is not set.");
        self.track_factory(
            layout.factory().expect("source layout factory is set"),
            options,
        );
        let root = (layout.clone() as Rc<dyn IDockable>)
            .as_root_dock()
            .or_else(|| {
                layout
                    .factory()
                    .expect("source layout factory is set")
                    .find_root(Some(layout), None)
            });
        self.tracked_root.replace(root);
    }
    /// Unsubscribes in source order and resets tracking options/scope, keeping dirty state.
    pub fn stop_tracking(&self) {
        if self.tracking_factory.borrow().is_none() {
            return;
        }
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        self.tracking_factory.replace(None);
        self.tracked_root.replace(None);
        self.tracking_options
            .replace(DockWorkspaceTrackingOptions::new());
    }
    /// Clears the current layout and active workspace dirty flags, raising only transitions.
    pub fn mark_clean(&self) {
        self.layout_dirty.set(false);
        if let Some(workspace) = self.active_workspace() {
            workspace.set_dirty(false);
        }
        self.update_dirty_state();
    }
    fn mark_dirty(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.should_track_dockable(dockable) {
            self.set_dirty();
        }
    }
    fn mark_dirty_window(&self, window: Option<Rc<dyn IDockWindow>>) {
        if self.should_track_window(window) {
            self.set_dirty();
        }
    }
    fn set_dirty(&self) {
        if let Some(workspace) = self.active_workspace() {
            if workspace.set_dirty(true) {
                self.update_dirty_state();
            }
        } else if !self.layout_dirty.get() {
            self.layout_dirty.set(true);
            self.update_dirty_state();
        }
    }
    fn should_track_dockable(&self, dockable: Option<Rc<dyn IDockable>>) -> bool {
        if self.tracking_factory.borrow().is_none() {
            return false;
        }
        let tracked = self.tracked_root.borrow().clone();
        if let (Some(_), Some(dockable)) = (tracked, dockable.clone()) {
            let factory = self
                .tracking_factory
                .borrow()
                .clone()
                .expect("source tracking factory exists");
            let Some(root) = factory.find_root(Some(dockable), None) else {
                return false;
            };
            if !self
                .tracked_root
                .borrow()
                .as_ref()
                .is_some_and(|tracked| Rc::ptr_eq(&root, tracked))
            {
                return false;
            }
        }
        let filter = self.tracking_options().dockable_filter();
        filter.is_none_or(|filter| filter(dockable))
    }
    fn should_track_window(&self, window: Option<Rc<dyn IDockWindow>>) -> bool {
        if self.tracking_factory.borrow().is_none() {
            return false;
        }
        let tracked = self.tracked_root.borrow().clone();
        if let (Some(_), Some(window)) = (tracked, window.clone()) {
            let mut root = window.layout();
            if root.is_none() {
                if let Some(owner) = window.owner() {
                    let factory = self
                        .tracking_factory
                        .borrow()
                        .clone()
                        .expect("source tracking factory exists");
                    root = factory.find_root(Some(owner), None);
                }
            }
            let Some(root) = root else {
                return false;
            };
            if !self
                .tracked_root
                .borrow()
                .as_ref()
                .is_some_and(|tracked| Rc::ptr_eq(&root, tracked))
            {
                return false;
            }
        }
        let filter = self.tracking_options().dockable_filter();
        filter.is_none_or(|filter| filter(window.and_then(|w| w.owner())))
    }
    fn update_dirty_state(&self) {
        let dirty = self.is_dirty();
        if dirty == self.last_reported_dirty.get() {
            return;
        }
        self.last_reported_dirty.set(dirty);
        self.changed.raise(&DockWorkspaceDirtyChangedEventArgs::new(
            self.active_workspace(),
            dirty,
        ));
    }
    fn subscribe_factory(&self, factory: Rc<dyn IFactory>) {
        macro_rules! dockable {
            ($method:ident) => {{
                let weak = self.runtime.clone();
                let subscription = factory.$method(Rc::new(move |args| {
                    if let Some(manager) = weak.upgrade() {
                        manager.mark_dirty(args.dockable());
                    }
                }));
                self.subscriptions.borrow_mut().push(subscription);
            }};
        }
        macro_rules! window {
            ($method:ident) => {{
                let weak = self.runtime.clone();
                let subscription = factory.$method(Rc::new(move |args| {
                    if let Some(manager) = weak.upgrade() {
                        manager.mark_dirty_window(args.window());
                    }
                }));
                self.subscriptions.borrow_mut().push(subscription);
            }};
        }
        dockable!(dockable_added);
        dockable!(dockable_removed);
        dockable!(dockable_closed);
        dockable!(dockable_moved);
        dockable!(dockable_docked);
        dockable!(dockable_undocked);
        dockable!(dockable_swapped);
        dockable!(dockable_pinned);
        dockable!(dockable_unpinned);
        dockable!(dockable_hidden);
        dockable!(dockable_restored);
        window!(window_opened);
        window!(window_closed);
        window!(window_added);
        window!(window_removed);
        let weak = self.runtime.clone();
        let subscription = factory.window_move_drag_end(Rc::new(move |args| {
            if let Some(manager) = weak.upgrade() {
                if manager.tracking_options().track_window_moves() {
                    manager.mark_dirty_window(args.window());
                }
            }
        }));
        self.subscriptions.borrow_mut().push(subscription);
    }
}
impl<S: IDockSerializer> Drop for DockWorkspaceManager<S> {
    fn drop(&mut self) {
        // Native weak callbacks do not keep managers alive through factory events.
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::key;
    #[test]
    fn ordinal_id_keys_match_actual_dotnet_comparison_corpus() {
        fn decode(hex: &str) -> String {
            let bytes: Vec<u8> = hex
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            String::from_utf8(bytes).unwrap()
        }
        let mut count = 0;
        for line in include_str!("../../../tests/Dock/data/workspace-ordinal.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
        {
            let fields: Vec<_> = line.split('\t').collect();
            let left = decode(fields[0]);
            let right = decode(fields[1]);
            assert_eq!(
                key(&left) == key(&right),
                fields[2] == "1",
                "{left:?} versus {right:?}"
            );
            count += 1;
        }
        assert_eq!(count, 3005);
    }
}
