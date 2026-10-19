//! `TestViewModel` (`TestModels/TestViewModel.cs`).

use super::TestAddress;
use std::rc::Rc;

notifying_model! {
    /// A view model with a name, an age and an address (`TestViewModel`).
    pub struct TestViewModel (changing: true, check_equality: true) {
        name / set_name: String = String::new(),
        age / set_age: i32 = 0,
        address / set_address: Option<Rc<TestAddress>> = None,
    }
}
