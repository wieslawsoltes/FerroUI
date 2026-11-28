//! Native RibbonButton/DropDownButton with the source's local Click contract.
use super::ribbon_item_helper::{IRibbonItemInternal, OwnedItemValues};
use super::{
    RibbonItemHelper, RibbonItemHelperError, RibbonItemLayout, RibbonKeyTipResult,
    RibbonOverflowMenuItems, RibbonSimplifiedLabel,
};
use crate::primitives::RibbonMetrics;
use ferroribbon_core::{RibbonItemSize, RibbonSimplifiedVisibility, layout::RibbonSizeDefinition};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledElementImplExt, StyledProperty, TypeInfo, VisualImpl, VisualImplExt,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    input::{ICommand, InputElement, InputElementImpl},
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    Button, ButtonImpl, ContentControlImpl, Control, ControlImpl,
    automation::peers::ButtonAutomationPeer,
    primitives::{FlyoutBase, TemplatedControlImpl},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
#[path = "ribbon_button_common.rs"]
mod common;
#[repr(C)]
pub struct RibbonButton {
    base: Button,
    layout_applied: Cell<bool>,
    owned_values: OwnedItemValues,
    command_enablement: super::ribbon_command_enabled::CommandEnablement,
    is_chrome_button: Cell<bool>,
    tracked_flyout: RefCell<Option<Ref<FlyoutBase>>>,
}
ferro_class! {RibbonButton:Button,virtuals RibbonButtonImpl:ButtonImpl {
 fn on_ribbon_property_changed(this,change:&ferroui_base::FerroPropertyChangedEventArgs<'_>);
 fn on_ribbon_click(this);
 fn create_linked_copy_core(this)->Option<Ref<Control>>;
 fn create_copy_instance(this)->Ref<RibbonButton>;
 fn on_unlinked_core(this);
 fn create_overflow_menu_items_core(this)->RibbonOverflowMenuItems;
 fn on_key_tip_core(this)->Rc<RibbonKeyTipResult>;
 fn invoke_core(this)->bool;
}}
ferro_class_info!(RibbonButton {new:RibbonButton::new,markup:{namespace:"FerroRibbon.Controls",properties:[
EffectiveSizeDefinition:RibbonSizeDefinition{get:RibbonButton::effective_size_definition},
ShowLabelInSimplifiedMode:bool{get:RibbonButton::show_label_in_simplified_mode},
IsChromeButton:bool{get:RibbonButton::is_chrome_button,set:RibbonButton::set_is_chrome_button},
],methods:[fn ApplyLayout(RibbonItemLayout)=>|this:&RibbonButton,layout|this.apply_layout(layout),fn PerformClick()=>|this:&RibbonButton|this.perform_click()]},interfaces:[
Rc<dyn super::IRibbonCommandSource> =>|value|Rc::new(CommandSourceView(value)) as Rc<dyn super::IRibbonCommandSource>,
Rc<dyn super::IRibbonItem> =>common::as_item,Rc<dyn super::IRibbonItemEditable> =>common::as_editable,Rc<dyn IRibbonItemInternal> =>common::as_internal,
]});
ferro_impl_classes!(RibbonButton:LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
ferro_properties! {impl RibbonButton {
 pub fn show_chevron_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowChevron",false)}
    pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonButton,_>("Id",None)}
    pub fn label_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonButton,_>("Label",None)}
    pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonButton,_>("Icon",None)}
    pub fn large_icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonButton,_>("LargeIcon",None)}
    pub fn size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonButton,_>("Size",RibbonItemSize::Medium)}
    pub fn size_definition_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonButton,_>("SizeDefinition",None)}
    pub fn current_size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonButton,_>("CurrentSize",RibbonItemSize::Medium)}
    pub fn is_simplified_property()->StyledProperty<bool>{FerroProperty::register::<RibbonButton,_>("IsSimplified",false)}
    pub fn actual_show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonButton,_>("ActualShowLabel",true)}
    pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonButton,_>("KeyTip",None)}
    pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonButton,_>("ScreenTip",None)}
    pub fn command_id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonButton,_>("CommandId",None)}
    pub fn shortcut_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonButton,_>("Shortcut",None)}
    pub fn simplified_visibility_property()->StyledProperty<RibbonSimplifiedVisibility>{FerroProperty::register::<RibbonButton,_>("SimplifiedVisibility",RibbonSimplifiedVisibility::Auto)}
    pub fn simplified_label_property()->StyledProperty<RibbonSimplifiedLabel>{FerroProperty::register::<RibbonButton,_>("SimplifiedLabel",RibbonSimplifiedLabel::Auto)}
    pub fn show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonButton,_>("ShowLabel",true)}
    pub fn can_add_to_quick_access_property()->StyledProperty<bool>{FerroProperty::register::<RibbonButton,_>("CanAddToQuickAccess",true)}
    pub fn metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>>{FerroProperty::register::<RibbonButton,_>("Metrics",Some(RibbonMetrics::comfortable()))}
}}
impl RibbonButton {
    fn static_constructor() {
        super::ribbon_command_enabled::CommandEnablement::register::<Self>();
        Self::register_common_callbacks();
    }
    pub fn construct() -> Self {
        Self {
            base: Button::construct(),
            layout_applied: Cell::new(false),
            owned_values: OwnedItemValues::default(),
            command_enablement: super::ribbon_command_enabled::CommandEnablement::default(),
            is_chrome_button: Cell::new(false),
            tracked_flyout: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn show_chevron(&self) -> bool {
        self.get_value(Self::show_chevron_property())
    }
    pub fn set_show_chevron(&self, value: bool) {
        self.set_value(Self::show_chevron_property(), value)
    }
    pub fn is_chrome_button(&self) -> bool {
        self.is_chrome_button.get()
    }
    pub fn set_is_chrome_button(&self, value: bool) {
        self.is_chrome_button.set(value)
    }
    pub fn perform_click(&self) {
        if self.is_enabled() && self.is_effectively_enabled() {
            ButtonAutomationPeer::new(self)
                .invoke()
                .unwrap_or_else(|error| panic!("{error:?}"));
        }
    }
}
impl FerroObjectImpl for RibbonButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        super::ribbon_command_enabled::CommandEnablement::install(this);
        this.initialize_common_item();
        this.click(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonButton>() {
                this.on_ribbon_click();
            }
        });
        this.add_property_changed_handler(Rc::new(|e| {
            if e.property() == Button::flyout_property().as_property()
                && e.is_effective_value_change()
                && let Some(this) = e.sender().downcast_ref::<RibbonButton>()
            {
                let old = this.tracked_flyout.borrow().clone();
                RibbonItemHelper::track_flyout_owner(old, this.flyout())
                    .unwrap_or_else(|error| panic!("{error}"));
                let old = this.tracked_flyout.replace(this.flyout());
                drop(old);
            }
        }));
    }
}
impl StyledElementImpl for RibbonButton {
    fn on_control_theme_changed(this: &Self) {
        this.command_enablement.theme_changed(this, || {
            Self::parent_on_control_theme_changed(this);
        });
    }
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl ButtonImpl for RibbonButton {
    fn uses_default_command_enablement(_this: &Self) -> bool {
        false
    }
    fn on_click(this: &Self) {
        use ferroui_controls::platform::{FeedbackAction, PlatformFeedbackExtensions};
        if !this.is_effectively_enabled() {
            return;
        }
        // Pinned source ButtonBase.mux: feedback, Click, command (Handled is not a veto),
        // then Button.mux opens the CURRENT flyout after user code has run.
        this.perform_feedback(FeedbackAction::click());
        let args = ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event());
        this.raise_event_local(&args);
        if let Some(command) = this.command() {
            let parameter = this.command_parameter();
            if command.can_execute(this.command_parameter().as_ref()) {
                command.execute(parameter.as_ref());
            }
        }
        if let Some(flyout) = this.flyout() {
            flyout.show_at(this);
        }
    }
}
impl RibbonButtonImpl for RibbonButton {
    fn on_ribbon_property_changed(
        _this: &Self,
        _change: &ferroui_base::FerroPropertyChangedEventArgs<'_>,
    ) {
    }
    fn on_ribbon_click(this: &Self) {
        if this.flyout().is_some() || this.is_chrome_button() {
            return;
        }
        if let Some(source) =
            RibbonItemHelper::get_source_item(this).and_then(|source| source.cast::<RibbonButton>())
        {
            source.perform_click();
            return;
        }
        RibbonItemHelper::notify_invoked(this, None);
    }
    fn create_linked_copy_core(this: &Self) -> Option<Ref<Control>> {
        let copy = this.create_copy_instance();
        RibbonItemHelper::link_common(this, &copy);
        copy.set_flyout(this.flyout());
        copy.set_show_chevron(this.show_chevron());
        Some(copy.upcast())
    }
    fn create_copy_instance(_this: &Self) -> Ref<RibbonButton> {
        RibbonButton::new()
    }
    fn on_unlinked_core(this: &Self) {
        this.set_flyout(None);
    }
    fn create_overflow_menu_items_core(this: &Self) -> RibbonOverflowMenuItems {
        if let Some(menu) = this
            .flyout()
            .and_then(|f| f.cast::<ferroui_controls::MenuFlyout>())
        {
            let sub = crate::compat::MenuFlyoutSubItem::new();
            sub.set_text(Some(this.label().unwrap_or_default()));
            sub.set_icon(
                RibbonItemHelper::create_menu_icon(this.icon())
                    .expect("overflow submenu icon")
                    .map(Control::boxed),
            );
            for entry in super::RibbonMenuCloner::clone(Some(
                super::ribbon_menu_cloner::native_menu_items(menu.items().clone()),
            ))
            .iter()
            {
                sub.items().add(entry);
            }
            return ferroribbon_core::model::RibbonItemsSource::from_values(vec![Some(
                sub.upcast(),
            )]);
        }
        let source = this.to_ref();
        let entry =
            super::ribbon_menu::overflow_entry(this, this.label(), this.icon(), None, move || {
                source.invoke_core();
            });
        ferroribbon_core::model::RibbonItemsSource::from_values(vec![Some(entry)])
    }
    fn on_key_tip_core(this: &Self) -> Rc<RibbonKeyTipResult> {
        if let Some(flyout) = this.flyout() {
            flyout.show_at(this);
        } else {
            this.invoke_core();
        }
        RibbonKeyTipResult::close()
    }
    fn invoke_core(this: &Self) -> bool {
        if !this.is_enabled() || !this.is_effectively_enabled() {
            return false;
        }
        if let Some(flyout) = this.flyout() {
            flyout.show_at(this);
            return true;
        }
        this.perform_click();
        true
    }
}
struct CommandSourceView(Ref<RibbonButton>);
impl super::IRibbonCommandSource for CommandSourceView {
    fn as_object(&self) -> Ref<ferroui_base::FerroObject> {
        self.0.clone().upcast()
    }
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.0.command()
    }
    fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.0.set_command(value)
    }
    fn command_parameter(&self) -> Option<BoxedValue> {
        self.0.command_parameter()
    }
    fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.0.set_command_parameter(value)
    }
}
#[repr(C)]
pub struct RibbonDropDownButton {
    base: RibbonButton,
}
ferro_class!(RibbonDropDownButton:RibbonButton);
ferro_class_info!(RibbonDropDownButton{new:RibbonDropDownButton::new,markup:{namespace:"FerroRibbon.Controls"}});
ferro_impl_classes!(RibbonDropDownButton:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl RibbonDropDownButton {
    pub fn construct() -> Self {
        Self {
            base: RibbonButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
impl FerroObjectImpl for RibbonDropDownButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_show_chevron(true);
    }
}
impl StyledElementImpl for RibbonDropDownButton {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl RibbonButtonImpl for RibbonDropDownButton {
    fn create_copy_instance(_this: &Self) -> Ref<RibbonButton> {
        RibbonDropDownButton::new().upcast()
    }
}

impl super::ribbon_command_enabled::CommandEnabledOwner for RibbonButton {
    fn command_enablement(&self) -> &super::ribbon_command_enabled::CommandEnablement {
        &self.command_enablement
    }
    fn native_button(&self) -> &ferroui_controls::Button {
        self
    }
}

impl VisualImpl for RibbonButton {
    fn on_attached_to_visual_tree_core(
        this: &Self,
        event: &ferroui_base::VisualTreeAttachmentEventArgs,
    ) {
        Self::parent_on_attached_to_visual_tree_core(this, event);
        this.command_enablement.visual_enter(this);
    }
    fn on_detached_from_visual_tree_core(
        this: &Self,
        event: &ferroui_base::VisualTreeAttachmentEventArgs,
    ) {
        Self::parent_on_detached_from_visual_tree_core(this, event);
        this.command_enablement.visual_leave();
    }
}
