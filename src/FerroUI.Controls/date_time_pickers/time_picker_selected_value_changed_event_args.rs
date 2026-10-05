use ferroui_base::animation::TimeSpan;

/// Defines the argument passed when the `SelectedTime` of a time picker
/// changes.
#[derive(Clone, Debug, PartialEq)]
pub struct TimePickerSelectedValueChangedEventArgs {
    old_time: Option<TimeSpan>,
    new_time: Option<TimeSpan>,
}

impl TimePickerSelectedValueChangedEventArgs {
    /// Creates args for a change of the selected time.
    pub fn new(old: Option<TimeSpan>, new_t: Option<TimeSpan>) -> Self {
        Self { old_time: old, new_time: new_t }
    }

    /// The selected time before the change.
    #[inline]
    pub fn old_time(&self) -> Option<TimeSpan> {
        self.old_time
    }

    /// The selected time after the change.
    #[inline]
    pub fn new_time(&self) -> Option<TimeSpan> {
        self.new_time
    }
}
