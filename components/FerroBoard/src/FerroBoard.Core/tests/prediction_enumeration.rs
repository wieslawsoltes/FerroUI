//! Native prediction iteration with unchanged source candidate/error/replacement fixtures.
use ferroboard_core::text::*;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/prediction-enumeration.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
struct Probe {
    provider: Weak<DictionarySuggestionProvider>,
    mode: String,
    drops: Rc<Cell<usize>>,
}
impl PredictionSource for Probe {
    fn len(&self) -> SuggestionResult<usize> {
        if self.mode == "indexed-count-error" {
            Err(SuggestionError::Custom("count-error".into()))
        } else {
            Ok(2)
        }
    }
    fn get(&self, index: usize) -> SuggestionResult<PredictionValue> {
        if self.mode == "indexed-index-error" {
            Err(SuggestionError::Custom("index-error".into()))
        } else {
            Ok(if index == 1 {
                None
            } else {
                Some(format!("indexed{index}").as_str().into())
            })
        }
    }
    fn iter(&self) -> SuggestionResult<PredictionIter<'_>> {
        if self.mode.starts_with("indexed") {
            return Ok(Box::new((0..self.len()?).map(|index| self.get(index))));
        }
        if self.mode == "get-error" {
            return Err(SuggestionError::Custom("get-error".into()));
        }
        Ok(Box::new(ProbeIter {
            provider: self.provider.clone(),
            mode: self.mode.clone(),
            drops: self.drops.clone(),
            index: 0,
        }))
    }
}
struct ProbeIter {
    provider: Weak<DictionarySuggestionProvider>,
    mode: String,
    drops: Rc<Cell<usize>>,
    index: usize,
}
impl Iterator for ProbeIter {
    type Item = SuggestionResult<PredictionValue>;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        if index >= 2 {
            return None;
        }
        self.index += 1;
        if self.mode == "move-error" {
            return Some(Err(SuggestionError::Custom("move-error".into())));
        }
        if self.mode == "replace" {
            self.provider
                .upgrade()
                .unwrap()
                .set_default_predictions(None);
        }
        if self.mode == "current-error" {
            return Some(Err(SuggestionError::Custom("current-error".into())));
        }
        Some(Ok(if index == 1 {
            None
        } else {
            Some(format!("item{index}").as_str().into())
        }))
    }
}
impl Drop for ProbeIter {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

#[test]
fn native_prediction_results_match_source_values_limits_errors_and_reentrant_replacement() {
    let mut matched = 0;
    let mut retired = 0;
    for row in rows() {
        let mode = row["mode"].as_str().unwrap_or_default();
        if row["tag"] == "collection"
            || matches!(
                mode,
                "null-enumerator"
                    | "dispose-error"
                    | "both-errors"
                    | "early-dispose-error"
                    | "once-dispose-error"
                    | "indexed-negative-count"
            )
        {
            retired += 1;
            continue;
        }
        let provider = Rc::new(DictionarySuggestionProvider::new_nullable(None, None));
        let drops = Rc::new(Cell::new(0));
        let original: PredictionList = Rc::new(Probe {
            provider: Rc::downgrade(&provider),
            mode: mode.into(),
            drops: drops.clone(),
        });
        provider.set_default_predictions(Some(original.clone()));
        let request = SuggestionRequest::from_values(SuggestionRequestValues {
            max_count: if matches!(mode, "early" | "indexed-early") {
                1
            } else {
                3
            },
            ..Default::default()
        });
        let result = match provider.suggest(&request) {
            Ok(values) => {
                json!({"values":values.snapshot().unwrap().iter().map(|v|v.as_ref().unwrap().text().map(|s|s.units().to_vec())).collect::<Vec<_>>()})
            }
            Err(SuggestionError::Custom(message)) => json!({"error":message}),
            Err(other) => panic!("unexpected {other:?}"),
        };
        assert_eq!(result, row["result"], "{mode}");
        assert_eq!(
            provider
                .default_predictions()
                .as_ref()
                .is_some_and(|v| Rc::ptr_eq(v, &original)),
            row["retained"].as_bool().unwrap(),
            "{mode}"
        );
        if !mode.starts_with("indexed") {
            assert_eq!(drops.get(), usize::from(mode != "get-error"), "{mode}");
        }
        matched += 1;
    }
    assert_eq!((matched, retired), (10, 12));
}
#[test]
fn native_prediction_buffers_retain_identity_utf16_nulls_and_snapshot_values() {
    let values = Rc::new(RefCell::new(vec![
        Some(Utf16Text::from_units(vec![0xd800])),
        None,
    ]));
    let buffer = Rc::new(PredictionListBuffer::from_shared(values.clone()));
    assert!(Rc::ptr_eq(&buffer.shared_values(), &values));
    let original: PredictionList = buffer.clone();
    let any: Rc<dyn std::any::Any> = original.clone();
    assert!(Rc::ptr_eq(
        &any.downcast::<PredictionListBuffer>().unwrap(),
        &buffer
    ));
    let mut snapshot = original.iter().unwrap();
    buffer.set_nullable(0, None).unwrap();
    buffer.push("new".into());
    assert_eq!(original.len().unwrap(), 3);
    assert!(original.get(0).unwrap().is_none());
    assert_eq!(snapshot.next().unwrap().unwrap().unwrap().units(), [0xd800]);
    assert!(snapshot.next().unwrap().unwrap().is_none());
    assert!(snapshot.next().is_none());
    assert!(matches!(
        buffer.get(3),
        Err(SuggestionError::PredictionIndex(3))
    ));
}
#[test]
fn retained_iterators_release_their_own_native_captures_without_a_cursor_registry() {
    let provider = Rc::new(DictionarySuggestionProvider::new_nullable(None, None));
    let drops = Rc::new(Cell::new(0));
    let source: PredictionList = Rc::new(Probe {
        provider: Rc::downgrade(&provider),
        mode: "normal".into(),
        drops: drops.clone(),
    });
    let weak = Rc::downgrade(&source);
    let mut iter = source.iter().unwrap();
    assert!(iter.next().unwrap().unwrap().is_some());
    drop(iter);
    assert_eq!(drops.get(), 1);
    drop(source);
    assert!(weak.upgrade().is_none());
}
