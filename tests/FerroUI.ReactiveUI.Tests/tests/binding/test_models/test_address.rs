//! `TestAddress` (`TestModels/TestAddress.cs`).

notifying_model! {
    /// An address (`TestAddress`).
    pub struct TestAddress (changing: false, check_equality: true) {
        city / set_city: String = String::new(),
    }
}
