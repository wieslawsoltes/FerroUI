// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DockControl, DocumentTabStripItem, HostWindow};
use crate::{
    automation::peers::DocumentTabStripAutomationPeer,
    contract::IExternalDockSurface,
    internal::{
        scroll_viewer_mouse_wheel_hook_helper::ScrollViewerMouseWheelHookHelper,
        window_drag_helper::WindowDragHelper,
    },
    model_projection,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, TappedEventArgs},
    instantiate,
    interactivity::InteractiveImpl,
    layout::{Layoutable, LayoutableImpl, Orientation},
    reactive::{AnonymousObserver, IDisposable},
    styling::ControlTheme,
    BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, StyledElementImpl, StyledProperty,
    Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    generators::RecycleKey,
    primitives::{
        SelectingItemsControlImpl, TabStrip, TemplateAppliedEventArgs, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
    templates::IDataTemplate,
    Button, Control, ControlImpl, DockPanel, ItemsControlImpl, ScrollViewer, TopLevel, Window,
    WindowState,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
/// Original document tab strip and external docking surface.
#[repr(C)]
pub struct DocumentTabStrip {
    base: TabStrip,
    attached_window: RefCell<Option<Ref<HostWindow>>>,
    grip: RefCell<Option<Ref<Control>>>,
    panel: RefCell<Option<Ref<Control>>>,
    leading_spacer: RefCell<Option<Ref<Control>>>,
    create_button_host: RefCell<Option<Ref<Control>>>,
    scroll_viewer: RefCell<Option<Ref<ScrollViewer>>>,
    template_observables: RefCell<Vec<Rc<dyn IDisposable>>>,
    template_observables_version: Cell<u32>,
    scroll_viewer_wheel_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    double_tapped_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    window_drag_helper: RefCell<Option<Rc<WindowDragHelper>>>,
    external_owner: RefCell<Option<Ref<DockControl>>>,
    external_handle: RefCell<Weak<ExternalSurface>>,
}
ferro_class!(DocumentTabStrip:TabStrip);
ferro_class_info!(DocumentTabStrip {new:DocumentTabStrip::new,interfaces:[Rc<dyn IExternalDockSurface> => DocumentTabStrip::as_external_surface],markup:{attributes:[PseudoClasses(":create",":active")]}});
ferro_impl_classes!(DocumentTabStrip:LayoutableImpl,InteractiveImpl,InputElementImpl,SelectingItemsControlImpl);
impl StyledElementImpl for DocumentTabStrip {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl FerroObjectImpl for DocumentTabStrip {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes_create(this.can_create_item());
        this.update_pseudo_classes_active(this.is_active());
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::can_create_item_property().as_property() {
            this.update_pseudo_classes_create(change.get_new_value::<bool>());
        }
        if change.property() == Self::is_active_property().as_property() {
            this.update_pseudo_classes_active(change.get_new_value::<bool>());
        }
        if change.property() == Self::enable_window_drag_property().as_property() {
            if change.get_new_value::<bool>() {
                this.attach_to_window();
            } else {
                this.detach_from_window();
            }
        }
        if change.property()
            == Self::enable_window_state_toggle_on_double_tap_property().as_property()
        {
            this.attach_double_tapped();
        }
        if change.property() == Self::mouse_wheel_scroll_orientation_property().as_property() {
            this.attach_scroll_viewer_wheel();
        }
    }
}
impl ControlImpl for DocumentTabStrip {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DocumentTabStripAutomationPeer::new(this).upcast()
    }
}
impl ItemsControlImpl for DocumentTabStrip {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _key: Option<RecycleKey>,
    ) -> Ref<Control> {
        DocumentTabStripItem::new().upcast()
    }
    fn needs_container_override(
        this: &Self,
        item: &Option<BoxedValue>,
        _index: i32,
    ) -> (bool, Option<RecycleKey>) {
        this.needs_container::<DocumentTabStripItem>(item)
    }
}
impl TemplatedControlImpl for DocumentTabStrip {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.detach_scroll_viewer_wheel();
        this.clear_template_observables();
        this.grip
            .replace(e.name_scope().find_as::<Control>("PART_BorderFill"));
        this.panel
            .replace(e.name_scope().find_as::<Control>("PART_Panel"));
        this.leading_spacer
            .replace(e.name_scope().find_as::<Control>("PART_LeadingSpacer"));
        this.create_button_host
            .replace(e.name_scope().find_as::<Control>("PART_CreateButtonHost"));
        this.scroll_viewer
            .replace(e.name_scope().find_as::<ScrollViewer>("PART_ScrollViewer"));
        this.attach_double_tapped();
        this.attach_scroll_viewer_wheel();
        this.attach_template_observables();
        this.attach_to_window();
    }
}
impl VisualImpl for DocumentTabStrip {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attach_scroll_viewer_wheel();
        this.attach_to_window();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.detach_double_tapped();
        this.detach_scroll_viewer_wheel();
        this.clear_template_observables();
        this.detach_from_window();
    }
}
impl DocumentTabStrip {
    /// Constructs the native source class state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: TabStrip::construct(),
            attached_window: RefCell::new(None),
            grip: RefCell::new(None),
            panel: RefCell::new(None),
            leading_spacer: RefCell::new(None),
            create_button_host: RefCell::new(None),
            scroll_viewer: RefCell::new(None),
            template_observables: RefCell::new(Vec::new()),
            template_observables_version: Cell::new(0),
            scroll_viewer_wheel_subscription: RefCell::new(None),
            double_tapped_subscription: RefCell::new(None),
            window_drag_helper: RefCell::new(None),
            external_owner: RefCell::new(None),
            external_handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates the original document tab strip.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn static_constructor() {
        ferroui_base::data::core::ValueTypes::register_interface_object::<
            Rc<dyn IExternalDockSurface>,
        >(|s| s.as_object());
    }
    /// Returns the original external-surface view with the same native receiver.
    pub fn as_external_surface(owner: Ref<Self>) -> Rc<dyn IExternalDockSurface> {
        let cached = owner.external_handle.borrow().upgrade();
        if let Some(cached) = cached {
            return cached;
        }
        let handle = Rc::new(ExternalSurface(owner.clone()));
        owner.external_handle.replace(Rc::downgrade(&handle));
        handle
    }
    fn panel(&self) -> Option<Ref<Control>> {
        self.panel.borrow().clone()
    }
    fn create_button_host(&self) -> Option<Ref<Control>> {
        self.create_button_host.borrow().clone()
    }
    fn leading_spacer(&self) -> Option<Ref<Control>> {
        self.leading_spacer.borrow().clone()
    }
    fn add_template_observable(&self, subscription: Rc<dyn IDisposable>) {
        self.template_observables.borrow_mut().push(subscription);
        self.template_observables_version
            .set(self.template_observables_version.get().wrapping_add(1));
    }
    fn observe_bounds(&self, target: &Control) {
        let owner = self.to_ref();
        let subscription = FerroObject::get_observable(target, Visual::bounds_property())
            .subscribe(Rc::new(AnonymousObserver::new(move |_: Rect| {
                owner.update_tab_host_max_width()
            })));
        self.add_template_observable(subscription);
    }
    fn attach_template_observables(&self) {
        let Some(dock_panel) = self
            .panel()
            .and_then(|p| p.parent())
            .and_then(|p| p.cast::<DockPanel>())
        else {
            return;
        };
        self.observe_bounds(&dock_panel);
        self.observe_bounds(self);
        if self.leading_spacer().is_some() {
            self.observe_bounds(&self.leading_spacer().expect("source spacer re-read"));
        }
        if self.create_button_host().is_some() {
            self.observe_bounds(
                &self
                    .create_button_host()
                    .expect("source create host re-read"),
            );
            let owner = self.to_ref();
            let host = self
                .create_button_host()
                .expect("source create host re-read");
            let subscription = FerroObject::get_observable(&host, Visual::is_visible_property())
                .subscribe(Rc::new(AnonymousObserver::new(move |_: bool| {
                    owner.update_tab_host_max_width()
                })));
            self.add_template_observable(subscription);
        }
        self.update_tab_host_max_width();
    }
    fn clear_template_observables(&self) {
        let version = self.template_observables_version.get();
        let mut index = 0;
        loop {
            assert_eq!(
                version,
                self.template_observables_version.get(),
                "Collection was modified; enumeration operation may not execute."
            );
            let disposable = self.template_observables.borrow().get(index).cloned();
            index += 1;
            let Some(disposable) = disposable else {
                break;
            };
            disposable.dispose();
        }
        let old = std::mem::take(&mut *self.template_observables.borrow_mut());
        self.template_observables_version
            .set(self.template_observables_version.get().wrapping_add(1));
        drop(old);
    }
    fn update_tab_host_max_width(&self) {
        let Some(dock_panel) = self
            .panel()
            .and_then(|p| p.parent())
            .and_then(|p| p.cast::<DockPanel>())
        else {
            return;
        };
        let host_width = dock_panel.bounds().width;
        if host_width.is_nan() || host_width.is_infinite() || host_width <= 0.0 {
            self.panel()
                .expect("source panel re-read")
                .set_current_value(Layoutable::max_width_property(), f64::INFINITY);
            return;
        }
        let mut reserved = 0.0;
        if let Some(spacer) = self.leading_spacer().filter(|c| c.is_visible()) {
            let width = max(spacer.bounds().width, spacer.desired_size().width);
            reserved += width + spacer.margin().left + spacer.margin().right;
        }
        if let Some(host) = self.create_button_host().filter(|c| c.is_visible()) {
            let width = max(host.bounds().width, host.desired_size().width);
            reserved += width + host.margin().left + host.margin().right;
        }
        self.panel()
            .expect("source panel re-read")
            .set_current_value(
                Layoutable::max_width_property(),
                max(0.0, host_width - reserved),
            );
    }
    fn on_double_tapped(&self, e: &TappedEventArgs) {
        if !self.enable_window_state_toggle_on_double_tap()
            || self.should_ignore_window_state_toggle_source(e.source().as_deref())
        {
            return;
        }
        let Some(window) = TopLevel::get_top_level(Some(self)).and_then(|t| t.cast::<Window>())
        else {
            return;
        };
        let state = match window.window_state() {
            WindowState::Normal => WindowState::Maximized,
            WindowState::Maximized => WindowState::Normal,
            _ => window.window_state(),
        };
        window.set_window_state(state);
        e.set_handled(true);
    }
    pub(crate) fn should_ignore_window_state_toggle_source(
        &self,
        source: Option<&FerroObject>,
    ) -> bool {
        source
            .and_then(|s| s.to_ref().cast::<Control>())
            .is_some_and(|c| {
                c.is::<Button>() || WindowDragHelper::is_child_of_type::<Button>(self, &c)
            })
    }
    fn attach_double_tapped(&self) {
        self.detach_double_tapped();
        if !self.enable_window_state_toggle_on_double_tap() {
            return;
        }
        let owner = self.to_ref();
        let subscription = self
            .add_disposable_handler(InputElement::double_tapped_event(), move |_, e| {
                owner.on_double_tapped(e)
            });
        self.double_tapped_subscription.replace(Some(subscription));
    }
    fn detach_double_tapped(&self) {
        let subscription = self.double_tapped_subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.double_tapped_subscription.replace(None);
    }
    fn update_pseudo_classes_create(&self, value: bool) {
        self.pseudo_classes().set(":create", value);
    }
    fn update_pseudo_classes_active(&self, value: bool) {
        self.pseudo_classes().set(":active", value);
    }
    fn create_drag_helper(&self) -> Rc<WindowDragHelper> {
        let enabled = self.to_ref();
        let owner = self.to_ref();
        WindowDragHelper::new(
            self.to_ref().upcast(),
            Rc::new(move || enabled.enable_window_drag()),
            Rc::new(move |source| {
                if source.is_some_and(|s| s.to_ref().ptr_eq(&owner.clone().upcast::<Control>())) {
                    return true;
                }
                let button =
                    source.is_some_and(|s| WindowDragHelper::is_child_of_type::<Button>(&owner, s));
                let tab = source.is_some_and(|s| {
                    s.is::<DocumentTabStripItem>()
                        || WindowDragHelper::is_child_of_type::<DocumentTabStripItem>(&owner, s)
                });
                let base = source.is_some() && !tab && !button;
                let override_allow = !base
                    && !button
                    && owner.items().count() == 1
                    && model_projection::dockable(owner.data_context().as_ref())
                        .and_then(|d| d.as_dock())
                        .is_some_and(|d| !d.can_close_last_dockable());
                base || override_allow
            }),
            true,
            Some(Rc::new(|_| {
                ferrodock_settings::DockSettings::document_window_drag_scope()
            })),
        )
    }
    fn attach_to_window(&self) {
        if !self.enable_window_drag() {
            self.detach_from_window();
            return;
        }
        let helper = self.window_drag_helper.borrow().clone();
        if let Some(helper) = helper {
            helper.detach();
            self.window_drag_helper.replace(None);
        }
        self.window_drag_helper
            .replace(Some(self.create_drag_helper()));
        let helper = self
            .window_drag_helper
            .borrow()
            .clone()
            .expect("source helper");
        helper.attach();
    }
    fn detach_from_window(&self) {
        let window = self.attached_window.borrow().clone();
        if let Some(window) = window {
            if self.grip.borrow().is_some() {
                window.detach_grip(
                    &self.grip.borrow().clone().expect("source grip re-read"),
                    ":documentwindow",
                );
                self.attached_window.replace(None);
            }
        }
        let helper = self.window_drag_helper.borrow().clone();
        if let Some(helper) = helper {
            helper.detach();
            self.window_drag_helper.replace(None);
        }
    }
    fn detach_scroll_viewer_wheel(&self) {
        let subscription = self.scroll_viewer_wheel_subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.scroll_viewer_wheel_subscription.replace(None);
    }
    fn attach_scroll_viewer_wheel(&self) {
        let subscription = self.scroll_viewer_wheel_subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let viewer = self.scroll_viewer.borrow().clone();
        let subscription = ScrollViewerMouseWheelHookHelper::attach(
            viewer.as_deref(),
            self.mouse_wheel_scroll_orientation(),
        );
        self.scroll_viewer_wheel_subscription.replace(subscription);
    }
}
struct ExternalSurface(Ref<DocumentTabStrip>);
impl IExternalDockSurface for ExternalSurface {
    fn as_object(&self) -> Option<Ref<FerroObject>> {
        Some(self.0.clone().upcast())
    }
    fn dock_control(&self) -> Option<Ref<DockControl>> {
        self.0.external_owner.borrow().clone()
    }
    fn set_dock_control(&self, value: Option<Ref<DockControl>>) {
        let old = self.0.external_owner.replace(value);
        drop(old);
    }
    fn surface_control(&self) -> Ref<Control> {
        self.0.clone().upcast()
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
ferro_properties! {impl DocumentTabStrip {
    /// Identifies CanCreateItem.
    pub fn can_create_item_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("CanCreateItem",false)}
    /// Identifies IsActive.
    pub fn is_active_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsActive",false)}
    /// Identifies EnableWindowDrag.
    pub fn enable_window_drag_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("EnableWindowDrag",false)}
    /// Identifies EnableWindowStateToggleOnDoubleTap.
    pub fn enable_window_state_toggle_on_double_tap_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("EnableWindowStateToggleOnDoubleTap",false)}
    /// Identifies Orientation.
    pub fn orientation_property()->StyledProperty<Orientation> {FerroProperty::register::<Self,Orientation>("Orientation",Orientation::Horizontal)}
    /// Identifies MouseWheelScrollOrientation.
    pub fn mouse_wheel_scroll_orientation_property()->StyledProperty<Orientation> {FerroProperty::register::<Self,Orientation>("MouseWheelScrollOrientation",Orientation::Horizontal)}
    /// Identifies CreateButtonTheme.
    pub fn create_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>> {FerroProperty::register::<Self,Option<Ref<ControlTheme>>>("CreateButtonTheme",None)}
    /// Identifies CloseButtonTheme.
    pub fn close_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>> {FerroProperty::register::<Self,Option<Ref<ControlTheme>>>("CloseButtonTheme",None)}
    /// Identifies IconTemplate.
    pub fn icon_template_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("IconTemplate",None)}
    /// Identifies HeaderTemplate.
    pub fn header_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("HeaderTemplate",None)}
    /// Identifies ModifiedTemplate.
    pub fn modified_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("ModifiedTemplate",None)}
    /// Identifies CloseTemplate.
    pub fn close_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("CloseTemplate",None)}
    /// Identifies CreateButtonTemplate.
    pub fn create_button_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("CreateButtonTemplate",None)}
    /// Identifies LeftContent.
    pub fn left_content_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("LeftContent",None)}
    /// Identifies TopContent.
    pub fn top_content_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("TopContent",None)}
    /// Identifies RightContent.
    pub fn right_content_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("RightContent",None)}
    /// Identifies BottomContent.
    pub fn bottom_content_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("BottomContent",None)}
    /// Identifies LeftContentTemplate.
    pub fn left_content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("LeftContentTemplate",None)}
    /// Identifies TopContentTemplate.
    pub fn top_content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("TopContentTemplate",None)}
    /// Identifies RightContentTemplate.
    pub fn right_content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("RightContentTemplate",None)}
    /// Identifies BottomContentTemplate.
    pub fn bottom_content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("BottomContentTemplate",None)}
}}
impl DocumentTabStrip {
    /// Gets CanCreateItem.
    pub fn can_create_item(&self) -> bool {
        self.get_value(Self::can_create_item_property())
    }
    /// Sets CanCreateItem.
    pub fn set_can_create_item(&self, value: bool) {
        self.set_value(Self::can_create_item_property(), value);
    }
    /// Gets IsActive.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }
    /// Sets IsActive.
    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
    /// Gets EnableWindowDrag.
    pub fn enable_window_drag(&self) -> bool {
        self.get_value(Self::enable_window_drag_property())
    }
    /// Sets EnableWindowDrag.
    pub fn set_enable_window_drag(&self, value: bool) {
        self.set_value(Self::enable_window_drag_property(), value);
    }
    /// Gets EnableWindowStateToggleOnDoubleTap.
    pub fn enable_window_state_toggle_on_double_tap(&self) -> bool {
        self.get_value(Self::enable_window_state_toggle_on_double_tap_property())
    }
    /// Sets EnableWindowStateToggleOnDoubleTap.
    pub fn set_enable_window_state_toggle_on_double_tap(&self, value: bool) {
        self.set_value(
            Self::enable_window_state_toggle_on_double_tap_property(),
            value,
        );
    }
    /// Gets Orientation.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    /// Sets Orientation.
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value);
    }
    /// Gets MouseWheelScrollOrientation.
    pub fn mouse_wheel_scroll_orientation(&self) -> Orientation {
        self.get_value(Self::mouse_wheel_scroll_orientation_property())
    }
    /// Sets MouseWheelScrollOrientation.
    pub fn set_mouse_wheel_scroll_orientation(&self, value: Orientation) {
        self.set_value(Self::mouse_wheel_scroll_orientation_property(), value);
    }
    /// Gets CreateButtonTheme.
    pub fn create_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::create_button_theme_property())
    }
    /// Sets CreateButtonTheme.
    pub fn set_create_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::create_button_theme_property(), value);
    }
    /// Gets CloseButtonTheme.
    pub fn close_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::close_button_theme_property())
    }
    /// Sets CloseButtonTheme.
    pub fn set_close_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::close_button_theme_property(), value);
    }
    /// Gets IconTemplate.
    pub fn icon_template(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_template_property())
    }
    /// Sets IconTemplate.
    pub fn set_icon_template(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_template_property(), value);
    }
    /// Gets HeaderTemplate.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    /// Sets HeaderTemplate.
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
    /// Gets ModifiedTemplate.
    pub fn modified_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::modified_template_property())
    }
    /// Sets ModifiedTemplate.
    pub fn set_modified_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::modified_template_property(), value);
    }
    /// Gets CloseTemplate.
    pub fn close_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::close_template_property())
    }
    /// Sets CloseTemplate.
    pub fn set_close_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::close_template_property(), value);
    }
    /// Gets CreateButtonTemplate.
    pub fn create_button_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::create_button_template_property())
    }
    /// Sets CreateButtonTemplate.
    pub fn set_create_button_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::create_button_template_property(), value);
    }
    /// Gets LeftContent.
    pub fn left_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::left_content_property())
    }
    /// Sets LeftContent.
    pub fn set_left_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::left_content_property(), value);
    }
    /// Gets TopContent.
    pub fn top_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::top_content_property())
    }
    /// Sets TopContent.
    pub fn set_top_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::top_content_property(), value);
    }
    /// Gets RightContent.
    pub fn right_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::right_content_property())
    }
    /// Sets RightContent.
    pub fn set_right_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::right_content_property(), value);
    }
    /// Gets BottomContent.
    pub fn bottom_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::bottom_content_property())
    }
    /// Sets BottomContent.
    pub fn set_bottom_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::bottom_content_property(), value);
    }
    /// Gets LeftContentTemplate.
    pub fn left_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::left_content_template_property())
    }
    /// Sets LeftContentTemplate.
    pub fn set_left_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::left_content_template_property(), value);
    }
    /// Gets TopContentTemplate.
    pub fn top_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::top_content_template_property())
    }
    /// Sets TopContentTemplate.
    pub fn set_top_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::top_content_template_property(), value);
    }
    /// Gets RightContentTemplate.
    pub fn right_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::right_content_template_property())
    }
    /// Sets RightContentTemplate.
    pub fn set_right_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::right_content_template_property(), value);
    }
    /// Gets BottomContentTemplate.
    pub fn bottom_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::bottom_content_template_property())
    }
    /// Sets BottomContentTemplate.
    pub fn set_bottom_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::bottom_content_template_property(), value);
    }
}
