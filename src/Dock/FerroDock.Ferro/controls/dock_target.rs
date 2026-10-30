// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DockTargetBase, DockTargetBaseImpl, IDockAdornerHostProvider};
use ferrodock_model::core::DockOperation;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    AttachedProperty, DirectProperty, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledPropertyOptions, Thickness, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Control, ControlImpl};
use std::cell::Cell;
use std::rc::Rc;
/// Original local docking target and inherited preview-host/theme attached properties.
#[repr(C)]
pub struct DockTarget {
    base: DockTargetBase,
    target_margin: Cell<Thickness>,
}
ferro_class!(DockTarget:DockTargetBase);
ferro_class_info!(DockTarget {
    new: DockTarget::new
});
ferro_impl_classes!(DockTarget:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl DockTargetBaseImpl for DockTarget {
    fn default_dock_operation(_this: &Self) -> DockOperation {
        DockOperation::Window
    }
}
ferro_properties! {impl DockTarget {
    /// Identifies the inherited optional preview-host provider.
    pub fn adorner_host_provider_property()->AttachedProperty<Option<Rc<dyn IDockAdornerHostProvider>>>{FerroProperty::register_attached_with::<DockTarget,Control,_>("AdornerHostProvider",StyledPropertyOptions::new(None).inherits(true))}
    /// Identifies the read-only inset of the hovered surface in the adorner.
    pub fn target_margin_property()->DirectProperty<DockTarget,Thickness>{FerroProperty::register_direct::<DockTarget,_>("TargetMargin",|target|target.target_margin(),None,Thickness::default())}
    /// Identifies the inherited local docking-target theme.
    pub fn target_theme_property()->AttachedProperty<Option<Ref<ControlTheme>>>{FerroProperty::register_attached_with::<DockTarget,Control,_>("TargetTheme",StyledPropertyOptions::new(None).inherits(true))}
}}
impl DockTarget {
    /// Constructs the original base and zero target inset for a derived target.
    pub fn construct() -> Self {
        Self {
            base: DockTargetBase::construct(),
            target_margin: Cell::new(Thickness::default()),
        }
    }
    /// Creates the local dock target.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the local selector/indicator inset within the adorner.
    pub fn target_margin(&self) -> Thickness {
        self.target_margin.get()
    }
    // The original adorner helper is the next caller; keep this scope narrow.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn set_target_margin(&self, value: Thickness) {
        self.set_and_raise_cell(Self::target_margin_property(), &self.target_margin, value);
    }
    /// Gets the inherited optional preview-host provider.
    pub fn get_adorner_host_provider(
        surface: &FerroObject,
    ) -> Option<Rc<dyn IDockAdornerHostProvider>> {
        surface.get_value(Self::adorner_host_provider_property())
    }
    /// Sets the inherited optional preview-host provider.
    pub fn set_adorner_host_provider(
        surface: &FerroObject,
        value: Option<Rc<dyn IDockAdornerHostProvider>>,
    ) {
        surface.set_value(Self::adorner_host_provider_property(), value);
    }
    /// Gets the inherited local docking-target theme.
    pub fn get_target_theme(surface: &FerroObject) -> Option<Ref<ControlTheme>> {
        surface.get_value(Self::target_theme_property())
    }
    /// Sets the inherited local docking-target theme.
    pub fn set_target_theme(surface: &FerroObject, value: Option<Ref<ControlTheme>>) {
        surface.set_value(Self::target_theme_property(), value);
    }
}
