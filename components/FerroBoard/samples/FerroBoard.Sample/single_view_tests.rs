use crate::{
    composition::MainViewSession,
    sample_options::SampleOptions,
    sample_storage::{SampleStorage, SampleStoragePlatform, StorageResult},
};
use ferroui_base::{controls::NameScope, threading::Dispatcher};
use ferroui_controls::{Control, TextBox, Window, embedding::EmbeddableControlRoot};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

#[derive(Default)]
struct Storage {
    settings: RefCell<Option<String>>,
    writes: Cell<usize>,
}
impl SampleStoragePlatform for Storage {
    fn get_setting(&self, _: &str) -> StorageResult<Option<String>> {
        Ok(self.settings.borrow().clone())
    }
    fn set_setting(&self, _: &str, value: &str) -> StorageResult<()> {
        *self.settings.borrow_mut() = Some(value.to_owned());
        self.writes.set(self.writes.get() + 1);
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Ok(None)
    }
    fn log(&self, message: &str) {
        panic!("unexpected storage error: {message}");
    }
}
fn session() -> (Rc<MainViewSession>, Rc<Storage>) {
    let platform = Rc::new(Storage::default());
    let session = MainViewSession::with_options(
        SampleStorage::new(platform.clone()),
        Arc::new(SampleOptions::default()),
    )
    .unwrap();
    session.host.set_is_animated(false);
    (session, platform)
}
#[test]
fn compiled_sample_fields_preserve_engine_kinds_and_native_input_scopes() {
    crate::tests::run(|| {
        use ferroboard_controls::input::TextBoxInputTarget;
        use ferroboard_core::editor::{CapitalizationMode, EnterAction, InputKind};
        use ferroui_base::input::text_input::{
            TextInputContentType as Content, TextInputOptions, TextInputReturnKeyType,
        };

        let (session, _) = session();
        let scope = NameScope::get_name_scope(&session.view).unwrap();
        let mut mismatches = Vec::new();
        for (name, content, kind) in [
            ("MessageBox", Content::Normal, InputKind::Text),
            ("NameBox", Content::Name, InputKind::Text),
            ("EmailBox", Content::Email, InputKind::Email),
            ("WebsiteBox", Content::Url, InputKind::Uri),
            ("SearchBox", Content::Search, InputKind::Search),
            ("AmountBox", Content::Digits, InputKind::Number),
            ("PhoneBox", Content::Digits, InputKind::Phone),
            ("PriceBox", Content::Number, InputKind::Decimal),
        ] {
            let field = scope.find(name).unwrap().cast::<TextBox>().unwrap();
            let native = TextInputOptions::from_styled_element(&field);
            let engine = TextBoxInputTarget::compute_attributes(&field).unwrap();
            if native.content_type != content {
                mismatches.push(format!(
                    "{name}: expected {content:?}, got {:?}",
                    native.content_type
                ));
            }
            assert_eq!(
                engine.kind(),
                kind,
                "{name}: engine kind must remain source-compatible"
            );
            if name == "MessageBox" {
                if !native.multiline || !native.auto_capitalization {
                    mismatches.push(
                        "MessageBox: native multiline/sentence capitalization missing".to_owned(),
                    );
                }
                assert!(field.accepts_return());
                assert!(engine.is_multiline());
                assert_eq!(engine.capitalization(), CapitalizationMode::Sentences);
            } else {
                assert!(!native.multiline, "{name}");
            }
            if name == "NameBox" {
                assert_eq!(engine.capitalization(), CapitalizationMode::Words);
            }
            if name == "EmailBox" {
                if native.return_key_type != TextInputReturnKeyType::Next {
                    mismatches.push("EmailBox: native Next hint missing".to_owned());
                }
                assert_eq!(engine.enter_action(), EnterAction::Next);
            }
        }
        session.release();
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    });
}
#[test]
fn private_note_and_pin_keep_distinct_native_and_keyboard_contracts() {
    crate::tests::run(|| {
        use ferroboard_controls::input::{PasswordBoxInputTarget, TextBoxInputTarget};
        use ferroboard_core::editor::{ITextInputTarget, InputKind};
        use ferroui_base::input::{
            InputElement,
            text_input::{
                TextInputContentType, TextInputMethodClientRequestedEventArgs, TextInputOptions,
            },
        };
        let (session, _) = session();
        let scope = NameScope::get_name_scope(&session.view).unwrap();
        let note = scope
            .find("PrivateNoteBox")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        let attributes = TextBoxInputTarget::compute_attributes(&note).unwrap();
        assert_eq!(attributes.kind(), InputKind::Text);
        assert!(attributes.is_multiline() && attributes.is_private());
        assert!(!attributes.is_password());
        let options = TextInputOptions::from_styled_element(&note);
        assert_eq!(options.content_type, TextInputContentType::Normal);
        assert!(options.multiline && options.is_sensitive);
        assert_eq!(note.password_char(), '\0');
        let pin = scope.find("PinBox").unwrap().cast::<TextBox>().unwrap();
        let target = PasswordBoxInputTarget::new(pin.clone(), None).unwrap();
        assert_eq!(target.attributes().kind(), InputKind::NumericPassword);
        assert!(target.attributes().is_private() && target.attributes().is_password());
        assert!(!target.attributes().allow_suggestions());
        let request = TextInputMethodClientRequestedEventArgs::new();
        request.set_routed_event(Some(
            InputElement::text_input_method_client_requested_event(),
        ));
        pin.raise_event(&request);
        let native = request.client().unwrap().input_options(
            &pin.clone().upcast(),
            TextInputOptions::from_styled_element(&pin),
        );
        assert_eq!(native.content_type, TextInputContentType::Pin);
        assert!(native.is_sensitive);
        assert_eq!(
            TextInputOptions::get_content_type(&pin),
            TextInputContentType::Digits
        );
        target.detach();
        session.release();
    });
}
#[test]
fn single_view_session_uses_non_window_root_and_preserves_detach_reattach() {
    crate::tests::run(|| {
        let (session, storage) = session();
        // Headless has no embeddable-root factory. Supply its actual top-level
        // implementation explicitly, as an iOS host supplies its own implementation.
        let platform = ferroui_controls::platform::PlatformManager::create_window();
        platform.resize(
            ferroui_base::Size::new(540.0, 780.0),
            ferroui_controls::WindowResizeReason::Layout,
        );
        let root = EmbeddableControlRoot::with_impl(platform);
        root.set_is_visible(true);
        assert!(root.cast::<Window>().is_none());
        root.set_content(Some(Control::boxed(session.view.clone())));
        root.prepare();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = NameScope::get_name_scope(&session.view).unwrap();
        let message = scope.find("MessageBox").unwrap().cast::<TextBox>().unwrap();
        assert!(message.is_focused());
        assert_eq!(
            session.keyboard.target_element(),
            Some(message.clone().upcast())
        );
        assert!(session.host.is_keyboard_open());
        let email = scope.find("EmailBox").unwrap().cast::<TextBox>().unwrap();
        let website = scope.find("WebsiteBox").unwrap().cast::<TextBox>().unwrap();
        assert!(email.focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            session
                .keyboard
                .engine()
                .target()
                .unwrap()
                .perform_enter_action(ferroboard_core::editor::EnterAction::Next)
        );
        assert!(
            website.is_focused(),
            "Next must stay inside the non-Window input root"
        );
        assert!(message.focus());
        Dispatcher::ui_thread().run_jobs(None);
        session.keyboard.engine().type_text(&"q".into()).unwrap();
        assert_eq!(message.text().as_deref(), Some("Q"));
        let before = storage.writes.get();
        root.set_content(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            storage.writes.get() > before,
            "unloading must save the session"
        );
        root.set_content(Some(Control::boxed(session.view.clone())));
        root.prepare();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(message.is_focused());
        session.keyboard.engine().type_text(&"z".into()).unwrap();
        assert_eq!(message.text().as_deref(), Some("Qz"));
        root.set_content(None);
        session.release();
    });
}
#[test]
fn single_view_session_drop_retires_controls_and_saves_while_view_is_retained() {
    crate::tests::run(|| {
        let (session, storage) = session();
        let view = session.view.clone();
        let reset = crate::test_ui::view_ui(&session)
            .find("ResetButton")
            .unwrap()
            .cast::<ferroui_controls::Button>()
            .unwrap();
        let settings = session.keyboard.settings();
        settings.set_key_height(83.0);
        let before = storage.writes.get();
        drop(session);
        assert!(storage.writes.get() > before);
        assert_eq!(settings.key_height(), 83.0);
        reset.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
            ferroui_controls::Button::click_event(),
        ));
        assert_eq!(
            settings.key_height(),
            83.0,
            "retained controls cannot reach retired session state"
        );
        assert!(
            NameScope::get_name_scope(&view)
                .unwrap()
                .find("Host")
                .is_some()
        );
    });
}
