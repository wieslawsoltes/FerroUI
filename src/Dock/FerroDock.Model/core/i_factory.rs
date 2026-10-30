// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use crate::BoxedValue;
use ferrodock_model::collections::ListHandle;
use ferroui_system::collections::generic::Dictionary;
use std::cell::RefCell;
use std::rc::Rc;

/// Shared map from dockable identity to its presentation control.
pub type DockableControlMap = Rc<RefCell<Dictionary<Rc<dyn IDockable>, Rc<dyn IDockableControl>>>>;

/// Shared map from dockable identity to an object value.
pub type DockableObjectMap = Rc<RefCell<Dictionary<Rc<dyn IDockable>, BoxedValue>>>;

/// Creates an optional dockable context.
pub type ContextFactory = Rc<dyn Fn() -> Option<BoxedValue>>;

/// Creates an optional host window.
pub type HostWindowFactory = Rc<dyn Fn() -> Option<Rc<dyn IHostWindow>>>;

/// Locates an optional dockable.
pub type DockableFactory = Rc<dyn Fn() -> Option<Rc<dyn IDockable>>>;

/// Tests a source dockable reference, including null collection entries.
pub type DockablePredicate = Rc<dyn Fn(Option<Rc<dyn IDockable>>) -> bool>;

/// Tests a root dock during ownership lookup.
pub type RootPredicate = Rc<dyn Fn(Rc<dyn IRootDock>) -> bool>;

/// A shared registry of context factories.
pub type ContextLocatorMap = Rc<RefCell<Dictionary<String, Option<ContextFactory>>>>;

/// A shared registry of host factories.
pub type HostWindowLocatorMap = Rc<RefCell<Dictionary<String, Option<HostWindowFactory>>>>;

/// A shared registry of dockable factories.
pub type DockableLocatorMap = Rc<RefCell<Dictionary<String, Option<DockableFactory>>>>;

/// The `IFactory` docking contract.
/// Generic methods are available on concrete factories (`Self: Sized`).
pub trait IFactory: IFactoryEvents {
    /// Boxes the concrete enclosing reference for native class projections without cloning state.
    #[doc(hidden)]
    fn boxed_reference(self: Rc<Self>) -> crate::BoxedValue
    where
        Self: 'static,
    {
        Rc::new(self)
    }

    /// Resolves source FactoryBase membership for class-specific lifecycle operations.
    #[doc(hidden)]
    fn as_factory_base(self: Rc<Self>) -> Option<Rc<dyn crate::FactoryBaseImpl>> {
        None
    }
    /// Resolves the optional tool-to-document split policy.
    fn as_dock_split_policy(self: Rc<Self>) -> Option<Rc<dyn IDockSplitPolicy>> {
        None
    }
    /// Resolves the optional factory docking veto policy.
    fn as_dock_drop_policy(self: Rc<Self>) -> Option<Rc<dyn IDockDropPolicy>> {
        None
    }
    /// Resolves the optional enclosing-region outer split policy.
    fn as_dock_outer_split_policy(self: Rc<Self>) -> Option<Rc<dyn IDockOuterSplitPolicy>> {
        None
    }
    /// Resolves the optional operation transaction observer.
    fn as_dock_operation_observer(self: Rc<Self>) -> Option<Rc<dyn IDockOperationObserver>> {
        None
    }
    /// Resolves optional preservation of content when a host is closed.
    fn as_dock_window_close_policy(self: Rc<Self>) -> Option<Rc<dyn IDockWindowClosePolicy>> {
        None
    }
    /// Resolves the optional factory that creates hosts for an entire window model.
    fn as_dock_window_host_factory(self: Rc<Self>) -> Option<Rc<dyn IDockWindowHostFactory>> {
        None
    }
    /// Resolves optional synchronization of view-model state back to the layout.
    fn as_docking_window_state_sync(self: Rc<Self>) -> Option<Rc<dyn IDockingWindowStateSync>> {
        None
    }
    /// Resolves the optional item drag/reorder policy implemented by this factory.
    fn as_dock_item_drag_policy(self: Rc<Self>) -> Option<Rc<dyn IDockItemDragPolicy>> {
        None
    }
    /// Gets visible dockable controls.
    fn visible_dockable_controls(&self) -> DockableControlMap;
    /// Gets visible dockable root controls.
    fn visible_root_controls(&self) -> DockableObjectMap;
    /// Gets pinned dockable controls.
    fn pinned_dockable_controls(&self) -> DockableControlMap;
    /// Gets pinned dockable root controls.
    fn pinned_root_controls(&self) -> DockableObjectMap;
    /// Gets tab dockable controls.
    fn tab_dockable_controls(&self) -> DockableControlMap;
    /// Gets tab dockable root controls.
    fn tab_root_controls(&self) -> DockableObjectMap;
    /// Gets tool controls.
    fn tool_controls(&self) -> DockableObjectMap;
    /// Gets document controls.
    fn document_controls(&self) -> DockableObjectMap;
    /// Gets dock controls.
    fn dock_controls(&self) -> ListHandle<Rc<dyn IDockControl>>;
    /// Gets host windows.
    fn host_windows(&self) -> ListHandle<Rc<dyn IHostWindow>>;
    /// Gets current global dock tracking state.
    fn global_dock_tracking_state(&self) -> GlobalDockTrackingState;
    /// Gets currently tracked dockable across all roots/windows.
    fn current_dockable(&self) -> Option<Rc<dyn IDockable>>;
    /// Gets currently tracked root dock across all windows.
    fn current_root_dock(&self) -> Option<Rc<dyn IRootDock>>;
    /// Gets currently tracked dock window across all roots/windows.
    fn current_dock_window(&self) -> Option<Rc<dyn IDockWindow>>;
    /// Gets currently tracked host window across all roots/windows.
    fn current_host_window(&self) -> Option<Rc<dyn IHostWindow>>;
    /// When true closing a tool hides it instead of removing it.
    fn hide_tools_on_close(&self) -> bool;
    /// Sets `HideToolsOnClose`.
    fn set_hide_tools_on_close(&self, value: bool);
    /// When true closing a document hides it instead of removing it.
    fn hide_documents_on_close(&self) -> bool;
    /// Sets `HideDocumentsOnClose`.
    fn set_hide_documents_on_close(&self, value: bool);
    /// Gets or sets `IDockable.Context` default locator.
    fn default_context_locator(&self) -> Option<ContextFactory>;
    /// Sets `DefaultContextLocator`.
    fn set_default_context_locator(&self, value: Option<ContextFactory>);
    /// Gets or sets `IHostWindow` default locator.
    fn default_host_window_locator(&self) -> Option<HostWindowFactory>;
    /// Sets `DefaultHostWindowLocator`.
    fn set_default_host_window_locator(&self, value: Option<HostWindowFactory>);
    /// Gets or sets `IDockable.Context` locator registry.
    fn context_locator(&self) -> Option<ContextLocatorMap>;
    /// Sets `ContextLocator`.
    fn set_context_locator(&self, value: Option<ContextLocatorMap>);
    /// Gets or sets `IHostWindow` locator registry.
    fn host_window_locator(&self) -> Option<HostWindowLocatorMap>;
    /// Sets `HostWindowLocator`.
    fn set_host_window_locator(&self, value: Option<HostWindowLocatorMap>);
    /// Gets or sets `IDockable` locator registry.
    fn dockable_locator(&self) -> Option<DockableLocatorMap>;
    /// Sets `DockableLocator`.
    fn set_dockable_locator(&self, value: Option<DockableLocatorMap>);
    /// Creates an observable list of type `IList{T}`.
    fn create_list<T: Clone + PartialEq + 'static>(&self, items: Vec<T>) -> ListHandle<T>
    where
        Self: Sized;
    /// Dispatches the dockable specialization of virtual CreateList through an erased factory.
    #[doc(hidden)]
    fn create_dockable_list(
        &self,
        items: Vec<Option<Rc<dyn IDockable>>>,
    ) -> ListHandle<Option<Rc<dyn IDockable>>>;
    /// Creates `IRootDock`.
    fn create_root_dock(&self) -> Rc<dyn IRootDock>;
    /// Creates `IProportionalDock`.
    fn create_proportional_dock(&self) -> Rc<dyn IProportionalDock>;
    /// Creates `IDockDock`.
    fn create_dock_dock(&self) -> Rc<dyn IDockDock>;
    /// Creates `IStackDock`.
    fn create_stack_dock(&self) -> Rc<dyn IStackDock>;
    /// Creates `IGridDock`.
    fn create_grid_dock(&self) -> Rc<dyn IGridDock>;
    /// Creates `IWrapDock`.
    fn create_wrap_dock(&self) -> Rc<dyn IWrapDock>;
    /// Creates `IUniformGridDock`.
    fn create_uniform_grid_dock(&self) -> Rc<dyn IUniformGridDock>;
    /// Creates `IProportionalDockSplitter`.
    fn create_proportional_dock_splitter(&self) -> Rc<dyn IProportionalDockSplitter>;
    /// Creates `IGridDockSplitter`.
    fn create_grid_dock_splitter(&self) -> Rc<dyn IGridDockSplitter>;
    /// Creates `IToolDock`.
    fn create_tool_dock(&self) -> Rc<dyn IToolDock>;
    /// Creates `IDocumentDock`.
    fn create_document_dock(&self) -> Rc<dyn IDocumentDock>;
    /// Creates `ISplitViewDock`.
    fn create_split_view_dock(&self) -> Rc<dyn ISplitViewDock>;
    /// Creates `IDockWindow`.
    fn create_dock_window(&self) -> Rc<dyn IDockWindow>;
    /// Creates `IDocument`.
    fn create_document(&self) -> Rc<dyn IDocument>;
    /// Creates `ITool`.
    fn create_tool(&self) -> Rc<dyn ITool>;
    /// Creates layout.
    fn create_layout(&self) -> Option<Rc<dyn IRootDock>>;
    /// Gets registered context in `ContextLocator`.
    fn get_context(&self, id: String) -> Option<BoxedValue>;
    /// Gets registered host window.
    fn get_host_window(&self, id: String) -> Option<Rc<dyn IHostWindow>>;
    /// Gets registered dockable in `DockableLocator`.
    fn get_dockable<T: IDockable + ?Sized + 'static>(&self, id: String) -> Option<Rc<T>>
    where
        Self: Sized;
    /// Gets the generated dock item container that is currently tracked for the specified source item.
    fn get_container_from_item(&self, item: Option<BoxedValue>) -> Option<Rc<dyn IDockable>>;
    /// Initialize layout.
    fn init_layout(&self, layout: Rc<dyn IDockable>);
    /// Initialize dockable.
    fn init_dockable(&self, dockable: Option<Rc<dyn IDockable>>, owner: Option<Rc<dyn IDockable>>);
    /// Initialize dock window.
    fn init_dock_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
    );
    /// Initialize dock window.
    fn init_dock_window_with_host(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
        host: Option<Rc<dyn IHostWindow>>,
    );
    /// Initialize active dockable.
    fn init_active_dockable(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDock>>,
    );
    /// Activates the specified dockable window.
    fn activate_window(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Sets an active dockable. If the dockable is contained inside an dock it
    /// will become the selected dockable.
    fn set_active_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Sets the currently focused dockable updating IsActive flags.
    fn set_focused_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
    );
    /// Searches for root dockable.
    fn find_root(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>>;
    /// Searches dock for dockable.
    fn find_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        predicate: Option<DockablePredicate>,
    ) -> Option<Rc<dyn IDockable>>;
    /// Searches for dockables in all registered `IDockControl`.
    fn find(&self, predicate: Option<DockablePredicate>) -> ferrodock_model::DockableQuery;
    /// Searches dock for dockables in all registered `IDockControl`.
    fn find_in_dock(
        &self,
        dock: Option<Rc<dyn IDock>>,
        predicate: Option<DockablePredicate>,
    ) -> ferrodock_model::DockableQuery;
    /// Adds `IDockable` into dock `IDock.VisibleDockables` collection.
    fn add_dockable(&self, dock: Option<Rc<dyn IDock>>, dockable: Option<Rc<dyn IDockable>>);
    /// Inserts `IDockable` into dock `IDock.VisibleDockables` collection.
    fn insert_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        index: i32,
    );
    /// Removes dockable from owner `IDock.VisibleDockables` collection.
    fn remove_dockable(&self, dockable: Option<Rc<dyn IDockable>>, collapse: bool);
    /// Moves dockable inside `IDock.VisibleDockables` collection.
    fn move_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    );
    /// Moves dockable into another `IDock.VisibleDockables` collection.
    fn move_dockable_between_docks(
        &self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    );
    /// Swaps dockable in inside `IDock.VisibleDockables` collections.
    fn swap_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    );
    /// Swaps dockable into between `IDock.VisibleDockables` collections.
    fn swap_dockable_between_docks(
        &self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    );
    /// Pins or unpins a dockable.
    fn pin_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Pins or unpins the dockable supplied as a command parameter.
    fn pin_dockable_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.pin_dockable(Some(dockable));
        }
    }
    /// Unpins a dockable.
    fn unpin_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Temporarily shows a pinned dockable.
    fn preview_pinned_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Temporarily shows the pinned dockable supplied as a command parameter.
    fn preview_pinned_dockable_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.preview_pinned_dockable(Some(dockable));
        }
    }
    /// Toggles preview for a pinned dockable.
    fn toggle_preview_pinned_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Toggles preview for the pinned dockable supplied as a command parameter.
    fn toggle_preview_pinned_dockable_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.toggle_preview_pinned_dockable(Some(dockable));
        }
    }
    /// Hides all temporarily shown pinned dockables.
    fn hide_previewing_dockables(&self, root_dock: Option<Rc<dyn IRootDock>>);
    /// Returns true if dockable is pinned.
    fn is_dockable_pinned(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root_dock: Option<Rc<dyn IRootDock>>,
    ) -> bool;
    /// Moves pinned dockable within the same pinned collection (reordering).
    fn move_pinned_dockable(
        &self,
        source_dockable: Option<Rc<dyn IDockable>>,
        target_dockable: Option<Rc<dyn IDockable>>,
    );
    /// Floats dockable.
    fn float_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Floats the dockable supplied as a command parameter.
    fn float_dockable_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.float_dockable(Some(dockable));
        }
    }
    /// Floats the dockable supplied as a command parameter.
    fn float_dockable_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    );
    /// Floats owner dock with all dockables.
    fn float_all_dockables(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Floats the owner dock for the dockable supplied as a command parameter.
    fn float_all_dockables_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.float_all_dockables(Some(dockable));
        }
    }
    /// Floats the owner dock for the dockable supplied as a command parameter.
    fn float_all_dockables_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    );
    /// Docks dockable as tabbed document in the nearest document dock.
    fn dock_as_document(&self, dockable: Rc<dyn IDockable>);
    /// Docks the dockable supplied as a command parameter as a tabbed document.
    fn dock_as_document_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.dock_as_document(dockable);
        }
    }
    /// Removes dockable from owner `IDock.VisibleDockables` collection, and call IDockable.OnClose.
    fn close_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Closes the dockable supplied as a command parameter.
    fn close_dockable_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.close_dockable(Some(dockable));
        }
    }
    /// Calls `IFactory.CloseDockable(IDockable)` on all `IDock.VisibleDockables` of the dockable owner, excluding the dockable itself.
    fn close_other_dockables(&self, dockable: Rc<dyn IDockable>);
    /// Closes the other dockables for the dockable supplied as a command parameter.
    fn close_other_dockables_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.close_other_dockables(dockable);
        }
    }
    /// Calls `IFactory.CloseDockable(IDockable)` on all `IDock.VisibleDockables` of the dockable owner.
    fn close_all_dockables(&self, dockable: Rc<dyn IDockable>);
    /// Closes all dockables for the dockable supplied as a command parameter.
    fn close_all_dockables_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.close_all_dockables(dockable);
        }
    }
    /// Calls `IFactory.CloseDockable(IDockable)` on all tabs to the left of the dockable, from the `IDock.VisibleDockables` collection of the dockable owner.
    fn close_left_dockables(&self, dockable: Rc<dyn IDockable>);
    /// Closes the dockables to the left of the dockable supplied as a command parameter.
    fn close_left_dockables_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.close_left_dockables(dockable);
        }
    }
    /// Calls `IFactory.CloseDockable(IDockable)` on all tabs to the right of the dockable, from the `IDock.VisibleDockables` collection of the dockable owner.
    fn close_right_dockables(&self, dockable: Rc<dyn IDockable>);
    /// Closes the dockables to the right of the dockable supplied as a command parameter.
    fn close_right_dockables_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.close_right_dockables(dockable);
        }
    }
    /// Sets the tabs layout for the specified document dock.
    fn set_document_dock_tabs_layout(&self, dockable: Rc<dyn IDockable>, layout: DocumentTabLayout);
    /// Sets the tabs layout to left.
    fn set_document_dock_tabs_layout_left(&self, dockable: Rc<dyn IDockable>);
    /// Sets the tabs layout to left for the dockable supplied as a command parameter.
    fn set_document_dock_tabs_layout_left_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.set_document_dock_tabs_layout_left(dockable);
        }
    }
    /// Sets the tabs layout to top.
    fn set_document_dock_tabs_layout_top(&self, dockable: Rc<dyn IDockable>);
    /// Sets the tabs layout to top for the dockable supplied as a command parameter.
    fn set_document_dock_tabs_layout_top_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.set_document_dock_tabs_layout_top(dockable);
        }
    }
    /// Sets the tabs layout to right.
    fn set_document_dock_tabs_layout_right(&self, dockable: Rc<dyn IDockable>);
    /// Sets the tabs layout to right for the dockable supplied as a command parameter.
    fn set_document_dock_tabs_layout_right_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.set_document_dock_tabs_layout_right(dockable);
        }
    }
    /// Sets the layout mode for the specified document dock.
    fn set_document_dock_layout_mode(
        &self,
        dockable: Rc<dyn IDockable>,
        layout_mode: DocumentLayoutMode,
    );
    /// Sets the layout mode to tabbed.
    fn set_document_dock_layout_mode_tabbed(&self, dockable: Rc<dyn IDockable>);
    /// Sets tabbed layout mode for the dockable supplied as a command parameter.
    fn set_document_dock_layout_mode_tabbed_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.set_document_dock_layout_mode_tabbed(dockable);
        }
    }
    /// Sets the layout mode to MDI.
    fn set_document_dock_layout_mode_mdi(&self, dockable: Rc<dyn IDockable>);
    /// Sets MDI layout mode for the dockable supplied as a command parameter.
    fn set_document_dock_layout_mode_mdi_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.set_document_dock_layout_mode_mdi(dockable);
        }
    }
    /// Hides the dockable and stores it in `IRootDock.HiddenDockables`.
    fn hide_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Hides the dockable with the specified id.
    fn hide_dockable_by_id(&self, id: String);
    /// Restores a hidden dockable to its original dock.
    fn restore_dockable(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Restores a hidden dockable to its original dock.
    fn restore_dockable_by_id(&self, id: String) -> Option<Rc<dyn IDockable>>;
    /// Adds window into dock windows list.
    fn add_window(&self, root_dock: Option<Rc<dyn IRootDock>>, window: Option<Rc<dyn IDockWindow>>);
    /// Inserts window into dock windows list.
    fn insert_window(
        &self,
        root_dock: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
        index: i32,
    );
    /// Removes window from owner windows list.
    fn remove_window(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Closes window layout and its dockables.
    fn close_window(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Collapses dock.
    fn collapse_dock(&self, dock: Rc<dyn IDock>);
    /// Creates a new split layout from source dockable.
    fn create_split_layout(
        &self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> Rc<dyn IDock>;
    /// Splits dock and updates owner layout.
    fn split_to_dock(
        &self,
        dock: Rc<dyn IDock>,
        dockable: Rc<dyn IDockable>,
        operation: DockOperation,
    );
    /// Creates dock window from source dockable.
    fn create_window_from(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>>;
    /// Creates dock window from source dockable with the specified options.
    fn create_window_from_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>>;
    /// Splits dock to the `DockOperation.Window` and updates `IDockable.Owner` layout.
    fn split_to_window(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    );
    /// Splits dock to the `DockOperation.Window` and updates `IDockable.Owner` layout with the specified window options.
    #[allow(clippy::too_many_arguments)] // Preserve the source method.
    fn split_to_window_with_options(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<Rc<DockWindowOptions>>,
    );
    /// Splits document into a new horizontal document dock.
    fn new_horizontal_document_dock(&self, dockable: Rc<dyn IDockable>);
    /// Creates a horizontal document dock for the dockable supplied as a command parameter.
    fn new_horizontal_document_dock_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.new_horizontal_document_dock(dockable);
        }
    }
    /// Splits document into a new vertical document dock.
    fn new_vertical_document_dock(&self, dockable: Rc<dyn IDockable>);
    /// Creates a vertical document dock for the dockable supplied as a command parameter.
    fn new_vertical_document_dock_parameter(&self, parameter: Option<BoxedValue>) {
        if let Some(dockable) =
            parameter.and_then(|value| value.downcast_ref::<Rc<dyn IDockable>>().cloned())
        {
            self.new_vertical_document_dock(dockable);
        }
    }
}

impl PartialEq for dyn IFactory {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
