// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::DockOperation;
use ferroui_base::data::BindingMode;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    AttachedProperty, FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledPropertyOptions,
};
use ferroui_controls::Control;
/// Original native property object owning the docking attached properties.
#[repr(C)]
pub struct DockProperties {
    base: FerroObject,
}
ferro_class!(DockProperties:FerroObject);
ferro_class_info!(DockProperties {
    new: DockProperties::new
});
ferro_impl_classes!(DockProperties:FerroObjectImpl);
ferro_properties! {impl DockProperties {
    /// Identifies the original `IsDockTarget` attached property.
    pub fn is_dock_target_property()->AttachedProperty<bool>{FerroProperty::register_attached_with::<DockProperties,Control,_>("IsDockTarget",StyledPropertyOptions::new(false).inherits(false).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `IsDragArea` attached property.
    pub fn is_drag_area_property()->AttachedProperty<bool>{FerroProperty::register_attached_with::<DockProperties,Control,_>("IsDragArea",StyledPropertyOptions::new(false).inherits(false).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `IsDropArea` attached property.
    pub fn is_drop_area_property()->AttachedProperty<bool>{FerroProperty::register_attached_with::<DockProperties,Control,_>("IsDropArea",StyledPropertyOptions::new(false).inherits(false).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `IsDragEnabled` attached property.
    pub fn is_drag_enabled_property()->AttachedProperty<bool>{FerroProperty::register_attached_with::<DockProperties,Control,_>("IsDragEnabled",StyledPropertyOptions::new(true).inherits(true).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `IsDropEnabled` attached property.
    pub fn is_drop_enabled_property()->AttachedProperty<bool>{FerroProperty::register_attached_with::<DockProperties,Control,_>("IsDropEnabled",StyledPropertyOptions::new(true).inherits(true).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `ShowDockIndicatorOnly` attached property.
    pub fn show_dock_indicator_only_property()->AttachedProperty<bool>{FerroProperty::register_attached_with::<DockProperties,Control,_>("ShowDockIndicatorOnly",StyledPropertyOptions::new(false).inherits(false).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `IndicatorDockOperation` attached property.
    pub fn indicator_dock_operation_property()->AttachedProperty<DockOperation>{FerroProperty::register_attached_with::<DockProperties,Control,_>("IndicatorDockOperation",StyledPropertyOptions::new(DockOperation::Fill).inherits(false).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `DockAdornerHost` attached property.
    pub fn dock_adorner_host_property()->AttachedProperty<Option<Ref<Control>>>{FerroProperty::register_attached_with::<DockProperties,Control,_>("DockAdornerHost",StyledPropertyOptions::new(None).inherits(true).default_binding_mode(BindingMode::TwoWay))}
    /// Identifies the original `DockGroup` attached property.
    pub fn dock_group_property()->AttachedProperty<Option<String>>{FerroProperty::register_attached_with::<DockProperties,Control,_>("DockGroup",StyledPropertyOptions::new(None).inherits(true).default_binding_mode(BindingMode::TwoWay))}
}}
impl DockProperties {
    /// Constructs this or a derived source property owner.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    /// Creates the original property owner.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets `IsDockTarget` on the target object.
    pub fn get_is_dock_target(target: &FerroObject) -> bool {
        target.get_value(Self::is_dock_target_property())
    }
    /// Sets `IsDockTarget` on the target object.
    pub fn set_is_dock_target(target: &FerroObject, value: bool) {
        target.set_value(Self::is_dock_target_property(), value);
    }
    /// Gets `IsDragArea` on the target object.
    pub fn get_is_drag_area(target: &FerroObject) -> bool {
        target.get_value(Self::is_drag_area_property())
    }
    /// Sets `IsDragArea` on the target object.
    pub fn set_is_drag_area(target: &FerroObject, value: bool) {
        target.set_value(Self::is_drag_area_property(), value);
    }
    /// Gets `IsDropArea` on the target object.
    pub fn get_is_drop_area(target: &FerroObject) -> bool {
        target.get_value(Self::is_drop_area_property())
    }
    /// Sets `IsDropArea` on the target object.
    pub fn set_is_drop_area(target: &FerroObject, value: bool) {
        target.set_value(Self::is_drop_area_property(), value);
    }
    /// Gets `IsDragEnabled` on the target object.
    pub fn get_is_drag_enabled(target: &FerroObject) -> bool {
        target.get_value(Self::is_drag_enabled_property())
    }
    /// Sets `IsDragEnabled` on the target object.
    pub fn set_is_drag_enabled(target: &Control, value: bool) {
        target.set_value(Self::is_drag_enabled_property(), value);
    }
    /// Gets `IsDropEnabled` on the target object.
    pub fn get_is_drop_enabled(target: &FerroObject) -> bool {
        target.get_value(Self::is_drop_enabled_property())
    }
    /// Sets `IsDropEnabled` on the target object.
    pub fn set_is_drop_enabled(target: &FerroObject, value: bool) {
        target.set_value(Self::is_drop_enabled_property(), value);
    }
    /// Gets `ShowDockIndicatorOnly` on the target object.
    pub fn get_show_dock_indicator_only(target: &FerroObject) -> bool {
        target.get_value(Self::show_dock_indicator_only_property())
    }
    /// Sets `ShowDockIndicatorOnly` on the target object.
    pub fn set_show_dock_indicator_only(target: &FerroObject, value: bool) {
        target.set_value(Self::show_dock_indicator_only_property(), value);
    }
    /// Gets `IndicatorDockOperation` on the target object.
    pub fn get_indicator_dock_operation(target: &FerroObject) -> DockOperation {
        target.get_value(Self::indicator_dock_operation_property())
    }
    /// Sets `IndicatorDockOperation` on the target object.
    pub fn set_indicator_dock_operation(target: &FerroObject, value: DockOperation) {
        target.set_value(Self::indicator_dock_operation_property(), value);
    }
    /// Gets `DockAdornerHost` on the target object.
    pub fn get_dock_adorner_host(target: &FerroObject) -> Option<Ref<Control>> {
        target.get_value(Self::dock_adorner_host_property())
    }
    /// Sets `DockAdornerHost` on the target object.
    pub fn set_dock_adorner_host(target: &FerroObject, value: Option<Ref<Control>>) {
        target.set_value(Self::dock_adorner_host_property(), value);
    }
    /// Gets `DockGroup` on the target object.
    pub fn get_dock_group(target: &FerroObject) -> Option<String> {
        target.get_value(Self::dock_group_property())
    }
    /// Sets `DockGroup` on the target object.
    pub fn set_dock_group(target: &FerroObject, value: Option<String>) {
        target.set_value(Self::dock_group_property(), value);
    }
}
