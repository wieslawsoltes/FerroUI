// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{ManagedWindowLayer, PinnedDockWindow, ToolDockControl};
use crate::{automation::peers::PinnedDockControlAutomationPeer, model_projection};
use ferrodock_model::{
    controls::IRootDock,
    core::{Alignment, IDockable, PinnedDockDisplayMode},
};
use ferrodock_settings::DockSettings;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, PointerEventArgs},
    instantiate,
    interactivity::{InteractiveImpl, RoutingStrategies},
    layout::{LayoutableImpl, LayoutableImplExt},
    reactive::IDisposable,
    FerroObject, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, PixelPoint, Point,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions, Vector, Visual,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
        Thumb,
    },
    ColumnDefinition, ColumnDefinitions, ContentControl, Control, ControlImpl, Grid, GridLength,
    GridSplitter, GridUnitType, RowDefinition, RowDefinitions, TopLevel, Window,
};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    rc::Rc,
};
type NativeDockableSubscription = (Ref<FerroObject>, Rc<dyn IDisposable>);
struct Subscription(Box<dyn Fn()>);
impl Subscription {
    fn dispose(self) {
        (self.0)()
    }
}
/// Original pinned content host, including inline sizing and native/managed overlays.
#[repr(C)]
pub struct PinnedDockControl {
    base: TemplatedControl,
    pinned_dock_grid: RefCell<Option<Ref<Grid>>>,
    pinned_dock: RefCell<Option<Ref<ContentControl>>>,
    pinned_dock_splitter: RefCell<Option<Ref<Thumb>>>,
    window: RefCell<Option<Ref<PinnedDockWindow>>>,
    owner_window: RefCell<Option<Ref<Window>>>,
    managed_layer: RefCell<Option<Ref<ManagedWindowLayer>>>,
    managed_pinned_host: RefCell<Option<Ref<Control>>>,
    last_pinned_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    last_pinned_width: Cell<f64>,
    last_pinned_height: Cell<f64>,
    is_resizing_pinned_dock: Cell<bool>,
    last_effective_display_mode: Cell<PinnedDockDisplayMode>,
    tracked_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    tracked_native_dockable: RefCell<Option<NativeDockableSubscription>>,
    tracked_notify_dockable: RefCell<Option<Subscription>>,
    is_pinned_dock_empty: Cell<bool>,
    skip_pinned_bounds_update: Cell<bool>,
    layout_events: RefCell<Vec<Rc<dyn IDisposable>>>,
    splitter_events: RefCell<Vec<Rc<dyn IDisposable>>>,
    owner_events: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(PinnedDockControl:TemplatedControl);
ferro_class_info!(PinnedDockControl {new:PinnedDockControl::new,markup:{attributes:[TemplatePart("PART_PinnedDock",type(Ref<ContentControl>)),TemplatePart("PART_PinnedDockGrid",type(Ref<Grid>)),TemplatePart("PART_PinnedDockSplitter",type(Ref<Thumb>))]}});
ferro_impl_classes!(PinnedDockControl:FerroObjectImpl,StyledElementImpl,ferroui_base::VisualImpl,InteractiveImpl,InputElementImpl);
impl ControlImpl for PinnedDockControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        PinnedDockControlAutomationPeer::new(this).upcast()
    }
}
impl LayoutableImpl for PinnedDockControl {
    fn measure_override(this: &Self, size: Size) -> Size {
        this.apply_pinned_dock_size();
        Self::parent_measure_override(this, size)
    }
}
impl TemplatedControlImpl for PinnedDockControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.detach_layout_events();
        this.detach_splitter_handlers();
        this.pinned_dock_grid.replace(Some(
            e.name_scope()
                .find_as::<Grid>("PART_PinnedDockGrid")
                .expect("Required template part PART_PinnedDockGrid"),
        ));
        this.pinned_dock.replace(Some(
            e.name_scope()
                .find_as::<ContentControl>("PART_PinnedDock")
                .expect("Required template part PART_PinnedDock"),
        ));
        this.pinned_dock_splitter
            .replace(e.name_scope().find_as::<Thumb>("PART_PinnedDockSplitter"));
        this.attach_splitter_handlers();
        this.update_grid();
        let owner = this.to_ref();
        let layout = this.layout_updated(move || owner.on_layout_updated());
        let owner = this.to_ref();
        let attached = this.attached_to_visual_tree(move |_| owner.on_attached());
        let owner = this.to_ref();
        let detached = this.detached_from_visual_tree(move |_| owner.on_detached());
        this.layout_events.replace(vec![layout, attached, detached]);
    }
}
ferro_properties! {impl PinnedDockControl {
    /// Identifies the source pinned alignment.
    pub fn pinned_dock_alignment_property()->StyledProperty<Alignment> {FerroProperty::register::<Self,Alignment>("PinnedDockAlignment",Alignment::Left)}
    /// Identifies the source display mode.
    pub fn pinned_dock_display_mode_property()->StyledProperty<PinnedDockDisplayMode> {FerroProperty::register::<Self,PinnedDockDisplayMode>("PinnedDockDisplayMode",PinnedDockDisplayMode::Overlay)}
    /// Identifies the finite, nonnegative minimum pinned width.
    pub fn minimum_pinned_width_property()->StyledProperty<f64> {FerroProperty::register_with::<Self,f64>("MinimumPinnedWidth",StyledPropertyOptions::new(50.0).validate(|v:&f64|v.is_finite()&&*v>=0.0))}
    /// Identifies the finite, nonnegative minimum pinned height.
    pub fn minimum_pinned_height_property()->StyledProperty<f64> {FerroProperty::register_with::<Self,f64>("MinimumPinnedHeight",StyledPropertyOptions::new(50.0).validate(|v:&f64|v.is_finite()&&*v>=0.0))}
}}
impl PinnedDockControl {
    const BOUNDS_EPSILON: f64 = 0.5;
    fn static_constructor() {
        Self::pinned_dock_alignment_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_grid());
        Self::pinned_dock_display_mode_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_display_mode_changed());
        Self::minimum_pinned_width_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_grid());
        Self::minimum_pinned_height_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_grid());
    }
    /// Constructs all original fields for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            pinned_dock_grid: RefCell::new(None),
            pinned_dock: RefCell::new(None),
            pinned_dock_splitter: RefCell::new(None),
            window: RefCell::new(None),
            owner_window: RefCell::new(None),
            managed_layer: RefCell::new(None),
            managed_pinned_host: RefCell::new(None),
            last_pinned_dockable: RefCell::new(None),
            last_pinned_width: Cell::new(f64::NAN),
            last_pinned_height: Cell::new(f64::NAN),
            is_resizing_pinned_dock: Cell::new(false),
            last_effective_display_mode: Cell::new(PinnedDockDisplayMode::Overlay),
            tracked_dockable: RefCell::new(None),
            tracked_native_dockable: RefCell::new(None),
            tracked_notify_dockable: RefCell::new(None),
            is_pinned_dock_empty: Cell::new(true),
            skip_pinned_bounds_update: Cell::new(false),
            layout_events: RefCell::new(Vec::new()),
            splitter_events: RefCell::new(Vec::new()),
            owner_events: RefCell::new(Vec::new()),
        }
    }
    /// Creates the original pinned host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets PinnedDockAlignment.
    pub fn pinned_dock_alignment(&self) -> Alignment {
        self.get_value(Self::pinned_dock_alignment_property())
    }
    /// Sets PinnedDockAlignment.
    pub fn set_pinned_dock_alignment(&self, value: Alignment) {
        self.set_value(Self::pinned_dock_alignment_property(), value);
    }
    /// Gets PinnedDockDisplayMode.
    pub fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode {
        self.get_value(Self::pinned_dock_display_mode_property())
    }
    /// Sets PinnedDockDisplayMode.
    pub fn set_pinned_dock_display_mode(&self, value: PinnedDockDisplayMode) {
        self.set_value(Self::pinned_dock_display_mode_property(), value);
    }
    /// Gets MinimumPinnedWidth.
    pub fn minimum_pinned_width(&self) -> f64 {
        self.get_value(Self::minimum_pinned_width_property())
    }
    /// Sets MinimumPinnedWidth, validated by the original native property contract.
    pub fn set_minimum_pinned_width(&self, value: f64) {
        self.set_value(Self::minimum_pinned_width_property(), value);
    }
    /// Gets MinimumPinnedHeight.
    pub fn minimum_pinned_height(&self) -> f64 {
        self.get_value(Self::minimum_pinned_height_property())
    }
    /// Sets MinimumPinnedHeight, validated by the original native property contract.
    pub fn set_minimum_pinned_height(&self, value: f64) {
        self.set_value(Self::minimum_pinned_height_property(), value);
    }
    fn root(&self) -> Option<Rc<dyn IRootDock>> {
        model_projection::root(self.data_context().as_ref())
    }
    fn grid(&self) -> Ref<Grid> {
        self.pinned_dock_grid
            .borrow()
            .clone()
            .expect("source current pinned grid")
    }
    fn dock(&self) -> Ref<ContentControl> {
        self.pinned_dock
            .borrow()
            .clone()
            .expect("source current pinned content")
    }
    fn splitter(&self) -> Option<Ref<Thumb>> {
        self.pinned_dock_splitter.borrow().clone()
    }
    fn window(&self) -> Ref<PinnedDockWindow> {
        self.window
            .borrow()
            .clone()
            .expect("source current pinned window")
    }
    fn managed_layer(&self) -> Ref<ManagedWindowLayer> {
        self.managed_layer
            .borrow()
            .clone()
            .expect("source current managed layer")
    }
    fn owner_window(&self) -> Ref<Window> {
        self.owner_window
            .borrow()
            .clone()
            .expect("source current owner window")
    }
    fn is_overlay_display_mode(&self) -> bool {
        self.get_effective_display_mode() == PinnedDockDisplayMode::Overlay
    }
    fn get_effective_display_mode(&self) -> PinnedDockDisplayMode {
        if let Some(root) = self.root() {
            if let Some(mode) = Self::get_pinned_dockable(&*root)
                .and_then(|d| d.pinned_dock_display_mode_override())
            {
                return mode;
            }
        }
        self.pinned_dock_display_mode()
    }
    fn on_display_mode_changed(&self) {
        self.last_effective_display_mode
            .set(self.get_effective_display_mode());
        self.update_grid();
        if self.is_overlay_display_mode() {
            if DockSettings::use_pinned_dock_window() {
                self.update_window();
            }
        } else {
            self.close_window();
        }
        self.invalidate_measure();
    }
    fn add_column(&self, length: GridLength, min: Option<f64>) {
        let definitions = self.grid().column_definitions();
        let column = ColumnDefinition::with_width(length);
        if let Some(min) = min {
            column.set_min_width(min);
        }
        definitions.add(column);
    }
    fn add_row(&self, length: GridLength, min: Option<f64>) {
        let definitions = self.grid().row_definitions();
        let row = RowDefinition::with_height(length);
        if let Some(min) = min {
            row.set_min_height(min);
        }
        definitions.add(row);
    }
    fn update_grid(&self) {
        if self.pinned_dock_grid.borrow().is_none() || self.pinned_dock.borrow().is_none() {
            return;
        }
        self.grid().row_definitions().clear();
        self.grid().column_definitions().clear();
        let overlay = self.is_overlay_display_mode();
        match self.pinned_dock_alignment() {
            Alignment::Unset | Alignment::Left => {
                self.add_column(GridLength::AUTO, Some(self.minimum_pinned_width()));
                self.add_column(GridLength::AUTO, None);
                if overlay {
                    self.add_column(GridLength::STAR, Some(50.0));
                }
                Grid::set_column(&self.dock(), 0);
                Grid::set_row(&self.dock(), 0);
                if self.splitter().is_some() {
                    Grid::set_row(&self.splitter().expect("source splitter"), 0);
                    Grid::set_column(&self.splitter().expect("source splitter"), 1);
                }
            }
            Alignment::Bottom => {
                if overlay {
                    self.add_row(GridLength::STAR, Some(50.0));
                    self.add_row(GridLength::AUTO, None);
                    self.add_row(GridLength::AUTO, Some(self.minimum_pinned_height()));
                } else {
                    self.add_row(GridLength::AUTO, None);
                    self.add_row(GridLength::AUTO, Some(self.minimum_pinned_height()));
                }
                Grid::set_column(&self.dock(), 0);
                Grid::set_row(&self.dock(), if overlay { 2 } else { 1 });
                if self.splitter().is_some() {
                    Grid::set_row(
                        &self.splitter().expect("source splitter"),
                        if overlay { 1 } else { 0 },
                    );
                    Grid::set_column(&self.splitter().expect("source splitter"), 0);
                }
            }
            Alignment::Right => {
                if overlay {
                    self.add_column(GridLength::STAR, Some(50.0));
                    self.add_column(GridLength::AUTO, None);
                    self.add_column(GridLength::AUTO, Some(self.minimum_pinned_width()));
                } else {
                    self.add_column(GridLength::AUTO, None);
                    self.add_column(GridLength::AUTO, Some(self.minimum_pinned_width()));
                }
                Grid::set_column(&self.dock(), if overlay { 2 } else { 1 });
                Grid::set_row(&self.dock(), 0);
                if self.splitter().is_some() {
                    Grid::set_row(&self.splitter().expect("source splitter"), 0);
                    Grid::set_column(
                        &self.splitter().expect("source splitter"),
                        if overlay { 1 } else { 0 },
                    );
                }
            }
            Alignment::Top => {
                self.add_row(GridLength::AUTO, Some(self.minimum_pinned_height()));
                self.add_row(GridLength::AUTO, None);
                if overlay {
                    self.add_row(GridLength::STAR, Some(50.0));
                }
                Grid::set_column(&self.dock(), 0);
                Grid::set_row(&self.dock(), 0);
                if self.splitter().is_some() {
                    Grid::set_row(&self.splitter().expect("source splitter"), 1);
                    Grid::set_column(&self.splitter().expect("source splitter"), 0);
                }
            }
        }
        self.apply_pinned_dock_size();
        self.invalidate_measure();
    }
    fn detach_layout_events(&self) {
        let events = self.layout_events.borrow().clone();
        for event in events {
            event.dispose();
        }
        self.layout_events.replace(Vec::new());
    }
    fn on_attached(&self) {
        if DockSettings::use_pinned_dock_window() && self.is_overlay_display_mode() {
            self.update_window();
        }
    }
    fn on_detached(&self) {
        self.close_window();
        self.clear_pinned_dockable_subscription();
        self.detach_layout_events();
        self.detach_splitter_handlers();
    }
    fn on_layout_updated(&self) {
        self.update_pinned_dockable_subscription();
        self.update_pinned_dock_empty_state();
        let mode = self.get_effective_display_mode();
        if mode != self.last_effective_display_mode.get() {
            self.last_effective_display_mode.set(mode);
            self.on_display_mode_changed();
        }
        self.update_pinned_dockable_bounds();
        self.update_window();
    }
    fn update_pinned_dock_empty_state(&self) {
        let mut empty = true;
        if let Some(root) = self.root() {
            let pinned = root.pinned_dock();
            let list = pinned.and_then(|d| d.visible_dockables());
            empty = list.is_none_or(|list| list.count() == 0);
        }
        if empty == self.is_pinned_dock_empty.get() {
            return;
        }
        self.is_pinned_dock_empty.set(empty);
        if empty {
            self.last_pinned_dockable.replace(None);
            self.last_pinned_width.set(f64::NAN);
            self.last_pinned_height.set(f64::NAN);
            self.skip_pinned_bounds_update.set(false);
        }
        self.invalidate_measure();
    }
    fn update_pinned_dockable_subscription(&self) {
        let Some(root) = self.root() else {
            self.clear_pinned_dockable_subscription();
            return;
        };
        let dockable = Self::get_pinned_dockable(&*root);
        let tracked = self.tracked_dockable.borrow().clone();
        if same_dockable(dockable.as_ref(), tracked.as_ref()) {
            return;
        }
        self.skip_pinned_bounds_update.set(true);
        self.reset_pinned_dock_size();
        self.invalidate_measure();
        self.clear_pinned_dockable_subscription();
        let Some(dockable) = dockable else {
            return;
        };
        self.tracked_dockable.replace(Some(dockable.clone()));
        let native = dockable
            .clone()
            .boxed_interface(TypeId::of::<Ref<FerroObject>>())
            .and_then(|value| value.downcast_ref::<Ref<FerroObject>>().cloned());
        if let Some(native) = native {
            let owner = self.to_ref();
            let subscription =
                native.property_changed(move |e| owner.on_dockable_ferro_property_changed(e));
            self.tracked_native_dockable
                .replace(Some((native, subscription)));
        } else if let Some(notifier) = dockable.as_notify_property_changed() {
            let owner = self.to_ref();
            let token = notifier.property_changed().add(Rc::new(move |name| {
                owner.on_dockable_property_changed(name)
            }));
            self.tracked_notify_dockable
                .replace(Some(Subscription(Box::new(move || {
                    if let Some(notifier) = dockable.as_notify_property_changed() {
                        notifier.property_changed().remove(token);
                    }
                }))));
        }
        self.update_grid();
    }
    fn clear_pinned_dockable_subscription(&self) {
        let native = self.tracked_native_dockable.borrow().clone();
        if let Some((_, subscription)) = native {
            subscription.dispose();
            self.tracked_native_dockable.replace(None);
        }
        let notify = self.tracked_notify_dockable.borrow_mut().take();
        if let Some(notify) = notify {
            notify.dispose();
        }
        self.tracked_dockable.replace(None);
    }
    fn on_dockable_ferro_property_changed(&self, e: &FerroPropertyChangedEventArgs) {
        if e.property().name() != "PinnedDockDisplayModeOverride" {
            return;
        }
        if self.get_effective_display_mode() == self.last_effective_display_mode.get() {
            return;
        }
        self.on_display_mode_changed();
    }
    fn on_dockable_property_changed(&self, name: &str) {
        if name != "PinnedDockDisplayModeOverride" {
            return;
        }
        if self.get_effective_display_mode() == self.last_effective_display_mode.get() {
            return;
        }
        self.on_display_mode_changed();
    }
    fn update_window(&self) {
        if !DockSettings::use_pinned_dock_window()
            || !self.is_overlay_display_mode()
            || self.pinned_dock_grid.borrow().is_none()
            || self.pinned_dock.borrow().is_none()
        {
            return;
        }
        let Some(root) = self.root().filter(|r| r.pinned_dock().is_some()) else {
            self.close_window();
            return;
        };
        if DockSettings::is_managed_window_hosting_enabled(Some(&*root)) {
            self.update_managed_window(&*root);
            return;
        }
        if !root.pinned_dock().expect("source pinned dock").is_empty() {
            if self.window.borrow().is_none() {
                let window = PinnedDockWindow::new();
                let content = ToolDockControl::new();
                content.set_data_context(
                    root.pinned_dock()
                        .map(|d| model_projection::to_dockable_value(d)),
                );
                window.set_content(Some(Rc::new(content)));
                self.window.replace(Some(window));
                if let Some(owner) = self.visual_root().and_then(|root| root.cast::<Window>()) {
                    self.owner_window.replace(Some(owner));
                    let this = self.to_ref();
                    let position = self
                        .owner_window()
                        .position_changed(move |_| this.owner_window_position_changed());
                    let this = self.to_ref();
                    let resized = self
                        .owner_window()
                        .resized(move |_| this.owner_window_resized());
                    let this = self.to_ref();
                    let pressed = self.owner_window().add_disposable_handler_with(
                        InputElement::pointer_pressed_event(),
                        move |_, e| this.owner_window_pointer_pressed(e),
                        RoutingStrategies::TUNNEL,
                        false,
                    );
                    let this = self.to_ref();
                    let deactivated = self
                        .owner_window()
                        .deactivated(move || this.owner_window_deactivated());
                    self.owner_events
                        .replace(vec![position, resized, pressed, deactivated]);
                    self.window().show_with_owner(&self.owner_window());
                }
            }
            let point = self.dock().point_to_screen(Point::default());
            self.window()
                .set_position(PixelPoint::new(point.x, point.y));
            self.window().set_width(self.dock().bounds().width);
            self.window().set_height(self.dock().bounds().height);
            if self.dock().opacity() != 0.0 {
                self.dock().set_opacity(0.0);
                self.dock().set_is_hit_test_visible(false);
            }
        } else {
            self.close_window();
        }
    }
    fn close_window(&self) {
        if self.managed_layer.borrow().is_some() {
            self.managed_layer().hide_overlay("PinnedDock");
            self.managed_layer.replace(None);
        }
        self.managed_pinned_host.replace(None);
        if self.window.borrow().is_some() {
            self.window().close();
            self.window.replace(None);
        }
        if self.owner_window.borrow().is_some() {
            let events = self.owner_events.borrow().clone();
            for event in events {
                event.dispose();
            }
            self.owner_events.replace(Vec::new());
            self.owner_window.replace(None);
        }
        let dock = self.pinned_dock.borrow().clone();
        if let Some(dock) = dock.filter(|d| d.opacity() == 0.0) {
            dock.clear_value(Visual::opacity_property());
            dock.clear_value(InputElement::is_hit_test_visible_property());
        }
    }
    fn update_managed_window(&self, root: &dyn IRootDock) {
        if root.pinned_dock().is_none()
            || root.pinned_dock().expect("source pinned dock").is_empty()
        {
            self.close_window();
            return;
        }
        self.managed_layer
            .replace(ManagedWindowLayer::try_get_layer(Some(self)));
        if self.managed_layer.borrow().is_none() {
            return;
        }
        if self.managed_pinned_host.borrow().is_none() {
            let host = ToolDockControl::new();
            host.set_data_context(
                root.pinned_dock()
                    .map(|d| model_projection::to_dockable_value(d)),
            );
            self.managed_pinned_host.replace(Some(host.upcast()));
        }
        let pinned = self.pinned_dock.borrow().clone();
        let Some(pinned) = pinned else {
            return;
        };
        let point = pinned.point_to_screen(Point::default());
        let bounds = Self::get_managed_bounds(&self.managed_layer(), point, pinned.bounds().size());
        let host = self
            .managed_pinned_host
            .borrow()
            .clone()
            .expect("source managed pinned host");
        self.managed_layer()
            .show_overlay("PinnedDock", host, bounds, true);
        if pinned.opacity() != 0.0 {
            pinned.set_opacity(0.0);
            pinned.set_is_hit_test_visible(false);
        }
    }
    fn get_managed_bounds(layer: &ManagedWindowLayer, screen: PixelPoint, size: Size) -> Rect {
        let Some(top) = layer.visual_root().and_then(|r| r.cast::<TopLevel>()) else {
            return Rect::new(0.0, 0.0, size.width, size.height);
        };
        let client = top.point_to_client(screen);
        let origin = layer
            .translate_point(Point::default(), &top)
            .unwrap_or_default();
        let local = Point::new(client.x - origin.x, client.y - origin.y);
        Rect::new(local.x, local.y, size.width, size.height)
    }
    fn apply_pinned_dock_size(&self) {
        if self.is_resizing_pinned_dock.get() {
            return;
        }
        if self.pinned_dock_grid.borrow().is_none() {
            return;
        }
        let Some(root) = self.root() else {
            return;
        };
        let Some(dockable) = Self::get_pinned_dockable(&*root) else {
            return;
        };
        let last = self.last_pinned_dockable.borrow().clone();
        let changed = !same_dockable(last.as_ref(), Some(&dockable));
        if changed {
            self.reset_pinned_dock_size();
            self.last_pinned_dockable.replace(Some(dockable.clone()));
            self.last_pinned_width.set(f64::NAN);
            self.last_pinned_height.set(f64::NAN);
            self.skip_pinned_bounds_update.set(true);
        }
        let (_, _, width, height) = dockable.get_pinned_bounds();
        let overlay = self.is_overlay_display_mode();
        match self.pinned_dock_alignment() {
            Alignment::Unset | Alignment::Left => {
                if !Self::is_valid_size(width)
                    || Self::is_column_size_applied(&self.grid().column_definitions(), 0, width)
                {
                    return;
                }
                Self::set_column_size(&self.grid().column_definitions(), 0, width);
                self.last_pinned_dockable.replace(Some(dockable));
                self.last_pinned_width.set(width);
                self.invalidate_measure();
            }
            Alignment::Right => {
                if !Self::is_valid_size(width) {
                    return;
                }
                let index = if overlay { 2 } else { 1 };
                if Self::is_column_size_applied(&self.grid().column_definitions(), index, width) {
                    return;
                }
                Self::set_column_size(&self.grid().column_definitions(), index, width);
                self.last_pinned_dockable.replace(Some(dockable));
                self.last_pinned_width.set(width);
                self.invalidate_measure();
            }
            Alignment::Top => {
                if !Self::is_valid_size(height)
                    || Self::is_row_size_applied(&self.grid().row_definitions(), 0, height)
                {
                    return;
                }
                Self::set_row_size(&self.grid().row_definitions(), 0, height);
                self.last_pinned_dockable.replace(Some(dockable));
                self.last_pinned_height.set(height);
                self.invalidate_measure();
            }
            Alignment::Bottom => {
                if !Self::is_valid_size(height) {
                    return;
                }
                let index = if overlay { 2 } else { 1 };
                if Self::is_row_size_applied(&self.grid().row_definitions(), index, height) {
                    return;
                }
                Self::set_row_size(&self.grid().row_definitions(), index, height);
                self.last_pinned_dockable.replace(Some(dockable));
                self.last_pinned_height.set(height);
                self.invalidate_measure();
            }
        }
    }
    fn reset_pinned_dock_size(&self) {
        if self.pinned_dock_grid.borrow().is_none() {
            return;
        }
        let overlay = self.is_overlay_display_mode();
        match self.pinned_dock_alignment() {
            Alignment::Unset | Alignment::Left => {
                Self::reset_column_size(&self.grid().column_definitions(), 0)
            }
            Alignment::Right => Self::reset_column_size(
                &self.grid().column_definitions(),
                if overlay { 2 } else { 1 },
            ),
            Alignment::Top => Self::reset_row_size(&self.grid().row_definitions(), 0),
            Alignment::Bottom => {
                Self::reset_row_size(&self.grid().row_definitions(), if overlay { 2 } else { 1 })
            }
        }
        self.invalidate_measure();
    }
    fn update_pinned_dockable_bounds(&self) {
        if self.pinned_dock.borrow().is_none() {
            return;
        }
        let Some(root) = self.root() else {
            return;
        };
        let Some(dockable) = Self::get_pinned_dockable(&*root) else {
            return;
        };
        let (_, _, stored_width, stored_height) = dockable.get_pinned_bounds();
        let stored = Self::is_valid_size(stored_width) && Self::is_valid_size(stored_height);
        if self.skip_pinned_bounds_update.get() {
            self.skip_pinned_bounds_update.set(false);
            if !self.is_resizing_pinned_dock.get() && !stored {
                return;
            }
        }
        if !self.is_resizing_pinned_dock.get() && stored {
            return;
        }
        let width = self.dock().bounds().width;
        let height = self.dock().bounds().height;
        if !Self::is_valid_size(width) || !Self::is_valid_size(height) {
            return;
        }
        if Self::are_close(width, stored_width) && Self::are_close(height, stored_height) {
            return;
        }
        dockable.set_pinned_bounds(0.0, 0.0, width, height);
        self.last_pinned_dockable.replace(Some(dockable));
        self.last_pinned_width.set(width);
        self.last_pinned_height.set(height);
    }
    fn get_pinned_dockable(root: &dyn IRootDock) -> Option<Rc<dyn IDockable>> {
        let pinned = root.pinned_dock()?;
        if let Some(active) = pinned.active_dockable() {
            if active.clone().as_splitter().is_none()
                && pinned
                    .visible_dockables()
                    .is_some_and(|l| l.contains(&Some((active).clone())))
            {
                return Some(active);
            }
        }
        if let Some(list) = pinned.visible_dockables() {
            for dockable in list.iter() {
                if dockable
                    .clone()
                    .and_then(|item| item.as_splitter())
                    .is_none()
                {
                    return dockable;
                }
            }
        }
        None
    }
    fn set_column_size(definitions: &ColumnDefinitions, index: i32, size: f64) {
        if index < 0 || index as usize >= definitions.count() {
            return;
        }
        definitions
            .get(index as usize)
            .set_width(GridLength::new(size, GridUnitType::Pixel));
    }
    fn reset_column_size(definitions: &ColumnDefinitions, index: i32) {
        if index < 0 || index as usize >= definitions.count() {
            return;
        }
        definitions.get(index as usize).set_width(GridLength::AUTO);
    }
    fn set_row_size(definitions: &RowDefinitions, index: i32, size: f64) {
        if index < 0 || index as usize >= definitions.count() {
            return;
        }
        definitions
            .get(index as usize)
            .set_height(GridLength::new(size, GridUnitType::Pixel));
    }
    fn reset_row_size(definitions: &RowDefinitions, index: i32) {
        if index < 0 || index as usize >= definitions.count() {
            return;
        }
        definitions.get(index as usize).set_height(GridLength::AUTO);
    }
    fn is_column_size_applied(definitions: &ColumnDefinitions, index: i32, size: f64) -> bool {
        if index < 0 || index as usize >= definitions.count() {
            return false;
        }
        let length = definitions.get(index as usize).width();
        length.is_absolute() && Self::are_close(length.value(), size)
    }
    fn is_row_size_applied(definitions: &RowDefinitions, index: i32, size: f64) -> bool {
        if index < 0 || index as usize >= definitions.count() {
            return false;
        }
        let length = definitions.get(index as usize).height();
        length.is_absolute() && Self::are_close(length.value(), size)
    }
    fn is_valid_size(size: f64) -> bool {
        !size.is_nan() && !size.is_infinite() && size > 0.0
    }
    fn are_close(left: f64, right: f64) -> bool {
        (left - right).abs() <= Self::BOUNDS_EPSILON
    }
    pub(crate) fn try_resize_inline_pinned_dock(&self, delta: Vector) -> bool {
        if self.is_overlay_display_mode() {
            return false;
        }
        if self.pinned_dock_grid.borrow().is_none() || self.pinned_dock.borrow().is_none() {
            return false;
        }
        let Some(root) = self.root() else {
            return false;
        };
        let Some(dockable) = Self::get_pinned_dockable(&*root) else {
            return false;
        };
        let (_, _, mut width, mut height) = dockable.get_pinned_bounds();
        if !Self::is_valid_size(width) || !Self::is_valid_size(height) {
            width = self.dock().bounds().width;
            height = self.dock().bounds().height;
        }
        if !Self::is_valid_size(width) || !Self::is_valid_size(height) {
            return false;
        }
        let (mut new_width, mut new_height) = (width, height);
        match self.pinned_dock_alignment() {
            Alignment::Unset | Alignment::Left => {
                new_width = max(self.minimum_pinned_width(), width + delta.x);
                Self::set_column_size(&self.grid().column_definitions(), 0, new_width);
            }
            Alignment::Right => {
                new_width = max(self.minimum_pinned_width(), width - delta.x);
                Self::set_column_size(&self.grid().column_definitions(), 1, new_width);
            }
            Alignment::Top => {
                new_height = max(self.minimum_pinned_height(), height + delta.y);
                Self::set_row_size(&self.grid().row_definitions(), 0, new_height);
            }
            Alignment::Bottom => {
                new_height = max(self.minimum_pinned_height(), height - delta.y);
                Self::set_row_size(&self.grid().row_definitions(), 1, new_height);
            }
        }
        dockable.set_pinned_bounds(0.0, 0.0, new_width, new_height);
        self.last_pinned_dockable.replace(Some(dockable));
        self.last_pinned_width.set(new_width);
        self.last_pinned_height.set(new_height);
        self.invalidate_measure();
        true
    }
    fn attach_splitter_handlers(&self) {
        if !self
            .splitter()
            .is_some_and(|s| s.cast::<GridSplitter>().is_some())
        {
            return;
        }
        let this = self.to_ref();
        let started = self
            .splitter()
            .expect("source splitter")
            .add_disposable_handler_with(
                Thumb::drag_started_event(),
                move |_, _| this.on_pinned_dock_splitter_drag_started(),
                RoutingStrategies::BUBBLE,
                false,
            );
        let this = self.to_ref();
        let delta = self
            .splitter()
            .expect("source splitter")
            .add_disposable_handler_with(
                Thumb::drag_delta_event(),
                move |_, e| this.on_pinned_dock_splitter_drag_delta(e.vector),
                RoutingStrategies::BUBBLE,
                false,
            );
        let this = self.to_ref();
        let completed = self
            .splitter()
            .expect("source splitter")
            .add_disposable_handler_with(
                Thumb::drag_completed_event(),
                move |_, _| this.on_pinned_dock_splitter_drag_completed(),
                RoutingStrategies::BUBBLE,
                false,
            );
        self.splitter_events
            .replace(vec![started, delta, completed]);
    }
    fn detach_splitter_handlers(&self) {
        if !self
            .splitter()
            .is_some_and(|s| s.cast::<GridSplitter>().is_some())
        {
            return;
        }
        let events = self.splitter_events.borrow().clone();
        for event in events {
            event.dispose();
        }
        self.splitter_events.replace(Vec::new());
    }
    fn on_pinned_dock_splitter_drag_started(&self) {
        self.is_resizing_pinned_dock.set(true);
        if self.is_overlay_display_mode() {
            self.update_pinned_dockable_bounds();
        }
    }
    fn on_pinned_dock_splitter_drag_delta(&self, delta: Vector) {
        if self.is_overlay_display_mode() {
            self.update_pinned_dockable_bounds();
            return;
        }
        self.try_resize_inline_pinned_dock(delta);
    }
    fn on_pinned_dock_splitter_drag_completed(&self) {
        // Source reads the mode although both branches update the same bounds.
        let _ = self.is_overlay_display_mode();
        self.update_pinned_dockable_bounds();
        self.is_resizing_pinned_dock.set(false);
    }
    fn owner_window_position_changed(&self) {
        self.close_window();
        self.update_window();
    }
    fn owner_window_resized(&self) {
        self.update_window();
    }
    fn is_pointer_inside_pinned_dock(&self, e: &PointerEventArgs) -> bool {
        if self.pinned_dock_grid.borrow().is_none() {
            return false;
        }
        let point = e.get_position(Some(&self.grid()));
        point.x >= 0.0
            && point.y >= 0.0
            && point.x <= self.grid().bounds().width
            && point.y <= self.grid().bounds().height
    }
    fn should_keep_pinned_dockable_visible(&self) -> bool {
        let Some(root) = self.root() else {
            return false;
        };
        if root
            .pinned_dock()
            .and_then(|d| d.visible_dockables())
            .is_none()
        {
            return false;
        }
        for dockable in root
            .pinned_dock()
            .expect("source pinned dock")
            .visible_dockables()
            .expect("source visible list")
            .iter()
        {
            if dockable
                .expect("source dockable.KeepPinnedDockableVisible dereference")
                .keep_pinned_dockable_visible()
            {
                return true;
            }
        }
        false
    }
    fn owner_window_pointer_pressed(&self, e: &PointerEventArgs) {
        if self.should_keep_pinned_dockable_visible() {
            return;
        }
        if !self.is_pointer_inside_pinned_dock(e) {
            self.close_window();
        }
    }
    fn owner_window_deactivated(&self) {
        if self.should_keep_pinned_dockable_visible() {
            return;
        }
        self.close_window();
    }
}
fn same_dockable(a: Option<&Rc<dyn IDockable>>, b: Option<&Rc<dyn IDockable>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a > b || a == b && a.is_sign_positive() {
        a
    } else {
        b
    }
}

#[cfg(test)]
#[path = "pinned_dock_control_tests.rs"]
mod tests;
