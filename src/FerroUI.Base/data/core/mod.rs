//! The binding engine: binding expressions, expression nodes, plugins and
//! parsers.

pub mod expression_nodes;
pub mod parsers;
pub mod plugins;

mod binding_error;
mod binding_expression;
mod clr_property_info;
mod common_property_names;
mod expression_parse_exception;
mod i_binding_expression_sink;
mod i_property_info;
mod indexer_binding_expression;
mod multi_binding_expression;
mod target_type_converter;
mod typed_binding_expression;
mod untyped_binding_expression_base;
mod untyped_observable_binding_expression;
mod value_type;
mod weak_value;

pub use binding_error::ExpressionError;
pub use binding_expression::{BindingExpression, BindingExpressionOptions};
pub use common_property_names::INDEXER_NAME;
pub use clr_property_info::{BoxedPropertyGetter, BoxedPropertySetter, ClrPropertyInfo, FalliblePropertyGetter, Maybe, ModelRef, PropertyGetter, PropertyKind, PropertySetter, Untyped, Value};
pub use expression_parse_exception::ExpressionParseException;
pub(crate) use i_binding_expression_sink::IBindingExpressionSink;
pub use i_binding_expression_sink::SinkRef;
pub use i_property_info::IPropertyInfo;
pub use indexer_binding_expression::IndexerBindingExpression;
pub use multi_binding_expression::MultiBindingExpression;
pub use target_type_converter::TargetTypeConverter;
pub use typed_binding_expression::{TypedBindingExpression, TypedClrPropertyInfo};
pub use untyped_observable_binding_expression::UntypedObservableBindingExpression;
pub(crate) use untyped_binding_expression_base::impl_untyped_binding_expression;
pub use untyped_binding_expression_base::{
    ObservableSink, Publish, UntypedBindingExpression, UntypedBindingExpressionBase,
};
pub(crate) use value_type::value_type_id;
pub use value_type::{ValueType, ValueTypes};
pub use weak_value::WeakValue;
