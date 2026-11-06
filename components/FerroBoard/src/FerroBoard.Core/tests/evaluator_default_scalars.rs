#[path = "../../../tests/support/evaluator_default_scalars_contract.rs"]
mod contract;
#[test]
fn shared_default_scalar_init_matches_original_across_joined_workers_and_record_operations() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!(
            "../../../tests/fixtures/evaluator-default-scalars.jsonl"
        )),
        30
    );
}
