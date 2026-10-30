//! Test-only fixtures implementing the full docking contracts.
use ferrodock_controls_recycling_model::IControlRecyclingIdProvider;
use ferrodock_model::adapters::TrackingAdapter;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::ICommand;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type DockableList = ListHandle<Option<Rc<dyn IDockable>>>;
type WindowList = ListHandle<Option<Rc<dyn IDockWindow>>>;

pub(crate) struct TestCommand;

pub(crate) struct TestControl {
    layout: RefCell<Option<Rc<dyn IDock>>>,
    pub(crate) reads: Cell<usize>,
}
impl TestControl {
    pub(crate) fn new(layout: Option<Rc<dyn IDock>>) -> Rc<Self> {
        Rc::new(Self {
            layout: RefCell::new(layout),
            reads: Cell::new(0),
        })
    }
}
impl IDockControl for TestControl {
    fn layout(&self) -> Option<Rc<dyn IDock>> {
        self.reads.set(self.reads.get() + 1);
        self.layout.borrow().clone()
    }
    fn set_layout(&self, value: Option<Rc<dyn IDock>>) {
        self.layout.replace(value);
    }
    fn dock_manager(&self) -> Rc<dyn IDockManager> {
        panic!("unused control fixture member")
    }
    fn dock_control_state(&self) -> Rc<dyn IDockControlState> {
        panic!("unused control fixture member")
    }
    fn default_context(&self) -> Option<BoxedValue> {
        panic!("unused control fixture member")
    }
    fn set_default_context(&self, _: Option<BoxedValue>) {
        panic!("unused control fixture member")
    }
    fn initialize_layout(&self) -> bool {
        panic!("unused control fixture member")
    }
    fn set_initialize_layout(&self, _: bool) {
        panic!("unused control fixture member")
    }
    fn initialize_factory(&self) -> bool {
        panic!("unused control fixture member")
    }
    fn set_initialize_factory(&self, _: bool) {
        panic!("unused control fixture member")
    }
    fn factory(&self) -> Option<Rc<dyn IFactory>> {
        panic!("unused control fixture member")
    }
    fn set_factory(&self, _: Option<Rc<dyn IFactory>>) {
        panic!("unused control fixture member")
    }
}

impl ICommand for TestCommand {
    fn can_execute(&self, _: Option<&dyn Any>) -> bool {
        true
    }
    fn execute(&self, _: Option<&dyn Any>) {}
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

pub(crate) struct NodeState {
    pub(crate) id: RefCell<String>,
    pub(crate) title: RefCell<String>,
    pub(crate) context: RefCell<Option<BoxedValue>>,
    pub(crate) owner: RefCell<Option<Rc<dyn IDockable>>>,
    pub(crate) original_owner: RefCell<Option<Rc<dyn IDockable>>>,
    pub(crate) factory: RefCell<Option<Rc<dyn IFactory>>>,
    pub(crate) is_empty: Cell<bool>,
    pub(crate) is_collapsable: Cell<bool>,
    pub(crate) proportion: Cell<f64>,
    pub(crate) dock: Cell<DockMode>,
    pub(crate) docking_state: Cell<DockingWindowState>,
    pub(crate) column: Cell<i32>,
    pub(crate) row: Cell<i32>,
    pub(crate) column_span: Cell<i32>,
    pub(crate) row_span: Cell<i32>,
    pub(crate) is_shared_size_scope: Cell<bool>,
    pub(crate) collapsed_proportion: Cell<f64>,
    pub(crate) min_width: Cell<f64>,
    pub(crate) max_width: Cell<f64>,
    pub(crate) min_height: Cell<f64>,
    pub(crate) max_height: Cell<f64>,
    pub(crate) can_close: Cell<bool>,
    pub(crate) can_pin: Cell<bool>,
    pub(crate) keep_pinned_dockable_visible: Cell<bool>,
    pub(crate) pinned_dock_display_mode_override: RefCell<Option<PinnedDockDisplayMode>>,
    pub(crate) pinned_bounds: RefCell<Option<DockRect>>,
    pub(crate) can_float: Cell<bool>,
    pub(crate) can_drag: Cell<bool>,
    pub(crate) can_drop: Cell<bool>,
    pub(crate) can_dock_as_document: Cell<bool>,
    pub(crate) dock_capability_overrides: RefCell<Option<Rc<DockCapabilityOverrides>>>,
    pub(crate) is_modified: Cell<bool>,
    pub(crate) dock_group: RefCell<Option<String>>,
    pub(crate) on_group_read: RefCell<Option<Rc<dyn Fn()>>>,
    tracking: TrackingAdapter,
}
impl Default for NodeState {
    fn default() -> Self {
        Self {
            id: RefCell::new(Default::default()),
            title: RefCell::new(Default::default()),
            context: RefCell::new(Default::default()),
            owner: RefCell::new(Default::default()),
            original_owner: RefCell::new(Default::default()),
            factory: RefCell::new(Default::default()),
            is_empty: Cell::new(Default::default()),
            is_collapsable: Cell::new(Default::default()),
            proportion: Cell::new(Default::default()),
            dock: Cell::new(Default::default()),
            docking_state: Cell::new(Default::default()),
            column: Cell::new(Default::default()),
            row: Cell::new(Default::default()),
            column_span: Cell::new(Default::default()),
            row_span: Cell::new(Default::default()),
            is_shared_size_scope: Cell::new(Default::default()),
            collapsed_proportion: Cell::new(Default::default()),
            min_width: Cell::new(Default::default()),
            max_width: Cell::new(Default::default()),
            min_height: Cell::new(Default::default()),
            max_height: Cell::new(Default::default()),
            can_close: Cell::new(true),
            can_pin: Cell::new(true),
            keep_pinned_dockable_visible: Cell::new(Default::default()),
            pinned_dock_display_mode_override: RefCell::new(Default::default()),
            pinned_bounds: RefCell::new(Default::default()),
            can_float: Cell::new(true),
            can_drag: Cell::new(true),
            can_drop: Cell::new(true),
            can_dock_as_document: Cell::new(true),
            dock_capability_overrides: RefCell::new(Default::default()),
            is_modified: Cell::new(Default::default()),
            dock_group: RefCell::new(Default::default()),
            on_group_read: RefCell::new(None),
            tracking: TrackingAdapter::default(),
        }
    }
}

pub(crate) struct DockState {
    pub(crate) visible_dockables: RefCell<Option<DockableList>>,
    pub(crate) active_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    pub(crate) default_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    pub(crate) focused_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    pub(crate) is_active: Cell<bool>,
    pub(crate) opened_dockables_count: Cell<i32>,
    pub(crate) can_close_last_dockable: Cell<bool>,
    pub(crate) dock_capability_policy: RefCell<Option<Rc<dyn DockCapabilityPolicyImpl>>>,
    pub(crate) can_go_back: Cell<bool>,
    pub(crate) can_go_forward: Cell<bool>,
    pub(crate) go_back: RefCell<Rc<dyn ICommand>>,
    pub(crate) go_forward: RefCell<Rc<dyn ICommand>>,
    pub(crate) navigate: RefCell<Rc<dyn ICommand>>,
    pub(crate) close: RefCell<Rc<dyn ICommand>>,
    pub(crate) enable_global_docking: Cell<bool>,
}
impl Default for DockState {
    fn default() -> Self {
        Self {
            visible_dockables: RefCell::new(Some(ListHandle::new())),
            active_dockable: RefCell::new(Default::default()),
            default_dockable: RefCell::new(Default::default()),
            focused_dockable: RefCell::new(Default::default()),
            is_active: Cell::new(Default::default()),
            opened_dockables_count: Cell::new(Default::default()),
            can_close_last_dockable: Cell::new(true),
            dock_capability_policy: RefCell::new(Default::default()),
            can_go_back: Cell::new(Default::default()),
            can_go_forward: Cell::new(Default::default()),
            go_back: RefCell::new(Rc::new(TestCommand) as Rc<dyn ICommand>),
            go_forward: RefCell::new(Rc::new(TestCommand) as Rc<dyn ICommand>),
            navigate: RefCell::new(Rc::new(TestCommand) as Rc<dyn ICommand>),
            close: RefCell::new(Rc::new(TestCommand) as Rc<dyn ICommand>),
            enable_global_docking: Cell::new(true),
        }
    }
}

pub(crate) struct RootState {
    pub(crate) is_focusable_root: Cell<bool>,
    pub(crate) hidden_dockables: RefCell<Option<DockableList>>,
    pub(crate) left_pinned_dockables: RefCell<Option<DockableList>>,
    pub(crate) right_pinned_dockables: RefCell<Option<DockableList>>,
    pub(crate) top_pinned_dockables: RefCell<Option<DockableList>>,
    pub(crate) bottom_pinned_dockables: RefCell<Option<DockableList>>,
    pub(crate) pinned_dock: RefCell<Option<Rc<dyn IToolDock>>>,
    pub(crate) pinned_dock_display_mode: Cell<PinnedDockDisplayMode>,
    pub(crate) window: RefCell<Option<Rc<dyn IDockWindow>>>,
    pub(crate) windows: RefCell<Option<WindowList>>,
    pub(crate) floating_window_host_mode: Cell<DockFloatingWindowHostMode>,
    pub(crate) root_dock_capability_policy: RefCell<Option<Rc<dyn DockCapabilityPolicyImpl>>>,
    pub(crate) show_windows: RefCell<Rc<dyn ICommand>>,
    pub(crate) exit_windows: RefCell<Rc<dyn ICommand>>,
    pub(crate) enable_adaptive_global_dock_targets: Cell<bool>,
}
impl Default for RootState {
    fn default() -> Self {
        Self {
            is_focusable_root: Cell::new(Default::default()),
            hidden_dockables: RefCell::new(Default::default()),
            left_pinned_dockables: RefCell::new(Default::default()),
            right_pinned_dockables: RefCell::new(Default::default()),
            top_pinned_dockables: RefCell::new(Default::default()),
            bottom_pinned_dockables: RefCell::new(Default::default()),
            pinned_dock: RefCell::new(Default::default()),
            pinned_dock_display_mode: Cell::new(Default::default()),
            window: RefCell::new(Default::default()),
            windows: RefCell::new(Default::default()),
            floating_window_host_mode: Cell::new(Default::default()),
            root_dock_capability_policy: RefCell::new(Default::default()),
            show_windows: RefCell::new(Rc::new(TestCommand) as Rc<dyn ICommand>),
            exit_windows: RefCell::new(Rc::new(TestCommand) as Rc<dyn ICommand>),
            enable_adaptive_global_dock_targets: Cell::new(Default::default()),
        }
    }
}

type WindowLayoutGetter = Rc<dyn Fn() -> Option<Rc<dyn IRootDock>>>;

pub(crate) struct WindowState {
    pub(crate) id: RefCell<String>,
    pub(crate) x: Cell<f64>,
    pub(crate) y: Cell<f64>,
    pub(crate) width: Cell<f64>,
    pub(crate) height: Cell<f64>,
    pub(crate) window_state: Cell<DockWindowState>,
    pub(crate) topmost: Cell<bool>,
    pub(crate) title: RefCell<String>,
    pub(crate) owner_mode: Cell<DockWindowOwnerMode>,
    pub(crate) parent_window: RefCell<Option<Rc<dyn IDockWindow>>>,
    pub(crate) is_modal: Cell<bool>,
    pub(crate) show_in_taskbar: RefCell<Option<bool>>,
    pub(crate) owner: RefCell<Option<Rc<dyn IDockable>>>,
    pub(crate) factory: RefCell<Option<Rc<dyn IFactory>>>,
    pub(crate) layout: RefCell<Option<Rc<dyn IRootDock>>>,
    pub(crate) layout_getter: RefCell<Option<WindowLayoutGetter>>,
    pub(crate) host: RefCell<Option<Rc<dyn IHostWindow>>>,
    pub(crate) calls: RefCell<Vec<&'static str>>,
    pub(crate) on_exit: RefCell<Option<Rc<dyn Fn()>>>,
}
impl Default for WindowState {
    fn default() -> Self {
        Self {
            id: RefCell::new(Default::default()),
            x: Cell::new(Default::default()),
            y: Cell::new(Default::default()),
            width: Cell::new(Default::default()),
            height: Cell::new(Default::default()),
            window_state: Cell::new(Default::default()),
            topmost: Cell::new(Default::default()),
            title: RefCell::new(Default::default()),
            owner_mode: Cell::new(Default::default()),
            parent_window: RefCell::new(Default::default()),
            is_modal: Cell::new(Default::default()),
            show_in_taskbar: RefCell::new(Default::default()),
            owner: RefCell::new(Default::default()),
            factory: RefCell::new(Default::default()),
            layout: RefCell::new(Default::default()),
            layout_getter: RefCell::new(None),
            host: RefCell::new(Default::default()),
            calls: RefCell::new(Vec::new()),
            on_exit: RefCell::new(None),
        }
    }
}
macro_rules! impl_i_dockable { ($type:ty $(, $extra:item)*) => { impl IDockable for $type {
 fn id(&self) -> String { self.node.id.borrow().clone() }
 fn set_id(&self, value: String) { *self.node.id.borrow_mut() = value; }
 fn title(&self) -> String { self.node.title.borrow().clone() }
 fn set_title(&self, value: String) { *self.node.title.borrow_mut() = value; }
 fn context(&self) -> Option<BoxedValue> { self.node.context.borrow().clone() }
 fn set_context(&self, value: Option<BoxedValue>) { *self.node.context.borrow_mut() = value; }
 fn owner(&self) -> Option<Rc<dyn IDockable>> { self.node.owner.borrow().clone() }
 fn set_owner(&self, value: Option<Rc<dyn IDockable>>) { *self.node.owner.borrow_mut() = value; }
 fn original_owner(&self) -> Option<Rc<dyn IDockable>> { self.node.original_owner.borrow().clone() }
 fn set_original_owner(&self, value: Option<Rc<dyn IDockable>>) { *self.node.original_owner.borrow_mut() = value; }
 fn factory(&self) -> Option<Rc<dyn IFactory>> { self.node.factory.borrow().clone() }
 fn set_factory(&self, value: Option<Rc<dyn IFactory>>) { *self.node.factory.borrow_mut() = value; }
 fn is_empty(&self) -> bool { self.node.is_empty.get() }
 fn set_is_empty(&self, value: bool) { self.node.is_empty.set(value); }
 fn is_collapsable(&self) -> bool { self.node.is_collapsable.get() }
 fn set_is_collapsable(&self, value: bool) { self.node.is_collapsable.set(value); }
 fn proportion(&self) -> f64 { self.node.proportion.get() }
 fn set_proportion(&self, value: f64) { self.node.proportion.set(value); }
 fn dock(&self) -> DockMode { self.node.dock.get() }
 fn set_dock(&self, value: DockMode) { self.node.dock.set(value); }
 fn docking_state(&self) -> DockingWindowState { self.node.docking_state.get() }
 fn set_docking_state(&self, value: DockingWindowState) { self.node.docking_state.set(value); }
 fn column(&self) -> i32 { self.node.column.get() }
 fn set_column(&self, value: i32) { self.node.column.set(value); }
 fn row(&self) -> i32 { self.node.row.get() }
 fn set_row(&self, value: i32) { self.node.row.set(value); }
 fn column_span(&self) -> i32 { self.node.column_span.get() }
 fn set_column_span(&self, value: i32) { self.node.column_span.set(value); }
 fn row_span(&self) -> i32 { self.node.row_span.get() }
 fn set_row_span(&self, value: i32) { self.node.row_span.set(value); }
 fn is_shared_size_scope(&self) -> bool { self.node.is_shared_size_scope.get() }
 fn set_is_shared_size_scope(&self, value: bool) { self.node.is_shared_size_scope.set(value); }
 fn collapsed_proportion(&self) -> f64 { self.node.collapsed_proportion.get() }
 fn set_collapsed_proportion(&self, value: f64) { self.node.collapsed_proportion.set(value); }
 fn min_width(&self) -> f64 { self.node.min_width.get() }
 fn set_min_width(&self, value: f64) { self.node.min_width.set(value); }
 fn max_width(&self) -> f64 { self.node.max_width.get() }
 fn set_max_width(&self, value: f64) { self.node.max_width.set(value); }
 fn min_height(&self) -> f64 { self.node.min_height.get() }
 fn set_min_height(&self, value: f64) { self.node.min_height.set(value); }
 fn max_height(&self) -> f64 { self.node.max_height.get() }
 fn set_max_height(&self, value: f64) { self.node.max_height.set(value); }
 fn can_close(&self) -> bool { self.node.can_close.get() }
 fn set_can_close(&self, value: bool) { self.node.can_close.set(value); }
 fn can_pin(&self) -> bool { self.node.can_pin.get() }
 fn set_can_pin(&self, value: bool) { self.node.can_pin.set(value); }
 fn keep_pinned_dockable_visible(&self) -> bool { self.node.keep_pinned_dockable_visible.get() }
 fn set_keep_pinned_dockable_visible(&self, value: bool) { self.node.keep_pinned_dockable_visible.set(value); }
 fn pinned_dock_display_mode_override(&self) -> Option<PinnedDockDisplayMode> { self.node.pinned_dock_display_mode_override.borrow().clone() }
 fn set_pinned_dock_display_mode_override(&self, value: Option<PinnedDockDisplayMode>) { *self.node.pinned_dock_display_mode_override.borrow_mut() = value; }
 fn pinned_bounds(&self) -> Option<DockRect> { self.node.pinned_bounds.borrow().clone() }
 fn set_pinned_bounds_rect(&self, value: Option<DockRect>) { *self.node.pinned_bounds.borrow_mut() = value; }
 fn can_float(&self) -> bool { self.node.can_float.get() }
 fn set_can_float(&self, value: bool) { self.node.can_float.set(value); }
 fn can_drag(&self) -> bool { self.node.can_drag.get() }
 fn set_can_drag(&self, value: bool) { self.node.can_drag.set(value); }
 fn can_drop(&self) -> bool { self.node.can_drop.get() }
 fn set_can_drop(&self, value: bool) { self.node.can_drop.set(value); }
 fn can_dock_as_document(&self) -> bool { self.node.can_dock_as_document.get() }
 fn set_can_dock_as_document(&self, value: bool) { self.node.can_dock_as_document.set(value); }
 fn dock_capability_overrides(&self) -> Option<Rc<DockCapabilityOverrides>> { self.node.dock_capability_overrides.borrow().clone() }
 fn set_dock_capability_overrides(&self, value: Option<Rc<DockCapabilityOverrides>>) { *self.node.dock_capability_overrides.borrow_mut() = value; }
 fn is_modified(&self) -> bool { self.node.is_modified.get() }
 fn set_is_modified(&self, value: bool) { self.node.is_modified.set(value); }
 fn dock_group(&self) -> Option<String> {
     let callback = self.node.on_group_read.borrow().clone();
     if let Some(callback) = callback { callback(); }
     self.node.dock_group.borrow().clone()
 }
 fn set_dock_group(&self, value: Option<String>) { *self.node.dock_group.borrow_mut() = value; }
 fn on_close(&self) -> bool { true }
 fn on_selected(&self) {  }
 fn get_visible_bounds(&self) -> (f64, f64, f64, f64) { self.node.tracking.get_visible_bounds() }
 fn set_visible_bounds(&self, _x: f64, _y: f64, _width: f64, _height: f64) { self.node.tracking.set_visible_bounds(_x, _y, _width, _height); }
 fn on_visible_bounds_changed(&self, _x: f64, _y: f64, _width: f64, _height: f64) {  }
 fn get_pinned_bounds(&self) -> (f64, f64, f64, f64) { self.node.tracking.get_pinned_bounds() }
 fn set_pinned_bounds(&self, _x: f64, _y: f64, _width: f64, _height: f64) { self.node.tracking.set_pinned_bounds(_x, _y, _width, _height); }
 fn on_pinned_bounds_changed(&self, _x: f64, _y: f64, _width: f64, _height: f64) {  }
 fn get_tab_bounds(&self) -> (f64, f64, f64, f64) { self.node.tracking.get_tab_bounds() }
 fn set_tab_bounds(&self, _x: f64, _y: f64, _width: f64, _height: f64) { self.node.tracking.set_tab_bounds(_x, _y, _width, _height); }
 fn on_tab_bounds_changed(&self, _x: f64, _y: f64, _width: f64, _height: f64) {  }
 fn get_pointer_position(&self) -> (f64, f64) { self.node.tracking.get_pointer_position() }
 fn set_pointer_position(&self, _x: f64, _y: f64) { self.node.tracking.set_pointer_position(_x, _y); }
 fn on_pointer_position_changed(&self, _x: f64, _y: f64) {  }
 fn get_pointer_screen_position(&self) -> (f64, f64) { self.node.tracking.get_pointer_screen_position() }
 fn set_pointer_screen_position(&self, _x: f64, _y: f64) { self.node.tracking.set_pointer_screen_position(_x, _y); }
 fn on_pointer_screen_position_changed(&self, _x: f64, _y: f64) {  }
 $($extra)*
} }; }
macro_rules! impl_i_dock { ($type:ty $(, $extra:item)*) => { impl IDock for $type {
 fn visible_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> { self.dock.visible_dockables.borrow().clone() }
 fn set_visible_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) { *self.dock.visible_dockables.borrow_mut() = value; }
 fn active_dockable(&self) -> Option<Rc<dyn IDockable>> { self.dock.active_dockable.borrow().clone() }
 fn set_active_dockable(&self, value: Option<Rc<dyn IDockable>>) { *self.dock.active_dockable.borrow_mut() = value; }
 fn default_dockable(&self) -> Option<Rc<dyn IDockable>> { self.dock.default_dockable.borrow().clone() }
 fn set_default_dockable(&self, value: Option<Rc<dyn IDockable>>) { *self.dock.default_dockable.borrow_mut() = value; }
 fn focused_dockable(&self) -> Option<Rc<dyn IDockable>> { self.dock.focused_dockable.borrow().clone() }
 fn set_focused_dockable(&self, value: Option<Rc<dyn IDockable>>) { *self.dock.focused_dockable.borrow_mut() = value; }
 fn is_active(&self) -> bool { self.dock.is_active.get() }
 fn set_is_active(&self, value: bool) { self.dock.is_active.set(value); }
 fn opened_dockables_count(&self) -> i32 { self.dock.opened_dockables_count.get() }
 fn set_opened_dockables_count(&self, value: i32) { self.dock.opened_dockables_count.set(value); }
 fn can_close_last_dockable(&self) -> bool { self.dock.can_close_last_dockable.get() }
 fn set_can_close_last_dockable(&self, value: bool) { self.dock.can_close_last_dockable.set(value); }
 fn dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> { self.dock.dock_capability_policy.borrow().clone() }
 fn set_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>) { *self.dock.dock_capability_policy.borrow_mut() = value; }
 fn can_go_back(&self) -> bool { self.dock.can_go_back.get() }
 fn can_go_forward(&self) -> bool { self.dock.can_go_forward.get() }
 fn go_back(&self) -> Rc<dyn ICommand> { self.dock.go_back.borrow().clone() }
 fn go_forward(&self) -> Rc<dyn ICommand> { self.dock.go_forward.borrow().clone() }
 fn navigate(&self) -> Rc<dyn ICommand> { self.dock.navigate.borrow().clone() }
 fn close(&self) -> Rc<dyn ICommand> { self.dock.close.borrow().clone() }
 fn enable_global_docking(&self) -> bool { self.dock.enable_global_docking.get() }
 fn set_enable_global_docking(&self, value: bool) { self.dock.enable_global_docking.set(value); }
 $($extra)*
} }; }
macro_rules! impl_i_root_dock { ($type:ty $(, $extra:item)*) => { impl IRootDock for $type {
 fn is_focusable_root(&self) -> bool { self.root.is_focusable_root.get() }
 fn set_is_focusable_root(&self, value: bool) { self.root.is_focusable_root.set(value); }
 fn hidden_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> { self.root.hidden_dockables.borrow().clone() }
 fn set_hidden_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) { *self.root.hidden_dockables.borrow_mut() = value; }
 fn left_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> { self.root.left_pinned_dockables.borrow().clone() }
 fn set_left_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) { *self.root.left_pinned_dockables.borrow_mut() = value; }
 fn right_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> { self.root.right_pinned_dockables.borrow().clone() }
 fn set_right_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) { *self.root.right_pinned_dockables.borrow_mut() = value; }
 fn top_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> { self.root.top_pinned_dockables.borrow().clone() }
 fn set_top_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) { *self.root.top_pinned_dockables.borrow_mut() = value; }
 fn bottom_pinned_dockables(&self) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> { self.root.bottom_pinned_dockables.borrow().clone() }
 fn set_bottom_pinned_dockables(&self, value: Option<ListHandle<Option<Rc<dyn IDockable>>>>) { *self.root.bottom_pinned_dockables.borrow_mut() = value; }
 fn pinned_dock(&self) -> Option<Rc<dyn IToolDock>> { self.root.pinned_dock.borrow().clone() }
 fn set_pinned_dock(&self, value: Option<Rc<dyn IToolDock>>) { *self.root.pinned_dock.borrow_mut() = value; }
 fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode { self.root.pinned_dock_display_mode.get() }
 fn set_pinned_dock_display_mode(&self, value: PinnedDockDisplayMode) { self.root.pinned_dock_display_mode.set(value); }
 fn window(&self) -> Option<Rc<dyn IDockWindow>> { self.root.window.borrow().clone() }
 fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) { *self.root.window.borrow_mut() = value; }
 fn windows(&self) -> Option<ListHandle<Option<Rc<dyn IDockWindow>>>> { self.root.windows.borrow().clone() }
 fn set_windows(&self, value: Option<ListHandle<Option<Rc<dyn IDockWindow>>>>) { *self.root.windows.borrow_mut() = value; }
 fn floating_window_host_mode(&self) -> DockFloatingWindowHostMode { self.root.floating_window_host_mode.get() }
 fn set_floating_window_host_mode(&self, value: DockFloatingWindowHostMode) { self.root.floating_window_host_mode.set(value); }
 fn root_dock_capability_policy(&self) -> Option<Rc<dyn DockCapabilityPolicyImpl>> { self.root.root_dock_capability_policy.borrow().clone() }
 fn set_root_dock_capability_policy(&self, value: Option<Rc<dyn DockCapabilityPolicyImpl>>) { *self.root.root_dock_capability_policy.borrow_mut() = value; }
 fn show_windows(&self) -> Rc<dyn ICommand> { self.root.show_windows.borrow().clone() }
 fn exit_windows(&self) -> Rc<dyn ICommand> { self.root.exit_windows.borrow().clone() }
 fn enable_adaptive_global_dock_targets(&self) -> bool { self.root.enable_adaptive_global_dock_targets.get() }
 fn set_enable_adaptive_global_dock_targets(&self, value: bool) { self.root.enable_adaptive_global_dock_targets.set(value); }
 $($extra)*
} }; }
macro_rules! impl_i_dock_window { ($type:ty $(, $extra:item)*) => { impl IDockWindow for $type {
 fn id(&self) -> String { self.window.id.borrow().clone() }
 fn set_id(&self, value: String) { self.window.calls.borrow_mut().push("Id"); *self.window.id.borrow_mut() = value; }
 fn x(&self) -> f64 { self.window.x.get() }
 fn set_x(&self, value: f64) { self.window.calls.borrow_mut().push("X"); self.window.x.set(value); }
 fn y(&self) -> f64 { self.window.y.get() }
 fn set_y(&self, value: f64) { self.window.calls.borrow_mut().push("Y"); self.window.y.set(value); }
 fn width(&self) -> f64 { self.window.width.get() }
 fn set_width(&self, value: f64) { self.window.calls.borrow_mut().push("Width"); self.window.width.set(value); }
 fn height(&self) -> f64 { self.window.height.get() }
 fn set_height(&self, value: f64) { self.window.calls.borrow_mut().push("Height"); self.window.height.set(value); }
 fn window_state(&self) -> DockWindowState { self.window.window_state.get() }
 fn set_window_state(&self, value: DockWindowState) { self.window.calls.borrow_mut().push("WindowState"); self.window.window_state.set(value); }
 fn topmost(&self) -> bool { self.window.topmost.get() }
 fn set_topmost(&self, value: bool) { self.window.calls.borrow_mut().push("Topmost"); self.window.topmost.set(value); }
 fn title(&self) -> String { self.window.title.borrow().clone() }
 fn set_title(&self, value: String) { self.window.calls.borrow_mut().push("Title"); *self.window.title.borrow_mut() = value; }
 fn owner_mode(&self) -> DockWindowOwnerMode { self.window.owner_mode.get() }
 fn set_owner_mode(&self, value: DockWindowOwnerMode) { self.window.calls.borrow_mut().push("OwnerMode"); self.window.owner_mode.set(value); }
 fn parent_window(&self) -> Option<Rc<dyn IDockWindow>> { self.window.parent_window.borrow().clone() }
 fn set_parent_window(&self, value: Option<Rc<dyn IDockWindow>>) { self.window.calls.borrow_mut().push("ParentWindow"); *self.window.parent_window.borrow_mut() = value; }
 fn is_modal(&self) -> bool { self.window.is_modal.get() }
 fn set_is_modal(&self, value: bool) { self.window.calls.borrow_mut().push("IsModal"); self.window.is_modal.set(value); }
 fn show_in_taskbar(&self) -> Option<bool> { self.window.show_in_taskbar.borrow().clone() }
 fn set_show_in_taskbar(&self, value: Option<bool>) { self.window.calls.borrow_mut().push("ShowInTaskbar"); *self.window.show_in_taskbar.borrow_mut() = value; }
 fn owner(&self) -> Option<Rc<dyn IDockable>> { self.window.owner.borrow().clone() }
 fn set_owner(&self, value: Option<Rc<dyn IDockable>>) { self.window.calls.borrow_mut().push("Owner"); *self.window.owner.borrow_mut() = value; }
 fn factory(&self) -> Option<Rc<dyn IFactory>> { self.window.factory.borrow().clone() }
 fn set_factory(&self, value: Option<Rc<dyn IFactory>>) { self.window.calls.borrow_mut().push("Factory"); *self.window.factory.borrow_mut() = value; }
 fn layout(&self) -> Option<Rc<dyn IRootDock>> { let getter = self.window.layout_getter.borrow().clone(); getter.map_or_else(|| self.window.layout.borrow().clone(), |getter| getter()) }
 fn set_layout(&self, value: Option<Rc<dyn IRootDock>>) { self.window.calls.borrow_mut().push("Layout"); *self.window.layout.borrow_mut() = value; }
 fn host(&self) -> Option<Rc<dyn IHostWindow>> { self.window.host.borrow().clone() }
 fn set_host(&self, value: Option<Rc<dyn IHostWindow>>) { self.window.calls.borrow_mut().push("Host"); *self.window.host.borrow_mut() = value; }
 fn on_close(&self) -> bool { true }
 fn on_move_drag_begin(&self) -> bool { true }
 fn on_move_drag(&self) {  }
 fn on_move_drag_end(&self) {  }
 fn save(&self) { self.window.calls.borrow_mut().push("Save"); }
 fn present(&self, is_dialog: bool) { self.window.calls.borrow_mut().push(if is_dialog { "PresentModal" } else { "Present" }); }
 fn exit(&self) {
     self.window.calls.borrow_mut().push("Exit");
     let on_exit = self.window.on_exit.borrow().clone();
     if let Some(on_exit) = on_exit { on_exit(); }
 }
 fn set_active(&self) {  }
 $($extra)*
} }; }
pub(crate) struct TestNode {
    pub(crate) node: NodeState,
}
impl TestNode {
    pub(crate) fn new(group: Option<&str>) -> Rc<Self> {
        let instance = Rc::new(Self {
            node: NodeState::default(),
        });
        *instance.node.dock_group.borrow_mut() = group.map(str::to_owned);
        instance
    }
}
impl IControlRecyclingIdProvider for TestNode {
    fn get_control_recycling_id(&self) -> Option<String> {
        Some(self.id())
    }
}
impl_i_dockable!(TestNode);
pub(crate) struct TestDock {
    pub(crate) node: NodeState,
    pub(crate) dock: DockState,
}
impl TestDock {
    pub(crate) fn new(group: Option<&str>) -> Rc<Self> {
        let instance = Rc::new(Self {
            node: NodeState::default(),
            dock: DockState::default(),
        });
        *instance.node.dock_group.borrow_mut() = group.map(str::to_owned);
        instance
    }
}
impl IControlRecyclingIdProvider for TestDock {
    fn get_control_recycling_id(&self) -> Option<String> {
        Some(self.id())
    }
}
impl_i_dockable!(
    TestDock,
    fn as_dock(self: Rc<Self>) -> Option<Rc<dyn IDock>> {
        Some(self)
    }
);
impl_i_dock!(TestDock);

#[derive(Default)]
pub(crate) struct SplitState {
    compact_pane_length: Cell<f64>,
    display_mode: Cell<SplitViewDisplayMode>,
    is_pane_open: Cell<bool>,
    open_pane_length: Cell<f64>,
    pane_placement: Cell<SplitViewPanePlacement>,
    use_light_dismiss_overlay_mode: Cell<bool>,
    pub(crate) pane_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    pub(crate) content_dockable: RefCell<Option<Rc<dyn IDockable>>>,
}
#[derive(Default)]
pub(crate) struct TestSplit {
    pub(crate) node: NodeState,
    pub(crate) dock: DockState,
    pub(crate) split: SplitState,
}
impl IControlRecyclingIdProvider for TestSplit {
    fn get_control_recycling_id(&self) -> Option<String> {
        Some(self.id())
    }
}
impl_i_dockable!(
    TestSplit,
    fn as_dock(self: Rc<Self>) -> Option<Rc<dyn IDock>> {
        Some(self)
    },
    fn as_split_view_dock(self: Rc<Self>) -> Option<Rc<dyn ISplitViewDock>> {
        Some(self)
    }
);
impl_i_dock!(TestSplit);
impl ISplitViewDock for TestSplit {
    fn compact_pane_length(&self) -> f64 {
        self.split.compact_pane_length.get()
    }
    fn set_compact_pane_length(&self, value: f64) {
        self.split.compact_pane_length.set(value);
    }
    fn display_mode(&self) -> SplitViewDisplayMode {
        self.split.display_mode.get()
    }
    fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.split.display_mode.set(value);
    }
    fn is_pane_open(&self) -> bool {
        self.split.is_pane_open.get()
    }
    fn set_is_pane_open(&self, value: bool) {
        self.split.is_pane_open.set(value);
    }
    fn open_pane_length(&self) -> f64 {
        self.split.open_pane_length.get()
    }
    fn set_open_pane_length(&self, value: f64) {
        self.split.open_pane_length.set(value);
    }
    fn pane_placement(&self) -> SplitViewPanePlacement {
        self.split.pane_placement.get()
    }
    fn set_pane_placement(&self, value: SplitViewPanePlacement) {
        self.split.pane_placement.set(value);
    }
    fn use_light_dismiss_overlay_mode(&self) -> bool {
        self.split.use_light_dismiss_overlay_mode.get()
    }
    fn set_use_light_dismiss_overlay_mode(&self, value: bool) {
        self.split.use_light_dismiss_overlay_mode.set(value);
    }
    fn pane_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.split.pane_dockable.borrow().clone()
    }
    fn set_pane_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        *self.split.pane_dockable.borrow_mut() = value;
    }
    fn content_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.split.content_dockable.borrow().clone()
    }
    fn set_content_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        *self.split.content_dockable.borrow_mut() = value;
    }
    fn open_pane(&self) {
        self.set_is_pane_open(true);
    }
    fn close_pane(&self) {
        self.set_is_pane_open(false);
    }
    fn toggle_pane(&self) {
        self.set_is_pane_open(!self.is_pane_open());
    }
}
pub(crate) struct TestRoot {
    pub(crate) node: NodeState,
    pub(crate) dock: DockState,
    pub(crate) root: RootState,
}
impl TestRoot {
    pub(crate) fn new(group: Option<&str>) -> Rc<Self> {
        let instance = Rc::new(Self {
            node: NodeState::default(),
            dock: DockState::default(),
            root: RootState::default(),
        });
        *instance.node.dock_group.borrow_mut() = group.map(str::to_owned);
        instance
    }
}
impl IControlRecyclingIdProvider for TestRoot {
    fn get_control_recycling_id(&self) -> Option<String> {
        Some(self.id())
    }
}
impl_i_dockable!(
    TestRoot,
    fn as_dock(self: Rc<Self>) -> Option<Rc<dyn IDock>> {
        Some(self)
    },
    fn as_root_dock(self: Rc<Self>) -> Option<Rc<dyn IRootDock>> {
        Some(self)
    }
);
impl_i_dock!(TestRoot);
impl_i_root_dock!(TestRoot);
pub(crate) struct TestWindow {
    pub(crate) window: WindowState,
}
impl TestWindow {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            window: WindowState::default(),
        })
    }
}
impl_i_dock_window!(TestWindow);

#[derive(Default)]
pub(crate) struct TestHost {
    pub(crate) window: RefCell<Option<Rc<dyn IDockWindow>>>,
}
impl IHostWindow for TestHost {
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        None
    }
    fn is_tracked(&self) -> bool {
        false
    }
    fn set_is_tracked(&self, _: bool) {}
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        *self.window.borrow_mut() = value;
    }
    fn present(&self, _: bool) {}
    fn exit(&self) {}
    fn set_position(&self, _: f64, _: f64) {}
    fn get_position(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_size(&self, _: f64, _: f64) {}
    fn get_size(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_window_state(&self, _: DockWindowState) {}
    fn get_window_state(&self) -> DockWindowState {
        DockWindowState::Normal
    }
    fn set_title(&self, _: Option<String>) {}
    fn set_layout(&self, _: Rc<dyn IDock>) {}
    fn set_active(&self) {}
}
