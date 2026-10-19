//! `TestFixture` (`TestModels/TestFixture.cs`).

notifying_model! {
    /// An object with string properties that raises `PropertyChanging` and `PropertyChanged`
    /// (`TestFixture`).
    pub struct TestFixture (changing: true, check_equality: true) {
        is_not_null_string / set_is_not_null_string: String = "Foo".to_string(),
        is_only_one_word / set_is_only_one_word: String = "Baz".to_string(),
        poco_property / set_poco_property: String = String::new(),
        nullable_int / set_nullable_int: Option<i32> = None,
        uses_expr_raise_set / set_uses_expr_raise_set: String = String::new(),
    }
}
