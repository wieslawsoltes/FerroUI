//! Explicit selected-version UI command adapter; execution stays in the real ReactiveCommand.
use ferroui_base::{BoxedValue, input::ICommand};
use ferroui_system::IDisposable;
use std::rc::Rc;
pub struct NativeCommand {
    original: Rc<dyn ferroui_reactiveui::ICommand>,
}
impl NativeCommand {
    pub fn wrap(original: Rc<dyn ferroui_reactiveui::ICommand>) -> Rc<dyn ICommand> {
        Rc::new(Self { original })
    }
}
impl ICommand for NativeCommand {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        self.original.can_execute(p.map(|p| p.as_ref().as_any()))
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        self.original.execute(p.map(|p| p.as_ref().as_any()));
    }
    fn can_execute_changed(&self, h: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.original.can_execute_changed(h)
    }
}
