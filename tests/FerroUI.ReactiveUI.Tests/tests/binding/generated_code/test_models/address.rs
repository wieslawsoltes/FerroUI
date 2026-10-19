//! `Address` (`ReactiveUI.Binding.GeneratedCode.TestModels/TestModels/Address.cs`).

generated_code_model! {
    /// A postal address, the intermediate object of the deep chains (`Address`).
    pub struct Address (changing: true) {
        street / set_street: String = String::new(),
        city / set_city: String = String::new(),
        zip_code / set_zip_code: String = String::new(),
    }
}
