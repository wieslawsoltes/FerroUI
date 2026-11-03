#[path = "../../../tests/support/layout_deferred_json_contract.rs"]
mod contract;
#[test]
fn unchanged_source_deferred_json_access_and_layout_algorithms() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!(
            "../../../tests/fixtures/layout-deferred-json.jsonl"
        )),
        1308
    );
}
