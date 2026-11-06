//! Internal port of KeyboardEngine.Input.cs shift methods and KeyboardEngine.SetShiftState.
//! Effects are invoked in source order with no outstanding borrow across reentrant engine callbacks.
use crate::keys::{KeyboardMode, ShiftState};
use std::{
    cell::Cell,
    time::{Duration, SystemTime},
};
pub(crate) trait ShiftEffects {
    type Error;
    fn recompute_keyboard(&self) -> Result<(), Self::Error>;
    fn state_changed(&self) -> Result<(), Self::Error>;
    fn update_auto_capitalization(&self) -> Result<(), Self::Error>;
}
pub(crate) struct EngineShiftState {
    state: Cell<ShiftState>,
    held: Cell<bool>,
    used: Cell<bool>,
    dirty: Cell<bool>,
    last_up: Cell<Option<SystemTime>>,
}
impl Default for EngineShiftState {
    fn default() -> Self {
        Self {
            state: Cell::new(ShiftState::Unshifted),
            held: Cell::new(false),
            used: Cell::new(false),
            dirty: Cell::new(false),
            last_up: Cell::new(None),
        }
    }
}
impl EngineShiftState {
    pub fn assign_state(&self, state: ShiftState) {
        self.state.set(state);
    }
    pub fn state(&self) -> ShiftState {
        self.state.get()
    }
    pub fn held(&self) -> bool {
        self.held.get()
    }
    pub fn set_state<E: ShiftEffects>(
        &self,
        state: ShiftState,
        effects: &E,
    ) -> Result<(), E::Error> {
        if self.state() == state {
            return Ok(());
        }
        self.state.set(state);
        effects.recompute_keyboard()?;
        effects.state_changed()
    }
    pub fn down<E: ShiftEffects>(
        &self,
        clock: impl FnOnce() -> Result<SystemTime, E::Error>,
        double_tap: impl FnOnce() -> bool,
        delay_ms: impl FnOnce() -> i32,
        effects: &E,
    ) -> Result<(), E::Error> {
        let now = clock()?;
        self.held.set(true);
        self.used.set(false);
        if double_tap()
            && self.state() == ShiftState::ShiftedManual
            && crate::editor::within_window(
                now,
                self.last_up.get(),
                Duration::from_millis(
                    u64::try_from(delay_ms()).expect(
                        "KeyboardSettings clamps double-tap delay to positive milliseconds",
                    ),
                ),
            )
        {
            self.set_state(ShiftState::CapsLock, effects)
        } else if self.state() == ShiftState::Unshifted {
            self.set_state(ShiftState::ShiftedManual, effects)
        } else {
            self.set_state(ShiftState::Unshifted, effects)
        }
    }
    pub fn up<E: ShiftEffects>(
        &self,
        clock: impl FnOnce() -> Result<SystemTime, E::Error>,
        effects: &E,
    ) -> Result<(), E::Error> {
        self.held.set(false);
        self.last_up.set(Some(clock()?));
        if self.used.get() && self.state() != ShiftState::CapsLock {
            self.last_up.set(None);
            self.set_state(ShiftState::Unshifted, effects)?;
            effects.update_auto_capitalization()?;
        }
        self.used.set(false);
        Ok(())
    }
    pub fn cancel(&self) {
        self.held.set(false);
    }
    /// Called at the source OnKeyUp tail after a non-shift key finished processing.
    pub fn mark_key_released(&self) {
        if self.held() {
            self.used.set(true);
        }
    }
    pub fn after_character(&self) {
        if matches!(
            self.state(),
            ShiftState::ShiftedManual | ShiftState::ShiftedAutomatic
        ) && !self.held()
        {
            self.state.set(ShiftState::Unshifted);
            self.dirty.set(true);
        }
    }
    /// The closure evaluates the source settings/editor/attribute capitalization expression lazily.
    pub fn update_auto<E: ShiftEffects>(
        &self,
        has_editor: bool,
        mode: KeyboardMode,
        should_cap: impl FnOnce() -> Result<bool, E::Error>,
        effects: &E,
    ) -> Result<(), E::Error> {
        let mut new_state = self.state();
        if has_editor
            && mode == KeyboardMode::Characters
            && !matches!(
                self.state(),
                ShiftState::CapsLock | ShiftState::ShiftedManual
            )
            && !self.held()
        {
            new_state = if should_cap()? {
                ShiftState::ShiftedAutomatic
            } else {
                ShiftState::Unshifted
            };
        }
        if new_state != self.state() || self.dirty.get() {
            self.state.set(new_state);
            self.dirty.set(false);
            effects.recompute_keyboard()?;
            effects.state_changed()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_clock;
    use serde_json::{Value, json};
    use std::{cell::RefCell, rc::Rc};
    fn rows() -> Vec<Value> {
        include_str!("../../../tests/fixtures/engine-shift.jsonl")
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
    fn snap(s: &EngineShiftState) -> Value {
        json!({"state":s.state().0,"held":s.held(),"used":s.used.get(),"dirty":s.dirty.get(),"last":s.last_up.get().map_or(0, fixture_clock::ticks)})
    }
    struct Effects {
        shift: Rc<EngineShiftState>,
        log: RefCell<Vec<Value>>,
        attached: bool,
        cap: bool,
        fail_keyboard: Cell<bool>,
        mode: RefCell<Option<String>>,
        once: Cell<bool>,
    }
    impl ShiftEffects for Effects {
        type Error = &'static str;
        fn recompute_keyboard(&self) -> Result<(), Self::Error> {
            self.log
                .borrow_mut()
                .push(json!({"kind":"keyboard","snapshot":snap(&self.shift)}));
            let mode = self.mode.borrow().clone();
            if mode.as_deref() == Some("keyboard-throw") {
                return Err("keyboard");
            }
            if mode.as_deref() == Some("keyboard-reenter") && !self.once.replace(true) {
                self.shift.set_state(ShiftState::CapsLock, self)?;
            }
            if self.fail_keyboard.get() {
                return Err("compute failed");
            }
            Ok(())
        }
        fn state_changed(&self) -> Result<(), Self::Error> {
            self.log
                .borrow_mut()
                .push(json!({"kind":"state","snapshot":snap(&self.shift)}));
            let mode = self.mode.borrow().clone();
            if mode.as_deref() == Some("state-reenter") && !self.once.replace(true) {
                self.shift.set_state(ShiftState::CapsLock, self)?;
            }
            Ok(())
        }
        fn update_auto_capitalization(&self) -> Result<(), Self::Error> {
            self.shift.update_auto(
                self.attached,
                KeyboardMode::Characters,
                || Ok(self.cap),
                self,
            )
        }
    }
    fn effects(attached: bool, cap: bool) -> Effects {
        Effects {
            shift: Rc::new(EngineShiftState::default()),
            log: RefCell::new(Vec::new()),
            attached,
            cap,
            fail_keyboard: Cell::new(false),
            mode: RefCell::new(None),
            once: Cell::new(false),
        }
    }
    #[test]
    fn source_shift_timing_chording_and_event_snapshots() {
        for row in rows().iter().filter(|r| r["kind"] == "timing") {
            let e = effects(false, false);
            let initial = match row["initial"].as_i64().unwrap() {
                0 => ShiftState::Unshifted,
                1 => ShiftState::ShiftedManual,
                2 => ShiftState::ShiftedAutomatic,
                3 => ShiftState::CapsLock,
                _ => unreachable!(),
            };
            e.shift.set_state(initial, &e).unwrap();

            let now = Cell::new(1_000_000_000_000);
            let calls = Cell::new(0);
            let clock = || {
                calls.set(calls.get() + 1);
                Ok(fixture_clock::time(now.get()))
            };
            let mut downs = 0;
            for op in row["ops"].as_array().unwrap() {
                e.log.borrow_mut().clear();
                match op["name"].as_str().unwrap() {
                    "down" => {
                        if downs > 0 {
                            now.set(now.get() + row["delta"].as_i64().unwrap());
                        }
                        downs += 1;
                        e.shift
                            .down(clock, || row["enabled"].as_bool().unwrap(), || 300, &e)
                            .unwrap();
                    }
                    "mark" => e.shift.mark_key_released(),
                    "up" => e.shift.up(clock, &e).unwrap(),
                    "cancel" => e.shift.cancel(),
                    _ => unreachable!(),
                };
                assert_eq!(snap(&e.shift), op["snapshot"], "{row} {op}");
                assert_eq!(json!(*e.log.borrow()), op["events"], "{row} {op}");
                assert_eq!(json!(calls.get()), op["calls"]);
            }
        }
    }
    #[test]
    fn source_after_character_dirty_shift_and_auto_capitalization() {
        for row in rows().iter().filter(|r| r["kind"] == "auto") {
            let e = effects(
                row["attached"].as_bool().unwrap(),
                row["cap"].as_bool().unwrap(),
            );
            let state = match row["initial"].as_i64().unwrap() {
                0 => ShiftState::Unshifted,
                1 => ShiftState::ShiftedManual,
                2 => ShiftState::ShiftedAutomatic,
                3 => ShiftState::CapsLock,
                _ => unreachable!(),
            };
            e.shift.set_state(state, &e).unwrap();
            e.log.borrow_mut().clear();
            e.shift.after_character();
            assert_eq!(snap(&e.shift), row["after"]);
            e.update_auto_capitalization().unwrap();
            assert_eq!(snap(&e.shift), row["final"]);
            assert_eq!(json!(*e.log.borrow()), row["events"]);
        }
    }
    #[test]
    fn interrupted_effects_leave_source_partial_state_and_suppress_later_events() {
        let e = effects(false, false);
        e.fail_keyboard.set(true);
        assert!(
            e.shift
                .down(|| Ok(fixture_clock::time(1_000_000)), || true, || 300, &e)
                .is_err()
        );
        assert_eq!(e.shift.state(), ShiftState::ShiftedManual);
        assert!(e.shift.held());
        assert_eq!(e.log.borrow().len(), 1);
        e.shift.used.set(true);
        let err = e.shift.up(|| Ok(fixture_clock::time(2_000_000)), &e);
        assert!(err.is_err());
        assert!(e.shift.used.get());
        assert!(!e.shift.held());
        assert_eq!(e.shift.last_up.get(), None);
    }
    #[test]
    fn source_reentrant_clock_and_notification_mutations() {
        for row in rows().iter().filter(|r| r["kind"] == "reentrant") {
            let e = effects(false, false);
            let time = fixture_clock::time(1_000_000_000_000);
            let mode = row["mode"].as_str().unwrap();
            if mode == "clock-config" {
                e.shift.down(|| Ok(time), || true, || 300, &e).unwrap();
                e.shift.up(|| Ok(time), &e).unwrap();
            }
            e.log.borrow_mut().clear();
            *e.mode.borrow_mut() = Some(mode.into());
            let enabled = Cell::new(true);
            let actual = e.shift.down(
                || {
                    if mode == "clock-state" {
                        e.shift.set_state(ShiftState::CapsLock, &e).unwrap();
                    }
                    if mode == "clock-config" {
                        enabled.set(false);
                    }
                    Ok(time)
                },
                || enabled.get(),
                || 300,
                &e,
            );
            assert_eq!(
                json!(actual.err().map(|_| "ApplicationException")),
                row["error"]
            );
            assert_eq!(snap(&e.shift), row["final"], "{row}");
            assert_eq!(json!(*e.log.borrow()), row["events"], "{row}");
        }
    }
    #[test]
    fn source_clock_failures_keep_distinct_down_and_up_partial_states() {
        for row in rows().iter().filter(|r| r["kind"] == "clock-failure") {
            let e = effects(false, false);
            e.shift
                .down(
                    || Ok(fixture_clock::time(1_000_000_000_000)),
                    || true,
                    || 300,
                    &e,
                )
                .unwrap();
            e.shift.mark_key_released();
            assert_eq!(snap(&e.shift), row["before"]);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if row["operation"] == "down" {
                    e.shift
                        .down(|| panic!("clock"), || true, || 300, &e)
                        .unwrap();
                } else {
                    e.shift.up(|| panic!("clock"), &e).unwrap();
                }
            }));
            assert!(outcome.is_err());
            assert_eq!(snap(&e.shift), row["after"]);
        }
    }

    #[test]
    fn returned_clock_errors_preserve_the_source_down_and_up_failure_states() {
        for row in rows().iter().filter(|r| r["kind"] == "clock-failure") {
            let e = effects(false, false);
            e.shift
                .down(
                    || Ok(fixture_clock::time(1_000_000_000_000)),
                    || true,
                    || 300,
                    &e,
                )
                .unwrap();
            e.shift.mark_key_released();
            assert_eq!(snap(&e.shift), row["before"]);
            let result = if row["operation"] == "down" {
                e.shift.down(|| Err("clock"), || true, || 300, &e)
            } else {
                e.shift.up(|| Err("clock"), &e)
            };
            assert_eq!(result, Err("clock"));
            assert_eq!(snap(&e.shift), row["after"]);
        }
    }
}
