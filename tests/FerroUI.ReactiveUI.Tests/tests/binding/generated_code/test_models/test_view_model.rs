//! `TestViewModel` (`ReactiveUI.Binding.GeneratedCode.TestModels/TestModels/TestViewModel.cs`).

use std::rc::Rc;

generated_code_model! {
    /// A view model with a name, an age and a nested child that may be absent (`TestViewModel`).
    pub struct TestViewModel (changing: true) {
        name / set_name: String = String::new(),
        age / set_age: i32 = 0,
        child / set_child: Option<Rc<TestViewModel>> = None,
    }
}
