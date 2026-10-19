//! `DispatchStubViewModel` (`TestModels/DispatchStubViewModel.cs`).
//!
//! The members of the command and interaction bindings (`Run`, `Confirm`, `Signal`) are left out:
//! the tests that use them belong to the command binding.

notifying_model! {
    /// A view model whose setters raise `PropertyChanged` on every write (`DispatchStubViewModel`).
    pub struct DispatchStubViewModel (changing: false, check_equality: false) {
        caption / set_caption: String = "a".to_string(),
        parameter / set_parameter: String = "a".to_string(),
    }
}
