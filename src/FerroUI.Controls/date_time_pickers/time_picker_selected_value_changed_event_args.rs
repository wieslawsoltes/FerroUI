use ferroui_base::animation::SignedDuration;

/// Defines the argument passed when the `SelectedTime` of a time picker
/// changes.
#[derive(Clone, Debug, PartialEq)]
pub struct TimePickerSelectedValueChangedEventArgs {
    old_time: Option<SignedDuration>,
    new_time: Option<SignedDuration>,
}

impl TimePickerSelectedValueChangedEventArgs {
    /// Creates args for a change of the selected time.
    pub fn new(old: Option<SignedDuration>, new_t: Option<SignedDuration>) -> Self {
        Self { old_time: old, new_time: new_t }
    }

    /// The selected time before the change.
    #[inline]
    pub fn old_time(&self) -> Option<SignedDuration> {
        self.old_time
    }

    /// The selected time after the change.
    #[inline]
    pub fn new_time(&self) -> Option<SignedDuration> {
        self.new_time
    }
}
