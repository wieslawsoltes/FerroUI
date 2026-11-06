//! Shared suggestion values and fallible native provider iteration.
use super::{Suggestion, SuggestionError, SuggestionResult};
use std::{cell::RefCell, rc::Rc};

pub type SuggestionValue = Option<Rc<Suggestion>>;
pub type SharedSuggestions = Rc<RefCell<Vec<SuggestionValue>>>;
pub type SuggestionIter<'a> = Box<dyn Iterator<Item = SuggestionResult<SuggestionValue>> + 'a>;

/// A live result supplied by a custom provider. Iterators own their cleanup through Drop.
/// No borrow of the list is held while dispatching to caller code.
pub trait SuggestionSource {
    fn len(&self) -> SuggestionResult<usize>;
    fn is_empty(&self) -> SuggestionResult<bool> {
        self.len().map(|len| len == 0)
    }
    fn get(&self, index: usize) -> SuggestionResult<SuggestionValue>;
    fn iter(&self) -> SuggestionResult<SuggestionIter<'_>> {
        Ok(Box::new((0..self.len()?).map(|index| self.get(index))))
    }
}

#[derive(Clone)]
enum Storage {
    Shared(SharedSuggestions),
    Custom(Rc<dyn SuggestionSource>),
}
#[derive(Clone)]
pub struct SuggestionList(Storage);
impl Default for SuggestionList {
    fn default() -> Self {
        Self::from_nullable(Vec::new())
    }
}
impl SuggestionList {
    pub fn new(values: Vec<Rc<Suggestion>>) -> Self {
        Self::from_nullable(values.into_iter().map(Some).collect())
    }
    pub fn from_nullable(values: Vec<SuggestionValue>) -> Self {
        Self::from_shared(Rc::new(RefCell::new(values)))
    }
    pub fn from_shared(values: SharedSuggestions) -> Self {
        Self(Storage::Shared(values))
    }
    pub fn from_custom(values: Rc<dyn SuggestionSource>) -> Self {
        Self(Storage::Custom(values))
    }
    pub fn shared_values(&self) -> Option<SharedSuggestions> {
        match &self.0 {
            Storage::Shared(values) => Some(values.clone()),
            Storage::Custom(_) => None,
        }
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Storage::Shared(a), Storage::Shared(b)) => Rc::ptr_eq(a, b),
            (Storage::Custom(a), Storage::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
    /// Opaque identity of the actual retained allocation; never dereference this pointer.
    pub fn storage_ptr(&self) -> *const () {
        match &self.0 {
            Storage::Shared(values) => Rc::as_ptr(values).cast(),
            Storage::Custom(values) => Rc::as_ptr(values).cast(),
        }
    }
    pub fn len(&self) -> SuggestionResult<usize> {
        match &self.0 {
            Storage::Shared(values) => Ok(values.borrow().len()),
            Storage::Custom(values) => values.len(),
        }
    }
    pub fn is_empty(&self) -> SuggestionResult<bool> {
        self.len().map(|len| len == 0)
    }
    pub fn get(&self, index: usize) -> SuggestionResult<SuggestionValue> {
        match &self.0 {
            Storage::Shared(values) => values
                .borrow()
                .get(index)
                .cloned()
                .ok_or(SuggestionError::SuggestionIndex(index)),
            Storage::Custom(values) => values.get(index),
        }
    }
    /// Shared vectors are snapshotted when iteration begins, releasing the borrow before
    /// any caller callback. Custom sources can supply a lazy fallible iterator.
    pub fn iter(&self) -> SuggestionResult<SuggestionIter<'_>> {
        match &self.0 {
            Storage::Shared(values) => Ok(Box::new(values.borrow().clone().into_iter().map(Ok))),
            Storage::Custom(values) => values.iter(),
        }
    }
    pub fn snapshot(&self) -> SuggestionResult<Vec<SuggestionValue>> {
        self.iter()?.collect()
    }
    pub(crate) fn first(
        &self,
        mut predicate: impl FnMut(SuggestionValue) -> SuggestionResult<bool>,
    ) -> SuggestionResult<SuggestionValue> {
        for value in self.iter()? {
            let value = value?;
            if predicate(value.clone())? {
                return Ok(value);
            }
        }
        Ok(None)
    }
    pub(crate) fn append_to(&self, output: &mut Vec<SuggestionValue>) -> SuggestionResult<()> {
        for value in self.iter()? {
            output.push(value?);
        }
        Ok(())
    }
}
impl FromIterator<Rc<Suggestion>> for SuggestionList {
    fn from_iter<I: IntoIterator<Item = Rc<Suggestion>>>(values: I) -> Self {
        Self::new(values.into_iter().collect())
    }
}
impl FromIterator<SuggestionValue> for SuggestionList {
    fn from_iter<I: IntoIterator<Item = SuggestionValue>>(values: I) -> Self {
        Self::from_nullable(values.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Guard(Rc<Cell<usize>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    struct Source {
        drops: Rc<Cell<usize>>,
        value: Rc<Suggestion>,
    }
    impl SuggestionSource for Source {
        fn len(&self) -> SuggestionResult<usize> {
            Ok(1)
        }
        fn get(&self, index: usize) -> SuggestionResult<SuggestionValue> {
            if index == 0 {
                Ok(Some(self.value.clone()))
            } else {
                Err(SuggestionError::SuggestionIndex(index))
            }
        }
        fn iter(&self) -> SuggestionResult<SuggestionIter<'_>> {
            let guard = Guard(self.drops.clone());
            Ok(Box::new(
                std::iter::once(Ok(Some(self.value.clone()))).inspect(move |_| {
                    let _retain_until_iterator_drop = &guard;
                }),
            ))
        }
    }
    #[test]
    fn native_iterator_cleanup_runs_on_early_match_error_and_unwind() {
        for outcome in 0..3 {
            let drops = Rc::new(Cell::new(0));
            let list = SuggestionList::from_custom(Rc::new(Source {
                drops: drops.clone(),
                value: Rc::new(Suggestion::new(
                    "native".into(),
                    super::super::SuggestionKind::Completion,
                )),
            }));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                list.first(|_| match outcome {
                    0 => Ok(true),
                    1 => Err(SuggestionError::Custom("predicate failed".into())),
                    _ => panic!("predicate panicked"),
                })
            }));
            match outcome {
                0 => assert!(result.unwrap().unwrap().is_some()),
                1 => assert!(matches!(result.unwrap(), Err(SuggestionError::Custom(_)))),
                _ => assert!(result.is_err()),
            }
            assert_eq!(drops.get(), 1);
        }
    }
}
