// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

//! Source FluentExtensions as extension traits of shared model/factory references.
//! Import this module's traits to enable chaining. Methods retain the same shared
//! handle (and its Rust type). Creation `_into` overloads write the source out
//! parameter and return the factory; None means no configuration callback.

use crate::collections::ListHandle;
use crate::controls::*;
use crate::core::*;
use crate::BoxedValue;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;

macro_rules! setters {
    ($name:ident, $bound:ident, {$($method:ident($value:ident: $type:ty) => $setter:ident;)*}) => {
        #[doc = concat!("Fluent source property helpers for `", stringify!($bound), "` handles.")]
        pub trait $name: Sized {
            $(#[doc = concat!("Calls `", stringify!($setter), "` and returns the same handle.")]
            fn $method(self, $value: $type) -> Self;)*
        }
        impl<T: $bound + ?Sized> $name for Rc<T> {
            $(fn $method(self, $value: $type) -> Self {
                self.$setter($value);
                self
            })*
        }
    };
}
setters!(DockableFluentExt, IDockable, {
    with_id(id: String) => set_id;
    with_title(title: String) => set_title;
    with_context(context: Option<BoxedValue>) => set_context;
    with_owner(owner: Option<Rc<dyn IDockable>>) => set_owner;
    with_original_owner(owner: Option<Rc<dyn IDockable>>) => set_original_owner;
    with_factory(factory: Option<Rc<dyn IFactory>>) => set_factory;
    with_is_empty(value: bool) => set_is_empty;
    with_is_collapsable(value: bool) => set_is_collapsable;
    with_proportion(value: f64) => set_proportion;
    with_dock_mode(mode: DockMode) => set_dock;
    with_shared_size_scope(value: bool) => set_is_shared_size_scope;
    with_collapsed_proportion(value: f64) => set_collapsed_proportion;
    with_min_width(value: f64) => set_min_width;
    with_max_width(value: f64) => set_max_width;
    with_min_height(value: f64) => set_min_height;
    with_max_height(value: f64) => set_max_height;
    with_can_close(value: bool) => set_can_close;
    with_can_pin(value: bool) => set_can_pin;
    with_keep_pinned_dockable_visible(value: bool) => set_keep_pinned_dockable_visible;
    with_pinned_dock_display_mode_override(mode: Option<PinnedDockDisplayMode>) => set_pinned_dock_display_mode_override;
    with_can_float(value: bool) => set_can_float;
    with_can_drag(value: bool) => set_can_drag;
    with_can_drop(value: bool) => set_can_drop;
    with_can_dock_as_document(value: bool) => set_can_dock_as_document;
    with_is_modified(value: bool) => set_is_modified;
    with_dock_group(group: Option<String>) => set_dock_group;
});
setters!(DockingRestrictionsFluentExt, IDockableDockingRestrictions, {
    with_allowed_dock_operations(value: DockOperationMask) => set_allowed_dock_operations;
    with_allowed_drop_operations(value: DockOperationMask) => set_allowed_drop_operations;
});
setters!(DockPropertiesFluentExt, IDock, {
    with_active_dockable(item: Option<Rc<dyn IDockable>>) => set_active_dockable;
    with_default_dockable(item: Option<Rc<dyn IDockable>>) => set_default_dockable;
    with_focused_dockable(item: Option<Rc<dyn IDockable>>) => set_focused_dockable;
    with_is_active(value: bool) => set_is_active;
    with_can_close_last_dockable(value: bool) => set_can_close_last_dockable;
    with_enable_global_docking(value: bool) => set_enable_global_docking;
});
setters!(RootDockPropertiesFluentExt, IRootDock, {
    with_is_focusable_root(value: bool) => set_is_focusable_root;
    with_pinned_dock(pinned: Option<Rc<dyn IToolDock>>) => set_pinned_dock;
    with_pinned_dock_display_mode(mode: PinnedDockDisplayMode) => set_pinned_dock_display_mode;
    with_window(window: Option<Rc<dyn IDockWindow>>) => set_window;
    with_floating_window_host_mode(mode: DockFloatingWindowHostMode) => set_floating_window_host_mode;
});
setters!(ProportionalDockFluentExt, IProportionalDock, {
    with_orientation(orientation: Orientation) => set_orientation;
});
setters!(WrapDockFluentExt, IWrapDock, {
    with_orientation(orientation: Orientation) => set_orientation;
});
setters!(StackDockFluentExt, IStackDock, {
    with_orientation(orientation: Orientation) => set_orientation;
    with_spacing(spacing: f64) => set_spacing;
});
setters!(GridDockFluentExt, IGridDock, {
    with_column_definitions(definitions: Option<String>) => set_column_definitions;
    with_row_definitions(definitions: Option<String>) => set_row_definitions;
});
setters!(UniformGridDockFluentExt, IUniformGridDock, {
    with_rows(rows: i32) => set_rows;
    with_columns(columns: i32) => set_columns;
});
setters!(DockDockFluentExt, IDockDock, {
    with_last_child_fill(value: bool) => set_last_child_fill;
});
setters!(DocumentDockPropertiesFluentExt, IDocumentDock, {
    with_can_create_document(value: bool) => set_can_create_document;
    with_create_document(command: Option<Rc<dyn ICommand>>) => set_create_document;
    with_enable_window_drag(value: bool) => set_enable_window_drag;
    with_tabs_layout(layout: DocumentTabLayout) => set_tabs_layout;
    with_layout_mode(mode: DocumentLayoutMode) => set_layout_mode;
    with_close_button_show_mode(mode: DocumentCloseButtonShowMode) => set_close_button_show_mode;
    with_empty_content(content: Option<BoxedValue>) => set_empty_content;
});
setters!(DocumentDockContentFluentExt, IDocumentDockContent, {
    with_document_template(template: Option<Rc<dyn IDocumentTemplate>>) => set_document_template;
});
setters!(ToolDockContentFluentExt, IToolDockContent, {
    with_tool_template(template: Option<Rc<dyn IToolTemplate>>) => set_tool_template;
});
setters!(ToolDockPropertiesFluentExt, IToolDock, {
    with_alignment(alignment: Alignment) => set_alignment;
    with_is_expanded(value: bool) => set_is_expanded;
    with_auto_hide(value: bool) => set_auto_hide;
    with_grip_mode(mode: GripMode) => set_grip_mode;
});
setters!(ProportionalDockSplitterFluentExt, IProportionalDockSplitter, {
    with_can_resize(value: bool) => set_can_resize;
    with_resize_preview(value: bool) => set_resize_preview;
});
setters!(GridDockSplitterFluentExt, IGridDockSplitter, {
    with_resize_direction(direction: GridResizeDirection) => set_resize_direction;
});
setters!(DockWindowPropertiesFluentExt, IDockWindow, {
    with_id(id: String) => set_id;
    with_window_state(state: DockWindowState) => set_window_state;
    with_topmost(value: bool) => set_topmost;
    with_title(title: String) => set_title;
    with_owner_mode(mode: DockWindowOwnerMode) => set_owner_mode;
    with_parent_window(window: Option<Rc<dyn IDockWindow>>) => set_parent_window;
    with_modal(value: bool) => set_is_modal;
    with_show_in_taskbar(value: Option<bool>) => set_show_in_taskbar;
    with_owner(owner: Option<Rc<dyn IDockable>>) => set_owner;
    with_factory(factory: Option<Rc<dyn IFactory>>) => set_factory;
    with_layout(layout: Option<Rc<dyn IRootDock>>) => set_layout;
    with_host(host: Option<Rc<dyn IHostWindow>>) => set_host;
});
setters!(SplitViewDockFluentExt, ISplitViewDock, {
    with_compact_pane_length(length: f64) => set_compact_pane_length;
    with_display_mode(mode: SplitViewDisplayMode) => set_display_mode;
    with_is_pane_open(value: bool) => set_is_pane_open;
    with_open_pane_length(length: f64) => set_open_pane_length;
    with_pane_placement(placement: SplitViewPanePlacement) => set_pane_placement;
    with_use_light_dismiss_overlay_mode(value: bool) => set_use_light_dismiss_overlay_mode;
    with_pane_dockable(dockable: Option<Rc<dyn IDockable>>) => set_pane_dockable;
    with_content_dockable(dockable: Option<Rc<dyn IDockable>>) => set_content_dockable;
});

/// The source multi-property grid placement helper.
pub trait DockableGridFluentExt: Sized {
    /// Sets column, row, column span and row span, in source order.
    fn with_grid(self, column: i32, row: i32, column_span: i32, row_span: i32) -> Self;
    /// Uses the source defaults: column/row zero, both spans one.
    fn with_grid_default(self) -> Self {
        self.with_grid(0, 0, 1, 1)
    }
}
impl<T: IDockable + ?Sized> DockableGridFluentExt for Rc<T> {
    fn with_grid(self, column: i32, row: i32, column_span: i32, row_span: i32) -> Self {
        self.set_column(column);
        self.set_row(row);
        self.set_column_span(column_span);
        self.set_row_span(row_span);
        self
    }
}
/// Source window geometry helpers; values are assigned without validation.
pub trait DockWindowGeometryFluentExt: Sized {
    /// Sets X then Y, retaining the same window.
    fn with_position(self, x: f64, y: f64) -> Self;
    /// Sets Width then Height, retaining the same window.
    fn with_size(self, width: f64, height: f64) -> Self;
}
impl<T: IDockWindow + ?Sized> DockWindowGeometryFluentExt for Rc<T> {
    fn with_position(self, x: f64, y: f64) -> Self {
        self.set_x(x);
        self.set_y(y);
        self
    }
    fn with_size(self, width: f64, height: f64) -> Self {
        self.set_width(width);
        self.set_height(height);
        self
    }
}
/// Source append helpers for document docks. These dispatch the enclosing virtual methods.
pub trait DocumentDockAppendFluentExt: Sized {
    /// Calls AddDocument and returns the same dock.
    fn append_document(self, document: Rc<dyn IDockable>) -> Self;
    /// Calls AddTool and returns the same dock.
    fn append_tool(self, tool: Rc<dyn IDockable>) -> Self;
}
impl<T: IDocumentDock + ?Sized> DocumentDockAppendFluentExt for Rc<T> {
    fn append_document(self, document: Rc<dyn IDockable>) -> Self {
        self.add_document(Some(document));
        self
    }
    fn append_tool(self, tool: Rc<dyn IDockable>) -> Self {
        self.add_tool(Some(tool));
        self
    }
}
/// Source append helper for tool docks, retaining virtual AddTool behavior.
pub trait ToolDockAppendFluentExt: Sized {
    /// Calls AddTool and returns the same dock.
    fn append_tool(self, tool: Rc<dyn IDockable>) -> Self;
}
impl<T: IToolDock + ?Sized> ToolDockAppendFluentExt for Rc<T> {
    fn append_tool(self, tool: Rc<dyn IDockable>) -> Self {
        self.add_tool(Some(tool));
        self
    }
}

fn ensure_list<T: Clone + PartialEq + 'static>(list: Option<ListHandle<T>>) -> ListHandle<T> {
    list.unwrap_or_else(|| ListHandle::from_plain(Vec::new()))
}
/// Source visible-list helpers, appending without setting owners, factory or selection.
pub trait DockListFluentExt: Sized {
    /// Reassigns the existing list (or a new plain list), then appends the supplied
    /// items in enumeration order, rereading VisibleDockables for every add.
    fn with_visible_dockables<I: IntoIterator<Item = Option<Rc<dyn IDockable>>>>(
        self,
        items: I,
    ) -> Self;
    /// The source Add alias, with the same append behavior.
    fn add<I: IntoIterator<Item = Option<Rc<dyn IDockable>>>>(self, items: I) -> Self {
        self.with_visible_dockables(items)
    }
}
impl<T: IDock + ?Sized> DockListFluentExt for Rc<T> {
    fn with_visible_dockables<I: IntoIterator<Item = Option<Rc<dyn IDockable>>>>(
        self,
        items: I,
    ) -> Self {
        self.set_visible_dockables(Some(ensure_list(self.visible_dockables())));
        for item in items {
            self.visible_dockables()
                .expect("source visible list exists")
                .add(item);
        }
        self
    }
}
macro_rules! root_lists {
    ($($method:ident($item:ty) => $get:ident, $set:ident;)*) => {
        /// Source root collection append helpers, using plain lists only for null properties.
        pub trait RootDockListFluentExt: Sized {
            $(#[doc = concat!("Ensures `", stringify!($get), "` then appends, preserving source per-item property reads.")]
            fn $method<I: IntoIterator<Item = $item>>(self, items: I) -> Self;)*
        }
        impl<T: IRootDock + ?Sized> RootDockListFluentExt for Rc<T> {
            $(fn $method<I: IntoIterator<Item = $item>>(self, items: I) -> Self {
                self.$set(Some(ensure_list(self.$get())));
                for item in items { self.$get().expect("source root list exists").add(item); }
                self
            })*
        }
    };
}
root_lists! {
    with_hidden_dockables(Option<Rc<dyn IDockable>>) => hidden_dockables, set_hidden_dockables;
    with_left_pinned(Option<Rc<dyn IDockable>>) => left_pinned_dockables, set_left_pinned_dockables;
    with_right_pinned(Option<Rc<dyn IDockable>>) => right_pinned_dockables, set_right_pinned_dockables;
    with_top_pinned(Option<Rc<dyn IDockable>>) => top_pinned_dockables, set_top_pinned_dockables;
    with_bottom_pinned(Option<Rc<dyn IDockable>>) => bottom_pinned_dockables, set_bottom_pinned_dockables;
    with_windows(Option<Rc<dyn IDockWindow>>) => windows, set_windows;
}

macro_rules! assign_before {
    (before, $output:ident, $item:ident) => {
        *$output = Some($item.clone());
    };
    (after, $output:ident, $item:ident) => {};
}
macro_rules! assign_after {
    (after, $output:ident, $item:ident) => {
        *$output = Some($item);
    };
    (before, $output:ident, $item:ident) => {};
}
macro_rules! factory_creators {
    ($($method:ident, $into:ident, $type:ident, $create:ident, $order:ident, [$($arg:ident: $arg_type:ty => $setter:ident),*];)*) => {
        /// Source factory creation helpers. Explicit parameters correspond to the
        /// source optional arguments; `_default` adapters below retain their defaults.
        pub trait FactoryFluentExt: Sized {
            $(#[doc = concat!("Calls `", stringify!($create), "`, assigns initial properties, configures once, and returns the model.")]
            fn $method(self, $($arg: $arg_type,)* configure: Option<&dyn Fn(Rc<dyn $type>)>) -> Rc<dyn $type>;
            #[doc = concat!("The source out overload of `", stringify!($method), "`; returns the same factory and preserves output/configuration order.")]
            fn $into(self, output: &mut Option<Rc<dyn $type>>, $($arg: $arg_type,)* configure: Option<&dyn Fn(Rc<dyn $type>)>) -> Self;)*
            /// Creates a proportional dock with the source Horizontal orientation.
            fn proportional_dock_default(self, configure: Option<&dyn Fn(Rc<dyn IProportionalDock>)>) -> Rc<dyn IProportionalDock> {
                self.proportional_dock(Orientation::Horizontal, configure)
            }
            /// Creates a splitter with source CanResize=true and ResizePreview=false.
            fn proportional_dock_splitter_default(self, configure: Option<&dyn Fn(Rc<dyn IProportionalDockSplitter>)>) -> Rc<dyn IProportionalDockSplitter> {
                self.proportional_dock_splitter(true, false, configure)
            }
            /// Creates a tool dock with the source Left alignment.
            fn tool_dock_default(self, configure: Option<&dyn Fn(Rc<dyn IToolDock>)>) -> Rc<dyn IToolDock> {
                self.tool_dock(Alignment::Left, configure)
            }
            /// The out overload with source Horizontal orientation.
            fn proportional_dock_into_default(self, output: &mut Option<Rc<dyn IProportionalDock>>, configure: Option<&dyn Fn(Rc<dyn IProportionalDock>)>) -> Self {
                self.proportional_dock_into(output, Orientation::Horizontal, configure)
            }
            /// The out overload with source CanResize=true and ResizePreview=false.
            fn proportional_dock_splitter_into_default(self, output: &mut Option<Rc<dyn IProportionalDockSplitter>>, configure: Option<&dyn Fn(Rc<dyn IProportionalDockSplitter>)>) -> Self {
                self.proportional_dock_splitter_into(output, true, false, configure)
            }
            /// The out overload with source Left alignment.
            fn tool_dock_into_default(self, output: &mut Option<Rc<dyn IToolDock>>, configure: Option<&dyn Fn(Rc<dyn IToolDock>)>) -> Self {
                self.tool_dock_into(output, Alignment::Left, configure)
            }
        }
        impl<T: IFactory + ?Sized> FactoryFluentExt for Rc<T> {
            $(fn $method(self, $($arg: $arg_type,)* configure: Option<&dyn Fn(Rc<dyn $type>)>) -> Rc<dyn $type> {
                let item = self.$create();
                $(item.$setter($arg);)*
                if let Some(configure) = configure { configure(item.clone()); }
                item
            }
            fn $into(self, output: &mut Option<Rc<dyn $type>>, $($arg: $arg_type,)* configure: Option<&dyn Fn(Rc<dyn $type>)>) -> Self {
                let item = self.$create();
                assign_before!($order, output, item);
                $(item.$setter($arg);)*
                if let Some(configure) = configure { configure(item.clone()); }
                assign_after!($order, output, item);
                self
            })*
        }
    };
}
factory_creators! {
    document, document_into, IDocument, create_document, after, [];
    tool, tool_into, ITool, create_tool, after, [];
    root_dock, root_dock_into, IRootDock, create_root_dock, before, [];
    proportional_dock, proportional_dock_into, IProportionalDock, create_proportional_dock, before, [orientation: Orientation => set_orientation];
    dock_dock, dock_dock_into, IDockDock, create_dock_dock, before, [];
    stack_dock, stack_dock_into, IStackDock, create_stack_dock, before, [];
    grid_dock, grid_dock_into, IGridDock, create_grid_dock, before, [];
    wrap_dock, wrap_dock_into, IWrapDock, create_wrap_dock, before, [];
    uniform_grid_dock, uniform_grid_dock_into, IUniformGridDock, create_uniform_grid_dock, before, [];
    proportional_dock_splitter, proportional_dock_splitter_into, IProportionalDockSplitter, create_proportional_dock_splitter, before, [can_resize: bool => set_can_resize, resize_preview: bool => set_resize_preview];
    grid_dock_splitter, grid_dock_splitter_into, IGridDockSplitter, create_grid_dock_splitter, before, [];
    tool_dock, tool_dock_into, IToolDock, create_tool_dock, before, [alignment: Alignment => set_alignment];
    document_dock, document_dock_into, IDocumentDock, create_document_dock, before, [];
    dock_window, dock_window_into, IDockWindow, create_dock_window, before, [];
    split_view_dock, split_view_dock_into, ISplitViewDock, create_split_view_dock, before, [];
}
