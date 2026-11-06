#[path = "../../../tests/support/selector_api_contract.rs"]
mod contract;
#[test]
fn unchanged_source_selector_null_arms_init_reentrancy_and_nullable_evaluators() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!(
            "../../../tests/fixtures/selector-api.jsonl"
        )),
        (1778, 25)
    );
}

#[test]
fn replacing_selector_arms_releases_old_children_outside_storage_borrows() {
    use ferroboard_core::{layouts::*, text::Utf16Text};
    use std::{
        cell::RefCell,
        rc::{Rc, Weak},
    };
    struct Child {
        parent: Weak<CaseSelector>,
        trace: Rc<RefCell<Vec<i32>>>,
    }
    impl AbstractKeyData for Child {
        fn compute_nullable(
            self: Rc<Self>,
            _: Option<&dyn IKeyEvaluator>,
        ) -> Result<Option<TextKey>, KeyDataError> {
            Ok(None)
        }
        fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
            Ok(None)
        }
    }
    impl Drop for Child {
        fn drop(&mut self) {
            let parent = self.parent.upgrade().unwrap();
            self.trace.borrow_mut().push(
                parent
                    .lower()
                    .unwrap()
                    .compute(&KeyEvaluator::default())
                    .unwrap()
                    .unwrap()
                    .data()
                    .code(),
            );
            parent.set_upper(None);
        }
    }
    let parent = Rc::new(CaseSelector::default());
    let trace: Rc<RefCell<Vec<i32>>> = Rc::default();
    parent.set_upper(Some(TextKeyData::char("z".into(), None).unwrap()));
    parent.set_lower(Some(Rc::new(Child {
        parent: Rc::downgrade(&parent),
        trace: trace.clone(),
    })));
    parent.set_lower(Some(TextKeyData::char("a".into(), None).unwrap()));
    assert_eq!(*trace.borrow(), [97]);
    assert!(parent.upper().is_none());
}
