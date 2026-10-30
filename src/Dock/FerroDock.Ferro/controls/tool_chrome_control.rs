// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::HostWindow;
use crate::{
    automation::peers::ToolChromeControlAutomationPeer,
    internal::{tool_menu_lifetime::ToolMenuLifetime, window_drag_helper::WindowDragHelper},
    model_projection,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, PointerPressedEventArgs},
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies},
    layout::LayoutableImpl,
    styling::ControlTheme,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{
        FlyoutBase, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
    },
    Button, ContentControl, ContentControlImpl, Control, ControlImpl, TopLevel, Window,
    WindowState,
};
use ferroui_system::IDisposable;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
/// Original tool chrome with template grips, focus routing and flyout lifetime.
#[repr(C)]
pub struct ToolChromeControl {
    base: ContentControl,
    attached_window: RefCell<Option<Ref<HostWindow>>>,
    window_drag_helper: RefCell<Option<Rc<WindowDragHelper>>>,
    maximize_restore_button: RefCell<Option<Ref<Button>>>,
    maximize_restore_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    menu_lifetime: Rc<ToolMenuLifetime>,
    grip: RefCell<Option<Ref<Control>>>,
    close_button: RefCell<Option<Ref<Button>>>,
    pressed: Cell<Option<RoutedEventHandlerToken>>,
}
ferro_class!(ToolChromeControl:ContentControl);
ferro_class_info!(ToolChromeControl {new:ToolChromeControl::new,markup:{
    attributes:[PseudoClasses(":floating",":active",":pinned",":maximized"),TemplatePart("PART_Grip",type(Ref<Control>)),TemplatePart("PART_CloseButton",type(Ref<Button>)),TemplatePart("PART_MaximizeRestoreButton",type(Ref<Button>))],
    fields:[TitleProprty:&'static FerroProperty => (||ToolChromeControl::title_proprty().as_property())]
}});
ferro_impl_classes!(ToolChromeControl:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ContentControlImpl);
impl FerroObjectImpl for ToolChromeControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_active_property().as_property()
            || change.property() == Self::is_pinned_property().as_property()
            || change.property() == Self::is_floating_property().as_property()
            || change.property() == Self::is_maximized_property().as_property()
        {
            this.update_pseudo_classes();
        }
    }
}
impl ControlImpl for ToolChromeControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ToolChromeControlAutomationPeer::new(this).upcast()
    }
}
impl VisualImpl for ToolChromeControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attach_to_window();
        this.menu_lifetime.attach();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.remove_pressed_handler();
        this.remove_maximize_handler();
        this.detach_from_window();
        this.menu_lifetime.detach();
    }
}
impl TemplatedControlImpl for ToolChromeControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.detach_from_window();
        this.remove_pressed_handler();
        this.remove_maximize_handler();
        this.grip
            .replace(e.name_scope().find_as::<Control>("PART_Grip"));
        this.close_button
            .replace(e.name_scope().find_as::<Button>("PART_CloseButton"));
        this.menu_lifetime
            .set_button(e.name_scope().find_as::<Button>("PART_MenuButton"));
        let owner = this.to_ref();
        let token = this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| owner.pressed_handler(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        this.pressed.set(Some(token));
        this.attach_to_window();
        this.maximize_restore_button.replace(
            e.name_scope()
                .find_as::<Button>("PART_MaximizeRestoreButton"),
        );
        if let Some(button) = this.maximize_restore_button.borrow().clone() {
            let owner = this.to_ref();
            let subscription = button.add_disposable_handler_with(
                Button::click_event(),
                move |_, e| owner.on_maximize_restore_button_clicked(e),
                RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
                false,
            );
            this.maximize_restore_subscription
                .replace(Some(subscription));
        }
    }
}
ferro_properties! {impl ToolChromeControl {
    /// Identifies Title, preserving the source public field's TitleProprty spelling.
    pub fn title_proprty()->StyledProperty<Option<String>> {FerroProperty::register::<Self,Option<String>>("Title",None)}
    /// Identifies IsActive.
    pub fn is_active_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsActive",false)}
    /// Identifies IsPinned.
    pub fn is_pinned_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsPinned",false)}
    /// Identifies IsFloating.
    pub fn is_floating_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsFloating",false)}
    /// Identifies IsMaximized.
    pub fn is_maximized_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("IsMaximized",false)}
    /// Identifies ToolFlyout.
    pub fn tool_flyout_property()->StyledProperty<Option<Ref<FlyoutBase>>> {FerroProperty::register::<Self,Option<Ref<FlyoutBase>>>("ToolFlyout",None)}
    /// Identifies CloseButtonTheme.
    pub fn close_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>> {FerroProperty::register::<Self,Option<Ref<ControlTheme>>>("CloseButtonTheme",None)}
    /// Identifies MaximizeButtonTheme.
    pub fn maximize_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>> {FerroProperty::register::<Self,Option<Ref<ControlTheme>>>("MaximizeButtonTheme",None)}
    /// Identifies PinButtonTheme.
    pub fn pin_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>> {FerroProperty::register::<Self,Option<Ref<ControlTheme>>>("PinButtonTheme",None)}
    /// Identifies MenuButtonTheme.
    pub fn menu_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>> {FerroProperty::register::<Self,Option<Ref<ControlTheme>>>("MenuButtonTheme",None)}
}}
impl ToolChromeControl {
    /// Constructs original native source class state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            attached_window: RefCell::new(None),
            window_drag_helper: RefCell::new(None),
            maximize_restore_button: RefCell::new(None),
            maximize_restore_subscription: RefCell::new(None),
            menu_lifetime: ToolMenuLifetime::new(),
            grip: RefCell::new(None),
            close_button: RefCell::new(None),
            pressed: Cell::new(None),
        }
    }
    /// Creates original tool chrome with inactive pseudo classes.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the chrome title, including the original null property default.
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_proprty())
    }
    /// Sets the chrome title.
    pub fn set_title(&self, value: Option<String>) {
        self.set_value(Self::title_proprty(), value);
    }
    /// Gets IsActive.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }
    /// Sets IsActive.
    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
    /// Gets IsPinned.
    pub fn is_pinned(&self) -> bool {
        self.get_value(Self::is_pinned_property())
    }
    /// Sets IsPinned.
    pub fn set_is_pinned(&self, value: bool) {
        self.set_value(Self::is_pinned_property(), value);
    }
    /// Gets IsFloating.
    pub fn is_floating(&self) -> bool {
        self.get_value(Self::is_floating_property())
    }
    /// Sets IsFloating.
    pub fn set_is_floating(&self, value: bool) {
        self.set_value(Self::is_floating_property(), value);
    }
    /// Gets IsMaximized.
    pub fn is_maximized(&self) -> bool {
        self.get_value(Self::is_maximized_property())
    }
    /// Sets IsMaximized.
    pub fn set_is_maximized(&self, value: bool) {
        self.set_value(Self::is_maximized_property(), value);
    }
    /// Gets ToolFlyout.
    pub fn tool_flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.get_value(Self::tool_flyout_property())
    }
    /// Sets ToolFlyout.
    pub fn set_tool_flyout(&self, value: Option<Ref<FlyoutBase>>) {
        self.set_value(Self::tool_flyout_property(), value);
    }
    /// Gets CloseButtonTheme.
    pub fn close_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::close_button_theme_property())
    }
    /// Sets CloseButtonTheme.
    pub fn set_close_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::close_button_theme_property(), value);
    }
    /// Gets MaximizeButtonTheme.
    pub fn maximize_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::maximize_button_theme_property())
    }
    /// Sets MaximizeButtonTheme.
    pub fn set_maximize_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::maximize_button_theme_property(), value);
    }
    /// Gets PinButtonTheme.
    pub fn pin_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::pin_button_theme_property())
    }
    /// Sets PinButtonTheme.
    pub fn set_pin_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::pin_button_theme_property(), value);
    }
    /// Gets MenuButtonTheme.
    pub fn menu_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::menu_button_theme_property())
    }
    /// Sets MenuButtonTheme.
    pub fn set_menu_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::menu_button_theme_property(), value);
    }
    pub(crate) fn grip(&self) -> Option<Ref<Control>> {
        self.grip.borrow().clone()
    }
    pub(crate) fn close_button(&self) -> Option<Ref<Button>> {
        self.close_button.borrow().clone()
    }
    fn remove_pressed_handler(&self) {
        if let Some(token) = self.pressed.take() {
            self.remove_handler(InputElement::pointer_pressed_event(), token);
        }
    }
    fn remove_maximize_handler(&self) {
        if self.maximize_restore_button.borrow().is_some() {
            let subscription = self.maximize_restore_subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            self.maximize_restore_button.replace(None);
        }
    }
    fn pressed_handler(&self, _e: &PointerPressedEventArgs) {
        if let Some((dock, factory)) = model_projection::dockable(self.data_context().as_ref())
            .and_then(|d| d.as_dock())
            .and_then(|d| d.factory().map(|f| (d, f)))
        {
            if dock.active_dockable().is_some() {
                if let Some(root) = factory.find_root(
                    Some(dock.active_dockable().expect("source active re-read")),
                    Some(Rc::new(|_| true)),
                ) {
                    factory.set_focused_dockable(Some(root), dock.active_dockable());
                }
            }
        }
    }
    fn is_focusable_child(owner: &Control, source: &Control) -> bool {
        let mut current = Some(source.to_ref());
        while let Some(control) = current {
            if control.ptr_eq(&owner.to_ref()) {
                break;
            }
            if control.focusable() {
                return true;
            }
            current = control.parent().and_then(|p| p.cast::<Control>());
        }
        false
    }
    fn create_drag_helper(
        &self,
        owner: Ref<Control>,
        ignore_focusable: bool,
        handle_pointer_pressed: bool,
        is_enabled: Option<Rc<dyn Fn() -> bool>>,
    ) -> Rc<WindowDragHelper> {
        let captured_owner = owner.clone();
        WindowDragHelper::new(
            owner,
            is_enabled.unwrap_or_else(|| Rc::new(|| true)),
            Rc::new(move |source| {
                let Some(source) = source else {
                    return false;
                };
                if ignore_focusable && Self::is_focusable_child(&captured_owner, source) {
                    return false;
                }
                !source.is::<Button>()
                    && !WindowDragHelper::is_child_of_type::<Button>(&captured_owner, source)
            }),
            handle_pointer_pressed,
            None,
        )
    }
    fn attach_to_window(&self) {
        if self.grip().is_none() {
            return;
        }
        if let Some(window) = TopLevel::get_top_level(Some(self)).and_then(|t| t.cast::<Window>()) {
            if let Some(host) = window.cast::<HostWindow>() {
                host.attach_chrome_grip(self);
                self.attached_window.replace(Some(host.clone()));
                self.set_current_value(Self::is_floating_property(), true);
                let helper = self.create_drag_helper(
                    self.grip().expect("source grip re-read"),
                    true,
                    true,
                    Some(Rc::new(move || host.tool_chrome_controls_whole_window())),
                );
                self.window_drag_helper.replace(Some(helper));
                let helper = self
                    .window_drag_helper
                    .borrow()
                    .clone()
                    .expect("source helper");
                helper.attach();
            }
        }
        // The ordinary-window branch is disabled in the original source too.
    }
    fn detach_from_window(&self) {
        let attached = self.attached_window.borrow().clone();
        if let Some(attached) = attached {
            attached.detach_chrome_grip(self);
            self.attached_window.replace(None);
        }
        let helper = self.window_drag_helper.borrow().clone();
        if let Some(helper) = helper {
            helper.detach();
            self.window_drag_helper.replace(None);
        }
    }
    fn on_maximize_restore_button_clicked(&self, _e: &RoutedEventArgs) {
        if let Some(window) =
            TopLevel::get_top_level(Some(self)).and_then(|t| t.cast::<HostWindow>())
        {
            window.set_window_state(if window.window_state() == WindowState::Maximized {
                WindowState::Normal
            } else {
                WindowState::Maximized
            });
        }
    }
    fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(":active", self.is_active());
        self.pseudo_classes().set(":pinned", self.is_pinned());
        self.pseudo_classes().set(":floating", self.is_floating());
        self.pseudo_classes().set(":maximized", self.is_maximized());
    }
}
