//! Expressions of composition animations: the variant value type, the
//! expression tree, its parser and the functions callable from expressions.

mod built_in_expression_ffi;
mod delegate_expression_ffi;
mod expression;
mod expression_evaluation_context;
mod expression_parse_exception;
mod expression_parser;
mod expression_tracked_values;
mod expression_variant;
mod token_parser;

pub use built_in_expression_ffi::BuiltInExpressionFfi;
pub use delegate_expression_ffi::DelegateExpressionFfi;
pub use expression::{
    BinaryExpression, ConditionalExpression, ConstantExpression, Expression, ExpressionKeyword, ExpressionKeywords,
    ExpressionType, FunctionCallExpression, KeywordExpression, MemberAccessExpression, ParameterExpression,
    UnaryExpression,
};
pub use expression_evaluation_context::{
    ExpressionEvaluationContext, IExpressionForeignFunctionInterface, IExpressionObject,
    IExpressionParameterCollection,
};
pub use expression_parse_exception::ExpressionParseException;
pub use expression_parser::ExpressionParser;
pub use expression_tracked_values::{ExpressionObjectKey, ExpressionTrackedObjects, ExpressionTrackedObjectsPool};
pub use expression_variant::{ExpressionVariant, ExpressionVariantValue, VariantType};
pub use token_parser::TokenParser;

#[cfg(test)]
mod expression_tests;
