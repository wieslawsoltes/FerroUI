//! `WhenAnyTestFixture` (`TestModels/WhenAnyTestFixture.cs`).

notifying_model! {
    /// Twelve string properties (`WhenAnyTestFixture`).
    pub struct WhenAnyTestFixture (changing: false, check_equality: true) {
        value1 / set_value1: String = String::new(),
        value2 / set_value2: String = String::new(),
        value3 / set_value3: String = String::new(),
        value4 / set_value4: String = String::new(),
        value5 / set_value5: String = String::new(),
        value6 / set_value6: String = String::new(),
        value7 / set_value7: String = String::new(),
        value8 / set_value8: String = String::new(),
        value9 / set_value9: String = String::new(),
        value10 / set_value10: String = String::new(),
        value11 / set_value11: String = String::new(),
        value12 / set_value12: String = String::new(),
    }
}
