#[path = "../../../tests/support/engine_text_contract.rs"]
mod contract;
#[test]
fn unchanged_source_engine_text_null_privacy_failure_and_reentrant_contracts() {
    contract::verify::<contract::Core>(include_str!("../../../tests/fixtures/engine-utf16.jsonl"));
}
