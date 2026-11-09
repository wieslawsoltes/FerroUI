//! Consumer-only checks of real Android clipboard services and Core privacy gates.
use ferroboard_controls::{
    OnScreenKeyboard, SmartbarAction,
    services::{SystemClipboardService, SystemInputMethodGuard},
};
use ferroboard_core::{
    editor::{InputAttributes, InputAttributesValues, InputKind, TextBufferTarget},
    keys::KeyIcon,
};
use ferroui_android::log::{self, LogPriority};
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::TextBox;
use std::{cell::Cell, rc::Rc, sync::Arc};
type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn install(keyboard: &Ref<OnScreenKeyboard>, field: &Ref<TextBox>) {
    let actions = keyboard
        .smartbar_actions()
        .expect("public action collection");
    let original = actions
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    actions
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
    let busy = Rc::new(Cell::new(false));
    for (name, denied) in [("Clipboard probe", false), ("Clipboard denial", true)] {
        let keyboard = keyboard.downgrade();
        let field = field.downgrade();
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
                        log::write(LogPriority::Warn, "ferroboard-clipboard", "BUSY");
                        return Ok(());
                    }
                    let keyboard = keyboard.upgrade().expect("live probe keyboard");
                    let field = field.upgrade().expect("live probe field");
                    let busy = busy.clone();
                    drop(Dispatcher::current_dispatcher().invoke_async_task_local(
                        move || async move {
                            let result = if denied {
                                denial(&keyboard).await
                            } else {
                                privacy(&keyboard, &field).await
                            };
                            match result {
                                Ok(count) => log::write(
                                    LogPriority::Info,
                                    "ferroboard-clipboard",
                                    &format!("PASS {name} {count}"),
                                ),
                                Err(error) => log::write(
                                    LogPriority::Error,
                                    "ferroboard-clipboard",
                                    &format!("FAIL {name}: {error}"),
                                ),
                            }
                            busy.set(false);
                        },
                    ));
                    Ok(())
                })),
                None,
            )));
    }
    for action in original {
        actions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(action);
    }
}
fn require(condition: bool, message: &str) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned().into())
    }
}
async fn sample(keyboard: &Ref<OnScreenKeyboard>) {
    keyboard.refresh_system_clipboard();
    for _ in 0..3 {
        Dispatcher::yield_now().await;
    }
}
fn attach(keyboard: &Ref<OnScreenKeyboard>, attributes: InputAttributesValues) -> ProbeResult<()> {
    keyboard.attach(Rc::new(TextBufferTarget::new(
        "".into(),
        Some(Arc::new(InputAttributes::from_values(attributes))),
    )))?;
    Ok(())
}
async fn denial(keyboard: &Ref<OnScreenKeyboard>) -> ProbeResult<usize> {
    let service = SystemClipboardService::for_visual(keyboard.clone().upcast());
    require(
        service.read_text().await?.is_none(),
        "denied native read must return no text",
    )?;
    require(
        keyboard
            .engine()
            .clipboard_service()
            .get_text_async()
            .await?
            .is_none(),
        "engine fallback must receive no denied text",
    )?;
    Ok(2)
}
async fn privacy(keyboard: &Ref<OnScreenKeyboard>, field: &Ref<TextBox>) -> ProbeResult<usize> {
    let engine = keyboard.engine();
    let settings = engine.settings();
    let old_auto = keyboard.auto_attach();
    let old_enabled = settings.clipboard_history_enabled();
    let old_incognito = settings.incognito_mode();
    let _ime = SystemInputMethodGuard::new(field);
    settings.set_incognito_mode(true);
    keyboard.set_auto_attach(false);
    settings.set_clipboard_history_enabled(true);
    engine.clipboard_history().clear(true);
    let service = SystemClipboardService::for_visual(keyboard.clone().upcast());
    let result = async {
        let text = "FerroBoard 📋 e\u{301}\0native";
        service.write_text(text).await?;
        let observed = service.read_text().await?;
        require(
            observed.as_deref() == Some(text),
            &format!("Unicode/NUL native roundtrip: {observed:?}"),
        )?;
        attach(keyboard, InputAttributesValues::default())?;
        settings.set_incognito_mode(false);
        sample(keyboard).await;
        require(
            engine.clipboard_history().items()?.is_empty(),
            "first sample must only establish baseline",
        )?;
        service.write_text("public one").await?;
        sample(keyboard).await;
        require(
            engine.clipboard_history().items()?.len() == 1,
            "public copy must enter history",
        )?;
        for attrs in [
            InputAttributesValues {
                is_private: true,
                ..Default::default()
            },
            InputAttributesValues {
                kind: InputKind::Password,
                ..Default::default()
            },
            InputAttributesValues {
                kind: InputKind::NumericPassword,
                ..Default::default()
            },
        ] {
            attach(keyboard, attrs)?;
            service.write_text("hidden private copy").await?;
            sample(keyboard).await;
            require(
                engine.clipboard_history().items()?.len() == 1,
                "private/password copy entered history",
            )?;
            attach(keyboard, InputAttributesValues::default())?;
            sample(keyboard).await;
            require(
                engine.clipboard_history().items()?.len() == 1,
                "private copy replayed on target replacement",
            )?;
        }
        settings.set_incognito_mode(true);
        service.write_text("hidden incognito copy").await?;
        sample(keyboard).await;
        require(
            engine.clipboard_history().items()?.len() == 1,
            "incognito copy entered history",
        )?;
        settings.set_incognito_mode(false);
        sample(keyboard).await;
        require(
            engine.clipboard_history().items()?.len() == 1,
            "incognito copy replayed",
        )?;
        settings.set_clipboard_history_enabled(false);
        service.write_text("hidden disabled copy").await?;
        sample(keyboard).await;
        settings.set_clipboard_history_enabled(true);
        sample(keyboard).await;
        require(
            engine.clipboard_history().items()?.len() == 1,
            "disabled-history copy replayed",
        )?;
        service.write_text("public two").await?;
        sample(keyboard).await;
        let items = engine.clipboard_history().items()?;
        require(
            items.len() == 2
                && items
                    .iter()
                    .all(|i| i.text == "public one".into() || i.text == "public two".into()),
            "history contains a hidden copy or missed public copy",
        )?;
        Ok(13)
    }
    .await;
    settings.set_incognito_mode(old_incognito);
    settings.set_clipboard_history_enabled(old_enabled);
    keyboard.set_auto_attach(old_auto);
    keyboard.attach_to(Some(field.clone().upcast()))?;
    field.focus();
    require(
        keyboard.target_element() == Some(field.clone().upcast())
            && engine.attributes().kind() == InputKind::Email
            && engine.is_incognito(),
        "native private field target was not restored",
    )?;
    result.map(|count| count + 1)
}
