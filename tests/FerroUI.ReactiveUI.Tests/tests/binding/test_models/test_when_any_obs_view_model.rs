//! `TestWhenAnyObsViewModel` (`TestModels/TestWhenAnyObsViewModel.cs`).

use ferroui_system::IObservable;
use std::rc::Rc;

notifying_model! {
    /// A view model whose properties hold observables (`TestWhenAnyObsViewModel`). Upstream skips
    /// a setter given the observable the property holds (`ReferenceEquals`); a dynamic observable
    /// has no equality here, so the setters always notify.
    pub struct TestWhenAnyObsViewModel (changing: false, check_equality: false) {
        command1 / set_command1: Option<Rc<dyn IObservable<i32>>> = None,
        command2 / set_command2: Option<Rc<dyn IObservable<i32>>> = None,
        command3 / set_command3: Option<Rc<dyn IObservable<i32>>> = None,
        changes / set_changes: Option<Rc<dyn IObservable<i32>>> = None,
    }
}
