//! `NonReactiveINotifyPropertyChangedObject` (`TestModels/NonReactiveINotifyPropertyChangedObject.cs`).

notifying_model! {
    /// An object that raises `PropertyChanged` only (`NonReactiveINotifyPropertyChangedObject`).
    pub struct NonReactiveINotifyPropertyChangedObject (changing: false, check_equality: true) {
        inpc_property / set_inpc_property: String = String::new(),
    }
}
