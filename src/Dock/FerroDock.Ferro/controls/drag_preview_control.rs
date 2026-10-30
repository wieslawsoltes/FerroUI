// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::automation::peers::DragPreviewControlAutomationPeer;
use ferrodock_controls_recycling::{ControlRecycling, IControlRecycling};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Control, ControlImpl};
use std::rc::Rc;
/// Original native drag preview with status and optional preview content.
#[repr(C)]
pub struct DragPreviewControl {
    base: TemplatedControl,
}
ferro_class!(DragPreviewControl:TemplatedControl);
ferro_class_info!(DragPreviewControl {
    new: DragPreviewControl::new
});
ferro_impl_classes!(DragPreviewControl:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl);
impl FerroObjectImpl for DragPreviewControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_control_recycling(Some(
            ControlRecycling::as_recycling(ControlRecycling::new()),
        ));
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::status_property().as_property() {
            if let Some(peer) = ControlAutomationPeer::from_element(this)
                .and_then(|p| p.cast::<DragPreviewControlAutomationPeer>())
            {
                peer.notify_status_changed(
                    change.get_old_value::<Option<String>>().flatten(),
                    change.get_new_value::<Option<String>>(),
                );
            }
        }
    }
}
impl ControlImpl for DragPreviewControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DragPreviewControlAutomationPeer::new(this).upcast()
    }
}
ferro_properties! {impl DragPreviewControl {
    /// Identifies the original `ContentTemplate` styled property.
    pub fn content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("ContentTemplate",None)}
    /// Identifies the original `Status` styled property.
    pub fn status_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,Option<String>>("Status",None)}
    /// Identifies the original `ShowContent` styled property.
    pub fn show_content_property()->StyledProperty<bool> {FerroProperty::register::<Self,bool>("ShowContent",false)}
    /// Identifies the original `PreviewContentWidth` styled property.
    pub fn preview_content_width_property()->StyledProperty<f64> {FerroProperty::register::<Self,f64>("PreviewContentWidth",f64::NAN)}
    /// Identifies the original `PreviewContentHeight` styled property.
    pub fn preview_content_height_property()->StyledProperty<f64> {FerroProperty::register::<Self,f64>("PreviewContentHeight",f64::NAN)}
    /// Identifies the original `ControlRecycling` styled property.
    pub fn control_recycling_property()->StyledProperty<Option<Rc<dyn IControlRecycling>>> {FerroProperty::register::<Self,Option<Rc<dyn IControlRecycling>>>("ControlRecycling",None)}
    /// Identifies the original `PreviewContent` styled property.
    pub fn preview_content_property()->StyledProperty<Option<Ref<Control>>> {FerroProperty::register::<Self,Option<Ref<Control>>>("PreviewContent",None)}
}}
impl DragPreviewControl {
    /// Constructs the native class state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
        }
    }
    /// Creates the preview with its own original recycling cache.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets `ContentTemplate`.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }
    /// Sets `ContentTemplate`.
    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value);
    }
    /// Gets `Status`.
    pub fn status(&self) -> Option<String> {
        self.get_value(Self::status_property())
    }
    /// Sets `Status`.
    pub fn set_status(&self, value: Option<String>) {
        self.set_value(Self::status_property(), value);
    }
    /// Gets `ShowContent`.
    pub fn show_content(&self) -> bool {
        self.get_value(Self::show_content_property())
    }
    /// Sets `ShowContent`.
    pub fn set_show_content(&self, value: bool) {
        self.set_value(Self::show_content_property(), value);
    }
    /// Gets `PreviewContentWidth`.
    pub fn preview_content_width(&self) -> f64 {
        self.get_value(Self::preview_content_width_property())
    }
    /// Sets `PreviewContentWidth`.
    pub fn set_preview_content_width(&self, value: f64) {
        self.set_value(Self::preview_content_width_property(), value);
    }
    /// Gets `PreviewContentHeight`.
    pub fn preview_content_height(&self) -> f64 {
        self.get_value(Self::preview_content_height_property())
    }
    /// Sets `PreviewContentHeight`.
    pub fn set_preview_content_height(&self, value: f64) {
        self.set_value(Self::preview_content_height_property(), value);
    }
    /// Gets `ControlRecycling`.
    pub fn control_recycling(&self) -> Option<Rc<dyn IControlRecycling>> {
        self.get_value(Self::control_recycling_property())
    }
    /// Sets `ControlRecycling`.
    pub fn set_control_recycling(&self, value: Option<Rc<dyn IControlRecycling>>) {
        self.set_value(Self::control_recycling_property(), value);
    }
    /// Gets `PreviewContent`.
    pub fn preview_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::preview_content_property())
    }
    /// Sets `PreviewContent`.
    pub fn set_preview_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::preview_content_property(), value);
    }
}
