//! Test-only native control/privacy observations. All editing remains in the library.
#[path = "../support/local_future.rs"]
mod native_future;
use ferroboard_controls::{
    OnScreenKeyboard, ScreenKeyboardHost, SmartbarAction, input::ProtectedPasswordTextBox,
    services::SystemClipboardService,
};
use ferroboard_core::{
    clipboard::{ClipboardFuture, IClipboardService},
    editor::InputKind,
    engine::KeyboardEngine,
    keys::KeyIcon,
    settings::SmartbarLayout,
    text::{Suggestion, SuggestionKind, Utf16Text},
};
use ferroui_base::{
    Ref,
    input::{
        InputElement, InputMethod,
        text_input::{TextInputMethodClientRequestedEventArgs, TextSelection},
    },
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, TextBox};
use std::{cell::Cell, rc::Rc};
type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn view() -> Ref<Control> {
    let value = ferroboard_native_consumer::compiled_xaml::try_load(
        None,
        &format!(
            "{}PrivacyHosted.xaml",
            ferroboard_native_consumer::compiled_xaml::ROOT_URI
        ),
    )
    .unwrap()
    .expect("dependency compiled native password view");
    let host = value
        .downcast_ref::<Ref<ScreenKeyboardHost>>()
        .unwrap()
        .clone();
    let fields = host.content().unwrap().cast::<StackPanel>().unwrap();
    let password = fields
        .children()
        .get(1)
        .cast::<ProtectedPasswordTextBox>()
        .unwrap();
    let pin = fields
        .children()
        .get(3)
        .cast::<ProtectedPasswordTextBox>()
        .unwrap();
    let public = fields.children().get(5).cast::<TextBox>().unwrap();
    let status = fields.children().get(6).cast::<TextBlock>().unwrap();
    let keyboard = host.keyboard();
    let engine = keyboard.engine();
    let settings = engine.settings();
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_glide_typing(false);
    settings.set_learn_words(true);
    settings.set_clipboard_history_enabled(true);
    settings.set_emoji_history_enabled(true);
    settings.set_smartbar_layout(SmartbarLayout::ActionsOnly);
    engine
        .language_models()
        .get_user_dictionary(&"en".into())
        .set_learn_threshold(1);
    let busy = Rc::new(Cell::new(false));
    let actions = keyboard.smartbar_actions().unwrap();
    actions
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
    for name in ["Verify", "Reveal", "Emoji", "Guard", "Sync"] {
        let keyboard = keyboard.downgrade();
        let password = password.downgrade();
        let pin = pin.downgrade();
        let public = public.downgrade();
        let status = status.downgrade();
        let fields = fields.downgrade();
        let busy = busy.clone();
        actions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Some(SmartbarAction::new(
                Some(name.into()),
                KeyIcon::None,
                Some(name.into()),
                Some(Rc::new(move |_| {
                    if busy.replace(true) {
                        return Ok(());
                    }
                    let keyboard = keyboard.upgrade().expect("live test keyboard");
                    let password = password.upgrade().expect("live test field");
                    let pin = pin.upgrade().expect("live test field");
                    let public = public.upgrade().expect("live test field");
                    let status = status.upgrade().expect("live test status");
                    let fields = fields.upgrade().expect("live test container");
                    let busy = busy.clone();
                    drop(Dispatcher::current_dispatcher().invoke_async_task_local(
                        move || async move {
                            let result = match name {
                                "Verify" => verify(&keyboard, &password, &pin, &public).await,
                                "Reveal" => reveal(&keyboard, &password),
                                "Emoji" => emoji(&keyboard, &password, &public),
                                "Guard" => guards(&keyboard, &password, &fields),
                                "Sync" => synchronize(&keyboard, &pin, &public).await,
                                _ => unreachable!("closed test action list"),
                            };
                            match result {
                                Ok(label) => status.set_text(Some(label)),
                                Err(error) => {
                                    status.set_text(Some(format!("FAIL {error}").as_str()))
                                }
                            }
                            busy.set(false);
                        },
                    ));
                    Ok(())
                })),
                None,
            )));
    }
    let password = password.downgrade();
    host.loaded(move |_, _| {
        if let Some(field) = password.upgrade() {
            field.focus();
        }
    });
    host.upcast()
}
fn require(condition: bool, message: &'static str) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
async fn sample(keyboard: &Ref<OnScreenKeyboard>) {
    keyboard.refresh_system_clipboard();
    for _ in 0..3 {
        Dispatcher::yield_now().await;
    }
}
async fn verify(
    keyboard: &Ref<OnScreenKeyboard>,
    password: &ProtectedPasswordTextBox,
    pin: &ProtectedPasswordTextBox,
    public: &TextBox,
) -> ProbeResult<&'static str> {
    let engine = keyboard.engine();
    let kind = engine.attributes().kind();
    let target = engine.target().ok_or("missing native target")?;
    let dictionary = engine.language_models().get_user_dictionary(&"en".into());
    let (field, expected, clipboard, label): (&TextBox, _, _, _) = match kind {
        InputKind::Password => (password, "zypm", "private-password", "PASS password"),
        InputKind::NumericPassword => (pin, "739", "private-pin", "PASS pin"),
        InputKind::Text => (public, "qzx ", "public-clipboard", "PASS public"),
        _ => return Err("unexpected native target kind".into()),
    };
    require(
        !InputMethod::get_is_input_method_enabled(field),
        "active native target lacks owned IME suppression",
    )?;
    require(
        field.text().as_deref() == Some(expected),
        "native editing/max-length mismatch",
    )?;
    require(
        target.text() == expected.into(),
        "native target text mismatch",
    )?;
    require(
        target.selection_start() == expected.encode_utf16().count() as i32,
        "native target caret mismatch",
    )?;
    require(
        dictionary.get_count(&"zyp".into()) == 0 && dictionary.get_count(&"zypm".into()) == 0,
        "private text entered learned words",
    )?;
    if kind != InputKind::Text {
        require(
            engine.attributes().is_private() && engine.is_incognito(),
            "password privacy attributes missing",
        )?;
        require(
            !engine.attributes().allow_suggestions() && engine.suggestions().count()? == 0,
            "password suggestions enabled",
        )?;
        require(dictionary.words().is_empty(), "password learned data")?;
    } else {
        require(
            !engine.is_incognito(),
            "public target retained password policy",
        )?;
        require(
            password.text().as_deref() == Some("zypm") && pin.text().as_deref() == Some("739"),
            "inactive native targets changed",
        )?;
        require(
            InputMethod::get_is_input_method_enabled(password)
                && InputMethod::get_is_input_method_enabled(pin),
            "inactive native targets retained IME suppression",
        )?;
        require(
            dictionary.get_count(&"qzx".into()) == 1,
            "public learning did not resume",
        )?;
    }
    let service = SystemClipboardService::for_visual(keyboard.clone().upcast());
    sample(keyboard).await;
    require(
        engine.clipboard_history().items()?.is_empty(),
        "private clipboard replayed",
    )?;
    service.write_text(clipboard).await?;
    require(
        service.read_text().await?.as_deref() == Some(clipboard),
        "native clipboard I/O unavailable",
    )?;
    sample(keyboard).await;
    let history = engine.clipboard_history().items()?;
    if kind == InputKind::Text {
        require(
            history.len() == 1 && history[0].text == clipboard.into(),
            "public clipboard history did not resume",
        )?;
    } else {
        require(history.is_empty(), "private clipboard recorded")?;
    }
    Ok(label)
}
fn reveal(
    keyboard: &OnScreenKeyboard,
    password: &ProtectedPasswordTextBox,
) -> ProbeResult<&'static str> {
    require(
        keyboard.engine().attributes().kind() == InputKind::Password,
        "reveal has wrong target",
    )?;
    password.set_reveal_password(true);
    require(
        password.password_char() == '●' && password.reveal_password(),
        "native reveal state mismatch",
    )?;
    Ok("PASS reveal")
}
fn emoji(
    keyboard: &OnScreenKeyboard,
    password: &ProtectedPasswordTextBox,
    public: &TextBox,
) -> ProbeResult<&'static str> {
    let engine = keyboard.engine();
    let kind = engine.attributes().kind();
    require(
        matches!(kind, InputKind::Password | InputKind::Text),
        "emoji has wrong target",
    )?;
    engine.commit_suggestion(&Suggestion::new("🙂".into(), SuggestionKind::Emoji))?;
    let recent = engine.emoji_history().recent().lock().clone();
    if kind == InputKind::Password {
        require(
            password.text().as_deref() == Some("🙂"),
            "native password Unicode insertion mismatch",
        )?;
        require(recent.is_empty(), "private emoji recorded")?;
        Ok("PASS private emoji")
    } else {
        require(
            public.text().is_some_and(|text| text.ends_with('🙂')),
            "native public emoji insertion mismatch",
        )?;
        require(
            recent == [Some("🙂".into())],
            "public emoji history did not resume",
        )?;
        Ok("PASS public emoji")
    }
}

fn guards(
    keyboard: &OnScreenKeyboard,
    password: &ProtectedPasswordTextBox,
    fields: &StackPanel,
) -> ProbeResult<&'static str> {
    if password.is_read_only() {
        password.set_is_read_only(false);
        return Ok("PASS unlocked");
    }
    let engine = keyboard.engine();
    require(
        engine.attributes().kind() == InputKind::Password,
        "guard action has wrong native target",
    )?;
    let target = engine.target().ok_or("missing guard target")?;
    let before = target.text();
    let caret = target.selection_start();
    for ancestor in [false, true] {
        if ancestor {
            fields.set_is_enabled(false);
        } else {
            password.set_is_enabled(false);
        }
        let result = target.replace(0, before.len() as i32, Some(&"blocked".into()));
        let unchanged = target.text() == before && target.selection_start() == caret;
        fields.set_is_enabled(true);
        password.set_is_enabled(true);
        require(
            result.is_ok() && unchanged,
            "disabled native target accepted replacement",
        )?;
    }
    password.set_is_read_only(true);
    password.focus();
    Ok("PASS locked")
}

#[derive(Default)]
struct SyncClipboard(native_future::Completion<Option<Utf16Text>>);
impl IClipboardService for SyncClipboard {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        self.0.future()
    }
    fn set_text_async(&self, _: Option<Utf16Text>) -> ClipboardFuture<()> {
        Box::pin(std::future::ready(Ok(())))
    }
}
struct RestoreClipboard {
    engine: Rc<KeyboardEngine>,
    original: Rc<dyn IClipboardService>,
}
impl Drop for RestoreClipboard {
    fn drop(&mut self) {
        self.engine.set_clipboard_service(self.original.clone());
    }
}
async fn synchronize(
    keyboard: &Ref<OnScreenKeyboard>,
    pin: &ProtectedPasswordTextBox,
    public: &TextBox,
) -> ProbeResult<&'static str> {
    let engine = keyboard.engine();
    require(
        engine.attributes().kind() == InputKind::Text,
        "sync has wrong initial native target",
    )?;
    let _restore = RestoreClipboard {
        engine: engine.clone(),
        original: engine.clipboard_service(),
    };
    let clipboard = Rc::new(SyncClipboard::default());
    engine.set_clipboard_service(clipboard.clone());
    let mut pending = engine.paste_async();
    require(
        native_future::poll(pending.as_mut()).is_pending(),
        "sync read did not suspend",
    )?;
    public.set_text(Some("a😀bc"));
    let request = TextInputMethodClientRequestedEventArgs::new();
    request.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    public.raise_event(&request);
    let client = request.client().ok_or("native field has no input client")?;
    client.set_selection(TextSelection::new(1, 3));
    Dispatcher::yield_now().await;
    let target = engine.target().ok_or("missing native range target")?;
    require(
        target.selection_start() == 1 && target.selection_length() == 2,
        "external native UTF16 selection mismatch",
    )?;
    drop(target);
    clipboard.0.complete(Some("é".into()));
    pending.await?;
    require(
        public.text().as_deref() == Some("aébc")
            && engine
                .target()
                .is_some_and(|t| t.selection_start() == 2 && t.selection_length() == 0),
        "paste used stale native text or selection",
    )?;

    let clipboard = Rc::new(SyncClipboard::default());
    engine.set_clipboard_service(clipboard.clone());
    let previous = Rc::downgrade(&engine.target().ok_or("missing previous native target")?);
    let mut pending = engine.paste_async();
    require(
        native_future::poll(pending.as_mut()).is_pending(),
        "replacement read did not suspend",
    )?;
    pin.set_text(Some(""));
    pin.focus();
    Dispatcher::yield_now().await;
    require(
        engine.attributes().kind() == InputKind::NumericPassword && previous.upgrade().is_none(),
        "focus replacement retained previous native target",
    )?;
    clipboard.0.complete(Some("867".into()));
    pending.await?;
    require(
        pin.text().as_deref() == Some("867") && public.text().as_deref() == Some("aébc"),
        "pending paste edited wrong native field",
    )?;
    require(
        engine.is_incognito() && engine.target().is_some_and(|t| t.selection_start() == 3),
        "replacement native password policy missing",
    )?;

    let clipboard = Rc::new(SyncClipboard::default());
    engine.set_clipboard_service(clipboard.clone());
    let previous = Rc::downgrade(&engine.target().ok_or("missing detach target")?);
    let mut pending = engine.paste_async();
    require(
        native_future::poll(pending.as_mut()).is_pending(),
        "detach read did not suspend",
    )?;
    keyboard.detach()?;
    Dispatcher::yield_now().await;
    require(
        previous.upgrade().is_none() && InputMethod::get_is_input_method_enabled(pin),
        "detached native adapter or IME guard retained",
    )?;
    clipboard.0.complete(Some("late".into()));
    pending.await?;
    require(
        engine.target().is_none()
            && pin.text().as_deref() == Some("867")
            && public.text().as_deref() == Some("aébc"),
        "detached completion edited native field",
    )?;
    public.focus();
    Dispatcher::yield_now().await;
    require(
        engine.attributes().kind() == InputKind::Text
            && !InputMethod::get_is_input_method_enabled(public),
        "native reattachment failed",
    )?;
    public.set_text(Some(""));
    Dispatcher::yield_now().await;
    Ok("PASS synchronized")
}
