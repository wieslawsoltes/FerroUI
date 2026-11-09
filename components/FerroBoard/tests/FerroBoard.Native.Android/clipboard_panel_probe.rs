//! Deterministic history fixtures; interactions use the real panel and native target.
use ferroboard_controls::{OnScreenKeyboard, ScreenKeyboardHost, SmartbarAction};
use ferroboard_core::{
    clipboard::OffsetDateTime,
    editor::ITextInputTarget,
    keys::{KeyIcon, KeyboardUiMode},
};
use ferroui_base::{Ref, input::InputMethod};
use ferroui_controls::{Control, StackPanel, TextBlock, TextBox};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;
#[derive(Default)]
struct State {
    phase: Cell<usize>,
    tick: Cell<i64>,
    seeds: Cell<usize>,
    advances: Cell<usize>,
    target: RefCell<Option<Weak<dyn ITextInputTarget>>>,
}
pub fn view() -> Ref<Control> {
    let value = ferroboard_native_consumer::compiled_xaml::try_load(
        None,
        &format!(
            "{}PrivacyHosted.xaml",
            ferroboard_native_consumer::compiled_xaml::ROOT_URI
        ),
    )
    .unwrap()
    .expect("dependency compiled native fields");
    let host = value
        .downcast_ref::<Ref<ScreenKeyboardHost>>()
        .unwrap()
        .clone();
    let fields = host.content().unwrap().cast::<StackPanel>().unwrap();
    let public = fields.children().get(5).cast::<TextBox>().unwrap();
    public.set_text(Some(""));
    let status = fields.children().get(6).cast::<TextBlock>().unwrap();
    let keyboard = host.keyboard();
    let engine = keyboard.engine();
    let settings = engine.settings();
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    // OS monitoring is intentionally disabled: this fixture tests panel operations
    // with exact history timestamps, independently of the native clipboard suite.
    settings.set_clipboard_history_enabled(false);
    let state = Rc::new(State::default());
    let clock = state.clone();
    engine.clipboard_history().set_clock(Rc::new(move || {
        OffsetDateTime::UNIX_EPOCH
            + std::time::Duration::from_nanos(
                u64::try_from(clock.tick.get()).expect("nonnegative fixture clock") * 100,
            )
    }));
    engine
        .clipboard_history()
        .set_expire_after(Some(std::time::Duration::from_micros(10)));
    let actions = keyboard.smartbar_actions().unwrap();
    let mut actions = actions
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    actions.retain(|action| {
        action
            .as_ref()
            .is_some_and(|action| action.id.as_ref() == Some(&"clipboard".into()))
    });
    assert_eq!(actions.len(), 1);
    for name in ["Seed", "Advance", "Check"] {
        let weak_keyboard = keyboard.downgrade();
        let weak_public = public.downgrade();
        let weak_status = status.downgrade();
        let state = state.clone();
        actions.insert(
            0,
            Some(SmartbarAction::new(
                Some(name.into()),
                KeyIcon::None,
                Some(name.into()),
                Some(Rc::new(move |_| {
                    let keyboard = weak_keyboard.upgrade().expect("live keyboard");
                    let public = weak_public.upgrade().expect("live field");
                    let status = weak_status.upgrade().expect("live status");
                    let result = match name {
                        "Seed" => seed(&keyboard, &state),
                        "Advance" => {
                            let delta = if state.advances.get() == 0 { 100 } else { 1 };
                            state.tick.set(state.tick.get() + delta);
                            state.advances.set(state.advances.get() + 1);
                            Ok("ADVANCED")
                        }
                        _ => verify(&keyboard, &public, &state),
                    };
                    match result {
                        Ok(label) => status.set_text(Some(label)),
                        Err(error) => status.set_text(Some(&format!("FAIL {error}"))),
                    }
                    Ok(())
                })),
                None,
            )),
        );
    }
    drop(actions);
    let public = public.downgrade();
    host.loaded(move |_, _| {
        if let Some(public) = public.upgrade() {
            public.focus();
        }
    });
    host.upcast()
}
fn seed(keyboard: &OnScreenKeyboard, state: &State) -> ProbeResult<&'static str> {
    let engine = keyboard.engine();
    let history = engine.clipboard_history();
    let values = if state.seeds.get() == 0 {
        ["Alpha🙂", "Betaé"]
    } else {
        ["Gamma", "Delta"]
    };
    for value in values {
        history.add(Some(&value.into()))?;
    }
    state.seeds.set(state.seeds.get() + 1);
    Ok("SEEDED")
}
fn require(condition: bool, message: &'static str) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn verify(
    keyboard: &OnScreenKeyboard,
    public: &TextBox,
    state: &State,
) -> ProbeResult<&'static str> {
    let labels = [
        "PASS initial",
        "PASS pinned",
        "PASS pasted",
        "PASS boundary",
        "PASS expired",
        "PASS clear pinned",
        "PASS unpinned expired",
        "PASS disabled",
        "PASS enabled",
        "PASS reseeded",
        "PASS deleted",
        "PASS cleared",
        "PASS returned",
    ];
    let n = state.phase.get();
    let label = *labels.get(n).ok_or("unexpected Check action")?;
    let engine = keyboard.engine();
    let target = engine.target().ok_or("native target detached")?;
    require(
        public.is_focused() && !InputMethod::get_is_input_method_enabled(public),
        "native focus/IME ownership lost",
    )?;
    if n == 0 {
        *state.target.borrow_mut() = Some(Rc::downgrade(&target));
    }
    let saved = state
        .target
        .borrow()
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or("original native target released")?;
    require(Rc::ptr_eq(&saved, &target), "panel replaced editing target")?;
    let text = if n < 2 {
        ""
    } else if n == 12 {
        "Alpha🙂m"
    } else {
        "Alpha🙂"
    };
    require(
        public.text().unwrap_or_default() == text && target.text() == text.into(),
        "panel edit or nested action text mismatch",
    )?;
    require(
        target.selection_start() == text.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "UTF16 caret mismatch",
    )?;
    require(
        engine.ui_mode()
            == if n == 12 {
                KeyboardUiMode::Text
            } else {
                KeyboardUiMode::Clipboard
            },
        "wrong panel mode",
    )?;
    let expected: &[(&str, bool)] = match n {
        0 => &[("Betaé", false), ("Alpha🙂", false)],
        1..=3 => &[("Alpha🙂", true), ("Betaé", false)],
        4..=5 => &[("Alpha🙂", true)],
        9 => &[("Delta", false), ("Gamma", false)],
        10 => &[("Delta", false)],
        _ => &[],
    };
    let items = engine.clipboard_history().items()?;
    require(
        items.len() == expected.len()
            && items.iter().zip(expected).all(|(item, (text, pinned))| {
                item.text == (*text).into() && item.is_pinned() == *pinned
            }),
        "history order/pinning/expiry mismatch",
    )?;
    require(
        engine.settings().clipboard_history_enabled() == (n != 7),
        "history setting mismatch",
    )?;
    state.phase.set(n + 1);
    Ok(label)
}
