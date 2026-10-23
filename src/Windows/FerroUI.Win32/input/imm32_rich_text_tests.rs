use super::tests::{Call, FakeSystem};
use super::*;
use ferroui_base::Ref;
use ferroui_base::input::{InputElement, TextInputEventArgs};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use ferroui_controls::testing::{
    MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use ferroui_controls::{Border, Control, RichEditBox, StackPanel, Window};

const HWND: isize = 107;

#[derive(Default)]
struct Parent {
    keys: RefCell<Vec<Key>>,
    text: RefCell<Vec<String>>,
    hwnd: Cell<isize>,
    scale: Cell<f64>,
}

impl Imm32Parent for Parent {
    fn hwnd(&self) -> isize {
        self.hwnd.get()
    }
    fn desktop_scaling(&self) -> f64 {
        self.scale.get()
    }
    fn set_ignore_wm_char(&self, _value: bool) {}
    fn raw_text_input(&self, _timestamp: u64, text: &str) -> bool {
        self.text.borrow_mut().push(text.to_owned());
        true
    }
    fn raw_key_press(&self, key: Key, _physical_key: PhysicalKey) {
        self.keys.borrow_mut().push(key);
    }
}

struct Fixture {
    window: Ref<Window>,
    owner: Ref<RichEditBox>,
    outside: Ref<Border>,
    system: Rc<FakeSystem>,
    native: Rc<Imm32InputMethod>,
    parent: Rc<Parent>,
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

impl Fixture {
    fn new() -> Self {
        let system = FakeSystem::new();
        let native = Imm32InputMethod::new(system.clone());
        let parent = Rc::new(Parent::default());
        parent.hwnd.set(HWND);
        parent.scale.set(1.0);
        let source: Rc<dyn Imm32Parent> = parent.clone();
        native.set_language_and_window(Rc::downgrade(&source), HWND, 0x411);
        let method = native.clone();
        let app =
            UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
                MockWindowingPlatform::with_window_impl(move || {
                    let window = MockWindowingPlatform::create_window_mock();
                    window.setup_feature::<dyn ITextInputMethodImpl>(method.clone());
                    window
                }),
            ));
        let text = TextTestScope::new();
        let owner = RichEditBox::new();
        owner.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
            |_, scope| {
                let host = Border::new();
                host.set_name(Some("ContentElement".into()));
                host.register_in_name_scope(&**scope).upcast()
            },
        )));
        owner.set_height(70.0);
        owner.set_text("abcd").unwrap();
        owner.select(1, 3);
        owner.editor().clear_history();
        let outside = Border::new();
        outside.set_focusable(true);
        outside.set_height(25.0);
        let panel = StackPanel::new();
        panel.children().add(owner.clone());
        panel.children().add(outside.clone());
        let window = Window::new();
        window.set_width(250.0);
        window.set_height(120.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        owner.focus();
        system.run_posted();
        system.take_calls();
        assert!(native.has_managed_connection());
        Self {
            window,
            owner,
            outside,
            system,
            native,
            parent,
            _text: text,
            _app: app,
        }
    }

    fn preview(&self, text: &str) {
        *self.system.composition_string.borrow_mut() = Some(text.to_owned());
        self.native
            .handle_composition(0, GCS::GCS_COMPSTR as isize, 1);
    }

    fn commit(&self, text: &str) {
        *self.system.result_string.borrow_mut() = Some(text.to_owned());
        self.native
            .handle_composition(0, GCS::GCS_RESULTSTR as isize, 2);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn imm32_rich_start_never_deletes_selection_and_result_commits_one_transaction() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    f.native.handle_composition_start();
    assert!(f.parent.keys.borrow().is_empty());
    assert_eq!(f.owner.document().snapshot(), before);
    f.preview("候補");
    assert_eq!(f.owner.text(), "a候補d");
    assert!(!f.owner.can_undo());
    f.commit("確定");
    f.native.handle_composition_end(3);
    assert_eq!(f.owner.text(), "a確定d");
    assert!(f.parent.text.borrow().is_empty());
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
    let calls = f.system.take_calls();
    let acquired = calls
        .iter()
        .filter(|call| matches!(call, Call::GetContext(HWND)))
        .count();
    let released = calls
        .iter()
        .filter(|call| matches!(call, Call::ReleaseContext(HWND, _)))
        .count();
    assert_eq!(acquired, released);
}

#[test]
fn imm32_rich_end_without_result_cancels_instead_of_promoting_cached_preedit() {
    let f = Fixture::new();
    f.native.handle_composition_start();
    f.preview("preview");
    f.native.handle_composition_end(3);
    assert_eq!(f.owner.text(), "abcd");
    assert!(!f.owner.can_undo());
    assert!(f.parent.text.borrow().is_empty());
}

#[test]
fn imm32_rich_rejected_result_survives_end_and_an_explicit_result_retry() {
    let f = Fixture::new();
    f.native.handle_composition_start();
    f.preview("preview");
    let token = f.owner.add_handler_with(
        InputElement::text_input_event(),
        |_, event: &TextInputEventArgs| event.set_handled(true),
        RoutingStrategies::TUNNEL,
        false,
    );
    f.commit("accepted");
    f.native.handle_composition_end(3);
    assert_eq!(f.owner.text(), "apreviewd");
    assert!(f.owner.editor().is_composing());
    assert!(!f.owner.can_undo());
    f.owner
        .remove_handler(InputElement::text_input_event(), token);
    f.commit("accepted");
    assert_eq!(f.owner.text(), "aacceptedd");
    assert!(!f.owner.editor().is_composing());
}

#[test]
fn imm32_rich_korean_end_before_queued_result_preserves_the_native_session() {
    let f = Fixture::new();
    let source: Rc<dyn Imm32Parent> = f.parent.clone();
    f.native
        .set_language_and_window(Rc::downgrade(&source), HWND, 0x412);
    f.system.run_posted();
    f.native.handle_composition_start();
    f.preview("가");
    f.system.next_result.set(true);
    f.native.handle_composition_end(1);
    assert_eq!(f.owner.text(), "a가d");
    assert!(f.owner.editor().is_composing());
    f.commit("각");
    assert_eq!(f.owner.text(), "a각d");
    assert!(!f.native.is_composing());
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn imm32_rich_detached_client_drops_late_results_without_ordinary_text_fallback() {
    let f = Fixture::new();
    f.native.handle_composition_start();
    f.preview("preview");
    f.outside.focus();
    let before = f.owner.document().snapshot();
    f.commit("stale");
    f.native.handle_composition_end(3);
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.owner.text(), "abcd");
    assert!(f.parent.text.borrow().is_empty());
}

#[test]
fn imm32_rich_reset_cancels_the_native_composition_and_invalidates_posted_geometry() {
    let f = Fixture::new();
    f.system.active_window.set(HWND);
    f.native.handle_composition_start();
    f.preview("preview");
    f.native.set_cursor_rect(Rect::new(1.0, 2.0, 1.0, 12.0));
    f.outside.focus();
    f.system.run_posted();
    assert_eq!(f.owner.text(), "abcd");
    assert!(
        !f.system
            .take_calls()
            .iter()
            .any(|call| matches!(call, Call::SetCandidateWindow(..)))
    );
}

#[test]
fn imm32_rich_cancel_does_not_clear_a_composition_started_by_its_ended_handler() {
    let f = Fixture::new();
    f.native.handle_composition_start();
    f.preview("preview");
    let native = Rc::downgrade(&f.native);
    let system = f.system.clone();
    let armed = Cell::new(true);
    f.owner
        .add_handler(RichEditBox::text_composition_ended_event(), move |_, _| {
            if armed.replace(false) {
                let native = native.upgrade().unwrap();
                native.handle_composition_start();
                *system.composition_string.borrow_mut() = Some("new".to_owned());
                native.handle_composition(0, GCS::GCS_COMPSTR as isize, 4);
            }
        });

    f.native.handle_composition(0, 0, 3);

    assert_eq!(f.native.composition().as_deref(), Some("new"));
    assert_eq!(f.owner.text(), "anewd");
    assert!(f.native.is_composing());
    f.commit("accepted");
    assert_eq!(f.owner.text(), "aacceptedd");
}

#[test]
fn imm32_window_site_identity_prevents_an_old_site_from_detaching_a_new_client() {
    let system = FakeSystem::new();
    let native = Imm32InputMethod::new(system.clone());
    let first = Rc::new(Parent::default());
    first.hwnd.set(1);
    let second = Rc::new(Parent::default());
    second.hwnd.set(2);
    let first_parent: Rc<dyn Imm32Parent> = first;
    let second_parent: Rc<dyn Imm32Parent> = second;
    let first = native.for_window(Rc::downgrade(&first_parent));
    let second = native.for_window(Rc::downgrade(&second_parent));
    let client = super::tests::Client::new();
    // Each facade has a stable identity even when the OS reuses an HWND.
    second.set_client(Some(client.clone()));
    first.set_client(None);
    first.reset();
    drop(first);
    assert!(native.client().is_some());
    second.set_client(None);
    assert!(native.client().is_none());
}

#[test]
fn imm32_rich_window_handoff_cancels_the_old_native_context_before_queued_disable() {
    for run_disable_before_handoff in [false, true] {
        let f = Fixture::new();
        let old_parent: Rc<dyn Imm32Parent> = f.parent.clone();
        let old_site = f.native.for_window(Rc::downgrade(&old_parent));
        old_site.set_client(f.native.client());
        f.system.run_posted();
        f.native.handle_composition_start();
        f.preview("old preview");
        f.system.take_calls();

        let new_parent = Rc::new(Parent::default());
        new_parent.hwnd.set(HWND + 1);
        new_parent.scale.set(1.0);
        let new_parent: Rc<dyn Imm32Parent> = new_parent;
        let new_site = f.native.for_window(Rc::downgrade(&new_parent));
        let new_client: Rc<dyn TextInputMethodClient> = super::tests::Client::new();
        f.system.associations.borrow_mut().insert(HWND + 1, 0x3C);

        old_site.set_client(None);
        assert_eq!(f.owner.text(), "abcd");
        assert!(f.system.calls.borrow().contains(&Call::NotifyIme(
            0x1C,
            NI_COMPOSITIONSTR,
            CPS_CANCEL,
            0
        )));
        if run_disable_before_handoff {
            f.system.run_posted();
        }
        f.native
            .set_language_and_window(Rc::downgrade(&new_parent), HWND + 1, 0x411);
        new_site.set_client(Some(new_client.clone()));
        // Without the synchronous cancellation, this generation would discard
        // the old posted disable and leave the old IME composition alive.
        f.system.run_posted();
        assert_eq!(f.native.hwnd(), HWND + 1);
        assert!(Rc::ptr_eq(&f.native.client().unwrap(), &new_client));
        let notifications: Vec<_> = f
            .system
            .take_calls()
            .into_iter()
            .filter(|call| matches!(call, Call::NotifyIme(..)))
            .collect();
        assert_eq!(
            notifications,
            vec![Call::NotifyIme(0x1C, NI_COMPOSITIONSTR, CPS_CANCEL, 0)]
        );
        assert!(f.system.borrows.borrow().values().all(|count| *count == 0));
    }
}

#[test]
fn imm32_rich_reentrant_native_cancellation_preserves_the_replacement_window_client() {
    let f = Fixture::new();
    let old_parent: Rc<dyn Imm32Parent> = f.parent.clone();
    let old_site = f.native.for_window(Rc::downgrade(&old_parent));
    old_site.set_client(f.native.client());
    f.system.run_posted();
    f.native.handle_composition_start();
    f.preview("old preview");
    f.system.take_calls();

    let new_parent = Rc::new(Parent::default());
    new_parent.hwnd.set(HWND + 1);
    let new_parent: Rc<dyn Imm32Parent> = new_parent;
    let new_parent_weak = Rc::downgrade(&new_parent);
    let new_site = f.native.for_window(new_parent_weak.clone());
    let new_client: Rc<dyn TextInputMethodClient> = super::tests::Client::new();
    let callback_site = new_site.clone();
    let callback_client = new_client.clone();
    let native = Rc::downgrade(&f.native);
    f.system.associations.borrow_mut().insert(HWND + 1, 0x3C);
    *f.system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::NotifyIme(_, _, CPS_CANCEL, _)),
        Box::new(move || {
            native
                .upgrade()
                .unwrap()
                .set_language_and_window(new_parent_weak, HWND + 1, 0x411);
            callback_site.set_client(Some(callback_client));
        }),
    ));

    old_site.set_client(None);
    f.system.run_posted();

    assert_eq!(f.native.hwnd(), HWND + 1);
    assert!(Rc::ptr_eq(&f.native.client().unwrap(), &new_client));
    assert_eq!(f.owner.text(), "abcd");
    let notifications: Vec<_> = f
        .system
        .take_calls()
        .into_iter()
        .filter(|call| matches!(call, Call::NotifyIme(..)))
        .collect();
    assert_eq!(
        notifications,
        vec![Call::NotifyIme(0x1C, NI_COMPOSITIONSTR, CPS_CANCEL, 0)]
    );
    assert!(f.system.borrows.borrow().values().all(|count| *count == 0));
}

#[test]
fn imm32_input_scope_live_rich_names_preserve_composition_history_and_plain_text() {
    use ferroui_base::input::{InputScope, InputScopeName, InputScopeNameValue as Scope};
    let f = Fixture::new();
    let connection = f.native.connection.borrow().clone().unwrap();
    f.native.handle_composition_start();
    f.preview("preview");
    let before = f.owner.document().snapshot();
    let scope = InputScope::with_name_value(Scope::EmailSmtpAddress);
    scope
        .names()
        .add(InputScopeName::with_name_value(Scope::Digits));
    f.system.take_calls();
    f.owner.set_input_scope(scope.clone());
    scope.names().get(0).set_name_value(Scope::NumericPin);
    let calls = f.system.take_calls();
    assert_eq!(
        calls
            .iter()
            .filter(|call| matches!(call, Call::SetInputScopes(..)))
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            Call::SetInputScopes(HWND, vec![5, 28]),
            Call::SetInputScopes(HWND, vec![64, 28, 61])
        ]
    );
    assert!(!calls.iter().any(|call| matches!(call, Call::NotifyIme(..))));
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(Rc::ptr_eq(
        &connection,
        f.native.connection.borrow().as_ref().unwrap()
    ));
    assert!(f.owner.editor().is_composing());
    assert!(!f.owner.can_undo());
    // A numeric or secret keyboard hint does not filter or mask rich text.
    f.commit("accepted");
    assert_eq!(f.owner.text(), "aacceptedd");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn imm32_input_scope_recipient_release_retains_the_semantic_owners_options() {
    use ferroui_base::input::InputScopeNameValue as Scope;
    let f = Fixture::new();
    let source: Rc<dyn Imm32Parent> = f.parent.clone();
    let site = f.native.for_window(Rc::downgrade(&source));
    site.set_options(&TextInputOptions {
        input_scope_names: Some(Rc::from([Scope::EmailSmtpAddress])),
        ..Default::default()
    });
    site.set_client(f.native.client());
    let connection = f.native.connection.borrow().clone().unwrap();
    f.native.handle_composition_start();
    f.preview("preview");

    let temporary = Rc::new(Parent::default());
    temporary.hwnd.set(HWND + 1);
    temporary.scale.set(1.0);
    let temporary: Rc<dyn Imm32Parent> = temporary;
    f.native
        .set_language_and_window(Rc::downgrade(&temporary), HWND + 1, 0x411);
    f.system.take_calls();
    f.native
        .release_window(HWND + 1, &Rc::downgrade(&temporary));
    assert_eq!(
        f.system
            .take_calls()
            .into_iter()
            .filter(|call| matches!(call, Call::SetInputScopes(..)))
            .collect::<Vec<_>>(),
        vec![Call::SetInputScopes(HWND + 1, vec![0])]
    );
    assert!(f.native.is_active());
    assert_eq!(f.native.hwnd(), 0);
    assert!(Rc::ptr_eq(
        &connection,
        f.native.connection.borrow().as_ref().unwrap()
    ));
    assert!(f.owner.editor().is_composing());
    assert_eq!(f.owner.text(), "apreviewd");

    let replacement = Rc::new(Parent::default());
    replacement.hwnd.set(HWND + 2);
    replacement.scale.set(1.0);
    let replacement: Rc<dyn Imm32Parent> = replacement;
    f.native
        .set_language_and_window(Rc::downgrade(&replacement), HWND + 2, 0x411);
    assert_eq!(
        f.system
            .take_calls()
            .into_iter()
            .filter(|call| matches!(call, Call::SetInputScopes(..)))
            .collect::<Vec<_>>(),
        vec![Call::SetInputScopes(HWND + 2, vec![5])]
    );
    assert_eq!(
        f.system.input_scopes.borrow().get(&(HWND + 2)),
        Some(&vec![5])
    );
    f.commit("accepted");
    assert_eq!(f.owner.text(), "aacceptedd");
}

#[test]
fn imm32_input_scope_source_release_clears_its_distinct_physical_recipient() {
    use ferroui_base::input::InputScopeNameValue as Scope;
    let f = Fixture::new();
    let source: Rc<dyn Imm32Parent> = f.parent.clone();
    let site = f.native.for_window(Rc::downgrade(&source));
    site.set_options(&TextInputOptions {
        input_scope_names: Some(Rc::from([Scope::Digits])),
        ..Default::default()
    });
    site.set_client(f.native.client());
    f.native.handle_composition_start();
    f.preview("preview");
    let recipient = Rc::new(Parent::default());
    recipient.hwnd.set(HWND + 1);
    recipient.scale.set(1.0);
    let recipient: Rc<dyn Imm32Parent> = recipient;
    f.native
        .set_language_and_window(Rc::downgrade(&recipient), HWND + 1, 0x411);
    f.system.take_calls();
    f.native.release_window(HWND, &Rc::downgrade(&source));
    assert_eq!(
        f.system
            .take_calls()
            .into_iter()
            .filter(|call| matches!(call, Call::SetInputScopes(..)))
            .collect::<Vec<_>>(),
        vec![Call::SetInputScopes(HWND + 1, vec![0])]
    );
    assert!(!f.native.is_active());
    assert!(f.system.input_scopes.borrow().is_empty());
    assert_eq!(f.owner.text(), "abcd");
    assert!(!f.owner.can_undo());
    site.set_options(&TextInputOptions {
        input_scope_names: Some(Rc::from([Scope::Url])),
        ..Default::default()
    });
    f.system.run_posted();
    assert!(
        !f.system
            .take_calls()
            .iter()
            .any(|call| matches!(call, Call::SetInputScopes(..)))
    );
}
