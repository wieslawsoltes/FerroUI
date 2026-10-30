// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::detach::try_detach_from_parent;
use crate::value_key::ValueKey;
use crate::{from_recycling_value, to_recycling_value};
use ferrodock_controls_recycling_model::{IControlRecycling, IControlRecyclingIdProvider};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue,
    DirectProperty, FerroObject, FerroObjectImpl, FerroProperty, Ref,
};
use ferroui_controls::Control;
use ferroui_system::collections::generic::Dictionary;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A native property object caching and reparenting template-built content.
#[repr(C)]
pub struct ControlRecycling {
    base: FerroObject,
    cache: RefCell<Dictionary<ValueKey, BoxedValue>>,
    try_to_use_id_as_key: Cell<bool>,
    handle: RefCell<Weak<RecyclingHandle>>,
}
ferro_class!(ControlRecycling:FerroObject);
ferro_impl_classes!(ControlRecycling:FerroObjectImpl);
ferro_class_info!(ControlRecycling {new:ControlRecycling::new,
    interfaces:[Rc<dyn IControlRecycling> => ControlRecycling::as_recycling]});
ferro_properties! {impl ControlRecycling {
    /// Identifies whether a content-provided nonblank identifier is a cache key.
    pub fn try_to_use_id_as_key_property()->DirectProperty<ControlRecycling,bool> {
        FerroProperty::register_direct::<ControlRecycling,bool>("TryToUseIdAsKey",
            |o|o.try_to_use_id_as_key(), Some(|o,v|o.set_try_to_use_id_as_key(v)),false)
    }
}}
impl ControlRecycling {
    fn static_constructor() {
        ValueTypes::register_interface_object::<Rc<dyn IControlRecycling>>(|handle| {
            handle
                .as_any()?
                .downcast_ref::<RecyclingHandle>()
                .map(|h| h.0.clone().upcast())
        });
    }
    /// Constructs the source class state for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            cache: RefCell::new(Dictionary::new()),
            try_to_use_id_as_key: Cell::new(false),
            handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates an empty cache, using source objects as keys by default.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets `TryToUseIdAsKey`.
    pub fn try_to_use_id_as_key(&self) -> bool {
        self.try_to_use_id_as_key.get()
    }
    /// Sets `TryToUseIdAsKey` through the native direct property notifications.
    pub fn set_try_to_use_id_as_key(&self, value: bool) {
        self.set_and_raise_cell(
            Self::try_to_use_id_as_key_property(),
            &self.try_to_use_id_as_key,
            value,
        );
    }
    /// Returns a cached object for the exact supplied key. Identifier lookup is
    /// performed by Build/Remove, not by this method or Add.
    pub fn try_get_value(&self, data: Option<&BoxedValue>) -> Option<BoxedValue> {
        self.cache.borrow().get(&ValueKey(data?.clone())).cloned()
    }
    /// Adds or replaces a cached object under the supplied key.
    pub fn add(&self, data: BoxedValue, control: BoxedValue) {
        self.cache.borrow_mut().insert(ValueKey(data), control);
    }
    /// Creates, updates or reparents cached content with the source fallback order.
    pub fn build(
        &self,
        data: Option<&BoxedValue>,
        existing: Option<&BoxedValue>,
        parent: Option<&BoxedValue>,
    ) -> Option<BoxedValue> {
        let data = data?;
        let key = self.get_cache_key(data);
        let parent = parent.and_then(Control::from_boxed);
        if let Some(control) = self.try_get_value(Some(&key)) {
            if let Some(cached) = Control::from_boxed(&control) {
                let mut updated = cached.clone();
                if let Some(parent) = &parent {
                    if let Some(template) = parent.find_data_template(Some(data), None) {
                        if let Some(recycling) = template.as_recycling_data_template() {
                            if let Some(recycled) =
                                recycling.build_with_existing(Some(data), Some(cached.clone()))
                            {
                                updated = recycled;
                            }
                        }
                    }
                }
                if updated != cached {
                    self.add(key.clone(), Control::boxed(updated.clone()));
                }
                if existing.and_then(Control::from_boxed).as_ref() != Some(&updated)
                    && !try_detach_from_parent(&updated)
                {
                    let fallback = Self::build_fallback(parent.as_deref(), Some(data), existing);
                    if let Some(fallback) = &fallback {
                        self.add(key, fallback.clone());
                    }
                    return fallback;
                }
                return Some(Control::boxed(updated));
            }
            return Some(control);
        }
        let template = parent
            .as_ref()
            .and_then(|p| p.find_data_template(Some(data), None));
        let control = if let Some(recycling) = template
            .as_ref()
            .and_then(|t| t.as_recycling_data_template())
        {
            recycling.build_with_existing(Some(data), None)
        } else {
            template.as_ref().and_then(|t| t.build(&Some(data.clone())))
        }?;
        if existing.and_then(Control::from_boxed).as_ref() != Some(&control)
            && !try_detach_from_parent(&control)
        {
            let fallback = Self::build_fallback(parent.as_deref(), Some(data), existing);
            if let Some(fallback) = &fallback {
                self.add(key, fallback.clone());
            }
            return fallback;
        }
        let control = Control::boxed(control);
        self.add(key, control.clone());
        Some(control)
    }
    /// Removes the cache entry selected by data or its enabled identifier.
    pub fn remove(&self, data: Option<&BoxedValue>) -> bool {
        let Some(data) = data else {
            return false;
        };
        let key = self.get_cache_key(data);
        self.cache.borrow_mut().remove(&ValueKey(key)).is_some()
    }
    /// Releases all cached keys and content.
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }
    fn get_cache_key(&self, data: &BoxedValue) -> BoxedValue {
        if self.try_to_use_id_as_key() {
            if let Some(provider) =
                ValueTypes::try_cast(data, ValueType::of::<Rc<dyn IControlRecyclingIdProvider>>())
            {
                if let Some(id) = provider
                    .downcast_ref::<Rc<dyn IControlRecyclingIdProvider>>()
                    .and_then(|p| p.get_control_recycling_id())
                {
                    if !id.chars().all(char::is_whitespace) {
                        return Rc::new(id);
                    }
                }
            }
        }
        data.clone()
    }
    fn build_fallback(
        parent: Option<&Control>,
        data: Option<&BoxedValue>,
        existing: Option<&BoxedValue>,
    ) -> Option<BoxedValue> {
        let parent = parent?;
        let template = parent.find_data_template(data, None)?;
        if let Some(recycling) = template.as_recycling_data_template() {
            let existing = existing.and_then(Control::from_boxed);
            let control = recycling
                .build_with_existing(data, existing.clone())
                .or_else(|| recycling.build_with_existing(data, None))?;
            if Some(&control) == existing.as_ref() || try_detach_from_parent(&control) {
                return Some(Control::boxed(control));
            }
            let rebuilt = recycling.build_with_existing(data, None)?;
            return try_detach_from_parent(&rebuilt).then(|| Control::boxed(rebuilt));
        }
        let control = template.build(&data.cloned())?;
        if try_detach_from_parent(&control) {
            return Some(Control::boxed(control));
        }
        let rebuilt = template.build(&data.cloned())?;
        try_detach_from_parent(&rebuilt).then(|| Control::boxed(rebuilt))
    }
    /// Projects this native object to the UI-independent recycling contract.
    /// Live projections share the same canonical adapter and retain the object.
    pub fn as_recycling(this: Ref<Self>) -> Rc<dyn IControlRecycling> {
        if let Some(handle) = this.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(RecyclingHandle(this.clone()));
        this.handle.replace(Rc::downgrade(&handle));
        handle
    }
}
struct RecyclingHandle(Ref<ControlRecycling>);
impl IControlRecycling for RecyclingHandle {
    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
    fn try_to_use_id_as_key(&self) -> bool {
        self.0.try_to_use_id_as_key()
    }
    fn set_try_to_use_id_as_key(&self, value: bool) {
        self.0.set_try_to_use_id_as_key(value);
    }
    fn try_get_value(&self, data: Option<Rc<dyn Any>>) -> Option<Rc<dyn Any>> {
        self.0
            .try_get_value(data.as_ref().map(from_recycling_value).as_ref())
            .map(to_recycling_value)
    }
    fn add(&self, data: Rc<dyn Any>, control: Rc<dyn Any>) {
        self.0
            .add(from_recycling_value(&data), from_recycling_value(&control));
    }
    fn build(
        &self,
        data: Option<Rc<dyn Any>>,
        existing: Option<Rc<dyn Any>>,
        parent: Option<Rc<dyn Any>>,
    ) -> Option<Rc<dyn Any>> {
        self.0
            .build(
                data.as_ref().map(from_recycling_value).as_ref(),
                existing.as_ref().map(from_recycling_value).as_ref(),
                parent.as_ref().map(from_recycling_value).as_ref(),
            )
            .map(to_recycling_value)
    }
    fn clear(&self) {
        self.0.clear();
    }
}
