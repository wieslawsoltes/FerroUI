// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native class views over the original locator delegates and dictionaries.
use crate::DocumentFactoryReference;
use ferrodock_model::core::{
    ContextFactory, ContextLocatorMap, DockableFactory, DockableLocatorMap, HostWindowFactory,
    HostWindowLocatorMap, IDockable, IHostWindow,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{from_markup_value, MarkupDelegate, MarkupType, MarkupTyped},
    BoxedValue,
};
use ferroui_system::collections::generic::Dictionary;
use std::{cell::RefCell, rc::Rc};
/// Native view of the original ContextFactory delegate; clones retain its actual instance.
#[derive(Clone)]
pub struct ContextFactoryReference(ContextFactory);
impl ContextFactoryReference {
    /// Views an existing source delegate without invoking or replacing it.
    pub fn new(model: ContextFactory) -> Self {
        Self(model)
    }
    /// Gets the original shared source delegate.
    pub fn model(&self) -> ContextFactory {
        self.0.clone()
    }
    /// Invokes the original source delegate.
    pub fn invoke(&self) -> Option<BoxedValue> {
        (self.0)()
            .as_ref()
            .map(ferrodock_runtime::from_source_value)
    }
    /// Adapts a native method delegate without invoking it during assignment.
    pub fn from_delegate(delegate: MarkupDelegate) -> Option<Self> {
        if delegate
            .method()
            .is_some_and(|method| !method.parameters.is_empty() || method.return_type.is_none())
        {
            return None;
        }
        Some(Self::new(Rc::new(move || {
            let value = delegate
                .try_invoke(&[])
                .expect("the locator method invocation succeeds");
            crate::factory_metadata::parameter(value)
        })))
    }
}
impl PartialEq for ContextFactoryReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class ContextFactoryReference as "Func"{
 namespace:"System",generic:"Func`1"[BoxedValue],this:ContextFactoryReference,handles:[ContextFactoryReference,Option<ContextFactoryReference>],
 methods:[fn Invoke()->Option<BoxedValue> => ContextFactoryReference::invoke],
});
/// Native view of the original nullable ContextLocatorMap entries.
#[derive(Clone)]
pub struct ContextLocatorMapReference(ContextLocatorMap);
impl ContextLocatorMapReference {
    /// Creates the original dictionary with no entries.
    pub fn create() -> Self {
        Self(Rc::new(RefCell::new(Dictionary::new())))
    }
    /// Views the same source dictionary, retaining all keys, nulls and delegate instances.
    pub fn new(model: ContextLocatorMap) -> Self {
        Self(model)
    }
    /// Gets the same mutable original dictionary.
    pub fn model(&self) -> ContextLocatorMap {
        self.0.clone()
    }
    /// Gets the original Count.
    pub fn count(&self) -> i32 {
        i32::try_from(self.0.borrow().len()).expect("source dictionary Count fits Int32")
    }
    /// Gets an entry; missing keys report the source indexer failure.
    pub fn get(&self, key: String) -> Result<Option<ContextFactoryReference>, String> {
        self.0
            .borrow()
            .get(&key)
            .cloned()
            .map(|value| value.map(ContextFactoryReference::new))
            .ok_or_else(|| format!("The given key '{key}' was not present in the dictionary."))
    }
    /// Replaces an entry while releasing its former delegate outside the dictionary borrow.
    pub fn set(&self, key: String, value: Option<ContextFactoryReference>) {
        let previous = self
            .0
            .borrow_mut()
            .insert(key, value.map(|value| value.model()));
        drop(previous);
    }
    /// Adds an entry, preserving the original duplicate-key failure.
    pub fn add(&self, key: String, value: Option<ContextFactoryReference>) -> Result<(), String> {
        if self.0.borrow().contains_key(&key) {
            return Err("An item with the same key has already been added.".into());
        }
        self.0
            .borrow_mut()
            .add(key, value.map(|value| value.model()));
        Ok(())
    }
    /// Tests source key membership, including entries whose delegate is null.
    pub fn contains_key(&self, key: String) -> bool {
        self.0.borrow().contains_key(&key)
    }
    /// Removes an entry and releases its delegate outside the dictionary borrow.
    pub fn remove(&self, key: String) -> bool {
        let previous = self.0.borrow_mut().remove(&key);
        let removed = previous.is_some();
        drop(previous);
        removed
    }
    /// Clears the same dictionary identity, releasing captured objects outside its borrow.
    pub fn clear(&self) {
        let previous = std::mem::take(&mut *self.0.borrow_mut());
        drop(previous);
    }
}
impl PartialEq for ContextLocatorMapReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class ContextLocatorMapReference as "Dictionary"{
 namespace:"System.Collections.Generic",generic:"Dictionary`2"[String,ContextFactoryReference],this:ContextLocatorMapReference,handles:[ContextLocatorMapReference,Option<ContextLocatorMapReference>],constructors:[()=>ContextLocatorMapReference::create],
 properties:[Count:i32{get:ContextLocatorMapReference::count}],
 indexers:[(String)->Option<ContextFactoryReference>{try_get:ContextLocatorMapReference::get,set:ContextLocatorMapReference::set}],
 methods:[try fn Add(String,Option<ContextFactoryReference>) => ContextLocatorMapReference::add,fn ContainsKey(String)->bool => ContextLocatorMapReference::contains_key,fn Remove(String)->bool => ContextLocatorMapReference::remove,fn Clear() => ContextLocatorMapReference::clear],
});
/// Native view of the original HostWindowFactory delegate; clones retain its actual instance.
#[derive(Clone)]
pub struct HostWindowFactoryReference(HostWindowFactory);
impl HostWindowFactoryReference {
    /// Views an existing source delegate without invoking or replacing it.
    pub fn new(model: HostWindowFactory) -> Self {
        Self(model)
    }
    /// Gets the original shared source delegate.
    pub fn model(&self) -> HostWindowFactory {
        self.0.clone()
    }
    /// Invokes the original source delegate.
    pub fn invoke(&self) -> Option<Rc<dyn IHostWindow>> {
        (self.0)()
    }
    /// Adapts a native method delegate without invoking it during assignment.
    pub fn from_delegate(delegate: MarkupDelegate) -> Option<Self> {
        if delegate
            .method()
            .is_some_and(|method| !method.parameters.is_empty() || method.return_type.is_none())
        {
            return None;
        }
        if delegate.method().is_some_and(|method| {
            !ValueTypes::is_assignable(
                (method.return_type.expect("validated return type"))(),
                ferroui_base::data::core::ValueType::of::<Option<Rc<dyn IHostWindow>>>(),
            )
        }) {
            return None;
        }
        Some(Self::new(Rc::new(move || {
            let value = delegate
                .try_invoke(&[])
                .expect("the locator method invocation succeeds");
            from_markup_value::<Option<Rc<dyn IHostWindow>>>(&value)
                .expect("the locator method returns its declared source type")
        })))
    }
}
impl PartialEq for HostWindowFactoryReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class HostWindowFactoryReference as "Func"{
 namespace:"System",generic:"Func`1"[Rc<dyn IHostWindow>],this:HostWindowFactoryReference,handles:[HostWindowFactoryReference,Option<HostWindowFactoryReference>],
 methods:[fn Invoke()->Option<Rc<dyn IHostWindow>> => HostWindowFactoryReference::invoke],
});
/// Native view of the original nullable HostWindowLocatorMap entries.
#[derive(Clone)]
pub struct HostWindowLocatorMapReference(HostWindowLocatorMap);
impl HostWindowLocatorMapReference {
    /// Creates the original dictionary with no entries.
    pub fn create() -> Self {
        Self(Rc::new(RefCell::new(Dictionary::new())))
    }
    /// Views the same source dictionary, retaining all keys, nulls and delegate instances.
    pub fn new(model: HostWindowLocatorMap) -> Self {
        Self(model)
    }
    /// Gets the same mutable original dictionary.
    pub fn model(&self) -> HostWindowLocatorMap {
        self.0.clone()
    }
    /// Gets the original Count.
    pub fn count(&self) -> i32 {
        i32::try_from(self.0.borrow().len()).expect("source dictionary Count fits Int32")
    }
    /// Gets an entry; missing keys report the source indexer failure.
    pub fn get(&self, key: String) -> Result<Option<HostWindowFactoryReference>, String> {
        self.0
            .borrow()
            .get(&key)
            .cloned()
            .map(|value| value.map(HostWindowFactoryReference::new))
            .ok_or_else(|| format!("The given key '{key}' was not present in the dictionary."))
    }
    /// Replaces an entry while releasing its former delegate outside the dictionary borrow.
    pub fn set(&self, key: String, value: Option<HostWindowFactoryReference>) {
        let previous = self
            .0
            .borrow_mut()
            .insert(key, value.map(|value| value.model()));
        drop(previous);
    }
    /// Adds an entry, preserving the original duplicate-key failure.
    pub fn add(
        &self,
        key: String,
        value: Option<HostWindowFactoryReference>,
    ) -> Result<(), String> {
        if self.0.borrow().contains_key(&key) {
            return Err("An item with the same key has already been added.".into());
        }
        self.0
            .borrow_mut()
            .add(key, value.map(|value| value.model()));
        Ok(())
    }
    /// Tests source key membership, including entries whose delegate is null.
    pub fn contains_key(&self, key: String) -> bool {
        self.0.borrow().contains_key(&key)
    }
    /// Removes an entry and releases its delegate outside the dictionary borrow.
    pub fn remove(&self, key: String) -> bool {
        let previous = self.0.borrow_mut().remove(&key);
        let removed = previous.is_some();
        drop(previous);
        removed
    }
    /// Clears the same dictionary identity, releasing captured objects outside its borrow.
    pub fn clear(&self) {
        let previous = std::mem::take(&mut *self.0.borrow_mut());
        drop(previous);
    }
}
impl PartialEq for HostWindowLocatorMapReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class HostWindowLocatorMapReference as "Dictionary"{
 namespace:"System.Collections.Generic",generic:"Dictionary`2"[String,HostWindowFactoryReference],this:HostWindowLocatorMapReference,handles:[HostWindowLocatorMapReference,Option<HostWindowLocatorMapReference>],constructors:[()=>HostWindowLocatorMapReference::create],
 properties:[Count:i32{get:HostWindowLocatorMapReference::count}],
 indexers:[(String)->Option<HostWindowFactoryReference>{try_get:HostWindowLocatorMapReference::get,set:HostWindowLocatorMapReference::set}],
 methods:[try fn Add(String,Option<HostWindowFactoryReference>) => HostWindowLocatorMapReference::add,fn ContainsKey(String)->bool => HostWindowLocatorMapReference::contains_key,fn Remove(String)->bool => HostWindowLocatorMapReference::remove,fn Clear() => HostWindowLocatorMapReference::clear],
});
/// Native view of the original DockableFactory delegate; clones retain its actual instance.
#[derive(Clone)]
pub struct DockableFactoryReference(DockableFactory);
impl DockableFactoryReference {
    /// Views an existing source delegate without invoking or replacing it.
    pub fn new(model: DockableFactory) -> Self {
        Self(model)
    }
    /// Gets the original shared source delegate.
    pub fn model(&self) -> DockableFactory {
        self.0.clone()
    }
    /// Invokes the original source delegate.
    pub fn invoke(&self) -> Option<Rc<dyn IDockable>> {
        (self.0)()
    }
    /// Adapts a native method delegate without invoking it during assignment.
    pub fn from_delegate(delegate: MarkupDelegate) -> Option<Self> {
        if delegate
            .method()
            .is_some_and(|method| !method.parameters.is_empty() || method.return_type.is_none())
        {
            return None;
        }
        if delegate.method().is_some_and(|method| {
            !ValueTypes::is_assignable(
                (method.return_type.expect("validated return type"))(),
                ferroui_base::data::core::ValueType::of::<Option<Rc<dyn IDockable>>>(),
            )
        }) {
            return None;
        }
        Some(Self::new(Rc::new(move || {
            let value = delegate
                .try_invoke(&[])
                .expect("the locator method invocation succeeds");
            from_markup_value::<Option<Rc<dyn IDockable>>>(&value)
                .expect("the locator method returns its declared source type")
        })))
    }
}
impl PartialEq for DockableFactoryReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class DockableFactoryReference as "Func"{
 namespace:"System",generic:"Func`1"[Rc<dyn IDockable>],this:BoxedValue,handles:[DockableFactoryReference,Option<DockableFactoryReference>,DocumentFactoryReference,Option<DocumentFactoryReference>],
 methods:[try fn Invoke()->Option<Rc<dyn IDockable>> => crate::document_factory_reference::invoke],
});
/// Native view of the original nullable DockableLocatorMap entries.
#[derive(Clone)]
pub struct DockableLocatorMapReference(DockableLocatorMap);
impl DockableLocatorMapReference {
    /// Creates the original dictionary with no entries.
    pub fn create() -> Self {
        Self(Rc::new(RefCell::new(Dictionary::new())))
    }
    /// Views the same source dictionary, retaining all keys, nulls and delegate instances.
    pub fn new(model: DockableLocatorMap) -> Self {
        Self(model)
    }
    /// Gets the same mutable original dictionary.
    pub fn model(&self) -> DockableLocatorMap {
        self.0.clone()
    }
    /// Gets the original Count.
    pub fn count(&self) -> i32 {
        i32::try_from(self.0.borrow().len()).expect("source dictionary Count fits Int32")
    }
    /// Gets an entry; missing keys report the source indexer failure.
    pub fn get(&self, key: String) -> Result<Option<DockableFactoryReference>, String> {
        self.0
            .borrow()
            .get(&key)
            .cloned()
            .map(|value| value.map(DockableFactoryReference::new))
            .ok_or_else(|| format!("The given key '{key}' was not present in the dictionary."))
    }
    /// Replaces an entry while releasing its former delegate outside the dictionary borrow.
    pub fn set(&self, key: String, value: Option<DockableFactoryReference>) {
        let previous = self
            .0
            .borrow_mut()
            .insert(key, value.map(|value| value.model()));
        drop(previous);
    }
    /// Adds an entry, preserving the original duplicate-key failure.
    pub fn add(&self, key: String, value: Option<DockableFactoryReference>) -> Result<(), String> {
        if self.0.borrow().contains_key(&key) {
            return Err("An item with the same key has already been added.".into());
        }
        self.0
            .borrow_mut()
            .add(key, value.map(|value| value.model()));
        Ok(())
    }
    /// Tests source key membership, including entries whose delegate is null.
    pub fn contains_key(&self, key: String) -> bool {
        self.0.borrow().contains_key(&key)
    }
    /// Removes an entry and releases its delegate outside the dictionary borrow.
    pub fn remove(&self, key: String) -> bool {
        let previous = self.0.borrow_mut().remove(&key);
        let removed = previous.is_some();
        drop(previous);
        removed
    }
    /// Clears the same dictionary identity, releasing captured objects outside its borrow.
    pub fn clear(&self) {
        let previous = std::mem::take(&mut *self.0.borrow_mut());
        drop(previous);
    }
}
impl PartialEq for DockableLocatorMapReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class DockableLocatorMapReference as "Dictionary"{
 namespace:"System.Collections.Generic",generic:"Dictionary`2"[String,DockableFactoryReference],this:DockableLocatorMapReference,handles:[DockableLocatorMapReference,Option<DockableLocatorMapReference>],constructors:[()=>DockableLocatorMapReference::create],
 properties:[Count:i32{get:DockableLocatorMapReference::count}],
 indexers:[(String)->Option<DockableFactoryReference>{try_get:DockableLocatorMapReference::get,set:DockableLocatorMapReference::set}],
 methods:[try fn Add(String,Option<DockableFactoryReference>) => DockableLocatorMapReference::add,fn ContainsKey(String)->bool => DockableLocatorMapReference::contains_key,fn Remove(String)->bool => DockableLocatorMapReference::remove,fn Clear() => DockableLocatorMapReference::clear],
});
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(ContextFactoryReference::MARKUP);
    ValueTypes::register_nullable::<ContextFactoryReference>();
    ValueTypes::register_reference_identity::<ContextFactoryReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    MarkupType::register(ContextLocatorMapReference::MARKUP);
    ValueTypes::register_nullable::<ContextLocatorMapReference>();
    ValueTypes::register_reference_identity::<ContextLocatorMapReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_boxed_cast::<MarkupDelegate, ContextFactoryReference>(|value| {
        ContextFactoryReference::from_delegate(value.downcast_ref::<MarkupDelegate>()?.clone())
    });
    MarkupType::register(HostWindowFactoryReference::MARKUP);
    ValueTypes::register_nullable::<HostWindowFactoryReference>();
    ValueTypes::register_reference_identity::<HostWindowFactoryReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    MarkupType::register(HostWindowLocatorMapReference::MARKUP);
    ValueTypes::register_nullable::<HostWindowLocatorMapReference>();
    ValueTypes::register_reference_identity::<HostWindowLocatorMapReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_boxed_cast::<MarkupDelegate, HostWindowFactoryReference>(|value| {
        HostWindowFactoryReference::from_delegate(value.downcast_ref::<MarkupDelegate>()?.clone())
    });
    crate::document_factory_reference::register();
    MarkupType::register(DockableFactoryReference::MARKUP);
    ValueTypes::register_nullable::<DockableFactoryReference>();
    ValueTypes::register_reference_identity::<DockableFactoryReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    MarkupType::register(DockableLocatorMapReference::MARKUP);
    ValueTypes::register_nullable::<DockableLocatorMapReference>();
    ValueTypes::register_reference_identity::<DockableLocatorMapReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_boxed_cast::<MarkupDelegate, DockableFactoryReference>(|value| {
        DockableFactoryReference::from_delegate(value.downcast_ref::<MarkupDelegate>()?.clone())
    });
}
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::ContextFactoryReference,crate::HostWindowFactoryReference,crate::DockableFactoryReference,crate::ContextLocatorMapReference,crate::HostWindowLocatorMapReference,crate::DockableLocatorMapReference],contracts:[],generics:[],generic_contracts:[]}
