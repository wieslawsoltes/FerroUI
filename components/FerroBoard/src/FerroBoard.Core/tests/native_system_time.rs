//! Native clock API and event timing without DateTime/Kind/range emulation.
use ferroboard_core::{
    editor::{EditorClock, EditorController, ITextInputTarget, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    keys::{KeyCode, KeyType, ShiftState},
    layouts::{KeyboardResources, TextKeyData},
    settings::KeyboardSettings,
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn native_clock_accepts_pre_unix_and_beyond_calendar_wire_times() {
    let future = UNIX_EPOCH
        .checked_add(Duration::from_secs(1_000_000_000_000))
        .unwrap();
    for point in [UNIX_EPOCH - Duration::from_secs(1), future] {
        let callback: EditorClock = Rc::new(move || point);
        let target = Rc::new(TextBufferTarget::default());
        let editor = EditorController::new(target.clone());
        editor.set_clock(callback.clone());
        assert!(Rc::ptr_eq(&callback, &editor.clock().unwrap()));
        let returned: SystemTime = editor.clock().unwrap()();
        assert_eq!(returned, point);
        editor.commit_text(&"a".into()).unwrap();
        editor.commit_text(&"b".into()).unwrap();
        assert_eq!(target.text(), "ab".into());
        editor.undo().unwrap();
        assert_eq!(target.text(), "".into());
    }
}

#[test]
fn native_resolution_boundaries_and_backwards_adjustments_preserve_grouping_rules() {
    // SystemTime stores 100ns units on Windows, while Unix targets can retain 1ns.
    let quantum = if UNIX_EPOCH + Duration::from_nanos(1) > UNIX_EPOCH {
        Duration::from_nanos(1)
    } else {
        Duration::from_nanos(100)
    };
    assert!(UNIX_EPOCH + quantum > UNIX_EPOCH);
    for (window, next, grouped) in [
        (quantum * 2, UNIX_EPOCH + quantum, true),
        (quantum * 2, UNIX_EPOCH + quantum * 2, false),
        (quantum * 2, UNIX_EPOCH + quantum * 3, false),
        (Duration::ZERO, UNIX_EPOCH - quantum, true),
    ] {
        let now = Rc::new(Cell::new(UNIX_EPOCH));
        let clock = now.clone();
        let target = Rc::new(TextBufferTarget::default());
        let editor = EditorController::new(target.clone());
        editor.set_clock(Rc::new(move || clock.get()));
        editor.set_undo_group_timeout(window);
        editor.commit_text(&"a".into()).unwrap();
        now.set(next);
        editor.commit_text(&"b".into()).unwrap();
        editor.undo().unwrap();
        assert_eq!(
            target.text(),
            if grouped { "" } else { "a" }.into(),
            "{window:?} {next:?}"
        );
    }
}

#[test]
fn unset_previous_events_cannot_create_double_space_or_caps_lock() {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::new("a ".into(), None));
    engine.attach(target.clone()).unwrap();
    engine.set_clock(Rc::new(|| UNIX_EPOCH));
    engine.set_double_space_window(Duration::MAX);
    engine.input_key(TextKeyData::space_key().as_ref()).unwrap();
    assert_eq!(target.text(), "a  ".into());
    engine.set_shift_state(ShiftState::ShiftedManual).unwrap();
    engine
        .on_key_down(TextKeyData::function(KeyCode::Shift, KeyType::Modifier, None).as_ref())
        .unwrap();
    assert_eq!(engine.shift_state(), ShiftState::Unshifted);
    engine.dispose().unwrap();
}
