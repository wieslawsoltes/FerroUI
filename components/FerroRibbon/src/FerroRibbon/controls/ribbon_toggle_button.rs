use ferroui_controls::primitives::ToggleButtonImplExt;
// Original RibbonButton.cs checked control.
use super::ribbon_checked_events::{CheckedEvents, parameter};
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
    ButtonImpl, ContentControlImpl, Control, ControlImpl,
    primitives::{TemplatedControlImpl, ToggleButton, ToggleButtonImpl},
};
use std::{cell::Cell, rc::Rc, sync::Arc};
#[path = "ribbon_toggle_button_common.rs"]
mod common;
#[repr(C)]
pub struct RibbonToggleButton {
    base: ToggleButton,
    layout_applied: Cell<bool>,
    owned_values: OwnedItemValues,
    command_enablement: super::ribbon_command_enabled::CommandEnablement,
    checked_events: CheckedEvents,
}
ferro_class! {RibbonToggleButton:ToggleButton,virtuals RibbonToggleButtonImpl:ToggleButtonImpl {
 fn create_linked_copy_core(this)->Option<Ref<Control>>;
 fn on_unlinked_core(this);
 fn create_overflow_menu_items_core(this)->RibbonOverflowMenuItems;
 fn on_key_tip_core(this)->Rc<RibbonKeyTipResult>;
 fn invoke_core(this)->bool;
}}
ferro_class_info!(RibbonToggleButton{new:RibbonToggleButton::new,markup:{namespace:"FerroRibbon.Controls",properties:[EffectiveSizeDefinition:RibbonSizeDefinition{get:RibbonToggleButton::effective_size_definition},ShowLabelInSimplifiedMode:bool{get:RibbonToggleButton::show_label_in_simplified_mode}],methods:[fn ApplyLayout(RibbonItemLayout)=>|this:&RibbonToggleButton,layout|this.apply_layout(layout)],events:[Checked(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs){add:RibbonToggleButton::add_checked,remove:RibbonToggleButton::remove_checked},Unchecked(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs){add:RibbonToggleButton::add_unchecked,remove:RibbonToggleButton::remove_unchecked},Indeterminate(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs){add:RibbonToggleButton::add_indeterminate,remove:RibbonToggleButton::remove_indeterminate}]},interfaces:[
 Rc<dyn super::IRibbonCommandSource> => |v|Rc::new(CommandSourceView(v)) as Rc<dyn super::IRibbonCommandSource>,
 Rc<dyn super::IRibbonCheckable> => |v|Rc::new(CommandSourceView(v)) as Rc<dyn super::IRibbonCheckable>,
 Rc<dyn super::IRibbonItem> =>common::as_item,Rc<dyn super::IRibbonItemEditable> =>common::as_editable,Rc<dyn IRibbonItemInternal> =>common::as_internal]});
ferro_impl_classes!(RibbonToggleButton:LayoutableImpl,InteractiveImpl,ContentControlImpl,ControlImpl,InputElementImpl,TemplatedControlImpl);
ferro_properties! {impl RibbonToggleButton{ pub fn group_name_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("GroupName",None)}
 pub fn show_chevron_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowChevron",false)}
    pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonToggleButton,_>("Id",None)}
    pub fn label_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonToggleButton,_>("Label",None)}
    pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonToggleButton,_>("Icon",None)}
    pub fn large_icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonToggleButton,_>("LargeIcon",None)}
    pub fn size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonToggleButton,_>("Size",RibbonItemSize::Small)}
    pub fn size_definition_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonToggleButton,_>("SizeDefinition",None)}
    pub fn current_size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonToggleButton,_>("CurrentSize",RibbonItemSize::Small)}
    pub fn is_simplified_property()->StyledProperty<bool>{FerroProperty::register::<RibbonToggleButton,_>("IsSimplified",false)}
    pub fn actual_show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonToggleButton,_>("ActualShowLabel",true)}
    pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonToggleButton,_>("KeyTip",None)}
    pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonToggleButton,_>("ScreenTip",None)}
    pub fn command_id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonToggleButton,_>("CommandId",None)}
    pub fn shortcut_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonToggleButton,_>("Shortcut",None)}
    pub fn simplified_visibility_property()->StyledProperty<RibbonSimplifiedVisibility>{FerroProperty::register::<RibbonToggleButton,_>("SimplifiedVisibility",RibbonSimplifiedVisibility::Auto)}
    pub fn simplified_label_property()->StyledProperty<RibbonSimplifiedLabel>{FerroProperty::register::<RibbonToggleButton,_>("SimplifiedLabel",RibbonSimplifiedLabel::Auto)}
    pub fn show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonToggleButton,_>("ShowLabel",true)}
    pub fn can_add_to_quick_access_property()->StyledProperty<bool>{FerroProperty::register::<RibbonToggleButton,_>("CanAddToQuickAccess",true)}
    pub fn metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>>{FerroProperty::register::<RibbonToggleButton,_>("Metrics",Some(RibbonMetrics::comfortable()))}
}}
impl RibbonToggleButton {
    fn static_constructor() {
        super::ribbon_command_enabled::CommandEnablement::register::<Self>();
        Self::register_common_callbacks();
    }
    pub fn construct() -> Self {
        Self {
            base: ToggleButton::construct(),
            layout_applied: Cell::new(false),
            owned_values: OwnedItemValues::default(),
            command_enablement: super::ribbon_command_enabled::CommandEnablement::default(),
            checked_events: CheckedEvents::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    // The source private partial has no implementation and is erased by C#.
    fn on_ribbon_property_changed(
        &self,
        _change: &ferroui_base::FerroPropertyChangedEventArgs<'_>,
    ) {
    }
    pub fn add_checked(&self, h: Option<ferroui_base::metadata::MarkupDelegate>) {
        self.checked_events.add(0, h);
    }
    pub fn remove_checked(&self, h: Option<ferroui_base::metadata::MarkupDelegate>) {
        self.checked_events.remove(0, h);
    }
    pub fn add_unchecked(&self, h: Option<ferroui_base::metadata::MarkupDelegate>) {
        self.checked_events.add(1, h);
    }
    pub fn remove_unchecked(&self, h: Option<ferroui_base::metadata::MarkupDelegate>) {
        self.checked_events.remove(1, h);
    }
    pub fn add_indeterminate(&self, h: Option<ferroui_base::metadata::MarkupDelegate>) {
        self.checked_events.add(2, h);
    }
    pub fn remove_indeterminate(&self, h: Option<ferroui_base::metadata::MarkupDelegate>) {
        self.checked_events.remove(2, h);
    }
    pub fn group_name(&self) -> Option<String> {
        self.get_value(Self::group_name_property())
    }
    pub fn set_group_name(&self, v: Option<String>) {
        self.set_value(Self::group_name_property(), v);
    }
    pub fn show_chevron(&self) -> bool {
        self.get_value(Self::show_chevron_property())
    }
    pub fn set_show_chevron(&self, v: bool) {
        self.set_value(Self::show_chevron_property(), v);
    }
}
impl FerroObjectImpl for RibbonToggleButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        super::ribbon_command_enabled::CommandEnablement::install(this);
        this.initialize_common_item();
        this.click(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonToggleButton>()
                && RibbonItemHelper::get_source_item(this).is_none()
            {
                RibbonItemHelper::notify_invoked(this, parameter(this.is_checked()));
            }
        });
    }
}
impl StyledElementImpl for RibbonToggleButton {
    fn on_control_theme_changed(this: &Self) {
        this.command_enablement.theme_changed(this, || {
            Self::parent_on_control_theme_changed(this);
        });
    }
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl ButtonImpl for RibbonToggleButton {
    fn uses_default_command_enablement(_this: &Self) -> bool {
        false
    }
    fn on_click(this: &Self) {
        super::ribbon_checked_events::click(this);
    }
}
impl ToggleButtonImpl for RibbonToggleButton {
    fn toggle(this: &Self) {
        if this.group_name().as_ref().is_some_and(|n| !n.is_empty())
            && this.is_checked() == Some(true)
        {
            return;
        }
        Self::parent_toggle(this);
    }
    fn on_is_checked_changed(this: &Self, _e: &ferroui_base::interactivity::RoutedEventArgs) {
        let state = this.is_checked();
        this.checked_events.raise(this, state, || {
            if state == Some(true) {
                super::ribbon_toggle_groups::on_checked(this);
            }
        });
    }
}
impl RibbonToggleButtonImpl for RibbonToggleButton {
    fn create_linked_copy_core(this: &Self) -> Option<Ref<Control>> {
        let copy = RibbonToggleButton::new();
        RibbonItemHelper::link_common(this, &copy);
        RibbonItemHelper::link(this, &copy, Self::group_name_property(), false);
        RibbonItemHelper::link(this, &copy, Self::show_chevron_property(), false);
        RibbonItemHelper::link(this, &copy, ToggleButton::is_checked_property(), true);
        let weak = this.to_ref().downgrade();
        copy.click(move |_, _| {
            if let Some(source) = weak.upgrade() {
                RibbonItemHelper::execute(
                    &CommandSourceView(source.clone()),
                    source.command_parameter().as_ref(),
                );
                RibbonItemHelper::notify_invoked(&source, parameter(source.is_checked()));
            }
        });
        Some(copy.upcast())
    }
    // Source intentionally has no unlink-specific state beyond common registrations.
    fn on_unlinked_core(_this: &Self) {}
    fn create_overflow_menu_items_core(this: &Self) -> RibbonOverflowMenuItems {
        let source = this.to_ref();
        let entry = super::ribbon_menu::overflow_entry(
            this,
            this.label(),
            this.icon(),
            Some(this.is_checked() == Some(true)),
            move || {
                source.invoke_core();
            },
        );
        ferroribbon_core::model::RibbonItemsSource::from_values(vec![Some(entry)])
    }
    fn on_key_tip_core(this: &Self) -> Rc<RibbonKeyTipResult> {
        this.invoke_core();
        RibbonKeyTipResult::close()
    }
    fn invoke_core(this: &Self) -> bool {
        if !this.is_enabled() || !this.is_effectively_enabled() {
            return false;
        }
        ferroui_controls::automation::peers::ToggleButtonAutomationPeer::new(this)
            .toggle()
            .expect("source toggle peer");
        RibbonItemHelper::execute(
            &CommandSourceView(this.to_ref()),
            this.command_parameter().as_ref(),
        );
        RibbonItemHelper::notify_invoked(this, parameter(this.is_checked()));
        true
    }
}
struct CommandSourceView(Ref<RibbonToggleButton>);
impl super::IRibbonCommandSource for CommandSourceView {
    fn as_object(&self) -> Ref<ferroui_base::FerroObject> {
        self.0.clone().upcast()
    }
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.0.command()
    }
    fn set_command(&self, v: Option<Rc<dyn ICommand>>) {
        self.0.set_command(v);
    }
    fn command_parameter(&self) -> Option<BoxedValue> {
        self.0.command_parameter()
    }
    fn set_command_parameter(&self, v: Option<BoxedValue>) {
        self.0.set_command_parameter(v);
    }
}
impl super::IRibbonCheckable for CommandSourceView {
    fn as_object(&self) -> Ref<ferroui_base::FerroObject> {
        self.0.clone().upcast()
    }
    fn is_checked(&self) -> Option<bool> {
        self.0.is_checked()
    }
    fn set_is_checked(&self, v: Option<bool>) {
        self.0.set_is_checked(v);
    }
}

impl super::ribbon_command_enabled::CommandEnabledOwner for RibbonToggleButton {
    fn command_enablement(&self) -> &super::ribbon_command_enabled::CommandEnablement {
        &self.command_enablement
    }
    fn native_button(&self) -> &ferroui_controls::Button {
        self
    }
}

impl VisualImpl for RibbonToggleButton {
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
