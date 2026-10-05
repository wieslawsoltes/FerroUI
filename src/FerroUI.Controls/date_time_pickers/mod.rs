//! The date and time pickers: the pickers, the presenters shown in their
//! popups and the looping panel of the selectors.

mod date_picker;
mod date_picker_presenter;
mod date_picker_selected_value_changed_event_args;
mod date_time_picker_panel;
mod picker_presenter_base;
mod time_picker;
mod time_picker_presenter;
mod time_picker_selected_value_changed_event_args;

pub use date_picker::{DatePicker, DatePickerImpl, DatePickerImplExt, DatePickerVTable};
pub use date_picker_presenter::DatePickerPresenter;
pub use date_picker_selected_value_changed_event_args::DatePickerSelectedValueChangedEventArgs;
pub use date_time_picker_panel::{DateTimePickerPanel, DateTimePickerPanelType};
pub use picker_presenter_base::{
    PickerPresenterBase, PickerPresenterBaseImpl, PickerPresenterBaseImplExt, PickerPresenterBaseVTable,
};
pub use time_picker::{TimePicker, TimePickerImpl, TimePickerImplExt, TimePickerVTable};
pub use time_picker_presenter::TimePickerPresenter;
pub use time_picker_selected_value_changed_event_args::TimePickerSelectedValueChangedEventArgs;

#[cfg(test)]
mod date_picker_tests;
#[cfg(test)]
mod time_picker_tests;
