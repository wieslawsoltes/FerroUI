//! Port of Input/KeyboardTouchProcessor.cs, including its scheduler and popup types (Apache-2.0).
use super::ScheduledAction;
use crate::{
    engine::{EngineError, KeyboardEngine},
    glide::GesturePoint,
    keys::{KeyCode, ShiftState},
    layouts::{ComputedKey, KeyDataError, KeyRect, PopupKeys, SharedList},
    settings::{KeyboardSettings, SwipeAction},
};
use ferroui_system::events::Event;
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

/// Delayed UI-thread callbacks. Retain the native guard until completion;
/// explicit cancellation or the final guard drop cancels pending work.
/// Scheduler implementations may invoke/reenter synchronously.
pub trait IKeyboardScheduler {
    fn schedule(&self, delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction;
}
#[derive(Debug)]
pub enum TouchError {
    Engine(EngineError),
    StepCountOverflow,
    NegativePopupCount,
    PopupCountOverflow,
    PopupAllocation,
    EmptyPopupCandidates,
    PointerCollectionModified,
}
pub type TouchResult<T> = Result<T, TouchError>;
impl From<EngineError> for TouchError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}
impl From<KeyDataError> for TouchError {
    fn from(error: KeyDataError) -> Self {
        Self::Engine(error.into())
    }
}
impl std::fmt::Display for TouchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TouchError {}

/// Shared fixed-size source KeyRect[]; element replacement preserves array identity.
pub type PopupBounds = Rc<RefCell<Box<[KeyRect]>>>;
pub struct KeyPopupState {
    key: Rc<ComputedKey>,
    keys: PopupKeys,
    bounds: PopupBounds,
    selected_index: Cell<i32>,
}
impl KeyPopupState {
    pub fn key(&self) -> Rc<ComputedKey> {
        self.key.clone()
    }
    pub fn keys(&self) -> PopupKeys {
        self.keys.clone()
    }
    pub fn bounds(&self) -> PopupBounds {
        self.bounds.clone()
    }
    pub fn selected_index(&self) -> i32 {
        self.selected_index.get()
    }
    pub fn frame(&self) -> KeyRect {
        let bounds = self.bounds.borrow();
        if bounds.is_empty() {
            return KeyRect::default();
        }
        let left = enumerable_min(bounds.iter().map(|b| b.left()));
        let top = enumerable_min(bounds.iter().map(|b| b.top()));
        let right = enumerable_max(bounds.iter().map(|b| b.right()));
        let bottom = enumerable_max(bounds.iter().map(|b| b.bottom()));
        KeyRect {
            x: left,
            y: top,
            width: sub(right, left),
            height: sub(bottom, top),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    NoKey,
    Pressed,
    Popup,
    Glide,
    SpaceCursor,
    PreciseDelete,
    Swipe,
    Consumed,
}
struct PointerState {
    id: i64,
    start_x: f64,
    start_y: f64,
    last_x: Cell<f64>,
    last_y: Cell<f64>,
    anchor_x: Cell<f64>,
    key: Option<Rc<ComputedKey>>,
    phase: Cell<Phase>,
    repeated: Cell<bool>,
    steps: Cell<i32>,
    long_press_timer: RefCell<Option<ScheduledAction>>,
    repeat_timer: RefCell<Option<ScheduledAction>>,
}
impl PointerState {
    fn new(id: i64, x: f64, y: f64, key: Option<Rc<ComputedKey>>) -> Self {
        Self {
            id,
            start_x: x,
            start_y: y,
            last_x: Cell::new(x),
            last_y: Cell::new(y),
            anchor_x: Cell::new(x),
            key,
            phase: Cell::new(Phase::NoKey),
            repeated: Cell::new(false),
            steps: Cell::new(0),
            long_press_timer: RefCell::new(None),
            repeat_timer: RefCell::new(None),
        }
    }
    fn cancel_timers(&self) {
        // Preserve cancel-before-clear order under reentrant cancellation callbacks.
        let timer = self.long_press_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.cancel();
        }
        let previous = self.long_press_timer.replace(None);
        drop(previous);
        let timer = self.repeat_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.cancel();
        }
        let previous = self.repeat_timer.replace(None);
        drop(previous);
    }
}
#[derive(Default)]
struct PointerMap {
    values: RefCell<HashMap<i64, (usize, Rc<PointerState>)>>,
    // Pointer lookup must not choose rollover, cancellation or pressed-key order. Contacts
    // keep their position until removed; the most recently freed position is reused.
    order: RefCell<Vec<Option<i64>>>,
    free: RefCell<Vec<usize>>,
    version: Cell<u32>,
}
impl PointerMap {
    fn insert(&self, id: i64, state: Rc<PointerState>) {
        let mut values = self.values.borrow_mut();
        if let Some((_, current)) = values.get_mut(&id) {
            let previous = std::mem::replace(current, state);
            // A completed scheduler handle can run caller code on final Drop. Publish
            // the replacement and release storage before retiring the old contact.
            drop(values);
            drop(previous);
            return;
        }
        let mut order = self.order.borrow_mut();
        let slot = if let Some(slot) = self.free.borrow_mut().pop() {
            order[slot] = Some(id);
            slot
        } else {
            order.push(Some(id));
            order.len() - 1
        };
        values.insert(id, (slot, state));
        self.version.set(self.version.get().wrapping_add(1));
    }
    fn get(&self, id: i64) -> Option<Rc<PointerState>> {
        self.values
            .borrow()
            .get(&id)
            .map(|(_, state)| state.clone())
    }
    fn remove(&self, id: i64) {
        if let Some((slot, _)) = self.values.borrow_mut().remove(&id) {
            self.order.borrow_mut()[slot] = None;
            self.free.borrow_mut().push(slot);
        }
    }
    fn contains(&self, id: i64) -> bool {
        self.values.borrow().contains_key(&id)
    }
    fn count(&self) -> usize {
        self.values.borrow().len()
    }
    fn ids(&self) -> Vec<i64> {
        self.order.borrow().iter().flatten().copied().collect()
    }
    fn snapshot(&self) -> Vec<Rc<PointerState>> {
        let values = self.values.borrow();
        self.order
            .borrow()
            .iter()
            .flatten()
            .map(|id| values[id].1.clone())
            .collect()
    }
}
/// Deferred pressed-key query; additions invalidate an active iterator. Removals are skipped.
pub struct PressedKeys {
    pointers: Rc<PointerMap>,
}
pub struct PressedKeysIterator {
    pointers: Rc<PointerMap>,
    keys: Option<Vec<i64>>,
    version: u32,
    index: usize,
    ended: bool,
}
impl PressedKeys {
    pub fn iter(&self) -> PressedKeysIterator {
        PressedKeysIterator {
            pointers: self.pointers.clone(),
            keys: None,
            version: 0,
            index: 0,
            ended: false,
        }
    }
}
impl IntoIterator for PressedKeys {
    type Item = TouchResult<Rc<ComputedKey>>;
    type IntoIter = PressedKeysIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl Iterator for PressedKeysIterator {
    type Item = TouchResult<Rc<ComputedKey>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        if self.keys.is_none() {
            self.version = self.pointers.version.get();
            self.keys = Some(self.pointers.ids());
        }
        if self.version != self.pointers.version.get() {
            return Some(Err(TouchError::PointerCollectionModified));
        }
        let keys = self.keys.as_ref().expect("initialized enumeration");
        while let Some(&id) = keys.get(self.index) {
            self.index += 1;
            if let Some(state) = self.pointers.get(id)
                && state.phase.get() == Phase::Pressed
                && let Some(key) = state.key.clone()
            {
                return Some(Ok(key));
            }
        }
        self.ended = true;
        None
    }
}
/// Native-independent source touch state machine. Native capture and coordinate conversion belong
/// to the control adapter. CancelAll emits input cancellation explicitly. Native Drop only stops
/// owned timers, including reentrantly retired contacts; it does not emit gestures or edit
/// borrowed engine/target state.
pub struct KeyboardTouchProcessor {
    self_ref: Weak<Self>,
    engine: Rc<KeyboardEngine>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    pointers: Rc<PointerMap>,
    // Reentrant callbacks can retire a contact before its timer is installed. Keep only
    // weak cleanup references; live state-machine behavior still follows the source.
    retired_pointers: RefCell<Vec<Weak<PointerState>>>,
    glide_trail: SharedList<GesturePoint>,
    popup: RefCell<Option<Rc<KeyPopupState>>>,
    popup_max_columns: Cell<i32>,
    popup_min_y: Cell<f64>,
    cursor_step_distance: Cell<f64>,
    delete_step_distance: Cell<f64>,
    key_pressed: Event<Rc<ComputedKey>>,
    key_released: Event<Rc<ComputedKey>>,
    popup_changed: Event<Self>,
    glide_trail_changed: Event<Self>,
    glide_ended: Event<Self>,
}
impl KeyboardTouchProcessor {
    pub fn new(engine: Rc<KeyboardEngine>, scheduler: Rc<dyn IKeyboardScheduler>) -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            engine,
            scheduler,
            pointers: Rc::new(PointerMap::default()),
            retired_pointers: RefCell::new(Vec::new()),
            glide_trail: Rc::default(),
            popup: RefCell::new(None),
            popup_max_columns: Cell::new(6),
            popup_min_y: Cell::new(-48.0),
            cursor_step_distance: Cell::new(14.0),
            delete_step_distance: Cell::new(16.0),
            key_pressed: Event::new(),
            key_released: Event::new(),
            popup_changed: Event::new(),
            glide_trail_changed: Event::new(),
            glide_ended: Event::new(),
        })
    }
    pub fn popup_max_columns(&self) -> i32 {
        self.popup_max_columns.get()
    }
    pub fn set_popup_max_columns(&self, value: i32) {
        self.popup_max_columns.set(value);
    }
    pub fn popup_min_y(&self) -> f64 {
        self.popup_min_y.get()
    }
    pub fn set_popup_min_y(&self, value: f64) {
        self.popup_min_y.set(value);
    }
    pub fn cursor_step_distance(&self) -> f64 {
        self.cursor_step_distance.get()
    }
    pub fn set_cursor_step_distance(&self, value: f64) {
        self.cursor_step_distance.set(value);
    }
    pub fn delete_step_distance(&self) -> f64 {
        self.delete_step_distance.get()
    }
    pub fn set_delete_step_distance(&self, value: f64) {
        self.delete_step_distance.set(value);
    }
    pub fn popup(&self) -> Option<Rc<KeyPopupState>> {
        self.popup.borrow().clone()
    }
    pub fn glide_trail(&self) -> SharedList<GesturePoint> {
        self.glide_trail.clone()
    }
    pub fn pressed_keys(&self) -> PressedKeys {
        PressedKeys {
            pointers: self.pointers.clone(),
        }
    }
    pub fn key_pressed(&self) -> &Event<Rc<ComputedKey>> {
        &self.key_pressed
    }
    pub fn key_released(&self) -> &Event<Rc<ComputedKey>> {
        &self.key_released
    }
    pub fn popup_changed(&self) -> &Event<Self> {
        &self.popup_changed
    }
    pub fn glide_trail_changed(&self) -> &Event<Self> {
        &self.glide_trail_changed
    }
    pub fn glide_ended(&self) -> &Event<Self> {
        &self.glide_ended
    }
    fn settings(&self) -> Rc<KeyboardSettings> {
        self.engine.settings()
    }
    pub fn pointer_down(&self, pointer_id: i64, x: f64, y: f64) -> TouchResult<()> {
        if self.pointers.contains(pointer_id) {
            self.pointer_cancel(pointer_id)?;
        }
        let others = self.pointers.snapshot();
        for other in others {
            if other.phase.get() == Phase::Pressed
                && let Some(key) = other.key.clone()
                && key.code() != KeyCode::Shift
                && !other.repeated.get()
                && other
                    .key
                    .as_ref()
                    .expect("source key")
                    .data()
                    .produces_text()?
            {
                other.cancel_timers();
                self.release(&other.key.as_ref().expect("source key").clone());
                self.engine.on_key_up(key.data().as_ref())?;
                other.phase.set(Phase::Consumed);
            }
        }
        let key = self.engine.keyboard()?.hit_test(x, y)?;
        let state = Rc::new(PointerState::new(pointer_id, x, y, key.clone()));
        self.pointers.insert(pointer_id, state.clone());
        let Some(key) = key else {
            state.phase.set(Phase::NoKey);
            return Ok(());
        };
        state.phase.set(Phase::Pressed);
        self.press(&key);
        self.engine.on_key_down(key.data().as_ref())?;
        if key.code() == KeyCode::Delete
            && self.settings().delete_key_long_press() != SwipeAction::DeleteCharacter
        {
            let timer = self.schedule(
                &state,
                Duration::from_millis(self.settings().long_press_delay() as u64),
                false,
            );
            self.install_timer(&state, timer, false);
        } else if KeyboardEngine::is_repeatable(key.data().as_ref()) {
            let timer = self.schedule(
                &state,
                Duration::from_millis(self.settings().key_repeat_delay() as u64),
                true,
            );
            self.install_timer(&state, timer, true);
        } else {
            let timer = self.schedule(
                &state,
                Duration::from_millis(self.settings().long_press_delay() as u64),
                false,
            );
            self.install_timer(&state, timer, false);
        }
        Ok(())
    }
    fn install_timer(&self, state: &Rc<PointerState>, timer: ScheduledAction, repeat: bool) {
        let previous = if repeat {
            state.repeat_timer.replace(Some(timer))
        } else {
            state.long_press_timer.replace(Some(timer))
        };
        drop(previous);
        let current = self
            .pointers
            .get(state.id)
            .is_some_and(|current| Rc::ptr_eq(&current, state));
        let mut retired = self.retired_pointers.borrow_mut();
        retired.retain(|pointer| pointer.strong_count() != 0);
        if !current {
            let pointer = Rc::downgrade(state);
            if !retired.iter().any(|old| Weak::ptr_eq(old, &pointer)) {
                retired.push(pointer);
            }
        }
    }
    fn schedule(&self, state: &Rc<PointerState>, delay: Duration, repeat: bool) -> ScheduledAction {
        let receiver = self.self_ref.clone();
        let state = state.clone();
        self.scheduler.schedule(
            delay,
            Rc::new(move || {
                let Some(receiver) = receiver.upgrade() else {
                    return;
                };
                let result = if repeat {
                    receiver.on_repeat(&state)
                } else {
                    receiver.on_long_press(&state)
                };
                result.expect("KeyboardTouchProcessor scheduled callback failed");
            }),
        )
    }
    pub fn pointer_move(&self, pointer_id: i64, x: f64, y: f64) -> TouchResult<()> {
        let Some(state) = self.pointers.get(pointer_id) else {
            return Ok(());
        };
        state.last_x.set(x);
        state.last_y.set(y);
        let dx = x - state.start_x;
        let dy = y - state.start_y;
        let distance = (dx * dx + dy * dy).sqrt();
        let threshold = self.settings().swipe_distance_threshold();
        match state.phase.get() {
            Phase::Popup => return self.update_popup_selection(x, y),
            Phase::Glide => {
                self.engine.add_glide_point(x, y)?;
                self.add_trail_point(x, y);
                return Ok(());
            }
            Phase::SpaceCursor => {
                let steps = ((x - state.anchor_x.get()) / self.cursor_step_distance.get()) as i32;
                if steps != 0 {
                    state
                        .anchor_x
                        .set(state.anchor_x.get() + steps as f64 * self.cursor_step_distance.get());
                    let action = if steps < 0 {
                        self.settings().space_bar_swipe_left()
                    } else {
                        self.settings().space_bar_swipe_right()
                    };
                    self.engine.execute_swipe_action(
                        action,
                        steps.checked_abs().ok_or(TouchError::StepCountOverflow)?,
                    )?;
                }
                return Ok(());
            }
            Phase::PreciseDelete => {
                let steps = max(0.0, (state.start_x - x) / self.delete_step_distance.get()) as i32;
                if steps != state.steps.get() {
                    state.steps.set(steps);
                    let by_words = matches!(
                        self.settings().delete_key_swipe_left(),
                        SwipeAction::DeleteWordsPrecisely | SwipeAction::SelectWordsPrecisely
                    );
                    self.engine.update_precise_selection(steps, by_words)?;
                }
                return Ok(());
            }
            Phase::NoKey => {
                if distance > threshold {
                    state.phase.set(Phase::Swipe);
                }
                return Ok(());
            }
            Phase::Pressed => {
                if let Some(key) = state.key.clone() {
                    if distance <= threshold * 0.5
                        && !(key.is_character_key()? && distance > key.touch_bounds().width * 0.5)
                    {
                        return Ok(());
                    }
                    return self.begin_gesture(&state, &key, dx, dy, distance, threshold);
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn begin_gesture(
        &self,
        state: &Rc<PointerState>,
        key: &Rc<ComputedKey>,
        dx: f64,
        dy: f64,
        distance: f64,
        threshold: f64,
    ) -> TouchResult<()> {
        let horizontal = dx.abs() >= dy.abs();
        if key.data().data().is_space() && distance > threshold * 0.5 {
            if horizontal
                && (self.settings().space_bar_swipe_left() != SwipeAction::NoAction
                    || self.settings().space_bar_swipe_right() != SwipeAction::NoAction)
            {
                self.cancel_key(state)?;
                state.phase.set(Phase::SpaceCursor);
                state.anchor_x.set(state.start_x);
                return self.pointer_move(state.id, state.last_x.get(), state.last_y.get());
            }
            if !horizontal
                && dy < 0.0
                && distance > threshold
                && self.settings().space_bar_swipe_up() != SwipeAction::NoAction
            {
                self.cancel_key(state)?;
                state.phase.set(Phase::Consumed);
                self.engine
                    .execute_swipe_action(self.settings().space_bar_swipe_up(), 1)?;
                return Ok(());
            }
            return Ok(());
        }
        if key.code() == KeyCode::Delete && horizontal && dx < 0.0 && distance > threshold * 0.5 {
            match self.settings().delete_key_swipe_left() {
                SwipeAction::DeleteCharactersPrecisely
                | SwipeAction::DeleteWordsPrecisely
                | SwipeAction::SelectCharactersPrecisely
                | SwipeAction::SelectWordsPrecisely => {
                    self.cancel_key(state)?;
                    state.phase.set(Phase::PreciseDelete);
                    state.steps.set(0);
                    self.engine.begin_precise_selection()?;
                    return self.pointer_move(state.id, state.last_x.get(), state.last_y.get());
                }
                SwipeAction::NoAction => return Ok(()),
                _ => {
                    self.cancel_key(state)?;
                    state.phase.set(Phase::Consumed);
                    self.engine
                        .execute_swipe_action(self.settings().delete_key_swipe_left(), 1)?;
                    return Ok(());
                }
            }
        }
        if key.is_character_key()?
            && self.engine.can_glide()
            && self.pointers.count() == 1
            && distance > min(threshold, key.touch_bounds().width * 0.6)
        {
            self.cancel_key(state)?;
            state.phase.set(Phase::Glide);
            self.glide_trail.borrow_mut().clear();
            self.engine.begin_glide(state.start_x, state.start_y)?;
            self.add_trail_point(state.start_x, state.start_y);
            self.engine
                .add_glide_point(state.last_x.get(), state.last_y.get())?;
            self.add_trail_point(state.last_x.get(), state.last_y.get());
            return Ok(());
        }
        if distance > threshold && key.code() != KeyCode::Shift {
            self.cancel_key(state)?;
            state.phase.set(Phase::Swipe);
        }
        Ok(())
    }
    pub fn pointer_up(&self, pointer_id: i64, x: f64, y: f64) -> TouchResult<()> {
        let Some(state) = self.pointers.get(pointer_id) else {
            return Ok(());
        };
        self.pointers.remove(pointer_id);
        state.cancel_timers();
        state.last_x.set(x);
        state.last_y.set(y);
        match state.phase.get() {
            Phase::Pressed => {
                if let Some(key) = state.key.clone() {
                    self.release(&key);
                    if !state.repeated.get() || key.code() == KeyCode::Shift {
                        self.engine.on_key_up(key.data().as_ref())?;
                    }
                }
            }
            Phase::Popup => {
                let popup = self.popup();
                self.close_popup();
                if let Some(key) = state.key.clone() {
                    self.engine.on_key_cancel(key.data().as_ref());
                }
                if let Some(popup) = popup
                    && popup.selected_index.get() >= 0
                    && (popup.selected_index.get() as usize) < popup.keys.len()
                {
                    let selected = popup
                        .keys
                        .get(popup.selected_index.get() as usize)
                        .map_err(KeyDataError::from)
                        .map_err(EngineError::Key)?;
                    self.engine.input_key(selected.as_ref())?;
                }
            }
            Phase::Glide => {
                self.engine.finish_glide(true)?;
                self.glide_ended.raise(self);
            }
            Phase::PreciseDelete => {
                let delete = matches!(
                    self.settings().delete_key_swipe_left(),
                    SwipeAction::DeleteCharactersPrecisely | SwipeAction::DeleteWordsPrecisely
                );
                self.engine.end_precise_selection(delete)?;
            }
            Phase::Swipe => self.execute_swipe(&state)?,
            _ => {}
        }
        Ok(())
    }
    pub fn pointer_cancel(&self, pointer_id: i64) -> TouchResult<()> {
        let Some(state) = self.pointers.get(pointer_id) else {
            return Ok(());
        };
        self.pointers.remove(pointer_id);
        state.cancel_timers();
        match state.phase.get() {
            Phase::Pressed => {
                if let Some(key) = state.key.clone() {
                    self.release(&key);
                    self.engine.on_key_cancel(key.data().as_ref());
                }
            }
            Phase::Popup => self.close_popup(),
            Phase::Glide => {
                self.engine.finish_glide(false)?;
                self.glide_ended.raise(self);
            }
            Phase::PreciseDelete => self.engine.cancel_precise_selection()?,
            _ => {}
        }
        Ok(())
    }
    pub fn cancel_all(&self) -> TouchResult<()> {
        let ids = self.pointers.ids();
        for id in ids {
            self.pointer_cancel(id)?;
        }
        Ok(())
    }
    pub fn clear_glide_trail(&self) {
        self.glide_trail.borrow_mut().clear();
        self.glide_trail_changed.raise(self);
    }
    fn execute_swipe(&self, state: &PointerState) -> TouchResult<()> {
        let dx = state.last_x.get() - state.start_x;
        let dy = state.last_y.get() - state.start_y;
        if (dx * dx + dy * dy).sqrt() < self.settings().swipe_distance_threshold() {
            return Ok(());
        }
        let action = if dx.abs() >= dy.abs() {
            if dx < 0.0 {
                self.settings().swipe_left()
            } else {
                self.settings().swipe_right()
            }
        } else if dy < 0.0 {
            self.settings().swipe_up()
        } else {
            self.settings().swipe_down()
        };
        self.engine.execute_swipe_action(action, 1)?;
        Ok(())
    }
    fn on_repeat(&self, state: &Rc<PointerState>) -> TouchResult<()> {
        if !self.pointers.contains(state.id)
            || state.phase.get() != Phase::Pressed
            || state.key.is_none()
        {
            return Ok(());
        }
        state.repeated.set(true);
        self.engine
            .on_key_repeat(state.key.as_ref().expect("source key").data().as_ref())?;
        let timer = self.schedule(
            state,
            Duration::from_millis(self.settings().key_repeat_interval() as u64),
            true,
        );
        self.install_timer(state, timer, true);
        Ok(())
    }
    fn on_long_press(&self, state: &Rc<PointerState>) -> TouchResult<()> {
        if !self.pointers.contains(state.id) || state.phase.get() != Phase::Pressed {
            return Ok(());
        }
        let Some(key) = state.key.clone() else {
            return Ok(());
        };
        match key.code() {
            KeyCode::Shift => {
                state.repeated.set(true);
                self.engine.feedback_long_press();
                self.engine.set_shift_state(ShiftState::CapsLock)?;
                return Ok(());
            }
            KeyCode::Space => {
                if self.settings().space_bar_long_press() != SwipeAction::NoAction {
                    self.cancel_key(state)?;
                    state.phase.set(Phase::Consumed);
                    self.engine.feedback_long_press();
                    self.engine
                        .execute_swipe_action(self.settings().space_bar_long_press(), 1)?;
                }
                return Ok(());
            }
            KeyCode::Delete => {
                if self.settings().delete_key_long_press() == SwipeAction::DeleteCharacter {
                    self.on_repeat(state)?;
                } else {
                    self.cancel_key(state)?;
                    state.phase.set(Phase::Consumed);
                    self.engine
                        .execute_swipe_action(self.settings().delete_key_long_press(), 1)?;
                }
                return Ok(());
            }
            _ => {}
        }
        if key.popup_keys().is_empty() {
            return Ok(());
        }
        state.cancel_timers();
        self.release(&key);
        state.phase.set(Phase::Popup);
        self.engine.feedback_long_press();
        self.open_popup(&key)
    }
    fn open_popup(&self, key: &Rc<ComputedKey>) -> TouchResult<()> {
        let keyboard = self.engine.keyboard()?;
        let keys = key.popup_keys();
        let (bounds, selected) = Self::arrange_popup(
            key,
            i32::try_from(keys.len()).map_err(|_| TouchError::PopupCountOverflow)?,
            key.popup_default_index(),
            keyboard.width(),
            self.popup_max_columns.get(),
            self.popup_min_y.get(),
        )?;
        *self.popup.borrow_mut() = Some(Rc::new(KeyPopupState {
            key: key.clone(),
            keys,
            bounds,
            selected_index: Cell::new(selected),
        }));
        self.popup_changed.raise(self);
        Ok(())
    }
    fn close_popup(&self) {
        if self.popup().is_none() {
            return;
        }
        *self.popup.borrow_mut() = None;
        self.popup_changed.raise(self);
    }
    fn update_popup_selection(&self, x: f64, y: f64) -> TouchResult<()> {
        let Some(popup) = self.popup() else {
            return Ok(());
        };
        if popup.bounds.borrow().is_empty() {
            return Ok(());
        }
        let key = popup.key.visible_bounds();
        let selected = if y > key.bottom() + key.height {
            -1
        } else {
            let bottom_row_top = enumerable_max(popup.bounds.borrow().iter().map(|b| b.top()));
            let mut candidates = (0..popup.bounds.borrow().len()).collect::<Vec<_>>();
            if y >= popup.frame().bottom() {
                candidates
                    .retain(|&i| (popup.bounds.borrow()[i].top() - bottom_row_top).abs() < 0.5);
            }
            let mut ranked = Vec::new();
            for i in candidates {
                let distance = popup.bounds.borrow()[i]
                    .center_distance_squared(x, min(y, popup.frame().bottom() - 1.0));
                ranked.push((i, distance));
            }
            ranked.sort_by(|a, b| double_compare(a.1, b.1));
            ranked.first().ok_or(TouchError::EmptyPopupCandidates)?.0 as i32
        };
        if selected != popup.selected_index.get() {
            popup.selected_index.set(selected);
            self.engine.feedback_gesture_step();
            self.popup_changed.raise(self);
        }
        Ok(())
    }
    pub fn arrange_popup(
        key: &ComputedKey,
        count: i32,
        default_index: i32,
        keyboard_width: f64,
        max_columns: i32,
        min_y: f64,
    ) -> TouchResult<(PopupBounds, i32)> {
        let count = usize::try_from(count).map_err(|_| TouchError::NegativePopupCount)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(count)
            .map_err(|_| TouchError::PopupAllocation)?;
        result.resize(count, KeyRect::default());
        if count == 0 {
            return Ok((Rc::new(RefCell::new(result.into_boxed_slice())), -1));
        }
        let anchor = key.visible_bounds();
        let cell_w = max(anchor.width, 28.0);
        let cell_h = max(anchor.height, 28.0);
        let columns = (count as i32).min(max_columns.max(1));
        let rows = (count as f64 / columns as f64).ceil() as i32;
        let anchor_col = if keyboard_width > 0.0 && anchor.center_x() < keyboard_width / 3.0 {
            0
        } else if keyboard_width > 0.0 && anchor.center_x() > keyboard_width * 2.0 / 3.0 {
            columns - 1
        } else {
            (columns - 1) / 2
        };
        let mut left = sub(
            sub(anchor.center_x(), cell_w / 2.0),
            anchor_col as f64 * cell_w,
        );
        if keyboard_width > 0.0 {
            left = max(0.0, min(left, keyboard_width - columns as f64 * cell_w));
        }
        let mut top = sub(anchor.top(), rows as f64 * cell_h);
        if top < min_y {
            top = min_y;
        }
        let mut order = vec![anchor_col];
        let mut d = 1;
        while (order.len() as i32) < columns {
            if anchor_col + d < columns {
                order.push(anchor_col + d);
            }
            if anchor_col - d >= 0 && (order.len() as i32) < columns {
                order.push(anchor_col - d);
            }
            d += 1;
        }
        let mut items = (0..count).collect::<Vec<_>>();
        if default_index > 0 && (default_index as usize) < count {
            items.remove(default_index as usize);
            items.insert(0, default_index as usize);
        }
        let mut index = 0;
        for r in 0..rows {
            if index >= count {
                break;
            }
            let y = add(top, (rows - 1 - r) as f64 * cell_h);
            for &col in &order {
                if index >= count {
                    break;
                }
                result[items[index]] = KeyRect {
                    x: add(left, col as f64 * cell_w),
                    y,
                    width: cell_w,
                    height: cell_h,
                };
                index += 1;
            }
        }
        Ok((
            Rc::new(RefCell::new(result.into_boxed_slice())),
            default_index.max(0),
        ))
    }
    fn add_trail_point(&self, x: f64, y: f64) {
        if !self.settings().glide_show_trail() {
            return;
        }
        self.glide_trail.borrow_mut().push(GesturePoint { x, y });
        if self.glide_trail.borrow().len() > 400 {
            self.glide_trail.borrow_mut().remove(0);
        }
        self.glide_trail_changed.raise(self);
    }
    fn cancel_key(&self, state: &PointerState) -> TouchResult<()> {
        state.cancel_timers();
        if let Some(key) = state.key.clone()
            && state.phase.get() == Phase::Pressed
        {
            self.release(&key);
            self.engine.on_key_cancel(key.data().as_ref());
        }
        Ok(())
    }
    fn press(&self, key: &Rc<ComputedKey>) {
        self.key_pressed.raise(key);
    }
    fn release(&self, key: &Rc<ComputedKey>) {
        self.key_released.raise(key);
    }
}
impl Drop for KeyboardTouchProcessor {
    fn drop(&mut self) {
        // A custom cancellation callback may be the original panic. Do not invoke it
        // again while unwinding; late scheduled callbacks already have weak receivers.
        if std::thread::panicking() {
            return;
        }
        // Release the collection borrow before invoking custom cancellation code. Retained
        // PressedKeys views may still own this map; they must not retain the processor/engine.
        let mut states = self.pointers.snapshot();
        states.extend(
            self.retired_pointers
                .get_mut()
                .iter()
                .filter_map(Weak::upgrade),
        );
        for state in states {
            state.cancel_timers();
        }
    }
}
// .NET's evaluated arithmetic propagates the first NaN operand unchanged. LLVM can rewrite
// subtraction into addition of a negated operand and flip a NaN sign in optimized builds.
// Guard only pre-existing NaNs; all finite arithmetic and signed-zero operations stay literal.
fn sub(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        a - b
    }
}
fn add(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        a + b
    }
}
fn min(a: f64, b: f64) -> f64 {
    if a != b {
        if a.is_nan() || a < b { a } else { b }
    } else if a.is_sign_negative() {
        a
    } else {
        b
    }
}
fn max(a: f64, b: f64) -> f64 {
    -min(-a, -b)
}
fn enumerable_min(mut values: impl Iterator<Item = f64>) -> f64 {
    let mut value = values.next().expect("nonempty source sequence");
    if value.is_nan() {
        return value;
    }
    for next in values {
        if next < value {
            value = next;
        } else if next.is_nan() {
            return next;
        }
    }
    value
}
fn enumerable_max(values: impl Iterator<Item = f64>) -> f64 {
    values
        .reduce(|a, b| if b > a || a.is_nan() { b } else { a })
        .expect("nonempty source sequence")
}
fn double_compare(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b)
        .unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()).reverse())
}

#[cfg(test)]
#[path = "touch_reference_tests.rs"]
mod tests;
