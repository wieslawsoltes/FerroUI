//! A view of the ORIGINAL settings list, not replacement storage or an observable collection.
use ferroboard_core::settings::SubtypeList;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
pub struct SettingsSubtypeList(SubtypeList);
impl PartialEq for SettingsSubtypeList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Default for SettingsSubtypeList {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(Vec::new())))
    }
}
impl SettingsSubtypeList {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_original(list: SubtypeList) -> Self {
        Self(list)
    }
    pub fn original(&self) -> SubtypeList {
        self.0.clone()
    }
    pub fn count(&self) -> i32 {
        i32::try_from(self.0.borrow().len()).expect("source List.Count exceeds i32")
    }
    pub fn add(&self, value: Option<String>) {
        self.0.borrow_mut().push(value);
    }
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
    pub fn get(&self, index: i32) -> Result<Option<String>, &'static str> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.0.borrow().get(i).cloned())
            .ok_or("index was out of range")
    }
    pub fn set(&self, index: i32, value: Option<String>) -> Result<(), &'static str> {
        let index = usize::try_from(index).map_err(|_| "index was out of range")?;
        let mut list = self.0.borrow_mut();
        let slot = list.get_mut(index).ok_or("index was out of range")?;
        *slot = value;
        Ok(())
    }
    pub fn insert(&self, index: i32, value: Option<String>) -> Result<(), &'static str> {
        let index = usize::try_from(index).map_err(|_| "index was out of range")?;
        let mut list = self.0.borrow_mut();
        if index > list.len() {
            return Err("index was out of range");
        }
        list.insert(index, value);
        Ok(())
    }
    pub fn remove_at(&self, index: i32) -> Result<(), &'static str> {
        let index = usize::try_from(index).map_err(|_| "index was out of range")?;
        let mut list = self.0.borrow_mut();
        if index >= list.len() {
            return Err("index was out of range");
        }
        list.remove(index);
        Ok(())
    }
}
ferroui_base::ferro_markup_type!(class SettingsSubtypeList as "SubtypeList" {
    namespace:"FerroBoard.Settings",this:SettingsSubtypeList,
    handles:[SettingsSubtypeList,Option<SettingsSubtypeList>],
    constructors:[()=>SettingsSubtypeList::new],
    properties:[Count:i32{get:SettingsSubtypeList::count}],
    indexers:[(i32)->Option<String>{try_get:SettingsSubtypeList::get,try_set:SettingsSubtypeList::set}],
    methods:[fn Add(Option<String>)=>SettingsSubtypeList::add,fn Clear()=>SettingsSubtypeList::clear,try fn Insert(i32,Option<String>)=>SettingsSubtypeList::insert,try fn RemoveAt(i32)=>SettingsSubtypeList::remove_at],
});
