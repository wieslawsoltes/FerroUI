//! `ISetMethodBindingConverter` (`ReactiveUI.Binding/Bindings/ISetMethodBindingConverter.cs`).

use crate::ConverterType;
use std::any::Any;
use std::rc::Rc;

/// Sets a value on a binding target in a way of its own, such as filling a collection instead of
/// replacing it (`ISetMethodBindingConverter`).
/// [`SetMethodBindingConverterRegistry`](crate::SetMethodBindingConverterRegistry) picks the one
/// with the highest affinity for a pair of types.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's interface extends `IEnableLogger`; the logger of
// Splat is reached through `ferroui_splat::LogHost` instead.
pub trait ISetMethodBindingConverter {
    /// How well this converter sets a value of `from_type` on a target of `to_type`: a positive
    /// score when it can, where a higher score wins; zero or less when it cannot. Either type may
    /// be unknown (`GetAffinityForObjects`).
    fn get_affinity_for_objects(&self, from_type: Option<ConverterType>, to_type: Option<ConverterType>) -> i32;

    /// Sets `new_value` on `to_target`, with the `arguments` of an indexer if there are any, and
    /// returns the value the target property holds afterwards (`PerformSet`).
    fn perform_set(
        &self,
        to_target: Option<Rc<dyn Any>>,
        new_value: Option<Rc<dyn Any>>,
        arguments: Option<&[Option<Rc<dyn Any>>]>,
    ) -> Option<Rc<dyn Any>>;
}
