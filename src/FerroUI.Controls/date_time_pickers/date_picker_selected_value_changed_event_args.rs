use ferroui_base::utilities::DateTimeOffset;

/// Defines the argument passed when the `SelectedDate` of a date picker
/// changes.
#[derive(Clone, Debug, PartialEq)]
pub struct DatePickerSelectedValueChangedEventArgs {
    new_date: Option<DateTimeOffset>,
    old_date: Option<DateTimeOffset>,
}

impl DatePickerSelectedValueChangedEventArgs {
    /// Creates args for a change of the selected date.
    pub fn new(old_date: Option<DateTimeOffset>, new_date: Option<DateTimeOffset>) -> Self {
        Self { new_date, old_date }
    }

    /// The selected date after the change.
    #[inline]
    pub fn new_date(&self) -> Option<DateTimeOffset> {
        self.new_date
    }

    /// The selected date before the change.
    #[inline]
    pub fn old_date(&self) -> Option<DateTimeOffset> {
        self.old_date
    }
}
