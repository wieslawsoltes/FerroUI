//! Native elapsed-window contracts: strict typing thresholds, inclusive expiry,
//! backwards wall clocks and durations beyond the old signed tick range.
#[path = "../../../tests/support/fixture_clipboard_time.rs"]
mod fixture_clipboard_time;
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{
    clipboard::ClipboardHistory,
    editor::{EditorController, ITextInputTarget, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    layouts::{KeyboardResources, TextKeyData},
    settings::KeyboardSettings,
};
use std::{cell::Cell, rc::Rc, time::Duration};

#[test]
fn undo_duration_preserves_strict_thresholds_backwards_clocks_and_full_native_range() {
    for (window, delta, grouped) in [
        (Duration::from_secs(1), 9_999_999, true),
        (Duration::from_secs(1), 10_000_000, false),
        (Duration::from_secs(1), 10_000_001, false),
        (Duration::ZERO, 0, false),
        (Duration::ZERO, -1, true),
        (Duration::from_nanos(1), 0, true),
        (Duration::from_nanos(1), 1, false),
        (Duration::MAX, 3_155_378_975_999_999_999 - 30_000_000, true),
    ] {
        let target = Rc::new(TextBufferTarget::default());
        let editor = EditorController::new(target.clone());
        let now = Rc::new(Cell::new(30_000_000));
        let clock = now.clone();
        editor.set_clock(Rc::new(move || fixture_clock::time(clock.get())));
        editor.set_undo_group_timeout(window);
        assert_eq!(editor.undo_group_timeout(), window);
        editor.commit_text(&"a".into()).unwrap();
        now.set(now.get() + delta);
        editor.commit_text(&"b".into()).unwrap();
        editor.undo().unwrap();
        assert_eq!(
            target.text(),
            if grouped { "" } else { "a" }.into(),
            "{window:?} {delta}"
        );
    }
}

#[test]
fn clipboard_duration_preserves_inclusive_expiry_pins_future_dates_and_no_expiry() {
    for (window, delta, survives) in [
        (Some(Duration::from_secs(1)), 9_999_999, true),
        (Some(Duration::from_secs(1)), 10_000_000, true),
        (Some(Duration::from_secs(1)), 10_000_001, false),
        (Some(Duration::ZERO), 0, true),
        (Some(Duration::ZERO), 1, false),
        (Some(Duration::ZERO), -1, true),
        (Some(Duration::from_nanos(99)), 1, false),
        (Some(Duration::from_nanos(100)), 1, true),
        (
            Some(Duration::MAX),
            3_155_378_975_999_999_999 - 30_000_000,
            true,
        ),
        (None, 3_155_378_975_999_999_999 - 30_000_000, true),
    ] {
        let history = ClipboardHistory::default();
        let now = Rc::new(Cell::new(30_000_000));
        let clock = now.clone();
        history.set_clock(Rc::new(move || {
            fixture_clipboard_time::timestamp(clock.get(), 0).unwrap()
        }));
        history.set_expire_after(window);
        assert_eq!(history.expire_after(), window);
        let pinned = history.add(Some(&"pinned".into())).unwrap().unwrap();
        history.set_pinned(&pinned, true);
        let item = history.add(Some(&"ordinary".into())).unwrap().unwrap();
        now.set(now.get() + delta);
        let items = history.items().unwrap();
        assert!(Rc::ptr_eq(&items[0], &pinned));
        assert_eq!(
            items.iter().any(|i| Rc::ptr_eq(i, &item)),
            survives,
            "{window:?} {delta}"
        );
    }
}

#[test]
fn native_zero_and_large_double_space_windows_keep_the_engine_path() {
    for (window, expected) in [(Duration::ZERO, "a  "), (Duration::MAX, "a. ")] {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        settings.set_auto_correct(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        engine.set_clock(Rc::new(|| fixture_clock::time(30_000_000)));
        engine.set_double_space_window(window);
        assert_eq!(engine.double_space_window(), window);
        engine.type_text(&"a".into()).unwrap();
        engine.input_key(TextKeyData::space_key().as_ref()).unwrap();
        engine.input_key(TextKeyData::space_key().as_ref()).unwrap();
        assert_eq!(target.text(), expected.into());
        engine.dispose().unwrap();
    }
}
