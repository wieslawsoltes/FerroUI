// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::{DockableControlMap, DockableObjectMap, IDockControl, IHostWindow};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Identity-preserving view of the original dockable-to-control dictionary.
#[derive(Clone)]
pub struct DockableControlMapReference(DockableControlMap);
impl DockableControlMapReference {
    /// Views the same dictionary, without copying entries.
    pub fn new(model: DockableControlMap) -> Self {
        Self(model)
    }
    /// Gets the original dictionary and its mutable source API.
    pub fn model(&self) -> DockableControlMap {
        self.0.clone()
    }
}
impl PartialEq for DockableControlMapReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
/// Identity-preserving view of the original dockable-to-object dictionary.
#[derive(Clone)]
pub struct DockableObjectMapReference(DockableObjectMap);
impl DockableObjectMapReference {
    /// Views the same dictionary, without copying entries.
    pub fn new(model: DockableObjectMap) -> Self {
        Self(model)
    }
    /// Gets the original dictionary and its mutable source API.
    pub fn model(&self) -> DockableObjectMap {
        self.0.clone()
    }
}
impl PartialEq for DockableObjectMapReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class DockableControlMapReference as "Dictionary(IDockable,IDockableControl)" {
 namespace:"System.Collections.Generic",this:DockableControlMapReference,handles:[DockableControlMapReference],
 properties:[Count:i32{get:|this:&DockableControlMapReference|i32::try_from(this.0.borrow().len()).expect("source dictionary Count fits Int32")}],
});
ferroui_base::ferro_markup_type!(class DockableObjectMapReference as "Dictionary(IDockable,Object)" {
 namespace:"System.Collections.Generic",this:DockableObjectMapReference,handles:[DockableObjectMapReference],
 properties:[Count:i32{get:|this:&DockableObjectMapReference|i32::try_from(this.0.borrow().len()).expect("source dictionary Count fits Int32")}],
});
/// Descriptor of the actual source IList<IDockControl> handle.
#[doc(hidden)]
pub trait DockControlListMetadata {}
ferroui_base::ferro_markup_type!(interface dyn DockControlListMetadata as "IList(IDockControl)"{
 namespace:"System.Collections.Generic",this:ListHandle<Rc<dyn IDockControl>>,handles:[ListHandle<Rc<dyn IDockControl>>],
 properties:[Count:i32{get:|this:&ListHandle<Rc<dyn IDockControl>>|i32::try_from(this.count()).expect("source list Count fits Int32")}],
});
/// Descriptor of the actual source IList<IHostWindow> handle.
#[doc(hidden)]
pub trait HostWindowListMetadata {}
ferroui_base::ferro_markup_type!(interface dyn HostWindowListMetadata as "IList(IHostWindow)"{
 namespace:"System.Collections.Generic",this:ListHandle<Rc<dyn IHostWindow>>,handles:[ListHandle<Rc<dyn IHostWindow>>],
 properties:[Count:i32{get:|this:&ListHandle<Rc<dyn IHostWindow>>|i32::try_from(this.count()).expect("source list Count fits Int32")}],
});
static DOCK_CONTROLS_METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(|value| {
        value
            .downcast_ref::<ListHandle<Rc<dyn IDockControl>>>()?
            .as_notify_property_changed()
    }),
    ..*<dyn DockControlListMetadata as MarkupTyped>::MARKUP
};
static HOST_WINDOWS_METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(|value| {
        value
            .downcast_ref::<ListHandle<Rc<dyn IHostWindow>>>()?
            .as_notify_property_changed()
    }),
    ..*<dyn HostWindowListMetadata as MarkupTyped>::MARKUP
};
pub(crate) fn register() {
    MarkupType::register(DockableControlMapReference::MARKUP);
    MarkupType::register(DockableObjectMapReference::MARKUP);
    MarkupType::register(&DOCK_CONTROLS_METADATA);
    MarkupType::register(&HOST_WINDOWS_METADATA);
    ferroui_base::__ferro_compiler_metadata! {
     MarkupType::register_rust_paths(&[(&DOCK_CONTROLS_METADATA,"::ferrodock_model_ferro::DockControlListMetadata",true),(&HOST_WINDOWS_METADATA,"::ferrodock_model_ferro::HostWindowListMetadata",true)]);
    }
    ValueTypes::register_reference_identity::<DockableControlMapReference>(|v| {
        Rc::as_ptr(&v.0) as *const ()
    });
    ValueTypes::register_reference_identity::<DockableObjectMapReference>(|v| {
        Rc::as_ptr(&v.0) as *const ()
    });
}
