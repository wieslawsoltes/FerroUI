//! Tests of the pivot state machine (#443, PV-1), with callbacks that record what the machine
//! asks of the pivot, as WinUI's `PivotStateMachine` drives them.

use super::pivot_common::{PivotAnimationDirection, PIVOT_PANEL_MULTIPLIER};
use super::pivot_state_machine::{PivotCollectionChange, PivotState, PivotStateMachine, PivotStateMachineCallbacks};
use ferroui_base::Size;
use std::cell::{Cell, RefCell};

#[derive(Debug, Clone, PartialEq)]
enum Call {
    SectionOffset(f64),
    ViewportOffset(f64),
    SectionWidth(f32),
    SelectedIndex { index: i32, visual: bool, index_changed: bool, item: bool, hint: PivotAnimationDirection },
    ViewportEnabled(bool),
    FlyOut { from: f64, to_left: bool },
    FlyIn { to: f64, from_left: bool },
}

#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<Call>>,
    carousel: Cell<bool>,
    in_manipulation: Cell<bool>,
}

impl Recorder {
    fn new() -> Self {
        let recorder = Self::default();
        recorder.carousel.set(true);
        recorder
    }

    fn take(&self) -> Vec<Call> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }

    fn push(&self, call: Call) {
        self.calls.borrow_mut().push(call);
    }
}

impl PivotStateMachineCallbacks for Recorder {
    fn is_header_items_carousel_enabled(&self) -> bool {
        self.carousel.get()
    }

    fn pivot_panel_multiplier(&self) -> u32 {
        PIVOT_PANEL_MULTIPLIER
    }

    fn set_pivot_section_offset(&self, offset: f64) {
        self.push(Call::SectionOffset(offset));
    }

    fn set_viewport_offset(&self, offset: f64, _animate: bool) -> bool {
        self.push(Call::ViewportOffset(offset));
        true
    }

    fn set_pivot_section_width(&self, width: f32) {
        self.push(Call::SectionWidth(width));
    }

    fn set_selected_index_with(
        &self,
        index: i32,
        update_visual: bool,
        update_index: bool,
        update_item: bool,
        animation_hint: PivotAnimationDirection,
    ) {
        self.push(Call::SelectedIndex {
            index,
            visual: update_visual,
            index_changed: update_index,
            item: update_item,
            hint: animation_hint,
        });
    }

    fn set_parallax_relationship(&self, _section_offset: f64, _section_width: f64, _viewport_size: f32) {}

    fn set_snapping_behavior(&self, _mandatory_single: bool) {}

    fn set_viewport_enabled(&self, enabled: bool) {
        self.push(Call::ViewportEnabled(enabled));
    }

    fn start_fly_out_animation(&self, from: f64, _header_offset: f64, to_left: bool) {
        self.push(Call::FlyOut { from, to_left });
    }

    fn start_fly_in_animation(&self, to: f64, from_left: bool) {
        self.push(Call::FlyIn { to, from_left });
    }

    fn is_in_manipulation_animation(&self) -> bool {
        self.in_manipulation.get()
    }

    fn cancel_manipulation_animations(&self) {}

    fn update_scroll_viewer_drag_direction(&self, _direction: PivotAnimationDirection) {}

    fn update_focus_follower(&self) {}
}

const WIDTH: f64 = 400.0;
const MIDPOINT: f64 = WIDTH * PIVOT_PANEL_MULTIPLIER as f64 / 2.0;

/// A machine with `items` items, its template applied, measured and arranged at `WIDTH`.
fn idle_machine(recorder: &Recorder, items: i32) -> PivotStateMachine {
    idle_machine_at(recorder, items, 0)
}

/// [`idle_machine`] with `start` selected.
fn idle_machine_at(recorder: &Recorder, items: i32, start: i32) -> PivotStateMachine {
    let mut machine = PivotStateMachine::new();
    machine.initialize(start, 0, false);
    for index in 0..items {
        machine.items_collection_changed_event(recorder, index + 1, index, PivotCollectionChange::ItemInserted);
    }
    machine.header_widths_changed_event(vec![100.0; items as usize]);
    machine.apply_template_event(recorder, true);
    machine.measure_event(recorder, Size::new(WIDTH, 300.0));
    machine.arrange_event(recorder, Size::new(WIDTH, 300.0));
    machine
}

#[test]
fn layout_centres_the_viewport_of_the_carousel() {
    let recorder = Recorder::new();
    let machine = idle_machine(&recorder, 3);
    assert_eq!(machine.state(), PivotState::Idle);
    assert_eq!(machine.static_viewport_offset(), MIDPOINT);
    let calls = recorder.take();
    assert!(calls.contains(&Call::SectionWidth(WIDTH as f32)));
    assert!(calls.contains(&Call::SectionOffset(MIDPOINT)));
    assert!(calls.contains(&Call::ViewportOffset(MIDPOINT)));
    // Three items: the viewport is enabled.
    assert!(calls.contains(&Call::ViewportEnabled(true)));
}

#[test]
fn without_the_carousel_the_offset_follows_the_index() {
    let recorder = Recorder::new();
    recorder.carousel.set(false);
    let mut machine = PivotStateMachine::new();
    machine.initialize(2, 3, false);
    machine.header_widths_changed_event(vec![100.0; 3]);
    machine.apply_template_event(&recorder, true);
    machine.measure_event(&recorder, Size::new(WIDTH, 300.0));
    machine.arrange_event(&recorder, Size::new(WIDTH, 300.0));
    assert_eq!(machine.static_viewport_offset(), 2.0 * WIDTH);
}

#[test]
fn one_item_disables_the_viewport() {
    let recorder = Recorder::new();
    let _machine = idle_machine(&recorder, 1);
    assert!(recorder.take().contains(&Call::ViewportEnabled(false)));
}

#[test]
fn a_programmatic_change_flies_out_and_in() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    recorder.take();

    machine.selected_index_changed_event(&recorder, 1, false, false);
    assert_eq!(machine.state(), PivotState::InFlyOutProgrammaticInertial);
    let calls = recorder.take();
    // The viewport is disabled, the index and the item change at once (the events of the
    // application run now), then the content flies out to the left (moving right).
    assert_eq!(calls[0], Call::ViewportEnabled(false));
    assert_eq!(
        calls[1],
        Call::SelectedIndex {
            index: 1,
            visual: false,
            index_changed: true,
            item: true,
            hint: PivotAnimationDirection::Right,
        }
    );
    assert_eq!(calls[2], Call::FlyOut { from: MIDPOINT, to_left: true });

    machine.animation_complete_event(&recorder);
    assert_eq!(machine.state(), PivotState::InFlyInProgrammaticInertial);
    let calls = recorder.take();
    assert!(calls.contains(&Call::ViewportOffset(MIDPOINT + WIDTH)));
    assert!(calls.contains(&Call::SectionOffset(MIDPOINT + 2.0 * WIDTH)));
    assert!(calls.contains(&Call::FlyIn { to: MIDPOINT + WIDTH, from_left: false }));
    assert!(calls.contains(&Call::ViewportEnabled(true)));
    assert_eq!(machine.current_index(), 1);

    machine.animation_complete_event(&recorder);
    assert_eq!(machine.state(), PivotState::Idle);
}

#[test]
fn moving_to_the_previous_item_goes_left() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    recorder.take();
    // From 0, item 2 is the one just before (wrapping): to the left.
    machine.selected_index_changed_event(&recorder, 2, false, false);
    let calls = recorder.take();
    assert!(calls.contains(&Call::FlyOut { from: MIDPOINT, to_left: false }));
    machine.animation_complete_event(&recorder);
    assert!(recorder.take().contains(&Call::ViewportOffset(MIDPOINT - WIDTH)));
}

#[test]
fn a_pan_past_a_quarter_moves_the_selection() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    recorder.take();

    recorder.in_manipulation.set(true);
    machine.view_changed_event(&recorder, false, MIDPOINT + 50.0, MIDPOINT + 50.0, MIDPOINT);
    assert_eq!(machine.state(), PivotState::InNonInertial);
    // The pan is released and snaps to the next section.
    recorder.in_manipulation.set(false);
    machine.view_changed_event(&recorder, true, MIDPOINT + WIDTH, MIDPOINT + WIDTH, MIDPOINT);
    assert_eq!(machine.state(), PivotState::InFinishTransition);
    machine.final_view_event(&recorder);
    assert_eq!(machine.state(), PivotState::Idle);
    assert_eq!(machine.current_index(), 1);
    assert_eq!(machine.static_viewport_offset(), MIDPOINT + WIDTH);
    let calls = recorder.take();
    assert!(calls.contains(&Call::SelectedIndex {
        index: 1,
        visual: true,
        index_changed: true,
        item: true,
        hint: PivotAnimationDirection::Right,
    }));
    assert!(calls.contains(&Call::SectionOffset(MIDPOINT + WIDTH)));
}

#[test]
fn a_short_pan_snaps_back() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    recorder.take();
    recorder.in_manipulation.set(true);
    machine.view_changed_event(&recorder, false, MIDPOINT - 30.0, MIDPOINT - 30.0, MIDPOINT);
    recorder.in_manipulation.set(false);
    machine.view_changed_event(&recorder, true, MIDPOINT, MIDPOINT, MIDPOINT);
    assert_eq!(machine.state(), PivotState::InSnapBackInertial);
    machine.final_view_event(&recorder);
    assert_eq!(machine.state(), PivotState::Idle);
    assert_eq!(machine.current_index(), 0);
}

#[test]
fn an_inertial_view_off_a_snap_point_is_skipped() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    machine.view_changed_event(&recorder, true, MIDPOINT + 123.0, MIDPOINT + 123.0, MIDPOINT);
    assert_eq!(machine.state(), PivotState::Idle);
}

#[test]
fn locking_and_unlocking() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    recorder.take();
    machine.is_locked_changed_event(&recorder, true);
    assert_eq!(machine.state(), PivotState::Locked);
    assert!(recorder.take().contains(&Call::ViewportEnabled(false)));
    // Locked, a change of the selection is made at once.
    machine.selected_index_changed_event(&recorder, 2, false, false);
    assert_eq!(machine.state(), PivotState::Locked);
    assert_eq!(machine.current_index(), 2);
    machine.is_locked_changed_event(&recorder, false);
    assert_eq!(machine.state(), PivotState::Idle);
    assert!(recorder.take().contains(&Call::ViewportEnabled(true)));
}

#[test]
fn locking_while_panning_waits_for_the_view_to_rest() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    recorder.in_manipulation.set(true);
    machine.view_changed_event(&recorder, false, MIDPOINT + 10.0, MIDPOINT + 10.0, MIDPOINT);
    machine.is_locked_changed_event(&recorder, true);
    assert_eq!(machine.state(), PivotState::LockedPending);
    recorder.in_manipulation.set(false);
    machine.final_view_event(&recorder);
    assert_eq!(machine.state(), PivotState::Locked);
}

#[test]
fn removing_the_current_item_keeps_the_index() {
    let recorder = Recorder::new();
    let mut machine = idle_machine_at(&recorder, 3, 1);
    recorder.take();
    machine.items_collection_changed_event(&recorder, 2, 1, PivotCollectionChange::ItemRemoved);
    assert_eq!(machine.current_index(), 1);
    assert!(recorder.take().contains(&Call::SelectedIndex {
        index: 1,
        visual: true,
        index_changed: false,
        item: true,
        hint: PivotAnimationDirection::Left,
    }));
}

#[test]
fn inserting_before_the_current_item_moves_the_index() {
    let recorder = Recorder::new();
    let mut machine = idle_machine_at(&recorder, 3, 1);
    machine.items_collection_changed_event(&recorder, 4, 0, PivotCollectionChange::ItemInserted);
    assert_eq!(machine.current_index(), 2);
    // The change moved the machine to ArrangePending: until the next arrange it is not
    // initialized and changes only count the items, as in WinUI.
    assert_eq!(machine.state(), PivotState::ArrangePending);
    machine.arrange_event(&recorder, Size::new(WIDTH, 300.0));
    assert_eq!(machine.state(), PivotState::Idle);
    machine.items_collection_changed_event(&recorder, 2, -1, PivotCollectionChange::Reset);
    assert_eq!(machine.current_index(), 0);
    assert!(machine.is_pending_items_host_validation());
}

#[test]
fn uninitialized_a_change_is_made_at_once() {
    let recorder = Recorder::new();
    let mut machine = PivotStateMachine::new();
    machine.initialize(0, 3, false);
    machine.selected_index_changed_event(&recorder, 2, false, false);
    assert_eq!(machine.state(), PivotState::Uninitialized);
    assert_eq!(machine.current_index(), 2);
    assert_eq!(
        recorder.take(),
        vec![Call::SelectedIndex {
            index: 2,
            visual: false,
            index_changed: true,
            item: true,
            hint: PivotAnimationDirection::Reset,
        }]
    );
}

#[test]
fn idle_a_skipped_animation_leaves_the_machine_alone() {
    // WinUI's `skipAnimation` (the pivot is not loaded) only skips the programmatic move.
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 3);
    recorder.take();
    machine.selected_index_changed_event(&recorder, 2, false, true);
    assert_eq!(machine.state(), PivotState::Idle);
    assert_eq!(machine.current_index(), 0);
    assert!(recorder.take().is_empty());
}

#[test]
fn the_viewport_is_re_centred_far_from_the_middle() {
    let recorder = Recorder::new();
    let mut machine = idle_machine(&recorder, 2);
    // 301 sections to the right: the next rest re-centres.
    let far = MIDPOINT + 301.0 * WIDTH;
    recorder.in_manipulation.set(true);
    machine.view_changed_event(&recorder, false, far, far, MIDPOINT);
    recorder.in_manipulation.set(false);
    machine.view_changed_event(&recorder, true, far, far, MIDPOINT);
    recorder.take();
    machine.final_view_event(&recorder);
    assert_eq!(machine.static_viewport_offset(), MIDPOINT);
    assert!(recorder.take().contains(&Call::ViewportOffset(MIDPOINT)));
}
