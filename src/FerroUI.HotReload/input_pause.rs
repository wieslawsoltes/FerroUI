//! Temporary input suppression without replacing application values or bindings.
use ferroui_base::{
    data::BindingPriority, diagnostics::FerroObjectDiagnosticExtensions, input::InputElement,
    reactive::IDisposable,
};
use std::rc::Rc;

/// Disables an input root until dropped, leaving its local value and binding intact.
/// Used during an application's explicit restart handoff. It does not pause timers,
/// commands invoked programmatically, background work or arbitrary model mutation;
/// the application must quiesce those through its own restart callback/protocol.
/// Applications must not concurrently animate IsEnabled during the handoff.
pub struct InputPause(Rc<dyn IDisposable>);
impl InputPause {
    pub fn new(root: &InputElement) -> Result<Self, crate::ReloadError> {
        if root
            .get_diagnostic(InputElement::is_enabled_property())
            .priority()
            == BindingPriority::Animation
        {
            return Err(crate::ReloadError::RestartRequired(
                "Input root already has an animation or pause".into(),
            ));
        }
        Ok(Self(
            root.set_value_with_priority(
                InputElement::is_enabled_property(),
                false,
                BindingPriority::Animation,
            )
            .expect("temporary input value has a disposable contribution"),
        ))
    }
}
impl Drop for InputPause {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{
        reactive::{IObservable, IObserver, LightweightSubject},
        threading::Dispatcher,
    };
    use ferroui_controls::{Border, Button};

    #[test]
    fn pause_preserves_binding_and_exposes_latest_application_value() {
        let _scope = Dispatcher::unit_test_scope();
        let root = Border::new();
        let child = Button::new();
        root.set_child(child.clone());
        let source: LightweightSubject<bool> = LightweightSubject::new();
        let observable: Rc<dyn IObservable<bool>> = Rc::new(source.clone());
        let binding = root.bind(
            InputElement::is_enabled_property(),
            observable,
            BindingPriority::LocalValue,
        );
        source.on_next(true);
        for _ in 0..10 {
            let pause = InputPause::new(&root).unwrap();
            assert!(!root.is_enabled());
            assert!(
                InputPause::new(&root).is_err(),
                "nested pause must not replace the active contribution"
            );
            assert!(!child.is_effectively_enabled());
            source.on_next(false);
            source.on_next(true);
            assert!(!root.is_enabled());
            drop(pause);
            assert!(root.is_enabled());
            assert!(child.is_effectively_enabled());
            assert!(root.has_local_value_binding(InputElement::is_enabled_property()));
        }
        let pause = InputPause::new(&root).unwrap();
        source.on_next(false);
        drop(pause);
        assert!(
            !root.is_enabled(),
            "resume must not restore a stale enabled value"
        );
        binding.dispose();
    }
}
