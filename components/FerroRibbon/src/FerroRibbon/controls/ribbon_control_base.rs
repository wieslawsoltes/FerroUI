//! Native RibbonControlBase and its source generated common item surface.
use super::ribbon_item_helper::{IRibbonItemInternal, OwnedItemValues};
use super::{
    RibbonItemHelperError, RibbonItemLayout, RibbonKeyTipResult, RibbonOverflowMenuItems,
    RibbonSimplifiedLabel,
};
use crate::primitives::RibbonMetrics;
use ferroribbon_core::{RibbonItemSize, RibbonSimplifiedVisibility, layout::RibbonSizeDefinition};
use ferroui_base::input::{ICommand, InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::reactive::{IDisposable, SerialDisposable};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, StyledPropertyMetadata, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::{
    Control, ControlImpl,
    automation::{
        AutomationProperties,
        peers::{
            AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
            ControlAutomationPeerImpl, NoneAutomationPeer,
        },
    },
};
use std::sync::Arc;
#[path = "ribbon_control_base_common.rs"]
mod common;
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
pub struct RibbonControlBase {
    base: TemplatedControl,
    subscribed_command: RefCell<Option<Rc<dyn ICommand>>>,
    command_subscription: RefCell<Option<Rc<SerialDisposable>>>,
    disabled_by_command: Cell<bool>,
    setting_enabled: Cell<bool>,
    layout_applied: Cell<bool>,
    owned_values: OwnedItemValues,
}
ferro_class! {RibbonControlBase:TemplatedControl, virtuals RibbonControlBaseImpl:TemplatedControlImpl {
    fn update_can_execute(this);
    fn uses_command_can_execute(this)->bool;
    fn on_ribbon_property_changed(this, change:&ferroui_base::FerroPropertyChangedEventArgs<'_>);
    fn on_layout_applied_core(this, layout:RibbonItemLayout);
    fn create_linked_copy_core(this)->Option<Ref<Control>>;
    fn on_unlinked_core(this);
    fn create_overflow_menu_items_core(this)->RibbonOverflowMenuItems;
    fn on_key_tip_core(this)->Rc<RibbonKeyTipResult>;
    fn invoke_core(this)->bool;
}}
// Abstract: no public parameterless constructor or fake concrete default style.
ferro_class_info!(RibbonControlBase {
 markup:{namespace:"FerroRibbon.Controls",properties:[
  EffectiveSizeDefinition:RibbonSizeDefinition{get:RibbonControlBase::effective_size_definition},
  ShowLabelInSimplifiedMode:bool{get:RibbonControlBase::show_label_in_simplified_mode},
 ],methods:[fn ApplyLayout(RibbonItemLayout)=>|this:&RibbonControlBase,layout|this.apply_layout(layout)]},
 interfaces:[
  Rc<dyn super::IRibbonCommandSource> => |value|Rc::new(CommandSourceView(value)) as Rc<dyn super::IRibbonCommandSource>,
  Rc<dyn super::IRibbonItem> => common::as_item,
  Rc<dyn super::IRibbonItemEditable> => common::as_editable,
  Rc<dyn IRibbonItemInternal> => common::as_internal,
 ]
});
ferro_impl_classes!(RibbonControlBase:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl);

impl ControlImpl for RibbonControlBase {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        instantiate(RibbonNamedItemAutomationPeer {
            base: NoneAutomationPeer::construct(this),
        })
        .upcast()
    }
}

#[repr(C)]
struct RibbonNamedItemAutomationPeer {
    base: NoneAutomationPeer,
}

ferro_class!(RibbonNamedItemAutomationPeer:NoneAutomationPeer);
ferro_class_info!(RibbonNamedItemAutomationPeer {});
ferro_impl_classes!(RibbonNamedItemAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);

impl RibbonNamedItemAutomationPeer {
    fn is_named(&self) -> bool {
        let owner = self.owner();
        AutomationProperties::get_name(&owner).is_some_and(|name| !name.is_empty())
            || AutomationProperties::get_labeled_by(&owner).is_some()
    }
}

// WinUI promotes named composite controls to groups and leaves their children interactive.
impl AutomationPeerImpl for RibbonNamedItemAutomationPeer {
    fn get_automation_control_type_core(this: &Self) -> AutomationControlType {
        if this.is_named() {
            AutomationControlType::Group
        } else {
            Self::parent_get_automation_control_type_core(this)
        }
    }

    fn get_class_name_core(this: &Self) -> String {
        if this.is_named() {
            "NamedContainerAutomationPeer".into()
        } else {
            Self::parent_get_class_name_core(this)
        }
    }

    fn is_control_element_core(this: &Self) -> bool {
        this.is_named()
    }

    fn is_content_element_core(this: &Self) -> bool {
        this.is_named()
    }

    fn is_keyboard_focusable_core(this: &Self) -> bool {
        !this.is_named() && Self::parent_is_keyboard_focusable_core(this)
    }

    fn set_focus_core(this: &Self) {
        if !this.is_named() {
            Self::parent_set_focus_core(this);
        }
    }
}

ferro_properties! {impl RibbonControlBase {
    pub fn command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<RibbonControlBase,_>("Command",None)}
    pub fn command_parameter_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<RibbonControlBase,_>("CommandParameter",None)}
    pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonControlBase,_>("Id",None)}
    pub fn label_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonControlBase,_>("Label",None)}
    pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonControlBase,_>("Icon",None)}
    pub fn large_icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonControlBase,_>("LargeIcon",None)}
    pub fn size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonControlBase,_>("Size",RibbonItemSize::Medium)}
    pub fn size_definition_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonControlBase,_>("SizeDefinition",None)}
    pub fn current_size_property()->StyledProperty<RibbonItemSize>{FerroProperty::register::<RibbonControlBase,_>("CurrentSize",RibbonItemSize::Medium)}
    pub fn is_simplified_property()->StyledProperty<bool>{FerroProperty::register::<RibbonControlBase,_>("IsSimplified",false)}
    pub fn actual_show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonControlBase,_>("ActualShowLabel",true)}
    pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonControlBase,_>("KeyTip",None)}
    pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<RibbonControlBase,_>("ScreenTip",None)}
    pub fn command_id_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonControlBase,_>("CommandId",None)}
    pub fn shortcut_property()->StyledProperty<Option<String>>{FerroProperty::register::<RibbonControlBase,_>("Shortcut",None)}
    pub fn simplified_visibility_property()->StyledProperty<RibbonSimplifiedVisibility>{FerroProperty::register::<RibbonControlBase,_>("SimplifiedVisibility",RibbonSimplifiedVisibility::Auto)}
    pub fn simplified_label_property()->StyledProperty<RibbonSimplifiedLabel>{FerroProperty::register::<RibbonControlBase,_>("SimplifiedLabel",RibbonSimplifiedLabel::Auto)}
    pub fn show_label_property()->StyledProperty<bool>{FerroProperty::register::<RibbonControlBase,_>("ShowLabel",true)}
    pub fn can_add_to_quick_access_property()->StyledProperty<bool>{FerroProperty::register::<RibbonControlBase,_>("CanAddToQuickAccess",true)}
    pub fn metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>>{FerroProperty::register::<RibbonControlBase,_>("Metrics",Some(RibbonMetrics::comfortable()))}
}}
#[derive(Clone, Copy)]
enum CommandRegistration {
    Pending,
    Published,
    CancelledBeforePublication,
}

struct EnabledReset<'a>(&'a Cell<bool>);
impl Drop for EnabledReset<'_> {
    fn drop(&mut self) {
        self.0.set(false)
    }
}
impl RibbonControlBase {
    fn static_constructor() {
        // Source Control descendants take focus by default, and IsTabStop also
        // gates explicit focus. FerroUI separates that flag from Focusable.
        InputElement::focusable_property().override_metadata::<Self>(
            StyledPropertyMetadata::new(Some(true)).with_coerce(|owner, value| {
                value && owner.downcast_ref::<InputElement>().unwrap().is_tab_stop()
            }),
        );
        InputElement::is_tab_stop_property()
            .changed()
            .add_class_handler::<Self>(|this, change| {
                if change.is_effective_value_change() {
                    this.coerce_value(InputElement::focusable_property().as_property());
                }
            })
            .detach();
        Self::register_common_callbacks();
        Self::command_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (_, new) = e.get_old_and_new_value::<Option<Rc<dyn ICommand>>>();
                this.subscribe_command(if this.is_loaded() { new } else { None });
                this.update_can_execute();
            })
            .detach();
        Self::command_parameter_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_can_execute())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            subscribed_command: RefCell::new(None),
            command_subscription: RefCell::new(None),
            disabled_by_command: Cell::new(false),
            setting_enabled: Cell::new(false),
            layout_applied: Cell::new(false),
            owned_values: OwnedItemValues::default(),
        }
    }
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }
    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }
    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }
    fn owns_command_subscription(&self, subscription: &Rc<SerialDisposable>) -> bool {
        self.command_subscription
            .borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, subscription))
    }
    fn subscribe_command(&self, command: Option<Rc<dyn ICommand>>) {
        let current = self.subscribed_command.borrow().clone();
        if current == command {
            return;
        }
        let old = self.command_subscription.borrow().clone();
        if let Some(old) = old {
            old.dispose()
        }
        self.subscribed_command.replace(command.clone());
        self.command_subscription.replace(None);
        if let Some(command) = command {
            let subscription = Rc::new(SerialDisposable::new());
            self.command_subscription
                .replace(Some(subscription.clone()));
            let owner = self.to_ref().downgrade();
            let pending = Rc::downgrade(&subscription);
            let registration = Rc::new(Cell::new(CommandRegistration::Pending));
            let callback_state = registration.clone();
            let handler = Rc::new(move || {
                let Some(this) = owner.upgrade() else { return };
                let deliver = match callback_state.get() {
                    CommandRegistration::Pending => pending
                        .upgrade()
                        .is_some_and(|subscription| this.owns_command_subscription(&subscription)),
                    // A completed registration can already be in an event snapshot
                    // when the command is replaced.
                    CommandRegistration::Published => true,
                    CommandRegistration::CancelledBeforePublication => false,
                };
                if deliver {
                    this.update_can_execute()
                }
            });
            command.register_can_execute_changed(handler, &mut |token| {
                let state = if self.owns_command_subscription(&subscription) {
                    CommandRegistration::Published
                } else {
                    CommandRegistration::CancelledBeforePublication
                };
                registration.set(state);
                subscription.set(Some(token));
            });
            self.update_can_execute();
        }
    }
    fn set_enabled_from_command(&self, value: bool) {
        self.setting_enabled.set(true);
        let _reset = EnabledReset(&self.setting_enabled);
        self.set_is_enabled(value);
    }
    fn on_is_enabled_property_changed(&self) {
        if self.setting_enabled.get() {
            return;
        }
        if !self.is_effectively_enabled() {
            self.disabled_by_command.set(false)
        } else if let Some(command) = self.command()
            && self.uses_command_can_execute()
            && !command.can_execute(self.command_parameter().as_ref())
        {
            self.disabled_by_command.set(true);
            self.set_enabled_from_command(false);
        }
    }
}
impl FerroObjectImpl for RibbonControlBase {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.initialize_common_item();
        this.set_horizontal_alignment(HorizontalAlignment::Left);
        this.set_vertical_alignment(VerticalAlignment::Top);
        this.loaded(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonControlBase>() {
                this.subscribe_command(this.command())
            }
        });
        this.unloaded(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonControlBase>() {
                this.subscribe_command(None)
            }
        });
        let local_enabled = Cell::new(this.is_enabled());
        let local_effective_change = Cell::new(false);
        this.add_property_changed_handler(Rc::new(move |e| {
            let enabled = InputElement::is_enabled_property().as_property();
            let effective = InputElement::is_effectively_enabled_property().as_property();
            if e.is_effective_value_change()
                && (e.property() == enabled || e.property() == effective)
                && let Some(this) = e.sender().downcast_ref::<RibbonControlBase>()
            {
                if e.property() == enabled {
                    local_enabled.set(this.is_enabled());
                    // Complete the local write before delivering both callbacks.
                    // The first can deny the request, and the second then sees
                    // the disabled value and clears command ownership.
                    if local_effective_change.replace(false) {
                        this.on_is_enabled_property_changed();
                    }
                    this.on_is_enabled_property_changed();
                } else if local_enabled.get() == this.is_enabled() {
                    this.on_is_enabled_property_changed();
                } else {
                    // Effective enablement changes inside the local property's
                    // notification, before FerroUI finishes its disabled state.
                    local_effective_change.set(true);
                }
            }
        }));
    }
}
impl RibbonControlBaseImpl for RibbonControlBase {
    fn on_ribbon_property_changed(
        _this: &Self,
        _change: &ferroui_base::FerroPropertyChangedEventArgs<'_>,
    ) {
    }
    fn on_layout_applied_core(_this: &Self, _layout: RibbonItemLayout) {}
    fn create_linked_copy_core(_this: &Self) -> Option<Ref<Control>> {
        None
    }
    fn on_unlinked_core(_this: &Self) {}
    fn create_overflow_menu_items_core(this: &Self) -> RibbonOverflowMenuItems {
        let source = this.to_ref();
        let entry =
            super::ribbon_menu::overflow_entry(this, this.label(), this.icon(), None, move || {
                source.invoke_core();
            });
        ferroribbon_core::model::RibbonItemsSource::from_values(vec![Some(entry)])
    }
    fn on_key_tip_core(this: &Self) -> Rc<RibbonKeyTipResult> {
        this.focus_with(
            ferroui_base::input::NavigationMethod::Tab,
            ferroui_base::input::KeyModifiers::NONE,
        );
        RibbonKeyTipResult::close()
    }
    fn invoke_core(this: &Self) -> bool {
        this.execute_command(None)
            .unwrap_or_else(|error| panic!("{error}"))
    }
    fn update_can_execute(this: &Self) {
        let can_execute = this.command().is_none_or(|command| {
            !this.uses_command_can_execute()
                || command.can_execute(this.command_parameter().as_ref())
        });
        if !can_execute && this.is_effectively_enabled() {
            this.disabled_by_command.set(true);
            this.set_enabled_from_command(false)
        } else if can_execute && this.disabled_by_command.get() {
            this.disabled_by_command.set(false);
            this.set_enabled_from_command(true)
        }
    }
    fn uses_command_can_execute(_this: &Self) -> bool {
        true
    }
}
struct CommandSourceView(Ref<RibbonControlBase>);
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
