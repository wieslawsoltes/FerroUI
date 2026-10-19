//! `ReactivePropertyVm` (`ReactiveUI.Tests/ReactiveProperties/Mocks/ReactivePropertyVm.cs`).

use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactiveui::{reactive_object, ReactiveObject, ReactiveProperty};
use ferroui_system::threading::tasks::Task;
use std::rc::Rc;

/// The maximum length of [`ReactivePropertyVm::length_less_than_five_property`] (`MaxLength`).
const MAX_LENGTH: usize = 5;

/// Whether a string is null, empty or white space (`string.IsNullOrWhiteSpace`).
fn is_null_or_white_space(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|text| text.trim().is_empty())
}

/// A view model whose reactive properties validate their values (`ReactivePropertyVm`).
///
/// Upstream declares the validation of three properties with DataAnnotations attributes, which
/// `AddValidation` reads by reflection (not ported): `[Required(ErrorMessage = "error!")]` and
/// `[StringLength(5, ErrorMessage = "5over")]`. The mock writes them out as the validators the
/// attributes amount to: a required string is not null, empty or white space, and a string of
/// at most five characters may be null. The properties with the custom messages, which test the
/// formatting of the attributes, are left out with their tests.
pub struct ReactivePropertyVm {
    reactive: ReactiveObject,
    /// `IsRequiredProperty`: `[Required(ErrorMessage = "error!")]`.
    pub is_required_property: ReactiveProperty<Option<String>>,
    /// `LengthLessThanFiveProperty`: `[StringLength(5, ErrorMessage = "5over")]`, then a validator
    /// that requires a value.
    pub length_less_than_five_property: ReactiveProperty<Option<String>>,
    /// `TaskValidationTestProperty`: an asynchronous validator that requires a value.
    pub task_validation_test_property: ReactiveProperty<Option<String>>,
}

reactive_object!(ReactivePropertyVm);

impl Default for ReactivePropertyVm {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactivePropertyVm {
    /// A view model on the immediate scheduler (`ReactivePropertyVm()`).
    pub fn new() -> Self {
        Self::with_scheduler(None)
    }

    /// A view model whose properties notify on `scheduler`, the immediate one when `None`
    /// (`ReactivePropertyVm(ISequencer?)`).
    pub fn with_scheduler(scheduler: Option<Rc<dyn IScheduler>>) -> Self {
        let scheduler = scheduler.unwrap_or_else(ImmediateScheduler::instance);

        let is_required_property = ReactiveProperty::<Option<String>>::with_scheduler(None, Some(scheduler.clone()), false, false)
            .add_validation_error(|value: Option<String>| if is_null_or_white_space(&value) { Some("error!") } else { None });

        let length_less_than_five_property = ReactiveProperty::<Option<String>>::with_scheduler(None, Some(scheduler.clone()), false, false)
            .add_validation_error(|value: Option<String>| match value {
                Some(text) if text.chars().count() > MAX_LENGTH => Some("5over"),
                _ => None,
            })
            .add_validation_error(|value: Option<String>| if is_null_or_white_space(&value) { Some("required") } else { None });

        let task_validation_test_property = ReactiveProperty::<Option<String>>::with_scheduler(None, Some(scheduler), false, false)
            .add_validation_error_async(|value: Option<String>| {
                Task::from_result(if is_null_or_white_space(&value) { Some("required".to_owned()) } else { None })
            });

        Self {
            reactive: ReactiveObject::new(),
            is_required_property,
            length_less_than_five_property,
            task_validation_test_property,
        }
    }
}
