//! `HostTestFixture` (`ReactiveUI.Binding.GeneratedCode.TestModels/TestModels/HostTestFixture.cs`).

use super::TestViewModel;
use std::rc::Rc;

generated_code_model! {
    /// An object that holds a [`TestViewModel`] that may be absent, for the deep chains through a
    /// nullable intermediate (`HostTestFixture`).
    pub struct HostTestFixture (changing: true) {
        child / set_child: Option<Rc<TestViewModel>> = None,
    }
}
