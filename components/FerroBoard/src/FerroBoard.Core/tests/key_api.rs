#[path = "../../../tests/support/key_api_contract.rs"]
mod contract;
#[test]
fn original_key_factories_runtime_init_aliases_copies_and_reentrant_culture() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!("../../../tests/fixtures/key-api.jsonl")),
        (1168, 8, 40)
    );
}
