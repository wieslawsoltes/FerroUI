#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::{
    OnScreenKeyboard,
    input::{PasswordBoxInputTarget, TextBoxInputTarget},
};
use ferroboard_core::editor::{EnterAction, ITextInputTarget};
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::{Control, StackPanel, TextBox, Window};
use std::{cell::Cell, rc::Rc};

struct Fields {
    window: Ref<Window>,
    panel: Ref<StackPanel>,
    first: Ref<TextBox>,
    second: Ref<TextBox>,
}
impl Fields {
    fn new() -> Self {
        let first = TextBox::new();
        let second = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(first.clone());
        panel.children().add(second.clone());
        let window = Window::new();
        window.set_width(720.0);
        window.set_height(650.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        Self {
            window,
            panel,
            first,
            second,
        }
    }
}

#[test]
fn detached_adapters_cannot_navigate_the_unrelated_active_window() {
    headless::run(|| {
        let active = Fields::new();
        assert!(active.first.focus());
        let detached = TextBox::new();
        let target = TextBoxInputTarget::new(detached.clone(), None).unwrap();
        assert!(!target.perform_enter_action(EnterAction::Next));
        assert!(active.first.is_focused());
        target.detach();
        detached.set_password_char('●');
        let password = PasswordBoxInputTarget::new(detached, None).unwrap();
        assert!(!password.perform_enter_action(EnterAction::Previous));
        assert!(active.first.is_focused());
        password.detach();
        let retired = Fields::new();
        let closed = TextBoxInputTarget::new(retired.first.clone(), None).unwrap();
        retired.window.close();
        assert!(active.first.focus());
        assert!(!closed.perform_enter_action(EnterAction::Next));
        assert!(active.first.is_focused());
        closed.detach();
        active.window.close();
    });
}

#[test]
fn next_previous_use_own_root_and_current_local_focus_or_target_fallback() {
    headless::run(|| {
        let a = Fields::new();
        let b = Fields::new();
        let target = TextBoxInputTarget::new(a.first.clone(), None).unwrap();
        assert!(b.first.focus());
        assert!(target.perform_enter_action(EnterAction::Next));
        assert!(
            a.second.is_focused(),
            "foreign-window focus must not seed local traversal"
        );
        assert!(!b.second.is_focused());
        assert!(target.perform_enter_action(EnterAction::Previous));
        assert!(
            a.first.is_focused(),
            "same-window current focus must remain the traversal origin"
        );
        assert!(b.first.focus());
        a.first.set_password_char('●');
        let password = PasswordBoxInputTarget::new(a.first.clone(), None).unwrap();
        assert!(password.perform_enter_action(EnterAction::Next));
        assert!(a.second.is_focused());
        a.second.set_is_enabled(false);
        let third = TextBox::new();
        a.panel.children().add(third.clone());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(b.first.focus());
        assert!(target.perform_enter_action(EnterAction::Next));
        assert!(
            third.is_focused(),
            "disabled local fields are skipped within the own root"
        );
        target.detach();
        password.detach();
        a.window.close();
        b.window.close();
    });
}

#[test]
fn custom_enter_handler_keeps_precedence_and_detachment_during_callback_is_safe() {
    headless::run(|| {
        let a = Fields::new();
        let b = Fields::new();
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let target = TextBoxInputTarget::new(
            TextBox::new(),
            Some(Rc::new(move |_| {
                count.set(count.get() + 1);
                true
            })),
        )
        .unwrap();
        assert!(b.first.focus());
        assert!(target.perform_enter_action(EnterAction::Next));
        assert_eq!(calls.get(), 1);
        assert!(b.first.is_focused());
        let panel = a.panel.clone();
        let field = a.first.clone();
        let other = b.first.clone();
        let target2 = TextBoxInputTarget::new(
            a.first.clone(),
            Some(Rc::new(move |_| {
                panel.children().remove(&field);
                assert!(other.focus());
                false
            })),
        )
        .unwrap();
        assert!(!target2.perform_enter_action(EnterAction::Next));
        assert!(
            b.first.is_focused(),
            "fallthrough after reentrant detach must not traverse another window"
        );
        target.detach();
        target2.detach();
        a.window.close();
        b.window.close();
    });
}

#[test]
fn two_keyboards_track_their_roots_after_reparent_and_independent_teardown() {
    headless::run(|| {
        let a = Fields::new();
        let b = Fields::new();
        let ka = OnScreenKeyboard::new(None).unwrap();
        let kb = OnScreenKeyboard::new(None).unwrap();
        for k in [&ka, &kb] {
            k.engine().settings().set_auto_capitalization(false);
            k.engine().settings().set_glide_typing(false);
            k.engine().settings().set_clipboard_history_enabled(false);
            k.set_height(200.0);
        }
        a.panel.children().add(ka.clone());
        b.panel.children().add(kb.clone());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(a.first.focus());
        assert_eq!(ka.target_element(), Some(a.first.clone().upcast()));
        assert!(b.first.focus());
        assert_eq!(kb.target_element(), Some(b.first.clone().upcast()));
        assert_eq!(ka.target_element(), Some(a.first.clone().upcast()));
        ka.engine().input_text(&"alpha".into()).unwrap();
        kb.engine().input_text(&"beta".into()).unwrap();
        assert_eq!(a.first.text().as_deref(), Some("alpha"));
        assert_eq!(b.first.text().as_deref(), Some("beta"));
        a.panel.children().remove(&ka);
        b.panel.children().add(ka.clone());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(ka.target_element(), Some(b.first.clone().upcast()));
        assert!(a.second.focus());
        assert_eq!(
            ka.target_element(),
            Some(b.first.clone().upcast()),
            "old-root focus subscription must be gone"
        );
        assert!(b.second.focus());
        assert_eq!(ka.target_element(), Some(b.second.clone().upcast()));
        assert_eq!(kb.target_element(), Some(b.second.clone().upcast()));
        ka.release_native_subscriptions().unwrap();
        kb.engine().input_text(&"survives".into()).unwrap();
        assert_eq!(b.second.text().as_deref(), Some("survives"));
        kb.release_native_subscriptions().unwrap();
        a.window.close();
        b.window.close();
    });
}

#[test]
fn data_templated_window_uses_visual_input_root_instead_of_content_model() {
    use ferroui_base::data::TemplateBinding;
    use ferroui_controls::{
        ContentControl,
        presenters::ContentPresenter,
        templates::{FuncControlTemplate, FuncDataTemplate},
    };
    headless::run(|| {
        let first = TextBox::new();
        let second = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(first.clone());
        panel.children().add(second.clone());
        let view = panel.clone();
        let window = Window::new();
        // A real native content presenter renders a data model through a data template.
        // Window.Content is intentionally not a Control; neither field is detached.
        window.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".into()));
            for property in [
                ContentControl::content_property().as_property(),
                ContentControl::content_template_property().as_property(),
            ] {
                presenter.bind_binding(property, &TemplateBinding::new(property));
            }
            scope.register("PART_ContentPresenter", presenter.clone().upcast());
            scope.complete();
            presenter.upcast()
        })));
        window.set_content_template(Some(FuncDataTemplate::new(
            |_| true,
            move |_, _| Some(view.clone().upcast()),
            false,
        )));
        window.set_content(Some(Rc::new("form-model".to_string())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(
            first.visual_root().is_some(),
            "data template must attach the real editor"
        );
        assert!(first.focus());
        let target = TextBoxInputTarget::new(first.clone(), None).unwrap();
        assert!(target.perform_enter_action(EnterAction::Next));
        assert!(second.is_focused());
        assert!(target.perform_enter_action(EnterAction::Previous));
        assert!(first.is_focused());
        target.detach();
        window.close();
    });
}
