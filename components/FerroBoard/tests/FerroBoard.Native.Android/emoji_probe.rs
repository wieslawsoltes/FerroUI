//! Test-only observations of the actual emoji panel, private search and native targets.
use ferroboard_controls::{
    OnScreenKeyboard, ScreenKeyboardHost, SmartbarAction, input::ProtectedPasswordTextBox,
    primitives::UniformWrapPanel,
};
use ferroboard_core::{
    editor::{ITextInputTarget, InputKind},
    emoji::EmojiSkinTone,
    keys::{KeyIcon, KeyboardUiMode},
    text::Utf16Text,
};
use ferroui_base::{Ref, WeakRef, input::InputMethod};
use ferroui_controls::{
    Control, ScrollViewer, StackPanel, TextBlock, TextBox, automation::AutomationProperties,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct ProbeState {
    phase: Cell<usize>,
    saved: RefCell<Option<Weak<dyn ITextInputTarget>>>,
    searches: Cell<usize>,
    failure: RefCell<Option<&'static str>>,
    scroller: RefCell<Option<WeakRef<ScrollViewer>>>,
    history: RefCell<Vec<HistorySnapshot>>,
    before_pin: Cell<usize>,
}
#[derive(Debug, PartialEq, Eq)]
struct HistorySnapshot {
    recent: Vec<Option<Utf16Text>>,
    pinned: Vec<Option<Utf16Text>>,
}
fn history(recent: &[&str], pinned: &[&str]) -> HistorySnapshot {
    HistorySnapshot {
        recent: recent.iter().map(|value| Some((*value).into())).collect(),
        pinned: pinned.iter().map(|value| Some((*value).into())).collect(),
    }
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
    let password = fields
        .children()
        .get(1)
        .cast::<ProtectedPasswordTextBox>()
        .unwrap();
    let public = fields.children().get(5).cast::<TextBox>().unwrap();
    // Assert the value through native accessibility as well as the input target.
    // An absent native value is presented as the control name by Android's peer.
    public.set_text(Some(""));
    let status = fields.children().get(6).cast::<TextBlock>().unwrap();
    let keyboard = host.keyboard();
    let engine = keyboard.engine();
    let settings = engine.settings();
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    settings.set_learn_words(true);
    settings.set_emoji_history_enabled(true);
    settings.set_clipboard_history_enabled(false);
    engine
        .language_models()
        .get_user_dictionary(&"en".into())
        .set_learn_threshold(1);
    let state = Rc::new(ProbeState::default());
    let observed = state.clone();
    let weak_keyboard = keyboard.downgrade();
    engine.state_changed().add(Rc::new(move |engine| {
        if engine.attributes().kind() == InputKind::Search {
            observed.searches.set(observed.searches.get() + 1);
            if !engine.is_incognito()
                || engine.attributes().allow_suggestions()
                || engine.attributes().allow_auto_correct()
            {
                *observed.failure.borrow_mut() = Some("internal query lacks private search policy");
            }
        }
        if engine.ui_mode() == KeyboardUiMode::Media
            && observed.scroller.borrow().is_none()
            && let Some(keyboard) = weak_keyboard.upgrade()
        {
            let scroll = keyboard
                .get_visual_descendants()
                .filter_map(|visual| visual.downcast::<ScrollViewer>().ok())
                .find(|scroll| {
                    scroll
                        .content()
                        .as_ref()
                        .and_then(Control::from_boxed)
                        .is_some_and(|content| content.downcast::<UniformWrapPanel>().is_ok())
                });
            if let Some(scroll) = scroll {
                // Fixture identifier on the actual library scroll container. Gestures
                // are injected through Android; the fixture never sets its offset.
                AutomationProperties::set_automation_id(&scroll, Some("EmojiScrollBody"));
                *observed.scroller.borrow_mut() = Some(scroll.downgrade());
            }
        }
    }));
    let observed = state.clone();
    engine.emoji_history().changed().add(Rc::new(move |source| {
        observed.history.borrow_mut().push(HistorySnapshot {
            recent: source.recent().lock().clone(),
            pinned: source.pinned().lock().clone(),
        });
    }));
    let actions = keyboard.smartbar_actions().unwrap();
    let mut actions = actions
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Retain the real default Emoji action; the only custom action observes results.
    actions.retain(|action| {
        action
            .as_ref()
            .is_some_and(|action| action.id.as_ref() == Some(&"emoji".into()))
    });
    assert_eq!(actions.len(), 1);
    let weak_keyboard = keyboard.downgrade();
    let weak_password = password.downgrade();
    let weak_public = public.downgrade();
    let weak_status = status.downgrade();
    actions.insert(
        0,
        Some(SmartbarAction::new(
            Some("check".into()),
            KeyIcon::None,
            Some("Check".into()),
            Some(Rc::new(move |_| {
                let keyboard = weak_keyboard.upgrade().expect("live test keyboard");
                let password = weak_password.upgrade().expect("live test password");
                let public = weak_public.upgrade().expect("live test public field");
                let status = weak_status.upgrade().expect("live test status");
                let result = verify(&keyboard, &password, &public, &state);
                match result {
                    Ok(label) => status.set_text(Some(label)),
                    Err(error) => status.set_text(Some(&format!("FAIL {error}"))),
                }
                Ok(())
            })),
            None,
        )),
    );
    drop(actions);
    let password = password.downgrade();
    host.loaded(move |_, _| {
        if let Some(password) = password.upgrade() {
            password.focus();
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
fn verify(
    keyboard: &OnScreenKeyboard,
    password: &ProtectedPasswordTextBox,
    public: &TextBox,
    state: &ProbeState,
) -> ProbeResult<&'static str> {
    if let Some(failure) = *state.failure.borrow() {
        return Err(failure.into());
    }
    let phase = &state.phase;
    let saved = &state.saved;
    let searches = &state.searches;
    let engine = keyboard.engine();
    let target = engine.target().ok_or("missing restored native target")?;
    let n = phase.get();
    let private = !(3..14).contains(&n);
    let field: &TextBox = if private { password } else { public };
    require(n < 17, "unexpected extra Check action")?;
    require(
        field.is_focused() && !InputMethod::get_is_input_method_enabled(field),
        "native editing focus/IME ownership lost",
    )?;
    require(
        engine.ui_mode()
            == if (6..=12).contains(&n) {
                KeyboardUiMode::Media
            } else {
                KeyboardUiMode::Text
            },
        "keyboard did not return from emoji panel",
    )?;
    require(
        engine.attributes().kind()
            == if private {
                InputKind::Password
            } else {
                InputKind::Text
            },
        "search target was not restored",
    )?;
    require(
        engine.is_incognito() == private,
        "restored field privacy mismatch",
    )?;
    require(
        engine.settings().learn_words() && engine.settings().emoji_history_enabled(),
        "recording was globally disabled",
    )?;
    require(
        engine
            .language_models()
            .get_user_dictionary(&"en".into())
            .get_count(&"rocket".into())
            == 0,
        "private query entered learning",
    )?;
    let expected = match n {
        1 | 4 => "🚀",
        5..=8 => "🚀m",
        9..=11 | 13 => "🚀m👋🏽",
        12 => "🚀m👋🏽👋🏽",
        15 => "👋🏿",
        _ => "",
    };
    require(
        field.text().unwrap_or_default() == expected && target.text() == expected.into(),
        "native emoji/query text mismatch",
    )?;
    require(
        target.selection_start() == expected.encode_utf16().count() as i32
            && target.selection_length() == 0,
        "restored UTF16 caret mismatch",
    )?;
    if matches!(n, 0 | 3 | 14) {
        if matches!(n, 3 | 14) {
            require(
                saved
                    .borrow()
                    .as_ref()
                    .is_some_and(|old| old.upgrade().is_none()),
                "previous native adapter retained after focus replacement",
            )?;
        }
        *saved.borrow_mut() = Some(Rc::downgrade(&target));
        searches.set(0);
    } else {
        let original = saved
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or("original native adapter missing")?;
        require(
            Rc::ptr_eq(&original, &target),
            "emoji search replaced native adapter identity",
        )?;
        if n < 6 {
            require(searches.get() > 0, "real search state was never observed")?;
        }
    }
    let emoji_history = engine.emoji_history();
    let recent = emoji_history.recent().lock().clone();
    require(
        if n < 4 {
            recent.is_empty()
        } else if (9..=10).contains(&n) || n >= 12 {
            recent == [Some("👋🏽".into()), Some("🚀".into())]
        } else {
            recent == [Some("🚀".into())]
        },
        "private/public emoji history policy mismatch",
    )?;
    let pinned = emoji_history.pinned().lock().clone();
    require(
        if n == 10 {
            pinned == [Some("👋🏽".into())]
        } else {
            pinned.is_empty()
        },
        "native pin/removal mismatch",
    )?;
    if n >= 9 {
        require(
            emoji_history.get_preferred_tone(&"👋".into()) == Some(EmojiSkinTone::Medium),
            "native preferred tone missing",
        )?;
    }
    if (6..=8).contains(&n) {
        let scroll = state
            .scroller
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .ok_or("actual emoji scroll container missing")?;
        require(
            if n == 7 {
                scroll.offset().y > 0.0
            } else {
                scroll.offset().y == 0.0
            },
            "native touch scroll/category reset mismatch",
        )?;
    }
    if n == 9 {
        state.before_pin.set(state.history.borrow().len());
    }
    if n == 10 {
        require(
            state.history.borrow().len() == state.before_pin.get() + 1,
            "native pin notification count mismatch",
        )?;
    }
    if n == 11 {
        require(
            state.history.borrow()[state.before_pin.get()..]
                == [
                    history(&["👋🏽", "🚀"], &["👋🏽"]),
                    history(&["🚀"], &["👋🏽"]),
                    history(&["🚀"], &[]),
                ],
            "native remove/unpin notification order mismatch",
        )?;
    }
    if n >= 12 {
        require(
            state.history.borrow().len() == state.before_pin.get() + 4,
            "unexpected history/preference notification after deletion/private input",
        )?;
    }
    if n >= 14 {
        require(
            public.text().as_deref() == Some("🚀m👋🏽")
                && InputMethod::get_is_input_method_enabled(public),
            "private tone changed inactive public field/IME state",
        )?;
    }
    if !private {
        require(
            InputMethod::get_is_input_method_enabled(password),
            "inactive password retained IME guard",
        )?;
    }
    phase.set(n + 1);
    Ok([
        "PASS private initial",
        "PASS private selected",
        "PASS private cleared",
        "PASS public initial",
        "PASS public selected",
        "PASS public typed",
        "PASS category initial",
        "PASS category scrolled",
        "PASS category reset",
        "PASS tone selected",
        "PASS recent pinned",
        "PASS recent removed",
        "PASS preferred selected",
        "PASS tone deleted",
        "PASS private tone initial",
        "PASS private tone selected",
        "PASS private tone cleared",
    ][n])
}
