#[path = "../../../tests/support/popup_api_contract.rs"]
mod contract;
#[test]
fn unchanged_source_popup_init_nullable_collections_callbacks_and_finally_order() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!("../../../tests/fixtures/popup-api.jsonl")),
        (214, 2)
    );
}

#[test]
fn resource_iteration_drops_on_unwind_and_preserves_body_panic() {
    use ferroboard_core::layouts::*;
    use std::{cell::RefCell, rc::Rc};
    let trace = Rc::new(RefCell::new(Vec::new()));
    let row = ResourceReadOnlyList::<Key>::from_custom(Rc::new(contract::TraceList {
        data: vec![Some(TextKeyData::char("a".into(), None).unwrap())],
        trace: trace.clone(),
        failure: "none".into(),
        count: None,
        action: None,
    }));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        row.visit::<ResourceTextListError>(|_| std::panic::panic_any("body marker"))
    }));
    assert_eq!(
        result.unwrap_err().downcast_ref::<&str>(),
        Some(&"body marker")
    );
    assert_eq!(*trace.borrow(), ["get", "move:0", "current:0", "dispose"]);
}
