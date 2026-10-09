//! Binding expressions of the markup runtime.

mod dynamic_resource_expression;

pub(crate) use dynamic_resource_expression::{DynamicResourceAnchor, DynamicResourceExpression, HeldAnchor};

#[cfg(test)]
mod dynamic_resource_expression_tests;
