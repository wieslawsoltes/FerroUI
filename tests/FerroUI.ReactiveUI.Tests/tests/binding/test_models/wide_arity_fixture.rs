//! `WideArityFixture` (`TestModels/WideArityFixture.cs`).

notifying_model! {
    /// Sixteen string properties, starting at `"a"`, whose setters always notify, before and after
    /// the change (`WideArityFixture`).
    pub struct WideArityFixture (changing: true, check_equality: false) {
        value1 / set_value1: String = "a".to_string(),
        value2 / set_value2: String = "a".to_string(),
        value3 / set_value3: String = "a".to_string(),
        value4 / set_value4: String = "a".to_string(),
        value5 / set_value5: String = "a".to_string(),
        value6 / set_value6: String = "a".to_string(),
        value7 / set_value7: String = "a".to_string(),
        value8 / set_value8: String = "a".to_string(),
        value9 / set_value9: String = "a".to_string(),
        value10 / set_value10: String = "a".to_string(),
        value11 / set_value11: String = "a".to_string(),
        value12 / set_value12: String = "a".to_string(),
        value13 / set_value13: String = "a".to_string(),
        value14 / set_value14: String = "a".to_string(),
        value15 / set_value15: String = "a".to_string(),
        value16 / set_value16: String = "a".to_string(),
    }
}
