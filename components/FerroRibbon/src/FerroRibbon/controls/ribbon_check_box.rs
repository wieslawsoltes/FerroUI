use ferroui_controls::CheckBox;
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
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
#[path = "ribbon_check_box_common.rs"]
mod common;
#[repr(C)]
pub struct RibbonCheckBox {
    base: ferroui_controls::CheckBox,
    layout_applied: Cell<bool>,
    owned_values: OwnedItemValues,
    command_enablement: super::ribbon_command_enabled::CommandEnablement,
    checked_events: CheckedEvents,
    scope: RefCell<Option<ferroui_base::controls::NameScopeRef>>,
    is_pointer_over: Cell<bool>,
}
ferro_class! {RibbonCheckBox:CheckBox,virtuals RibbonCheckBoxImpl:ToggleButtonImpl {
 fn create_linked_copy_core(this)->Option<Ref<Control>>;
 fn on_unlinked_core(this);
 fn create_overflow_menu_items_core(this)->RibbonOverflowMenuItems;
 fn on_key_tip_core(this)->Rc<RibbonKeyTipResult>;
 fn invoke_core(this)->bool;
}}
ferro_class_info!(RibbonCheckBox{new:RibbonCheckBox::new,markup:{namespace:"FerroRibbon.Controls",properties:[EffectiveSizeDefinition:RibbonSizeDefinition{get:RibbonCheckBox::effective_size_definition},ShowLabelInSimplifiedMode:bool{get:RibbonCheckBox::show_label_in_simplified_mode}],methods:[fn ApplyLayout(RibbonItemLayout)=>|this:&RibbonCheckBox,layout|this.apply_layout(layout)],events:[Checked(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs){add:RibbonCheckBox::add_checked,remove:RibbonCheckBox::remove_checked},Unchecked(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs){add:RibbonCheckBox::add_unchecked,remove:RibbonCheckBox::remove_unchecked},Indeterminate(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs){add:RibbonCheckBox::add_indeterminate,remove:RibbonCheckBox::remove_indeterminate}]},interfaces:[
 Rc<dyn super::IRibbonCommandSource> => |v|Rc::new(CommandSourceView(v)) as Rc<dyn super::IRibbonCommandSource>,
 Rc<dyn super::IRibbonCheckable> => |v|Rc::new(CommandSourceView(v)) as Rc<dyn super::IRibbonCheckable>,
 Rc<dyn super::IRibbonItem> =>common::as_item,Rc<dyn super::IRibbonItemEditable> =>common::as_editable,Rc<dyn IRibbonItemInternal> =>common::as_internal]});
ferro_impl_classes!(RibbonCheckBox:InteractiveImpl,ContentControlImpl,ControlImpl);
ferro_properties! {impl RibbonCheckBox{     pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonCheckBox,_>("Id",None)}
    pub fn label_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonCheckBox,_>("Label",None)}
    pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonCheckBox,_>("Icon",None)}
    pub fn large_icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonCheckBox,_>("LargeIcon",None)}
    pub fn size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonCheckBox,_>("Size",RibbonItemSize::Medium)}
    pub fn size_definition_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonCheckBox,_>("SizeDefinition",None)}
    pub fn current_size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonCheckBox,_>("CurrentSize",RibbonItemSize::Medium)}
    pub fn is_simplified_property()->StyledProperty<bool>{FerroProperty::register::<RibbonCheckBox,_>("IsSimplified",false)}
    pub fn actual_show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonCheckBox,_>("ActualShowLabel",true)}
    pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonCheckBox,_>("KeyTip",None)}
    pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonCheckBox,_>("ScreenTip",None)}
    pub fn command_id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonCheckBox,_>("CommandId",None)}
    pub fn shortcut_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonCheckBox,_>("Shortcut",None)}
    pub fn simplified_visibility_property()->StyledProperty<RibbonSimplifiedVisibility>{FerroProperty::register::<RibbonCheckBox,_>("SimplifiedVisibility",RibbonSimplifiedVisibility::Auto)}
    pub fn simplified_label_property()->StyledProperty<RibbonSimplifiedLabel>{FerroProperty::register::<RibbonCheckBox,_>("SimplifiedLabel",RibbonSimplifiedLabel::Auto)}
    pub fn show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonCheckBox,_>("ShowLabel",true)}
    pub fn can_add_to_quick_access_property()->StyledProperty<bool>{FerroProperty::register::<RibbonCheckBox,_>("CanAddToQuickAccess",true)}
    pub fn metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>>{FerroProperty::register::<RibbonCheckBox,_>("Metrics",Some(RibbonMetrics::comfortable()))}
}}
impl RibbonCheckBox {
    fn static_constructor() {
        super::ribbon_command_enabled::CommandEnablement::register::<Self>();
        Self::register_common_callbacks();
    }
    pub fn construct() -> Self {
        Self {
            base: ferroui_controls::CheckBox::construct(),
            layout_applied: Cell::new(false),
            owned_values: OwnedItemValues::default(),
            command_enablement: super::ribbon_command_enabled::CommandEnablement::default(),
            checked_events: CheckedEvents::default(),
            scope: RefCell::default(),
            is_pointer_over: Cell::new(false),
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
}
impl FerroObjectImpl for RibbonCheckBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        super::ribbon_command_enabled::CommandEnablement::install(this);
        this.initialize_common_item();
        this.click(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonCheckBox>()
                && RibbonItemHelper::get_source_item(this).is_none()
            {
                RibbonItemHelper::notify_invoked(this, parameter(this.is_checked()));
            }
        });
        let weak = this.to_ref().downgrade();
        this.actual_theme_variant_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_check_visual();
            }
        })
        .detach();
        this.add_property_changed_handler(Rc::new(|e| {
            if (e.property() == ToggleButton::is_checked_property().as_property()
                || e.property() == InputElement::is_enabled_property().as_property())
                && let Some(this) = e.sender().downcast_ref::<RibbonCheckBox>()
            {
                this.update_check_visual();
            }
        }));
    }
}
impl StyledElementImpl for RibbonCheckBox {
    fn on_control_theme_changed(this: &Self) {
        this.command_enablement.theme_changed(this, || {
            Self::parent_on_control_theme_changed(this);
        });
    }
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl ButtonImpl for RibbonCheckBox {
    fn uses_default_command_enablement(_this: &Self) -> bool {
        false
    }
    fn on_click(this: &Self) {
        super::ribbon_checked_events::click(this);
    }
}
impl ToggleButtonImpl for RibbonCheckBox {
    fn toggle(this: &Self) {
        Self::parent_toggle(this);
        this.update_check_visual();
    }
    fn on_is_checked_changed(this: &Self, _e: &ferroui_base::interactivity::RoutedEventArgs) {
        let state = this.is_checked();
        this.checked_events.raise(this, state, || {});
    }
}
impl RibbonCheckBoxImpl for RibbonCheckBox {
    fn create_linked_copy_core(this: &Self) -> Option<Ref<Control>> {
        let copy = RibbonCheckBox::new();
        RibbonItemHelper::link_common(this, &copy);
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
struct CommandSourceView(Ref<RibbonCheckBox>);
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
impl RibbonCheckBox {
    fn update_check_visual(&self) {
        let scope = self.scope.borrow().clone();
        let Some(scope) = scope else { return };
        let Some(b) = scope
            .find("PART_Box")
            .and_then(|b| b.cast::<ferroui_controls::Border>())
        else {
            return;
        };
        let on = self.is_checked() != Some(false);
        let background = crate::theming::RibbonTheme::get_brush(
            Some(self),
            Some(if on {
                "RibbonAccentBrush"
            } else {
                "RibbonInputBackgroundBrush"
            }),
        )
        .expect("checkbox background");
        b.set_background(background.map(Into::into));
        let border = crate::theming::RibbonTheme::get_brush(
            Some(self),
            Some(if on {
                "RibbonAccentBrush"
            } else {
                "RibbonInputHoverBorderBrush"
            }),
        )
        .expect("checkbox border");
        b.set_border_brush(border.map(Into::into));
        if let Some(check) = scope.find("PART_Check").and_then(|c| c.cast::<Control>()) {
            check.set_is_visible(self.is_checked() == Some(true));
        }
        if let Some(check) = scope
            .find("PART_Indeterminate")
            .and_then(|c| c.cast::<Control>())
        {
            check.set_is_visible(self.is_checked().is_none());
        }
        b.set_opacity(if self.is_enabled() { 1. } else { 0.5 });
        if !self.is_enabled() {
            self.is_pointer_over.set(false);
        }
        self.update_ribbon_states();
    }
    fn update_ribbon_states(&self) {
        self.pseudo_classes()
            .set(":ribbon-disabled", !self.is_enabled());
        self.pseudo_classes().set(
            ":ribbon-hover",
            self.is_pointer_over.get() && self.is_enabled(),
        );
    }
}
impl TemplatedControlImpl for RibbonCheckBox {
    fn on_apply_template(this: &Self, e: &ferroui_controls::primitives::TemplateAppliedEventArgs) {
        use ferroui_controls::primitives::TemplatedControlImplExt;
        Self::parent_on_apply_template(this, e);
        this.scope.replace(Some(e.name_scope().clone()));
        this.update_check_visual();
    }
}
impl InputElementImpl for RibbonCheckBox {
    fn on_pointer_entered(this: &Self, e: &ferroui_base::input::PointerEventArgs) {
        use ferroui_base::input::InputElementImplExt;
        Self::parent_on_pointer_entered(this, e);
        this.is_pointer_over.set(true);
        this.update_ribbon_states();
    }
    fn on_pointer_exited(this: &Self, e: &ferroui_base::input::PointerEventArgs) {
        use ferroui_base::input::InputElementImplExt;
        Self::parent_on_pointer_exited(this, e);
        this.is_pointer_over.set(false);
        this.update_ribbon_states();
    }
    fn on_pointer_capture_lost(this: &Self, e: &ferroui_base::input::PointerCaptureLostEventArgs) {
        use ferroui_base::input::InputElementImplExt;
        Self::parent_on_pointer_capture_lost(this, e);
        this.update_ribbon_states();
    }
}

impl LayoutableImpl for RibbonCheckBox {
    fn apply_template(this: &Self) {
        use ferroui_base::layout::LayoutableImplExt;
        Self::parent_apply_template(this);
        if this.visual_children().count() == 0 {
            this.scope.replace(None);
        }
    }
}

impl super::ribbon_command_enabled::CommandEnabledOwner for RibbonCheckBox {
    fn command_enablement(&self) -> &super::ribbon_command_enabled::CommandEnablement {
        &self.command_enablement
    }
    fn native_button(&self) -> &ferroui_controls::Button {
        self
    }
}

impl VisualImpl for RibbonCheckBox {
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
