use crate::data::core::SinkRef;
use crate::data::BindingPriority;
use crate::property_store::{ImmediateValueFrame, IValueEntry};
use crate::reactive::IDisposable;
use crate::{FerroObject, FerroProperty};
use std::any::Any;
use std::rc::Weak;

/// The base of binding expressions: an instantiated binding on a target
/// object. It is also the value entry through which the target's value store
/// reads the binding's value.
#[allow(private_bounds, private_interfaces)]
pub trait BindingExpressionBase: IDisposable + IValueEntry + Any {
    /// The priority of the binding expression.
    fn priority(&self) -> BindingPriority;

    /// The property that the binding expression is targeting.
    fn target_property(&self) -> Option<&'static FerroProperty>;

    /// The preferred priority of the binding expression; the priority passed
    /// to [`attach`](Self::attach) may differ if the binding targets a direct
    /// property, in which case it is elevated to local value priority.
    fn default_priority(&self) -> BindingPriority;

    /// Whether data validation is enabled for the binding expression.
    fn is_data_validation_enabled(&self) -> bool;

    /// Sends the current binding target value to the binding source property
    /// in two-way or one-way-to-source bindings. Does nothing for other
    /// modes. If the update source trigger of the binding is explicit, this
    /// must be called for changes to propagate back to the source.
    fn update_source(&self) {}

    /// Forces a data transfer from the binding source to the binding target.
    fn update_target(&self) {}

    /// Attaches the binding expression to a sink but does not start it.
    #[doc(hidden)]
    fn attach(
        &self,
        sink: SinkRef,
        frame: Option<Weak<ImmediateValueFrame>>,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        priority: BindingPriority,
    );

    /// Starts the binding expression. `produce_value` indicates whether it
    /// should produce an initial value.
    #[doc(hidden)]
    fn start(&self, produce_value: bool);

    /// The expression as an untyped binding expression, if it is one.
    #[doc(hidden)]
    fn as_untyped(&self) -> Option<&dyn crate::data::core::UntypedBindingExpression> {
        None
    }

    fn as_any(&self) -> &dyn Any;
}

/// Handles compare by identity (reference equality), so that the expression
/// a binding returns can be held in untyped values.
impl PartialEq for dyn BindingExpressionBase {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
