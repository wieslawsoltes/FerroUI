//! `HostTestFixture` (`TestModels/HostTestFixture.cs`).

use super::{NonObservableTestFixture, TestFixture};
use std::rc::Rc;

notifying_model! {
    /// An object that holds a [`TestFixture`] that may be absent (`HostTestFixture`).
    pub struct HostTestFixture (changing: true, check_equality: true) {
        child / set_child: Option<Rc<TestFixture>> = None,
        some_other_param / set_some_other_param: i32 = 0,
        poco_child / set_poco_child: Option<Rc<NonObservableTestFixture>> = None,
    }
}
