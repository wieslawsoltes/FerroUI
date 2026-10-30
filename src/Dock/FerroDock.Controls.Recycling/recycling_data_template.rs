// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::detach::try_detach_from_parent;
use crate::{from_recycling_value, to_recycling_value};
use ferrodock_controls_recycling_model::IControlRecycling;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingMode;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    AttachedProperty, BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
    StyledPropertyOptions,
};
use ferroui_controls::templates::{IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam};
use ferroui_controls::Control;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A native recycling template that reparents controls or delegates to a parent's cache/template.
#[repr(C)]
pub struct ControlRecyclingDataTemplate {
    base: FerroObject,
    handle: RefCell<Weak<TemplateHandle>>,
}
ferro_class!(ControlRecyclingDataTemplate:FerroObject);
ferro_impl_classes!(ControlRecyclingDataTemplate:FerroObjectImpl);
ferro_class_info!(ControlRecyclingDataTemplate {new:ControlRecyclingDataTemplate::new,
    interfaces:[Rc<dyn IDataTemplate> => ControlRecyclingDataTemplate::as_data_template,
        Rc<dyn IRecyclingDataTemplate> => ControlRecyclingDataTemplate::as_recycling_template]});
ferro_properties! {impl ControlRecyclingDataTemplate {
    /// Identifies the inherited, TwoWay cache attached property.
    pub fn control_recycling_property()->AttachedProperty<Option<Rc<dyn IControlRecycling>>> {
        FerroProperty::register_attached_with::<ControlRecyclingDataTemplate,Control,_>("ControlRecycling",
            StyledPropertyOptions::new(None).inherits(true).default_binding_mode(BindingMode::TwoWay))
    }
    /// Identifies the parent used for cache and template lookup.
    pub fn parent_property()->StyledProperty<Option<Ref<Control>>> {
        FerroProperty::register::<ControlRecyclingDataTemplate,_>("Parent",None)
    }
}}
impl ControlRecyclingDataTemplate {
    fn static_constructor() {
        ValueTypes::register_interface_object::<Rc<dyn IDataTemplate>>(|handle| {
            handle
                .as_any()?
                .downcast_ref::<TemplateHandle>()
                .map(|h| h.0.clone().upcast())
        });
        ValueTypes::register_interface_object::<Rc<dyn IRecyclingDataTemplate>>(|handle| {
            handle
                .as_any()?
                .downcast_ref::<TemplateHandle>()
                .map(|h| h.0.clone().upcast())
        });
    }
    /// Constructs native class state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates a template with no parent.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the parent control.
    pub fn parent(&self) -> Option<Ref<Control>> {
        self.get_value(Self::parent_property())
    }
    /// Sets the parent control.
    pub fn set_parent(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::parent_property(), value);
    }
    /// Gets the inherited recycling cache.
    pub fn get_control_recycling(control: &FerroObject) -> Option<Rc<dyn IControlRecycling>> {
        control.get_value(Self::control_recycling_property())
    }
    /// Sets the recycling cache on a native property object.
    pub fn set_control_recycling(control: &FerroObject, value: Option<Rc<dyn IControlRecycling>>) {
        control.set_value(Self::control_recycling_property(), value);
    }
    /// Creates content without an existing control.
    pub fn build(&self, data: Option<&BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(data, None)
    }
    /// Matches all values, including null.
    pub fn match_(&self, _data: Option<&BoxedValue>) -> bool {
        true
    }
    /// Reuses, reparents or creates native content in source branch order.
    pub fn build_with_existing(
        &self,
        data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        if let Some(template) = data.and_then(|data| {
            ValueTypes::try_cast(data, ValueType::of::<Rc<dyn IRecyclingDataTemplate>>())
        }) {
            let template = template
                .downcast_ref::<Rc<dyn IRecyclingDataTemplate>>()
                .unwrap();
            let is_self = template
                .as_any()
                .and_then(|a| a.downcast_ref::<TemplateHandle>())
                .is_some_and(|h| h.0.ptr_eq(&self.to_ref()));
            if !is_self {
                return Self::build_from_recycling_template(&**template, data, existing);
            }
        }
        if let Some(control) = data.and_then(Control::from_boxed) {
            if Some(&control) == existing.as_ref() {
                return Some(control);
            }
            return try_detach_from_parent(&control).then_some(control);
        }
        let parent = self.parent()?;
        if let Some(cache) = Self::get_control_recycling(&parent) {
            let result = cache.build(
                data.cloned().map(to_recycling_value),
                existing.map(|e| to_recycling_value(Control::boxed(e))),
                Some(to_recycling_value(Control::boxed(parent))),
            );
            return result.and_then(|value| Control::from_boxed(&from_recycling_value(&value)));
        }
        let template = parent.find_data_template(data, None)?;
        if let Some(recycling) = template.as_recycling_data_template() {
            return Self::build_from_recycling_template(recycling, data, existing);
        }
        Self::build_from_data_template(&*template, data, existing)
    }
    fn build_from_recycling_template(
        template: &dyn IRecyclingDataTemplate,
        data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        let control = template.build_with_existing(data, existing.clone())?;
        if Some(&control) == existing.as_ref() {
            return Some(control);
        }
        if try_detach_from_parent(&control) {
            return Some(control);
        }
        let rebuilt = template.build_with_existing(data, None)?;
        if Some(&rebuilt) == existing.as_ref() {
            return Some(rebuilt);
        }
        try_detach_from_parent(&rebuilt).then_some(rebuilt)
    }
    fn build_from_data_template(
        template: &dyn IDataTemplate,
        data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        let control = template.build(&data.cloned())?;
        if Some(&control) == existing.as_ref() {
            return Some(control);
        }
        if try_detach_from_parent(&control) {
            return Some(control);
        }
        let rebuilt = template.build(&data.cloned())?;
        if Some(&rebuilt) == existing.as_ref() {
            return Some(rebuilt);
        }
        try_detach_from_parent(&rebuilt).then_some(rebuilt)
    }
    fn handle(this: Ref<Self>) -> Rc<TemplateHandle> {
        if let Some(handle) = this.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(TemplateHandle(this.clone()));
        this.handle.replace(Rc::downgrade(&handle));
        handle
    }
    /// Projects the native template to the shared data-template contract.
    pub fn as_data_template(this: Ref<Self>) -> Rc<dyn IDataTemplate> {
        Self::handle(this)
    }
    /// Projects the same native template to the shared recycling contract.
    pub fn as_recycling_template(this: Ref<Self>) -> Rc<dyn IRecyclingDataTemplate> {
        Self::handle(this)
    }
}
struct TemplateHandle(Ref<ControlRecyclingDataTemplate>);
impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for TemplateHandle {
    fn build(&self, data: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.0.build(data.as_ref())
    }
}
impl IDataTemplate for TemplateHandle {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        self.0.match_(data)
    }
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}
impl IRecyclingDataTemplate for TemplateHandle {
    fn build_with_existing(
        &self,
        data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        self.0.build_with_existing(data, existing)
    }
}
