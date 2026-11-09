//! Original sealed PopupMapping reference state from Layouts/LayoutModels.cs.
use super::{KeyDataError, PopupVariationMappings, ResourceReadOnlyTextList, ResourceTexts};
use crate::{keys::KeyVariation, text::Utf16Text};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
pub struct PopupMappingValues {
    pub extension_id: Option<Utf16Text>,
    pub id: Option<Utf16Text>,
    pub authors: Option<ResourceReadOnlyTextList>,
    pub mappings: Option<PopupVariationMappings>,
}
impl Default for PopupMappingValues {
    fn default() -> Self {
        Self {
            extension_id: Some("".into()),
            id: Some("".into()),
            authors: Some(ResourceReadOnlyTextList::default()),
            mappings: Some(PopupVariationMappings::default()),
        }
    }
}
pub struct PopupMapping {
    values: RefCell<PopupMappingValues>,
}
impl Default for PopupMapping {
    fn default() -> Self {
        Self::from_values(PopupMappingValues::default())
    }
}
impl PopupMapping {
    pub fn from_values(values: PopupMappingValues) -> Self {
        Self {
            values: RefCell::new(values),
        }
    }
    pub fn extension_id(&self) -> Option<Utf16Text> {
        self.values.borrow().extension_id.clone()
    }
    pub fn id(&self) -> Option<Utf16Text> {
        self.values.borrow().id.clone()
    }
    pub fn authors(&self) -> Option<ResourceReadOnlyTextList> {
        self.values.borrow().authors.clone()
    }
    pub fn mappings(&self) -> Option<PopupVariationMappings> {
        self.values.borrow().mappings.clone()
    }
    pub fn set_extension_id(&self, value: Option<Utf16Text>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().extension_id, value);
        drop(old);
    }
    pub fn set_id(&self, value: Option<Utf16Text>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().id, value);
        drop(old);
    }
    pub fn set_authors(&self, value: Option<ResourceTexts>) {
        self.set_authors_read_only(value.map(Into::into));
    }
    pub fn set_authors_read_only(&self, value: Option<ResourceReadOnlyTextList>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().authors, value);
        drop(old);
    }
    pub fn set_mappings(&self, value: Option<PopupVariationMappings>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().mappings, value);
        drop(old);
    }
    pub fn get(
        &self,
        variation: KeyVariation,
        label: &Utf16Text,
    ) -> Result<Option<Rc<super::PopupSet>>, KeyDataError> {
        self.get_nullable(variation, Some(label))
    }
    pub fn get_nullable(
        &self,
        variation: KeyVariation,
        label: Option<&Utf16Text>,
    ) -> Result<Option<Rc<super::PopupSet>>, KeyDataError> {
        // Source reads Mappings once; a callback can replace the property while its out-map stays live.
        let (found, map) = self
            .mappings()
            .ok_or(super::ResourceTextListError::NullReference)?
            .try_get_value(Some(&variation))?;
        if !found {
            return Ok(None);
        }
        let (found, set) = map
            .ok_or(super::ResourceTextListError::NullReference)?
            .try_get_value(label)?;
        Ok(if found { set } else { None })
    }
    pub fn to_text(&self) -> Utf16Text {
        "FerroBoard.Layouts.PopupMapping".into()
    }
}
impl PartialEq for PopupMapping {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for PopupMapping {}
