//! Useful author-list behavior over a native source; cursor/runtime facets are retired.
use ferroboard_core::{layouts::*, text::*};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
struct Authors {
    mode: String,
    trace: Rc<RefCell<Vec<String>>>,
    replace: Rc<dyn Fn()>,
}
impl PredictionSource for Authors {
    fn len(&self) -> SuggestionResult<usize> {
        self.trace.borrow_mut().push("len".into());
        if self.mode == "count-throw" {
            return Err(SuggestionError::Custom("MarkerException".into()));
        }
        if self.mode == "replace-on-count" {
            (self.replace)();
        }
        Ok(2)
    }
    fn get(&self, index: usize) -> SuggestionResult<PredictionValue> {
        self.trace.borrow_mut().push(format!("get:{index}"));
        if self.mode == "index-throw" {
            return Err(SuggestionError::Custom("MarkerException".into()));
        }
        Ok(match index {
            0 => Some(Utf16Text::from_units(vec![120, 0xd800, 0xd83d, 0xde00])),
            1 => None,
            _ => Some(format!("index:{index}").as_str().into()),
        })
    }
}
fn error(e: SuggestionError) -> Option<String> {
    match e {
        SuggestionError::Custom(message) => Some(message),
        other => panic!("{other:?}"),
    }
}
pub fn verify() {
    let rows: Vec<Value> = include_str!("../fixtures/popup-mapping-api.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut retain = 0;
    let mut indexed = 0;
    let mut count_error = 0;
    for mode in [
        "normal",
        "count-throw",
        "index-throw",
        "enum-throw",
        "null-cursor",
        "move-throw",
        "dispose-throw",
        "replace-on-count",
        "replace-on-dispose",
    ] {
        let owner = Rc::new(PopupMapping::default());
        let weak = Rc::downgrade(&owner);
        let trace = Rc::new(RefCell::new(Vec::new()));
        let original: PredictionList = Rc::new(Authors {
            mode: mode.into(),
            trace: trace.clone(),
            replace: Rc::new(move || weak.upgrade().unwrap().set_authors_read_only(None)),
        });
        owner.set_authors_read_only(Some(ResourceReadOnlyTextList::Custom(original.clone())));
        let held = match owner.authors().unwrap() {
            ResourceReadOnlyTextList::Custom(value) => value,
            _ => panic!("custom source was replaced"),
        };
        let row = rows
            .iter()
            .find(|v| v["tag"] == "retain" && v["mode"] == mode)
            .unwrap();
        assert_eq!(
            json!({"tag":"retain","mode":mode,"same":Rc::ptr_eq(&held,&original),"trace":*trace.borrow()}),
            *row
        );
        retain += 1;
        let len = held.len();
        if mode == "count-throw" {
            let row = rows
                .iter()
                .find(|v| v["tag"] == "operation" && v["mode"] == mode && v["op"] == "Count")
                .unwrap();
            assert_eq!(json!(error(len.unwrap_err())), row["error"]);
            count_error += 1;
        } else {
            assert_eq!(len.unwrap(), 2);
        }
        for row in rows
            .iter()
            .filter(|v| v["tag"] == "operation" && v["mode"] == mode)
        {
            let Some(index) = row["op"]
                .as_str()
                .unwrap()
                .strip_prefix("Item:")
                .and_then(|index| index.parse::<usize>().ok())
            else {
                continue;
            };
            let (value, failure) = match held.get(index) {
                Ok(value) => (json!(value.map(|v| v.units().to_vec())), None),
                Err(e) => (Value::Null, error(e)),
            };
            assert_eq!(value, row["value"], "{mode}/{index}");
            assert_eq!(json!(failure), row["error"], "{mode}/{index}");
            assert_eq!(
                owner.authors().is_none(),
                row["propertyNull"].as_bool().unwrap()
            );
            indexed += 1;
        }
        assert_eq!(trace.borrow()[0], "len");
        owner.set_authors_read_only(None);
    }
    assert_eq!((retain, indexed, count_error), (9, 36, 1));
}
