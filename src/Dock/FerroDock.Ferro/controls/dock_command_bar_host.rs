// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::automation::peers::DockCommandBarHostAutomationPeer;
use ferrodock_model::{collections::ReadOnlyListHandle, command_bars::DockCommandBarDefinition};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, input::InputElementImpl,
    instantiate, interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{TemplatedControl, TemplatedControlImpl},
    Control, ControlImpl,
};
use std::rc::Rc;
/// Original host for merged command bars.
#[repr(C)]
pub struct DockCommandBarHost {
    base: TemplatedControl,
}
ferro_class!(DockCommandBarHost:TemplatedControl);
ferro_class_info!(DockCommandBarHost {
    new: DockCommandBarHost::new
});
ferro_impl_classes!(DockCommandBarHost:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl);
impl FerroObjectImpl for DockCommandBarHost {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_current_value(Visual::is_visible_property(), false);
    }
}
impl ControlImpl for DockCommandBarHost {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DockCommandBarHostAutomationPeer::new(this).upcast()
    }
}
ferro_properties! {impl DockCommandBarHost {
    /// Identifies `MenuBars`.
    pub fn menu_bars_property()->StyledProperty<Option<ReadOnlyListHandle<Ref<Control>>>> {FerroProperty::register::<Self,Option<ReadOnlyListHandle<Ref<Control>>>>("MenuBars",None)}
    /// Identifies `ToolBars`.
    pub fn tool_bars_property()->StyledProperty<Option<ReadOnlyListHandle<Ref<Control>>>> {FerroProperty::register::<Self,Option<ReadOnlyListHandle<Ref<Control>>>>("ToolBars",None)}
    /// Identifies `RibbonBars`.
    pub fn ribbon_bars_property()->StyledProperty<Option<ReadOnlyListHandle<Ref<Control>>>> {FerroProperty::register::<Self,Option<ReadOnlyListHandle<Ref<Control>>>>("RibbonBars",None)}
    /// Identifies `BaseCommandBars`.
    pub fn base_command_bars_property()->StyledProperty<Option<ReadOnlyListHandle<Rc<DockCommandBarDefinition>>>> {FerroProperty::register::<Self,Option<ReadOnlyListHandle<Rc<DockCommandBarDefinition>>>>("BaseCommandBars",None)}
}}
impl DockCommandBarHost {
    /// Constructs native class state for this or a derived control.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
        }
    }
    /// Creates the originally hidden host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets `MenuBars`.
    pub fn menu_bars(&self) -> Option<ReadOnlyListHandle<Ref<Control>>> {
        self.get_value(Self::menu_bars_property())
    }
    /// Sets `MenuBars`.
    pub fn set_menu_bars(&self, value: Option<ReadOnlyListHandle<Ref<Control>>>) {
        self.set_value(Self::menu_bars_property(), value);
    }
    /// Gets `ToolBars`.
    pub fn tool_bars(&self) -> Option<ReadOnlyListHandle<Ref<Control>>> {
        self.get_value(Self::tool_bars_property())
    }
    /// Sets `ToolBars`.
    pub fn set_tool_bars(&self, value: Option<ReadOnlyListHandle<Ref<Control>>>) {
        self.set_value(Self::tool_bars_property(), value);
    }
    /// Gets `RibbonBars`.
    pub fn ribbon_bars(&self) -> Option<ReadOnlyListHandle<Ref<Control>>> {
        self.get_value(Self::ribbon_bars_property())
    }
    /// Sets `RibbonBars`.
    pub fn set_ribbon_bars(&self, value: Option<ReadOnlyListHandle<Ref<Control>>>) {
        self.set_value(Self::ribbon_bars_property(), value);
    }
    /// Gets `BaseCommandBars`.
    pub fn base_command_bars(&self) -> Option<ReadOnlyListHandle<Rc<DockCommandBarDefinition>>> {
        self.get_value(Self::base_command_bars_property())
    }
    /// Sets `BaseCommandBars`.
    pub fn set_base_command_bars(
        &self,
        value: Option<ReadOnlyListHandle<Rc<DockCommandBarDefinition>>>,
    ) {
        self.set_value(Self::base_command_bars_property(), value);
    }
}
