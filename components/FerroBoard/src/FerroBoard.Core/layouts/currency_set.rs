//! Sealed source CurrencySet reference state from Layouts/LayoutModels.cs.
use super::{KeyDataError, ResourceReadOnlyList, TextKey};
use crate::{keys::KeyCode, text::Utf16Text};
use std::cell::RefCell;
#[derive(Clone)]
pub struct CurrencySetValues {
    pub extension_id: Option<Utf16Text>,
    pub id: Option<Utf16Text>,
    pub label: Option<Utf16Text>,
    pub slots: Option<ResourceReadOnlyList<TextKey>>,
}
impl Default for CurrencySetValues {
    fn default() -> Self {
        Self {
            extension_id: Some("".into()),
            id: None,
            label: Some("".into()),
            slots: None,
        }
    }
}
pub struct CurrencySet {
    values: RefCell<CurrencySetValues>,
}
impl Default for CurrencySet {
    fn default() -> Self {
        Self::from_values(CurrencySetValues::default())
    }
}
impl CurrencySet {
    pub fn from_values(values: CurrencySetValues) -> Self {
        Self {
            values: RefCell::new(values),
        }
    }
    pub fn new(id: Utf16Text, slots: impl Into<ResourceReadOnlyList<TextKey>>) -> Self {
        Self::from_values(CurrencySetValues {
            id: Some(id),
            slots: Some(slots.into()),
            ..Default::default()
        })
    }
    pub fn extension_id(&self) -> Option<Utf16Text> {
        self.values.borrow().extension_id.clone()
    }
    pub fn id(&self) -> Option<Utf16Text> {
        self.values.borrow().id.clone()
    }
    pub fn label(&self) -> Option<Utf16Text> {
        self.values.borrow().label.clone()
    }
    pub fn slots(&self) -> Option<ResourceReadOnlyList<TextKey>> {
        self.values.borrow().slots.clone()
    }
    pub fn set_extension_id(&self, value: Option<Utf16Text>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().extension_id, value);
        drop(old);
    }
    pub fn set_id(&self, value: Option<Utf16Text>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().id, value);
        drop(old);
    }
    pub fn set_label(&self, value: Option<Utf16Text>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().label, value);
        drop(old);
    }
    pub fn set_slots(&self, value: Option<ResourceReadOnlyList<TextKey>>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().slots, value);
        drop(old);
    }
    pub fn get_slot(&self, code: i32) -> Result<Option<TextKey>, KeyDataError> {
        let index = KeyCode::CurrencySlot1.wrapping_sub(code);
        if index >= 0 && index < self.slots().ok_or(KeyDataError::NullReference)?.count()? {
            // Source reads Slots again after Count: a callback may replace it, even with null.
            Ok(self
                .slots()
                .ok_or(KeyDataError::NullReference)?
                .get(index)?)
        } else {
            Ok(None)
        }
    }
    pub fn to_text(&self) -> Utf16Text {
        "FerroBoard.Layouts.CurrencySet".into()
    }
}
impl PartialEq for CurrencySet {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for CurrencySet {}
