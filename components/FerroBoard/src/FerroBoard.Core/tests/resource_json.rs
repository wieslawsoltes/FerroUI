#[path = "../../../tests/support/resource_json_contract.rs"]
mod contract;
#[test]
fn original_json_utf16_null_extension_and_registry_contract() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!(
            "../../../tests/fixtures/resource-json.jsonl"
        )),
        (452, 0)
    );
}
