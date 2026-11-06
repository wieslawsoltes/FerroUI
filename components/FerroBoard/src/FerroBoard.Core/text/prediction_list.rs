//! Native default-word sources and snapshot iteration; no managed cursor/LINQ runtime.
use super::{SuggestionError, SuggestionResult, Utf16Text};
use std::{any::Any, cell::RefCell, rc::Rc};

pub type PredictionValue = Option<Utf16Text>;
pub type SharedPredictions = Rc<RefCell<Vec<PredictionValue>>>;
pub type PredictionIter<'a> = Box<dyn Iterator<Item = SuggestionResult<PredictionValue>> + 'a>;
pub trait PredictionSource: Any {
    fn len(&self) -> SuggestionResult<usize>;
    fn is_empty(&self) -> SuggestionResult<bool> {
        self.len().map(|len| len == 0)
    }
    fn get(&self, index: usize) -> SuggestionResult<PredictionValue>;
    fn iter(&self) -> SuggestionResult<PredictionIter<'_>> {
        Ok(Box::new((0..self.len()?).map(|index| self.get(index))))
    }
}
pub type PredictionList = Rc<dyn PredictionSource>;

pub struct PredictionListBuffer {
    values: SharedPredictions,
}
impl PredictionListBuffer {
    pub fn new(values: Vec<Utf16Text>) -> Self {
        Self::from_nullable(values.into_iter().map(Some).collect())
    }
    pub fn from_nullable(values: Vec<PredictionValue>) -> Self {
        Self::from_shared(Rc::new(RefCell::new(values)))
    }
    pub fn from_shared(values: SharedPredictions) -> Self {
        Self { values }
    }
    pub fn shared_values(&self) -> SharedPredictions {
        self.values.clone()
    }
    pub fn set(&self, index: usize, value: Utf16Text) -> SuggestionResult<()> {
        self.set_nullable(index, Some(value))
    }
    pub fn set_nullable(&self, index: usize, value: PredictionValue) -> SuggestionResult<()> {
        let mut values = self.values.borrow_mut();
        let slot = values
            .get_mut(index)
            .ok_or(SuggestionError::PredictionIndex(index))?;
        *slot = value;
        Ok(())
    }
    pub fn push(&self, value: Utf16Text) {
        self.push_nullable(Some(value));
    }
    pub fn push_nullable(&self, value: PredictionValue) {
        self.values.borrow_mut().push(value);
    }
}
impl PredictionSource for PredictionListBuffer {
    fn len(&self) -> SuggestionResult<usize> {
        Ok(self.values.borrow().len())
    }
    fn get(&self, index: usize) -> SuggestionResult<PredictionValue> {
        self.values
            .borrow()
            .get(index)
            .cloned()
            .ok_or(SuggestionError::PredictionIndex(index))
    }
    fn iter(&self) -> SuggestionResult<PredictionIter<'_>> {
        // Snapshot handles before calling custom code; no borrow spans iteration.
        Ok(Box::new(self.values.borrow().clone().into_iter().map(Ok)))
    }
}
pub(crate) fn visit_predictions(
    list: &PredictionList,
    max: i32,
    mut body: impl FnMut(PredictionValue) -> SuggestionResult<()>,
) -> SuggestionResult<()> {
    if max <= 0 {
        return Ok(());
    }
    for word in list.iter()?.take(max as usize) {
        body(word?)?;
    }
    Ok(())
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
        created: Rc<Cell<usize>>,
        drops: Rc<Cell<usize>>,
    }
    impl PredictionSource for Source {
        fn len(&self) -> SuggestionResult<usize> {
            Ok(3)
        }
        fn get(&self, index: usize) -> SuggestionResult<PredictionValue> {
            Ok(Some(format!("word{index}").as_str().into()))
        }
        fn iter(&self) -> SuggestionResult<PredictionIter<'_>> {
            self.created.set(self.created.get() + 1);
            let guard = Guard(self.drops.clone());
            Ok(Box::new((0..3).map(move |index| {
                let _retain_until_drop = &guard;
                if index == 1 {
                    Err(SuggestionError::Custom("iteration failed".into()))
                } else {
                    Ok(Some("word".into()))
                }
            })))
        }
    }
    #[test]
    fn native_prediction_limits_and_result_failures_release_iterators_once() {
        let created = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let source: PredictionList = Rc::new(Source {
            created: created.clone(),
            drops: drops.clone(),
        });
        for limit in [i32::MIN, 0] {
            visit_predictions(&source, limit, |_| panic!("zero limit must not visit")).unwrap();
        }
        assert_eq!((created.get(), drops.get()), (0, 0));
        let visited = Cell::new(0);
        visit_predictions(&source, 1, |_| {
            visited.set(visited.get() + 1);
            Ok(())
        })
        .unwrap();
        assert_eq!((visited.get(), created.get(), drops.get()), (1, 1, 1));
        assert!(
            matches!(visit_predictions(&source,3,|_|Ok(())),Err(SuggestionError::Custom(message)) if message=="iteration failed")
        );
        assert_eq!(drops.get(), 2);
        assert!(
            matches!(visit_predictions(&source,3,|_|Err(SuggestionError::Custom("body failed".into()))),Err(SuggestionError::Custom(message)) if message=="body failed")
        );
        assert_eq!(drops.get(), 3);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            visit_predictions(&source, 3, |_| panic!("body panicked"))
        }));
        assert!(panic.is_err());
        assert_eq!((created.get(), drops.get()), (4, 4));
    }
}
