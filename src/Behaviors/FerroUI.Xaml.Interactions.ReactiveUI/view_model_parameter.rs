//! Not in the reference: what the actions of the crate share in place of the casts of C#
//! (`parameter as IRoutableViewModel`) and of the boxing of their `bool` result.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use ferroui_reactiveui::RoutableViewModel;
use std::rc::Rc;

/// The parameter of an action as a routable view model (`parameter as IRoutableViewModel`): a
/// shared routable view model, a nullable one, or a value the untyped value conversions cast to
/// one (a view model type whose cast is registered). `None` for anything else.
pub(crate) fn routable_view_model_of(parameter: Option<&BoxedValue>) -> Option<RoutableViewModel> {
    let parameter = parameter?;
    if let Some(view_model) = parameter.downcast_ref::<RoutableViewModel>() {
        return Some(view_model.clone());
    }
    if let Some(view_model) = parameter.downcast_ref::<Option<RoutableViewModel>>() {
        return view_model.clone();
    }
    ValueTypes::try_cast(parameter, ValueType::of::<RoutableViewModel>())
        .and_then(|view_model| view_model.downcast_ref::<RoutableViewModel>().cloned())
}

/// The result of an action: whether it requested a navigation (a boxed `bool`, as the
/// reference returns `true` or `false` as an `object`).
pub(crate) fn result(requested: bool) -> Option<BoxedValue> {
    Some(Rc::new(requested) as BoxedValue)
}
