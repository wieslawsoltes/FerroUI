//! Shared resource text storage or an extensible native prediction source.
use super::ResourceTexts;
use crate::text::{PredictionIter, PredictionList, SuggestionError, SuggestionResult, Utf16Text};
use std::rc::Rc;

#[derive(Clone)]
pub enum ResourceReadOnlyTextList {
    Concrete(ResourceTexts),
    Custom(PredictionList),
}
impl Default for ResourceReadOnlyTextList {
    fn default() -> Self {
        ResourceTexts::default().into()
    }
}
impl From<ResourceTexts> for ResourceReadOnlyTextList {
    fn from(value: ResourceTexts) -> Self {
        Self::Concrete(value)
    }
}
impl ResourceReadOnlyTextList {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Concrete(a), Self::Concrete(b)) => a.ptr_eq(b),
            (Self::Custom(a), Self::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
    pub fn as_concrete(&self) -> Option<ResourceTexts> {
        match self {
            Self::Concrete(value) => Some(value.clone()),
            Self::Custom(_) => None,
        }
    }
    pub fn count(&self) -> SuggestionResult<i32> {
        match self {
            Self::Concrete(value) => i32::try_from(value.len())
                .map_err(|_| SuggestionError::Custom("resource text count exceeds i32".into())),
            Self::Custom(value) => i32::try_from(value.len()?)
                .map_err(|_| SuggestionError::Custom("resource text count exceeds i32".into())),
        }
    }
    pub fn get(&self, index: i32) -> SuggestionResult<Option<Utf16Text>> {
        match self {
            Self::Concrete(value) => value
                .get(usize::try_from(index).map_err(|_| {
                    SuggestionError::Collection(super::ResourceTextListError::IndexOutOfRange)
                })?)
                .map_err(SuggestionError::Collection),
            Self::Custom(value) => value.get(usize::try_from(index).map_err(|_| {
                SuggestionError::Collection(super::ResourceTextListError::IndexOutOfRange)
            })?),
        }
    }
    pub fn iter(&self) -> SuggestionResult<PredictionIter<'_>> {
        match self {
            Self::Concrete(value) => Ok(Box::new(value.iter().map(Ok))),
            Self::Custom(value) => value.iter(),
        }
    }
}
