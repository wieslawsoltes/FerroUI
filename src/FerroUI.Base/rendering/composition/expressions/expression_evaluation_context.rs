//! The context an expression is evaluated in and the interfaces of the
//! objects it reads from.
//!
//! These traits are the seam between the expression engine and the server
//! side composition objects: a server object implements [`IExpressionObject`],
//! a property set snapshot implements [`IExpressionParameterCollection`].

use super::expression_tracked_values::ExpressionObjectKey;
use super::expression_variant::ExpressionVariant;

/// The values and objects an expression is evaluated against.
#[derive(Clone, Copy, Default)]
pub struct ExpressionEvaluationContext<'a> {
    pub starting_value: ExpressionVariant,
    pub current_value: ExpressionVariant,
    pub final_value: ExpressionVariant,
    /// The object `this.Target` refers to.
    pub target: Option<&'a dyn IExpressionObject>,
    /// The parameters of the animation.
    pub parameters: Option<&'a dyn IExpressionParameterCollection>,
    /// The functions callable from the expression.
    pub foreign_function_interface: Option<&'a dyn IExpressionForeignFunctionInterface>,
}

/// An object whose properties can be read by an expression.
pub trait IExpressionObject {
    /// Gets the value of the property with the given name; `Invalid` when
    /// there is no such property.
    fn get_property(&self, name: &str) -> ExpressionVariant;

    /// The identity of the object, used to track the objects an expression
    /// depends on (see `ExpressionTrackedObjects`). The default is the
    /// address of the object, which is its reference identity for as long as
    /// the object is alive and not moved; an implementation that has a stable
    /// identifier should return that instead.
    fn key(&self) -> ExpressionObjectKey {
        ExpressionObjectKey(self as *const Self as *const () as usize)
    }
}

/// The named parameters of an expression animation.
pub trait IExpressionParameterCollection {
    /// Gets the value of a parameter; `Invalid` when there is no such parameter.
    fn get_parameter(&self, name: &str) -> ExpressionVariant;

    /// Gets the object a parameter refers to, if the parameter is an object.
    fn get_object_parameter(&self, name: &str) -> Option<&dyn IExpressionObject>;
}

/// The functions callable from an expression.
pub trait IExpressionForeignFunctionInterface {
    /// Calls the function with the given name and arguments. Returns `None`
    /// when there is no function with that name accepting the arguments.
    fn call(&self, name: &str, arguments: &[ExpressionVariant]) -> Option<ExpressionVariant>;
}
