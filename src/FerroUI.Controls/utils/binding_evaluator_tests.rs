use super::BindingEvaluator;
use crate::test_support::{boxed_str, string_of};

#[test]
fn clear_data_context_sets_data_context_to_null() {
    let evaluator = BindingEvaluator::new();
    evaluator.evaluate(&boxed_str("foo"));
    assert_eq!(Some("foo".to_string()), evaluator.data_context().as_ref().and_then(string_of));

    evaluator.clear_data_context();
    assert!(evaluator.data_context().is_none());
}
