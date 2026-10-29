//! Application extensions to state transfer across replacement subtrees.
use ferroui_base::{FerroProperty, Ref, StyledElement};

/// Capture application-specific state from a control before replacement.
///
/// Called only for controls with a source name or development key. Return `None`
/// for unsupported controls. Capture must not mutate the live tree. Store values
/// or application identities in the snapshot, not strong references to retired
/// controls. One adapter can support multiple control types.
pub trait ControlStateAdapter {
    fn capture(&self, control: &Ref<StyledElement>) -> Option<Box<dyn CapturedControlState>>;
}

/// State restored only after the complete revision succeeds and layout is updated.
/// Normally runs on the dispatcher; a subsequent reload completes pending state
/// before preparing its candidate. Identity must be unique on both sides and the
/// control's runtime type must be unchanged. Failed/superseded revisions discard
/// the snapshot without invoking this callback.
///
/// Restore must not panic. It is an application callback after commit, not a
/// reversible transaction. Authored values, bindings and new application state
/// take precedence over the snapshot; use the context's property guard and any
/// additional checks required by the custom control. Do not reenter the same
/// session from this callback; post additional reload work to the dispatcher.
pub trait CapturedControlState {
    fn restore(&self, control: &Ref<StyledElement>, context: &ControlStateRestoreContext<'_>);
}

/// Shared by a target and its accepted change. The queued callback and the next
/// session boundary race only in ordering on the UI thread; the action runs once.
#[derive(Clone, Default)]
pub(crate) struct StateRestoration(
    std::rc::Rc<std::cell::RefCell<Option<std::rc::Rc<PendingRestoration>>>>,
);
struct PendingRestoration(std::cell::RefCell<Option<Box<dyn FnOnce()>>>);
impl PendingRestoration {
    fn run(&self) {
        let action = self.0.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
}
impl StateRestoration {
    pub(crate) fn complete(&self) {
        let pending = self.0.borrow_mut().take();
        if let Some(pending) = pending {
            pending.run();
        }
    }
    pub(crate) fn schedule(&self, action: impl FnOnce() + 'static) {
        let pending = std::rc::Rc::new(PendingRestoration(std::cell::RefCell::new(Some(
            Box::new(action),
        ))));
        *self.0.borrow_mut() = Some(pending.clone());
        ferroui_base::threading::Dispatcher::current_dispatcher().post_local(
            move || pending.run(),
            ferroui_base::threading::DispatcherPriority::BEFORE_RENDER,
        );
    }
}

/// Common admission checks for an application's state-restoration callback.
pub struct ControlStateRestoreContext<'a> {
    pub(crate) control: &'a Ref<StyledElement>,
    pub(crate) identity: &'a [String],
    pub(crate) before: &'a str,
    pub(crate) after: &'a str,
    pub(crate) before_name: &'a str,
    pub(crate) name: &'a str,
}
impl ControlStateRestoreContext<'_> {
    /// Namescope-qualified identity within this document/component boundary.
    pub fn identity(&self) -> &[String] {
        self.identity
    }
    /// Rejects changed/removed authored values, markup expressions, existing
    /// property values and local bindings (including direct-property bindings).
    /// This is a necessary guard, not ownership inference for custom storage:
    /// the adapter must also respect application values not represented by the
    /// property system and coupled properties which a setter may update.
    pub fn may_restore_property(&self, property: &FerroProperty) -> bool {
        !self.control.has_local_value_binding(property)
            && !self.control.is_set(property)
            && crate::state::unchanged_unbound(
                self.before,
                self.after,
                self.before_name,
                self.name,
                property.name(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentReloadTarget, HotReloadSession, ReloadError, ReloadStatus, ReloadTarget};
    use ferroui_base::{threading::Dispatcher, BoxedValue};
    use ferroui_controls::{primitives::ToggleButton, CheckBox, ContentControl};
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    use std::{cell::Cell, rc::Rc};

    struct CheckedState(Rc<Cell<usize>>);
    struct CheckedSnapshot(Option<bool>, Rc<Cell<usize>>);
    impl ControlStateAdapter for CheckedState {
        fn capture(&self, control: &Ref<StyledElement>) -> Option<Box<dyn CapturedControlState>> {
            Some(Box::new(CheckedSnapshot(
                control.cast::<CheckBox>()?.is_checked(),
                self.0.clone(),
            )))
        }
    }
    impl CapturedControlState for CheckedSnapshot {
        fn restore(&self, control: &Ref<StyledElement>, context: &ControlStateRestoreContext<'_>) {
            assert_eq!(
                context.identity().last().map(String::as_str),
                Some("name:choice")
            );
            self.1.set(self.1.get() + 1);
            if let Some(source) =
                ferroui_markup_xaml::source_overlay::source("ferres://custom-state/View.xaml")
            {
                let width = control
                    .cast::<ferroui_base::layout::Layoutable>()
                    .unwrap()
                    .width();
                assert!(
                    source.contains(&format!(r#"Width="{width}""#)),
                    "pending state observed another revision's source overlay: {source}",
                );
            }
            if context.may_restore_property(ToggleButton::is_checked_property()) {
                control.cast::<CheckBox>().unwrap().set_is_checked(self.0);
            }
        }
    }
    fn source(attributes: &str) -> String {
        format!(r#"<CheckBox xmlns="https://github.com/ferroui" Name="choice" {attributes}/>"#)
    }
    fn load(source: &str) -> (BoxedValue, Ref<CheckBox>) {
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::new(source),
            None,
        )
        .unwrap();
        let control = Ref::<CheckBox>::from_xaml_object(&value).unwrap();
        (value, control)
    }
    #[test]
    fn custom_state_respects_authored_changes_removal_and_two_way_bindings() {
        let _scope = Dispatcher::unit_test_scope();
        let calls = Rc::new(Cell::new(0));
        let adapters: Vec<Rc<dyn ControlStateAdapter>> = vec![Rc::new(CheckedState(calls.clone()))];
        for (before_source, after_source) in [
            (source(""), source(r#"IsChecked="False""#)),
            (source(r#"IsChecked="False""#), source("")),
            (
                source(""),
                source(r#"IsChecked="{Binding IsChecked, Mode=TwoWay}""#),
            ),
        ] {
            let (before, old) = load(&before_source);
            old.set_is_checked(Some(true));
            let saved =
                crate::state::ViewState::capture_with(Some(&before), &before_source, &adapters);
            let (after, new) = load(&after_source);
            let model = CheckBox::new();
            model.set_is_checked(Some(false));
            new.set_data_context(Some(Rc::new(model.clone())));
            saved.restore(&after, &before_source, &after_source);
            assert_eq!(new.is_checked(), Some(false));
            assert_eq!(model.is_checked(), Some(false));
            if after_source.contains("Binding") {
                new.set_is_checked(Some(true));
                assert_eq!(
                    model.is_checked(),
                    Some(true),
                    "state callback replaced the binding"
                );
            }
        }
        assert_eq!(calls.get(), 3);
    }
    #[test]
    fn callbacks_run_only_for_current_successful_replacements_and_release_old_controls() {
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let calls = Rc::new(Cell::new(0));
        let host = ContentControl::new();
        let uri = "ferres://custom-state/View.xaml";
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), source("")).unwrap();
        let target: Rc<dyn ReloadTarget> = Rc::new(
            ContentReloadTarget::new(
                &host,
                uri.into(),
                vec![],
                RuntimeXamlLoaderConfiguration::default(),
            )
            .with_state_adapter(Rc::new(CheckedState(calls.clone()))),
        );
        session.register_target(&target);
        session.submit(uri.into(), source(""));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Applied
        );
        Dispatcher::ui_thread().run_jobs(None);
        let old = Ref::<CheckBox>::from_xaml_object(&host.content().unwrap()).unwrap();
        old.set_is_checked(Some(true));
        let retired = old.downgrade();
        session.submit(uri.into(), source(r#"Width="101""#));
        assert_eq!(
            session
                .apply_pending_with(|| Err(ReloadError::Failed("rollback".into())))
                .unwrap()
                .status,
            ReloadStatus::Failed
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(calls.get(), 0);
        assert!(old.is_checked().unwrap());
        session.submit(uri.into(), source(r#"Width="102""#));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Applied
        );
        assert_eq!(calls.get(), 0, "restore ran before the dispatcher boundary");
        let rendered = Rc::new(Cell::new(None));
        let observed = rendered.clone();
        let rendering_control =
            Ref::<CheckBox>::from_xaml_object(&host.content().unwrap()).unwrap();
        Dispatcher::ui_thread().post_local(
            move || observed.set(rendering_control.is_checked()),
            ferroui_base::threading::DispatcherPriority::RENDER,
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            rendered.get(),
            Some(true),
            "render observed unrestored input state"
        );
        let current = Ref::<CheckBox>::from_xaml_object(&host.content().unwrap()).unwrap();
        assert_eq!(current.is_checked(), Some(true));
        assert_eq!(calls.get(), 1);
        drop(old);
        assert!(retired.upgrade().is_none());
        for width in [103, 104] {
            session.submit(uri.into(), source(&format!(r#"Width="{width}""#)));
            assert_eq!(
                session.apply_pending().unwrap().status,
                ReloadStatus::Applied
            );
        }
        Dispatcher::ui_thread().run_jobs(None);
        let latest = Ref::<CheckBox>::from_xaml_object(&host.content().unwrap()).unwrap();
        assert_eq!(
            latest.is_checked(),
            Some(true),
            "rapid accepted revisions lost pending state"
        );
        assert_eq!(
            calls.get(),
            3,
            "each accepted snapshot must restore exactly once"
        );
        session.submit(uri.into(), source(r#"Width="105""#));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Applied
        );
        // The application replaces the graph before the queued callback executes.
        // State from the retired graph must not be applied to this new object.
        let replacement = CheckBox::new();
        replacement.set_name(Some("choice".into()));
        host.set_content(Some(Rc::new(replacement.clone())));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(replacement.is_checked(), Some(false));
        assert_eq!(calls.get(), 3);
    }
    #[test]
    fn ambiguous_identities_and_changed_control_types_skip_custom_callbacks() {
        use ferroui_controls::{Control, StackPanel, TextBox};
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types();
        let calls = Rc::new(Cell::new(0));
        let adapters: Vec<Rc<dyn ControlStateAdapter>> = vec![Rc::new(CheckedState(calls.clone()))];
        let tree = |count| -> BoxedValue {
            let panel = StackPanel::new();
            for _ in 0..count {
                let control = CheckBox::new();
                control.set_name(Some("choice".into()));
                panel.children().add(control.upcast::<Control>());
            }
            Rc::new(panel)
        };
        let source = source("");
        for (before_count, after_count) in [(2, 1), (1, 2)] {
            let before = tree(before_count);
            let state = crate::state::ViewState::capture_with(Some(&before), &source, &adapters);
            state.restore(&tree(after_count), &source, &source);
        }
        let before = tree(1);
        let state = crate::state::ViewState::capture_with(Some(&before), &source, &adapters);
        let changed = TextBox::new();
        changed.set_name(Some("choice".into()));
        state.restore(
            &(Rc::new(changed) as BoxedValue),
            &source,
            &source.replace("CheckBox", "TextBox"),
        );
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn panel_callbacks_follow_owned_children_and_skip_external_replacement() {
        use ferroui_controls::{Control, Panel};
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let source = r#"<StackPanel xmlns="https://github.com/ferroui"><CheckBox Name="choice"/><TextBlock Text="one"/></StackPanel>"#;
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::new(source),
            None,
        )
        .unwrap();
        let root = Ref::<Panel>::from_xaml_object(&value).unwrap();
        root.children().to_vec()[0]
            .cast::<CheckBox>()
            .unwrap()
            .set_is_checked(Some(true));
        let uri = "ferres://custom-state/Panel.xaml";
        let calls = Rc::new(Cell::new(0));
        let target: Rc<dyn ReloadTarget> = Rc::new(
            crate::PanelReloadTarget::new(
                &root,
                0..2,
                uri.into(),
                source.into(),
                RuntimeXamlLoaderConfiguration::default(),
            )
            .unwrap()
            .with_state_adapter(Rc::new(CheckedState(calls.clone()))),
        );
        let mut session = HotReloadSession::new();
        session
            .register_document(uri.into(), source.into())
            .unwrap();
        session.register_target(&target);
        for text in ["two", "three", "four"] {
            session.submit(uri.into(), source.replace("one", text));
            let outcome = session.apply_pending().unwrap();
            assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(calls.get(), 3);
        assert_eq!(
            root.children().to_vec()[0]
                .cast::<CheckBox>()
                .unwrap()
                .is_checked(),
            Some(true)
        );
        session.submit(uri.into(), source.replace("one", "five"));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Applied
        );
        let application_child = CheckBox::new();
        application_child.set_name(Some("choice".into()));
        root.children()
            .set(0, application_child.clone().upcast::<Control>());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(application_child.is_checked(), Some(false));
        assert_eq!(
            calls.get(),
            3,
            "callback ran after the application replaced an owned child"
        );
    }
}
