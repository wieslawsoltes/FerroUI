//! `ObservedValueFixture` (`TestModels/ObservedValueFixture.cs`).

notifying_model! {
    /// An object with one value that can raise its notification without a change
    /// (`ObservedValueFixture`; [`raise_property_changed`](Self::raise_property_changed) is
    /// `RaiseValueChanged`).
    pub struct ObservedValueFixture (changing: false, check_equality: true) {
        value / set_value: String = String::new(),
    }
}

#[allow(dead_code)]
impl ObservedValueFixture {
    /// Raises the notification of `Value` without changing it (`RaiseValueChanged`).
    pub fn raise_value_changed(&self) {
        self.raise_property_changed("Value");
    }
}
