use crate::{computed_collection_metadata::*, popup_dictionary_metadata::*};
use ferroboard_core::{layouts::*, text::Utf16Text};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn popup_sequences_read_live_maps_only_when_snapshot_is_requested() {
    crate::register_types();
    let map = ResourceDictionary::<Utf16Text, Rc<PopupSet>>::default();
    let popup = Rc::new(PopupSet::default());
    map.set("a".into(), Some(popup.clone()));
    let view = PopupLabelsDictionary::from_original(map.clone().into());
    let keys = view.keys().unwrap().unwrap();
    map.set("b".into(), None);
    let frozen = keys.snapshot().unwrap();
    assert_eq!(frozen.count(), 2);
    assert_eq!(frozen.get(0).unwrap(), Some("a".to_owned()));
    assert_eq!(frozen.get(1).unwrap(), Some("b".to_owned()));
    map.set("c".into(), None);
    assert_eq!(frozen.count(), 2);
    assert_eq!(keys.snapshot().unwrap().count(), 3);
    let values = view.values().unwrap().unwrap().snapshot().unwrap();
    assert!(Rc::ptr_eq(&values.get(0).unwrap().unwrap(), &popup));
    assert!(values.get(1).unwrap().is_none());
    let entries = view.snapshot().unwrap();
    assert_eq!(entries.get(0).unwrap().key(), Some(&Utf16Text::from("a")));
    assert!(Rc::ptr_eq(
        &entries.get(0).unwrap().value().unwrap(),
        &popup
    ));
    assert!(frozen.get(2).is_err());
}

#[derive(Default)]
struct State {
    events: RefCell<Vec<&'static str>>,
    fail_current: Cell<bool>,
    panic_current: Cell<bool>,
    on_dispose: RefCell<Option<Rc<dyn Fn()>>>,
}
struct Sequence(Rc<State>);
struct Cursor {
    state: Rc<State>,
    step: Cell<usize>,
}
impl ResourceSequence<Utf16Text> for Sequence {
    fn iter(&self) -> Result<ResourceItems<Utf16Text>, ResourceTextListError> {
        self.0.events.borrow_mut().push("get");
        Ok(Box::new(Cursor {
            state: self.0.clone(),
            step: Cell::new(0),
        }))
    }
}
impl Iterator for Cursor {
    type Item = Result<Option<Utf16Text>, ResourceTextListError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.state.events.borrow_mut().push("next");
        let n = self.step.get();
        self.step.set(n + 1);
        if n != 0 {
            return None;
        }
        assert!(!self.state.panic_current.get(), "snapshot source panic");
        Some(if self.state.fail_current.get() {
            Err(ResourceTextListError::NotSupported)
        } else {
            Ok(Some("a".into()))
        })
    }
}
impl Drop for Cursor {
    fn drop(&mut self) {
        self.state.events.borrow_mut().push("drop");
        let callback = self.state.on_dispose.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}
struct Map {
    data: ResourceDictionary<Utf16Text, Rc<PopupSet>>,
    state: Rc<State>,
}
impl IResourceReadOnlyDictionary<Utf16Text, Rc<PopupSet>> for Map {
    fn count(&self) -> Result<i32, ResourceTextListError> {
        self.data.count()
    }
    fn get(&self, key: Option<&Utf16Text>) -> Result<Option<Rc<PopupSet>>, ResourceTextListError> {
        self.data.get(key)
    }
    fn contains_key(&self, key: Option<&Utf16Text>) -> Result<bool, ResourceTextListError> {
        self.data.contains_key(key)
    }
    fn try_get_value(
        &self,
        key: Option<&Utf16Text>,
    ) -> Result<(bool, Option<Rc<PopupSet>>), ResourceTextListError> {
        self.data.try_get_value(key)
    }
    fn keys(&self) -> Result<Option<ResourceEnumerable<Utf16Text>>, ResourceTextListError> {
        Ok(Some(Rc::new(Sequence(self.state.clone()))))
    }
    fn values(&self) -> Result<Option<ResourceEnumerable<Rc<PopupSet>>>, ResourceTextListError> {
        Ok(Some(self.data.values()))
    }
    fn iter(
        &self,
    ) -> Result<
        ResourceItems<ResourceDictionaryEntry<Utf16Text, Rc<PopupSet>>>,
        ResourceTextListError,
    > {
        self.data.iter()
    }
}
#[test]
fn popup_snapshot_cleanup_preserves_reentrancy_failure_precedence_and_original_panics() {
    crate::register_types();
    let state = Rc::new(State::default());
    let map = Rc::new(Map {
        data: ResourceDictionary::default(),
        state: state.clone(),
    });
    map.data.set("a".into(), None);
    let weak = Rc::downgrade(&map);
    state.on_dispose.replace(Some(Rc::new(move || {
        assert!(
            weak.upgrade()
                .unwrap()
                .contains_key(Some(&"a".into()))
                .unwrap()
        )
    })));
    let view = PopupLabelsDictionary::from_original(ResourceReadOnlyDictionary::from_custom(map));
    let keys = view.keys().unwrap().unwrap();
    assert!(state.events.borrow().is_empty());
    assert_eq!(keys.snapshot().unwrap().count(), 1);
    assert_eq!(*state.events.borrow(), ["get", "next", "next", "drop"]);
    state.events.borrow_mut().clear();
    state.fail_current.set(true);
    assert!(
        format!("{:?}", keys.snapshot().err().unwrap())
            .contains("resource operation is unsupported")
    );
    assert_eq!(*state.events.borrow(), ["get", "next", "drop"]);
    state.fail_current.set(false);
    state.panic_current.set(true);
    state.events.borrow_mut().clear();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| keys.snapshot()))
        .err()
        .unwrap();
    assert!(
        panic
            .downcast_ref::<String>()
            .is_some_and(|s| s.contains("snapshot source panic"))
            || panic
                .downcast_ref::<&str>()
                .is_some_and(|s| s.contains("snapshot source panic"))
    );
    assert_eq!(*state.events.borrow(), ["get", "next", "drop"]);
}

#[test]
fn computed_queries_stay_lazy_with_independent_iterators_live_rows_and_nullable_results() {
    crate::register_types();
    let computer = LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()));
    let board = computer
        .compute(
            ferroboard_core::keys::KeyboardMode::Characters,
            std::sync::Arc::new(Subtype::default()),
            false,
        )
        .unwrap();
    board
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    let rows = ComputedRowsReadOnlyList::from_rows(board.rows());
    let first = board.rows().get(0).unwrap();
    let key = first.get(0).unwrap();
    let query = ComputedKeyQuery::from_query(board.keys());
    let count = query.snapshot().unwrap().count().unwrap();
    let mut a = query.iter();
    let mut b = query.iter();
    let first_a = a.next().unwrap().unwrap().unwrap();
    let first_b = b.next().unwrap().unwrap().unwrap();
    assert!(Rc::ptr_eq(&first_a, &first_b));
    first.push_nullable(None);
    assert_eq!(query.snapshot().unwrap().count().unwrap(), count + 1);
    assert!(
        rows.get(0)
            .unwrap()
            .unwrap()
            .get(first.len() as i32 - 1)
            .unwrap()
            .is_none()
    );
    assert!(
        ComputedKeyQuery::from_query(board.visible_keys())
            .snapshot()
            .is_err()
    );
    first.remove(first.len() - 1).unwrap();
    first.push(key.clone());
    let snapshot = query.snapshot().unwrap();
    assert_eq!(snapshot.count().unwrap(), count + 1);
    assert!(Rc::ptr_eq(&snapshot.get(0).unwrap().unwrap(), &key));
    board.rows().push_nullable(None);
    assert!(rows.get(board.rows().len() as i32 - 1).unwrap().is_none());
    assert!(query.snapshot().is_err());
}
