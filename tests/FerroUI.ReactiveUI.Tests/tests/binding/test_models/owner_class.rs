//! `OwnerClass` (`TestModels/OwnerClass.cs`).

notifying_model! {
    /// An object with a name (`OwnerClass`).
    pub struct OwnerClass (changing: false, check_equality: true) {
        name / set_name: String = String::new(),
    }
}
