// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::BoxedValue;
use ferrodock_controls_recycling_model::IControlRecyclingIdProvider;
use std::rc::Rc;

/// The `IDockable` docking contract.
pub trait IDockable: IControlRecyclingIdProvider + 'static {
    /// Native projection of the original runtime class name without reflection.
    /// Custom models can override this for an explicitly registered source name.
    #[doc(hidden)]
    fn runtime_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .expect("compiler type name")
    }
    /// Resolves the original optional local-target marker, retaining this model.
    /// Inherited source interfaces supply the default view. Marker-only custom
    /// models expose their own view alongside the marker trait implementation.
    fn as_local_target(self: Rc<Self>) -> Option<Rc<dyn ILocalTarget>> {
        // These original contracts inherit ILocalTarget. Trait upcasts retain
        // the enclosing model even when its implementation composes a base.
        if let Some(document) = self.clone().as_document() {
            return Some(document);
        }
        if let Some(tool) = self.clone().as_tool() {
            return Some(tool);
        }
        if let Some(dock) = self.clone().as_document_dock() {
            return Some(dock);
        }
        self.as_tool_dock().map(|dock| dock as Rc<dyn ILocalTarget>)
    }
    /// Resolves the original optional global-target marker, retaining this model.
    /// Inherited source interfaces supply the default view. Marker-only custom
    /// models expose their own view alongside the marker trait implementation.
    fn as_global_target(self: Rc<Self>) -> Option<Rc<dyn IGlobalTarget>> {
        self.as_proportional_dock()
            .map(|dock| dock as Rc<dyn IGlobalTarget>)
    }

    /// Resolves the optional original command-bar provider without replacing the model.
    fn as_command_bar_provider(
        self: Rc<Self>,
    ) -> Option<Rc<dyn crate::command_bars::IDockCommandBarProvider>> {
        None
    }

    /// Views this original model as a notification source when it implements the
    /// optional System contract. Plain non-notifying dockables return `None`.
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        None
    }

    /// Resolves tool content for source content-state preservation.
    fn as_tool_content(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IToolContent>> {
        None
    }
    /// Resolves document content for source content-state preservation.
    fn as_document_content(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IDocumentContent>> {
        None
    }
    /// Boxes this concrete reference for native typed locator casts without cloning the model.
    #[doc(hidden)]
    fn boxed_reference(self: Rc<Self>) -> BoxedValue {
        Rc::new(self)
    }
    /// Resolves an additional application-defined dockable interface for typed locator casts.
    /// Return the requested Rc contract boxed as an object, preserving runtime identity.
    #[doc(hidden)]
    fn boxed_interface(self: Rc<Self>, _target: std::any::TypeId) -> Option<BoxedValue> {
        None
    }
    /// Resolves optional document template content without reflection.
    fn as_document_dock_content(
        self: Rc<Self>,
    ) -> Option<Rc<dyn crate::controls::IDocumentDockContent>> {
        None
    }
    /// Resolves optional tool template content without reflection.
    fn as_tool_dock_content(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IToolDockContent>> {
        None
    }
    /// Resolves optional ItemsSource document generation without reflection.
    fn as_items_source_dock(self: Rc<Self>) -> Option<Rc<dyn IItemsSourceDock>> {
        None
    }
    /// Resolves optional ItemsSource tool generation without reflection.
    fn as_tool_items_source_dock(self: Rc<Self>) -> Option<Rc<dyn IToolItemsSourceDock>> {
        None
    }
    /// Resolves the document contract when this object implements it.
    fn as_document(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IDocument>> {
        None
    }
    /// Resolves the tool contract when this object implements it.
    fn as_tool(self: Rc<Self>) -> Option<Rc<dyn crate::controls::ITool>> {
        None
    }
    /// Resolves the MDI document contract when this object implements it.
    fn as_mdi_document(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IMdiDocument>> {
        None
    }
    /// Resolves the splitter marker when this object implements it.
    fn as_splitter(self: Rc<Self>) -> Option<Rc<dyn crate::controls::ISplitter>> {
        None
    }
    /// Resolves the proportional splitter contract without treating every splitter as proportional.
    fn as_proportional_dock_splitter(
        self: Rc<Self>,
    ) -> Option<Rc<dyn crate::controls::IProportionalDockSplitter>> {
        None
    }
    /// Resolves the grid splitter contract without treating every splitter as a grid splitter.
    fn as_grid_dock_splitter(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IGridDockSplitter>> {
        None
    }
    /// Resolves `IDockDock` membership without reflection.
    fn as_dock_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IDockDock>> {
        None
    }
    /// Resolves `IStackDock` membership without reflection.
    fn as_stack_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IStackDock>> {
        None
    }
    /// Resolves `IWrapDock` membership without reflection.
    fn as_wrap_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IWrapDock>> {
        None
    }
    /// Resolves `IUniformGridDock` membership without reflection.
    fn as_uniform_grid_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IUniformGridDock>> {
        None
    }
    /// Resolves `IGridDock` membership without reflection.
    fn as_grid_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IGridDock>> {
        None
    }
    /// Resolves `IProportionalDock` membership without reflection.
    fn as_proportional_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IProportionalDock>> {
        None
    }
    /// Resolves document container membership without reflection.
    fn as_document_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IDocumentDock>> {
        None
    }
    /// Resolves the optional document factory delegate contract.
    fn as_document_dock_factory(
        self: Rc<Self>,
    ) -> Option<Rc<dyn crate::controls::IDocumentDockFactory>> {
        None
    }
    /// Resolves `IToolDock` membership without reflection.
    fn as_tool_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IToolDock>> {
        None
    }
    /// Exposes split-view children for navigation without reflection.
    fn as_split_view_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::ISplitViewDock>> {
        None
    }
    /// Resolves optional docking-window lifecycle state on this object.
    fn as_docking_window_state(&self) -> Option<&dyn IDockingWindowState> {
        None
    }
    /// Resolves original generated-container presentation metadata without reflection.
    fn as_dock_item_container_metadata(&self) -> Option<&dyn IDockItemContainerMetadata> {
        None
    }
    /// Resolves optional selector presentation on this object.
    fn as_dock_selector_info(&self) -> Option<&dyn IDockSelectorInfo> {
        None
    }
    /// Resolves optional source/target operation restrictions on this object.
    fn as_docking_restrictions(&self) -> Option<&dyn IDockableDockingRestrictions> {
        None
    }
    /// Resolves the dock contract when this object implements `IDock`.
    /// Dock implementations override this hook; leaf dockables retain `None`.
    fn as_dock(self: Rc<Self>) -> Option<Rc<dyn IDock>> {
        None
    }
    /// Resolves the root dock contract when this object implements `IRootDock`.
    fn as_root_dock(self: Rc<Self>) -> Option<Rc<dyn crate::controls::IRootDock>> {
        None
    }
    /// Gets or sets dockable id.
    fn id(&self) -> String;
    /// Sets `Id`.
    fn set_id(&self, value: String);
    /// Gets or sets dockable title.
    fn title(&self) -> String;
    /// Sets `Title`.
    fn set_title(&self, value: String);
    /// Gets or sets dockable context.
    fn context(&self) -> Option<BoxedValue>;
    /// Sets `Context`.
    fn set_context(&self, value: Option<BoxedValue>);
    /// Gets or sets dockable owner.
    fn owner(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `Owner`.
    fn set_owner(&self, value: Option<Rc<dyn IDockable>>);
    /// Gets or sets dockable original owner.
    fn original_owner(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `OriginalOwner`.
    fn set_original_owner(&self, value: Option<Rc<dyn IDockable>>);
    /// Gets or sets dockable factory.
    fn factory(&self) -> Option<Rc<dyn IFactory>>;
    /// Sets `Factory`.
    fn set_factory(&self, value: Option<Rc<dyn IFactory>>);
    /// Gets if the dockable is empty.
    fn is_empty(&self) -> bool;
    /// Sets `IsEmpty`.
    fn set_is_empty(&self, value: bool);
    /// Gets or sets if the dockable collapses when all its children are removed.
    fn is_collapsable(&self) -> bool;
    /// Sets `IsCollapsable`.
    fn set_is_collapsable(&self, value: bool);
    /// Gets or sets splitter proportion.
    fn proportion(&self) -> f64;
    /// Sets `Proportion`.
    fn set_proportion(&self, value: f64);
    /// Gets or sets docking mode.
    fn dock(&self) -> DockMode;
    /// Sets `Dock`.
    fn set_dock(&self, value: DockMode);
    /// Gets or sets the current logical docking window state.
    fn docking_state(&self) -> DockingWindowState;
    /// Sets `DockingState`.
    fn set_docking_state(&self, value: DockingWindowState);
    /// Gets or sets grid column.
    fn column(&self) -> i32;
    /// Sets `Column`.
    fn set_column(&self, value: i32);
    /// Gets or sets grid row.
    fn row(&self) -> i32;
    /// Sets `Row`.
    fn set_row(&self, value: i32);
    /// Gets or sets grid column span.
    fn column_span(&self) -> i32;
    /// Sets `ColumnSpan`.
    fn set_column_span(&self, value: i32);
    /// Gets or sets grid row span.
    fn row_span(&self) -> i32;
    /// Sets `RowSpan`.
    fn set_row_span(&self, value: i32);
    /// Gets or sets whether this dock participates in shared size scope.
    fn is_shared_size_scope(&self) -> bool;
    /// Sets `IsSharedSizeScope`.
    fn set_is_shared_size_scope(&self, value: bool);
    /// Gets or sets last known proportion before collapse.
    fn collapsed_proportion(&self) -> f64;
    /// Sets `CollapsedProportion`.
    fn set_collapsed_proportion(&self, value: f64);
    /// Gets or sets minimum width.
    fn min_width(&self) -> f64;
    /// Sets `MinWidth`.
    fn set_min_width(&self, value: f64);
    /// Gets or sets maximum width.
    fn max_width(&self) -> f64;
    /// Sets `MaxWidth`.
    fn set_max_width(&self, value: f64);
    /// Gets or sets minimum height.
    fn min_height(&self) -> f64;
    /// Sets `MinHeight`.
    fn set_min_height(&self, value: f64);
    /// Gets or sets maximum height.
    fn max_height(&self) -> f64;
    /// Sets `MaxHeight`.
    fn set_max_height(&self, value: f64);
    /// Gets or sets if the dockable can be closed.
    fn can_close(&self) -> bool;
    /// Sets `CanClose`.
    fn set_can_close(&self, value: bool);
    /// Gets or sets if the dockable can be pinned.
    fn can_pin(&self) -> bool;
    /// Sets `CanPin`.
    fn set_can_pin(&self, value: bool);
    /// Gets or sets whether a pinned dockable stays visible when previewed.
    fn keep_pinned_dockable_visible(&self) -> bool;
    /// Sets `KeepPinnedDockableVisible`.
    fn set_keep_pinned_dockable_visible(&self, value: bool);
    /// Gets or sets an optional pinned dock display mode override.
    /// When null, the root display mode is used.
    fn pinned_dock_display_mode_override(&self) -> Option<PinnedDockDisplayMode>;
    /// Sets `PinnedDockDisplayModeOverride`.
    fn set_pinned_dock_display_mode_override(&self, value: Option<PinnedDockDisplayMode>);
    /// Gets or sets the stored bounds for pinned previews.
    fn pinned_bounds(&self) -> Option<DockRect>;
    /// Sets `PinnedBounds`.
    fn set_pinned_bounds_rect(&self, value: Option<DockRect>);
    /// Gets or sets if the dockable can be floated.
    fn can_float(&self) -> bool;
    /// Sets `CanFloat`.
    fn set_can_float(&self, value: bool);
    /// Gets or sets if the dockable can be dragged.
    fn can_drag(&self) -> bool;
    /// Sets `CanDrag`.
    fn set_can_drag(&self, value: bool);
    /// Gets or sets if the dockable can be dropped on.
    fn can_drop(&self) -> bool;
    /// Sets `CanDrop`.
    fn set_can_drop(&self, value: bool);
    /// Gets or sets if the dockable can be docked as a document.
    fn can_dock_as_document(&self) -> bool;
    /// Sets `CanDockAsDocument`.
    fn set_can_dock_as_document(&self, value: bool);
    /// Gets or sets optional per-dockable capability overrides.
    /// Null values inherit root and dock policy values.
    fn dock_capability_overrides(&self) -> Option<Rc<DockCapabilityOverrides>>;
    /// Sets `DockCapabilityOverrides`.
    fn set_dock_capability_overrides(&self, value: Option<Rc<DockCapabilityOverrides>>);
    /// Gets or sets whether the dockable has unsaved changes.
    fn is_modified(&self) -> bool;
    /// Sets `IsModified`.
    fn set_is_modified(&self, value: bool);
    /// Gets or sets the docking group identifier.
    /// Dockables can only be docked with other dockables that have the same group identifier.
    /// If null or empty, the dockable can be docked with any other dockable.
    /// This enables creating isolated docking areas where only specific types of content can be docked together.
    fn dock_group(&self) -> Option<String>;
    /// Sets `DockGroup`.
    fn set_dock_group(&self, value: Option<String>);
    /// Called when the dockable is closed.
    fn on_close(&self) -> bool;
    /// Called when the dockable becomes the selected dockable.
    fn on_selected(&self);
    /// Gets dockable visible bounds information used for tracking.
    fn get_visible_bounds(&self) -> (f64, f64, f64, f64);
    /// Sets dockable visible bounds information used for tracking.
    fn set_visible_bounds(&self, x: f64, y: f64, width: f64, height: f64);
    /// Called when dockable visible bounds changed.
    fn on_visible_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64);
    /// Gets dockable pinned bounds information used for tracking.
    fn get_pinned_bounds(&self) -> (f64, f64, f64, f64);
    /// Sets dockable pinned bounds information used for tracking.
    fn set_pinned_bounds(&self, x: f64, y: f64, width: f64, height: f64);
    /// Called when dockable pinned bounds changed.
    fn on_pinned_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64);
    /// Gets dockable tab bounds information used for tracking.
    fn get_tab_bounds(&self) -> (f64, f64, f64, f64);
    /// Sets dockable tab bounds information used for tracking.
    fn set_tab_bounds(&self, x: f64, y: f64, width: f64, height: f64);
    /// Called when dockable tab bounds changed.
    fn on_tab_bounds_changed(&self, x: f64, y: f64, width: f64, height: f64);
    /// Gets dockable pointer position used for tracking.
    fn get_pointer_position(&self) -> (f64, f64);
    /// Sets dockable pointer position used for tracking.
    fn set_pointer_position(&self, x: f64, y: f64);
    /// Called when dockable pointer position changed.
    fn on_pointer_position_changed(&self, x: f64, y: f64);
    /// Gets dockable pointer screen position used for tracking.
    fn get_pointer_screen_position(&self) -> (f64, f64);
    /// Sets dockable pointer screen position used for tracking.
    fn set_pointer_screen_position(&self, x: f64, y: f64);
    /// Called when dockable pointer screen position changed.
    fn on_pointer_screen_position_changed(&self, x: f64, y: f64);
}

impl PartialEq for dyn IDockable {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

impl Eq for dyn IDockable {}

impl std::hash::Hash for dyn IDockable {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&((self as *const Self).cast::<()>() as usize), state);
    }
}
