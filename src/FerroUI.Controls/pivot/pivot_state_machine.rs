//! WinUI's `PivotStateMachine` (`dxaml/xcp/components/pivot/PivotStateMachine.cpp` and
//! `inc/PivotStateMachine.h` of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md), ported one to one.
//!
//! The machine keeps the selected index of a [`Pivot`](super::Pivot) and the offset of its
//! scroll viewer in step: a pan that ends past a quarter of a section moves the selection, a
//! programmatic change of the selection flies the content out and in, and while the headers wrap
//! (the header carousel) the viewport is kept near the middle of a panel of
//! [`PIVOT_PANEL_MULTIPLIER`] sections, so that a flick never reaches an end. The machine itself
//! touches no element: it asks the pivot through [`PivotStateMachineCallbacks`], which the pivot
//! passes to every event (WinUI keeps the callback pointer in the machine).

use super::pivot_common::{are_close, are_close_within, header_shift_offset, positive_mod, PivotAnimationDirection};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::Size;

/// The changes of the items of the pivot that the machine tells apart (WinUI's
/// `CollectionChange`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PivotCollectionChange {
    Reset,
    ItemInserted,
    ItemRemoved,
    ItemChanged,
}

/// What the machine asks of the pivot (WinUI's `IPivotStateMachineCallbacks`).
pub(crate) trait PivotStateMachineCallbacks {
    fn is_header_items_carousel_enabled(&self) -> bool;
    fn pivot_panel_multiplier(&self) -> u32;
    /// Moves the items (and the headers that follow them) to the section at `offset`.
    fn set_pivot_section_offset(&self, offset: f64);
    /// Scrolls the viewport to `offset`; whether the scroll viewer moved.
    fn set_viewport_offset(&self, offset: f64, animate: bool) -> bool;
    fn set_pivot_section_width(&self, width: f32);
    fn set_selected_index_with(
        &self,
        index: i32,
        update_visual: bool,
        update_index: bool,
        update_item: bool,
        animation_hint: PivotAnimationDirection,
    );
    fn set_parallax_relationship(&self, section_offset: f64, section_width: f64, viewport_size: f32);
    fn set_snapping_behavior(&self, mandatory_single: bool);
    fn set_viewport_enabled(&self, enabled: bool);
    fn start_fly_out_animation(&self, from: f64, header_offset: f64, to_left: bool);
    fn start_fly_in_animation(&self, to: f64, from_left: bool);
    /// Whether the scroll viewer is moving under a manipulation or its inertia (WinUI's
    /// `GetIsInDManipAnimation`).
    fn is_in_manipulation_animation(&self) -> bool;
    fn cancel_manipulation_animations(&self);
    fn update_scroll_viewer_drag_direction(&self, direction: PivotAnimationDirection);
    fn update_focus_follower(&self);
}

/// The states of the machine (WinUI's `PivotState`). The initialized states come after `Idle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PivotState {
    Uninitialized,
    MeasurePending,
    ArrangePending,
    Idle,
    InNonInertial,
    InFinishTransition,
    InFlyOutProgrammaticInertial,
    InFlyInProgrammaticInertial,
    InSnapBackInertial,
    ProgrammaticInertial,
    /// The state of a lock requested while the viewport moves: the manipulation is cancelled and
    /// the viewport re-centred first.
    LockedPending,
    Locked,
}

/// WinUI's `m_deferredSetViewportOffsetData`.
#[derive(Clone, Copy, Debug, Default)]
struct DeferredSetViewportOffset {
    is_pending: bool,
    offset: f64,
    should_animate: bool,
}

/// WinUI's `c_pivotAnimationRatio`: the part of a section the fly-in starts from.
const PIVOT_ANIMATION_RATIO: f64 = 1.0;
/// WinUI's `c_sectionNormalizationThreshold`: how many sections from the middle the viewport may
/// wander before it is re-centred.
const SECTION_NORMALIZATION_THRESHOLD: f64 = 300.0;
/// WinUI's `c_sectionWidthDividerForOffsetComparisons`.
const SECTION_WIDTH_DIVIDER_FOR_OFFSET_COMPARISONS: f64 = 4.0;

/// The state machine of a pivot (WinUI's `PivotStateMachine`).
#[derive(Debug)]
pub(crate) struct PivotStateMachine {
    state: PivotState,
    pivot_section_width: f64,
    static_viewport_offset: f64,
    predicted_viewport_offset: f64,
    current_index: i32,
    next_index: i32,
    total_items: i32,
    header_widths: Vec<f64>,
    last_arrange_size: Size,
    direction: PivotAnimationDirection,
    is_locked: bool,
    pending_initialization: bool,
    pending_arrange: bool,
    pending_out_of_sequence_move: bool,
    pending_viewport_enabled_update: bool,
    pending_first_item_layout: bool,
    pending_item_host_validation: bool,
    snap_point_behavior_relaxed: bool,
    programmatic_view_change_in_progress: bool,
    within_transition: bool,
    using_static_headers: bool,
    viewport_size: f32,
    pending_zoom_to_rect: bool,
    pending_zoom_to_rect_offset: f64,
    pending_header_shift_offset: f64,
    manual_animation_in_progress: bool,
    deferred_set_viewport_offset: DeferredSetViewportOffset,
}

impl Default for PivotStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl PivotStateMachine {
    pub(crate) fn new() -> Self {
        Self {
            state: PivotState::Uninitialized,
            pivot_section_width: 0.0,
            static_viewport_offset: 0.0,
            predicted_viewport_offset: 0.0,
            current_index: 0,
            next_index: 0,
            total_items: 0,
            header_widths: Vec::new(),
            last_arrange_size: Size::default(),
            direction: PivotAnimationDirection::Reset,
            is_locked: false,
            pending_initialization: true,
            pending_arrange: false,
            pending_out_of_sequence_move: false,
            pending_viewport_enabled_update: true,
            pending_first_item_layout: true,
            pending_item_host_validation: false,
            snap_point_behavior_relaxed: false,
            programmatic_view_change_in_progress: false,
            within_transition: false,
            using_static_headers: false,
            viewport_size: 0.0,
            pending_zoom_to_rect: false,
            pending_zoom_to_rect_offset: 0.0,
            pending_header_shift_offset: 0.0,
            manual_animation_in_progress: false,
            deferred_set_viewport_offset: DeferredSetViewportOffset::default(),
        }
    }

    pub(crate) fn initialize(&mut self, starting_index: i32, item_count: i32, is_locked: bool) {
        debug_assert_eq!(self.state, PivotState::Uninitialized);
        self.is_locked = is_locked;
        self.total_items = item_count;
        self.current_index = starting_index;
    }

    pub(crate) fn state(&self) -> PivotState {
        self.state
    }

    /// The offset of the viewport at rest on the current section.
    pub(crate) fn static_viewport_offset(&self) -> f64 {
        self.static_viewport_offset
    }

    pub(crate) fn section_width(&self) -> f64 {
        self.pivot_section_width
    }

    pub(crate) fn current_index(&self) -> i32 {
        self.current_index
    }

    /// Whether the machine waits for a measure pass to refresh its items (a reset of the items).
    pub(crate) fn is_pending_items_host_validation(&self) -> bool {
        self.pending_item_host_validation
    }

    fn is_initialized(&self) -> bool {
        self.state >= PivotState::Idle
    }

    pub(crate) fn apply_template_event(&mut self, cb: &dyn PivotStateMachineCallbacks, is_correct: bool) {
        if is_correct {
            self.pending_initialization = true;
            // The items host is technically invalid from the point of view of the items control,
            // but the pivot waits for the validating measure pass before it looks for a
            // container; the flag only covers a reset of the items while initialized.
            self.pending_item_host_validation = false;
            self.transition(cb, PivotState::MeasurePending);
        } else {
            self.transition(cb, PivotState::Uninitialized);
        }
    }

    pub(crate) fn arrange_event(&mut self, cb: &dyn PivotStateMachineCallbacks, final_size: Size) {
        if self.state == PivotState::Uninitialized {
            return;
        }
        if self.pending_initialization
            || !are_close(final_size.width, self.last_arrange_size.width)
            || self.pending_first_item_layout
            || self.pending_arrange
        {
            self.last_arrange_size = final_size;
            if !self.within_transition {
                if self.is_locked {
                    self.transition(cb, PivotState::Locked);
                } else {
                    self.transition(cb, PivotState::Idle);
                }
            } else {
                self.pending_arrange = false;
            }
        }
        if self.pending_item_host_validation {
            // While flying out, the fly-in that follows changes the visible item at the right
            // time; updating it here would put the curves of the headers out of step.
            if self.state != PivotState::InFlyOutProgrammaticInertial {
                cb.set_selected_index_with(self.current_index, true, false, false, PivotAnimationDirection::Reset);
            }
            self.pending_item_host_validation = false;
        }
        if self.pending_first_item_layout && self.total_items > 0 {
            self.pending_first_item_layout = false;
        }
    }

    pub(crate) fn measure_event(&mut self, cb: &dyn PivotStateMachineCallbacks, available_size: Size) {
        if self.state == PivotState::Uninitialized {
            return;
        }
        if self.pending_initialization
            || !are_close(available_size.width, self.pivot_section_width)
            || self.pending_first_item_layout
        {
            // The middle of the panel is computed here and given to the scroll viewer after the
            // arrange pass; several measure passes may come before it.
            self.pivot_section_width = available_size.width;
            cb.set_pivot_section_width(self.pivot_section_width as f32);
            if matches!(
                self.state,
                PivotState::Uninitialized | PivotState::MeasurePending | PivotState::Idle | PivotState::Locked
            ) && !self.within_transition
            {
                self.transition(cb, PivotState::ArrangePending);
            } else {
                // The viewport offset is updated on the way to idle or locked.
                self.pending_arrange = true;
            }
        }
    }

    fn midpoint_viewport_offset(&self, cb: &dyn PivotStateMachineCallbacks) -> f64 {
        self.pivot_section_width * cb.pivot_panel_multiplier() as f64 / 2.0
    }

    /// A change of the view of the scroll viewer (WinUI's `ViewChanging`): `next_offset` is the
    /// offset shown next, `final_offset` where the view comes to rest (the same while panning).
    pub(crate) fn view_changed_event(
        &mut self,
        cb: &dyn PivotStateMachineCallbacks,
        is_inertial: bool,
        _next_offset: f64,
        final_offset: f64,
        _section_offset: f64,
    ) {
        let pivot_sections_moved = (self.static_viewport_offset + final_offset) / self.pivot_section_width;
        let is_on_snap_point = are_close(pivot_sections_moved, (pivot_sections_moved + 0.5).floor());
        if is_inertial && !is_on_snap_point {
            // The final offset of an inertial move is on a snap point; an event that breaks that
            // invariant (the scroll viewer sends some) is skipped.
            return;
        }
        if self.is_initialized() && !self.programmatic_view_change_in_progress && self.state != PivotState::Locked {
            let predicted = if self.deferred_set_viewport_offset.is_pending {
                self.deferred_set_viewport_offset.offset
            } else {
                final_offset
            };
            if self.state == PivotState::LockedPending {
                self.predicted_viewport_offset = predicted;
            } else if is_inertial {
                self.predicted_viewport_offset = predicted;
                if are_close_within(
                    final_offset,
                    self.static_viewport_offset,
                    self.pivot_section_width / SECTION_WIDTH_DIVIDER_FOR_OFFSET_COMPARISONS,
                ) {
                    if self.state == PivotState::InNonInertial {
                        self.transition(cb, PivotState::InSnapBackInertial);
                    }
                } else if matches!(
                    self.state,
                    PivotState::InNonInertial
                        | PivotState::InSnapBackInertial
                        | PivotState::Idle
                        | PivotState::ProgrammaticInertial
                ) {
                    if !self.pending_out_of_sequence_move {
                        self.direction = if self.predicted_viewport_offset < self.static_viewport_offset {
                            PivotAnimationDirection::Left
                        } else {
                            PivotAnimationDirection::Right
                        };
                    }
                    self.transition(cb, PivotState::InFinishTransition);
                }
            } else if self.state == PivotState::InFinishTransition {
                self.transition(cb, PivotState::Idle);
            } else {
                self.predicted_viewport_offset = predicted;
                cb.update_scroll_viewer_drag_direction(
                    if self.predicted_viewport_offset < self.static_viewport_offset {
                        PivotAnimationDirection::Left
                    } else {
                        PivotAnimationDirection::Right
                    },
                );
                // While flying out the machine moves on when its own animation completes.
                if self.state != PivotState::InFlyOutProgrammaticInertial {
                    self.transition(cb, PivotState::InNonInertial);
                }
            }
        }
    }

    /// The view came to rest (WinUI's `ViewChanged` with `IsIntermediate == false`).
    pub(crate) fn final_view_event(&mut self, cb: &dyn PivotStateMachineCallbacks) {
        // A viewport offset asked for during a running animation is set now.
        if self.deferred_set_viewport_offset.is_pending {
            self.deferred_set_viewport_offset.is_pending = false;
            let deferred = self.deferred_set_viewport_offset;
            self.set_viewport_offset(cb, deferred.offset, deferred.should_animate);
        }
        // The call above may have started an animation; with one running (or the machine's own)
        // another callback reverts to idle, otherwise this is the last chance.
        if !cb.is_in_manipulation_animation() && !self.manual_animation_in_progress {
            self.revert_to_idle(cb);
        }
    }

    pub(crate) fn selected_index_changed_event(
        &mut self,
        cb: &dyn PivotStateMachineCallbacks,
        new_index: i32,
        is_from_header_tap: bool,
        skip_animation: bool,
    ) {
        let in_manipulation = cb.is_in_manipulation_animation();
        let should_transition_state = !skip_animation
            && matches!(
                self.state,
                PivotState::Idle | PivotState::InFlyInProgrammaticInertial | PivotState::InFinishTransition
            )
            && !self.pending_first_item_layout
            && (!in_manipulation || is_from_header_tap);

        if should_transition_state {
            if self.current_index == new_index {
                return;
            }
            self.pending_out_of_sequence_move = true;
            self.next_index = new_index;
            self.direction = self.determine_direction(self.next_index, is_from_header_tap);
            // Without the carousel the selected index and the viewport offset are strictly
            // related; with it they are not.
            if self.is_header_items_carousel_enabled(cb) {
                let mut items_to_shift = new_index - self.current_index;
                if self.direction == PivotAnimationDirection::Right {
                    items_to_shift = positive_mod(items_to_shift, self.total_items);
                } else {
                    items_to_shift = -(self.total_items - positive_mod(items_to_shift, self.total_items));
                }
                self.predicted_viewport_offset =
                    self.static_viewport_offset + self.pivot_section_width * items_to_shift as f64;
            } else {
                self.predicted_viewport_offset = self.next_index as f64 * self.pivot_section_width;
            }

            // WinUI asserts that every item has its header width; the widths come from the
            // header manager, and a pivot without one has none.
            if self.in_scrolling_static_headers_mode() && self.header_widths.len() == self.total_items as usize {
                // The static headers shift by the difference of the shift offsets of the current
                // and the new header.
                let mut accumulated = 0.0;
                let (mut a_current, mut a_new, mut b_current, mut b_new) = (0.0, 0.0, 0.0, 0.0);
                for (i, width) in self.header_widths.iter().enumerate().take(self.total_items.max(0) as usize) {
                    if i as i32 == self.current_index {
                        a_current = accumulated;
                        b_current = *width;
                    } else if i as i32 == new_index {
                        a_new = accumulated;
                        b_new = *width;
                    }
                    accumulated += width;
                }
                let c_current = (accumulated - (a_current + b_current)).max(0.0);
                let c_new = (accumulated - (a_new + b_new)).max(0.0);
                let viewport = self.viewport_size as f64;
                self.pending_header_shift_offset = (header_shift_offset(a_new, b_new, c_new, viewport)
                    - header_shift_offset(a_current, b_current, c_current, viewport))
                .abs();
            } else if !self.using_static_headers && self.header_widths.len() == self.total_items as usize {
                // The dynamic headers shift so that the new header takes the place of the
                // current one: the widths of the headers in between. Moving left the current
                // header is left out and the new one counted, moving right the other way round.
                let mut distance = 0.0;
                let n = self.total_items;
                if self.direction == PivotAnimationDirection::Right {
                    let mut index = self.current_index;
                    while index != self.next_index {
                        distance += self.header_widths[index as usize];
                        index = positive_mod(index + 1, n);
                    }
                } else {
                    let mut index = positive_mod(self.current_index - 1, n);
                    let end = positive_mod(self.next_index - 1, n);
                    while index != end {
                        distance += self.header_widths[index as usize];
                        index = positive_mod(index - 1, n);
                    }
                }
                self.pending_header_shift_offset = distance;
            }
            self.transition(cb, PivotState::ProgrammaticInertial);
        } else if (self.pending_out_of_sequence_move && self.next_index != new_index)
            || (self.state == PivotState::InFlyInProgrammaticInertial && self.current_index != new_index)
            || (self.state == PivotState::InFinishTransition && self.current_index != new_index)
            || (self.state == PivotState::Locked && self.current_index != new_index)
            || !self.is_initialized()
            || (self.state == PivotState::Idle && self.pending_first_item_layout)
        {
            // Uninitialized or in motion: the change is made at once. Nothing useful can be done
            // while the scroll viewer is under a manipulation.
            self.next_index = new_index;
            self.current_index = new_index;
            if !self.is_header_items_carousel_enabled(cb) {
                self.predicted_viewport_offset = self.next_index as f64 * self.pivot_section_width;
                self.pending_zoom_to_rect = true;
                self.pending_zoom_to_rect_offset = self.predicted_viewport_offset;
            }
            cb.set_selected_index_with(
                self.current_index,
                self.is_initialized(),
                true,
                true,
                PivotAnimationDirection::Reset,
            );
            if self.state == PivotState::Locked || self.pending_first_item_layout {
                cb.set_parallax_relationship(self.static_viewport_offset, self.pivot_section_width, self.viewport_size);
            }
        }
    }

    pub(crate) fn is_locked_changed_event(&mut self, cb: &dyn PivotStateMachineCallbacks, is_locked: bool) {
        self.is_locked = is_locked;
        if !self.is_initialized() {
            return;
        }
        self.pending_viewport_enabled_update = true;
        if is_locked {
            if self.state == PivotState::Idle {
                self.transition(cb, PivotState::Locked);
            } else if self.state != PivotState::Locked && self.state != PivotState::LockedPending {
                self.transition(cb, PivotState::LockedPending);
            }
        } else if self.state == PivotState::Locked {
            self.transition(cb, PivotState::Idle);
        }
        // From LockedPending the transition to locked moves on to idle.
    }

    pub(crate) fn items_collection_changed_event(
        &mut self,
        cb: &dyn PivotStateMachineCallbacks,
        new_item_count: i32,
        change_index: i32,
        change: PivotCollectionChange,
    ) {
        let old_item_count = self.total_items;
        self.total_items = new_item_count;
        // Flagged even while uninitialized, so that the viewport is enabled once initialized.
        if (old_item_count <= 1 && new_item_count > 1) || (old_item_count > 1 && new_item_count <= 1) {
            self.pending_viewport_enabled_update = true;
        }
        // Uninitialized, the index and the item may slide around unvalidated, so that the
        // properties of a new pivot can be set in any order.
        if !self.is_initialized() {
            return;
        }

        // Either the current index is what the application sees, or a programmatic animated move
        // is running: the selection changed event was raised already, the current index is the
        // old item and the next index the new one.
        let mut index = if self.pending_out_of_sequence_move { self.next_index } else { self.current_index };
        let mut update_index = false;
        let mut update_item = false;
        let mut direction = PivotAnimationDirection::Reset;

        match change {
            PivotCollectionChange::Reset => {
                index = 0;
                update_index = true;
                update_item = true;
                // A reset clears the containers, which come back on the next measure; until
                // then the visible item cannot be updated.
                self.pending_item_host_validation = self.pending_item_host_validation || new_item_count > 0;
            }
            PivotCollectionChange::ItemChanged if change_index == index => {
                // The item shown (or, during a move, the one coming) was replaced.
                update_item = true;
            }
            PivotCollectionChange::ItemInserted if change_index <= index => {
                // The first item inserted is selected (the only coercion of the machine); an
                // item inserted before the current one moves the index so that the item stays.
                self.slide_index(&mut index, PivotAnimationDirection::Right);
                update_index = true;
                update_item = old_item_count == 0;
            }
            PivotCollectionChange::ItemRemoved => {
                direction = PivotAnimationDirection::Left;
                if change_index < index {
                    self.slide_index(&mut index, PivotAnimationDirection::Left);
                    update_index = true;
                } else if change_index == index {
                    update_index = new_item_count <= index;
                    update_item = true;
                    self.slide_index(&mut index, PivotAnimationDirection::Reset);
                }
            }
            _ => {}
        }
        if self.pending_out_of_sequence_move {
            self.next_index = index;
        } else {
            self.current_index = index;
        }

        let update_visual = !self.pending_out_of_sequence_move && !self.pending_item_host_validation;
        if update_index || update_item {
            cb.set_selected_index_with(index, update_visual, update_index, update_item, direction);
        }

        // With one item or none the viewport is locked; in motion the flag stays and the change
        // is made on the way to idle.
        if self.state == PivotState::Idle && self.pending_viewport_enabled_update {
            self.pending_viewport_enabled_update = false;
            cb.set_viewport_enabled(self.total_items > 1);
        }

        // Through ArrangePending, so that the viewport offset follows the current index of
        // static headers.
        if self.state == PivotState::Idle || self.state == PivotState::Locked {
            self.transition(cb, PivotState::ArrangePending);
        } else {
            self.pending_arrange = true;
        }
    }

    pub(crate) fn animation_complete_event(&mut self, cb: &dyn PivotStateMachineCallbacks) {
        debug_assert!(self.manual_animation_in_progress);
        self.manual_animation_in_progress = false;
        let in_manipulation = cb.is_in_manipulation_animation();
        if self.state == PivotState::InFlyOutProgrammaticInertial {
            self.transition(cb, PivotState::InFlyInProgrammaticInertial);
        } else if !in_manipulation {
            // The machine's own animation completed: the pivot is where it should be.
            debug_assert!(!self.deferred_set_viewport_offset.is_pending);
            self.revert_to_idle(cb);
        }
    }

    pub(crate) fn header_measure_event(&mut self, cb: &dyn PivotStateMachineCallbacks, viewport_size: f32) {
        self.viewport_size = viewport_size;
        // Not while an arrange is pending: the relationship would be applied before the new
        // viewport offset is set, a glitch while the pivot is resized.
        if !self.pending_arrange {
            cb.set_parallax_relationship(self.static_viewport_offset, self.pivot_section_width, viewport_size);
        }
    }

    pub(crate) fn is_header_items_carousel_enabled_changed_event(&mut self, cb: &dyn PivotStateMachineCallbacks) {
        if self.state == PivotState::Idle || self.state == PivotState::Locked {
            self.transition(cb, PivotState::ArrangePending);
        }
    }

    pub(crate) fn header_state_changed_event(
        &mut self,
        cb: &dyn PivotStateMachineCallbacks,
        using_static_headers: bool,
    ) {
        if self.using_static_headers != using_static_headers {
            self.using_static_headers = using_static_headers;
            if self.state == PivotState::Idle || self.state == PivotState::Locked {
                self.transition(cb, PivotState::ArrangePending);
            }
        }
    }

    pub(crate) fn header_widths_changed_event(&mut self, new_header_widths: Vec<f64>) {
        self.header_widths = new_header_widths;
    }

    fn transition(&mut self, cb: &dyn PivotStateMachineCallbacks, new_state: PivotState) {
        if new_state == self.state {
            return;
        }
        // WinUI fails fast on a re-entrant transition; the port logs it and ignores it.
        if self.within_transition {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                logger.log(None, &format!("Pivot: re-entrant transition to {new_state:?} ignored"));
            }
            return;
        }
        self.within_transition = true;
        let mut pending_state = None;

        if new_state == PivotState::ArrangePending {
            self.pending_arrange = true;
        }

        // Both are checked: some moves do not send enough view changes to reach the fly-in.
        if (matches!(
            new_state,
            PivotState::InFlyInProgrammaticInertial
                | PivotState::Idle
                | PivotState::InNonInertial
                | PivotState::InSnapBackInertial
        ) && self.state == PivotState::InFlyOutProgrammaticInertial)
            || (self.state == PivotState::InFinishTransition && new_state == PivotState::Idle)
        {
            if self.pending_out_of_sequence_move {
                self.current_index = self.next_index;
                self.static_viewport_offset = self.predicted_viewport_offset;
            } else {
                // With mandatory snap points a pan moves one section, but a programmatic move of
                // more than one relaxes them and the user can flick past several: the headers
                // follow as far as the flick goes.
                let mut loop_guard = 0;
                while !are_close_within(
                    self.static_viewport_offset,
                    self.predicted_viewport_offset,
                    self.pivot_section_width / SECTION_WIDTH_DIVIDER_FOR_OFFSET_COMPARISONS,
                ) && loop_guard < 1000
                {
                    loop_guard += 1;
                    if self.static_viewport_offset < self.predicted_viewport_offset {
                        self.static_viewport_offset += self.pivot_section_width;
                    } else {
                        self.static_viewport_offset -= self.pivot_section_width;
                    }
                    let direction = self.direction;
                    let mut index = self.current_index;
                    self.slide_index(&mut index, direction);
                    self.current_index = index;
                }
            }
            cb.set_selected_index_with(
                self.current_index,
                !self.pending_item_host_validation,
                !self.pending_out_of_sequence_move,
                !self.pending_out_of_sequence_move,
                self.direction,
            );

            if new_state == PivotState::InFlyInProgrammaticInertial {
                // Out of the fly-out: the scroll viewer goes where it is expected to be.
                if self.pending_zoom_to_rect {
                    let offset = self.pending_zoom_to_rect_offset;
                    self.set_viewport_offset(cb, offset, false);
                    self.pending_zoom_to_rect = false;
                    self.pending_zoom_to_rect_offset = 0.0;
                } else {
                    let offset = self.predicted_viewport_offset;
                    self.set_viewport_offset(cb, offset, false);
                }
                let mut new_offset = self.static_viewport_offset;
                if self.direction == PivotAnimationDirection::Left {
                    new_offset -= PIVOT_ANIMATION_RATIO * self.pivot_section_width;
                } else {
                    new_offset += PIVOT_ANIMATION_RATIO * self.pivot_section_width;
                }
                cb.set_pivot_section_offset(new_offset);
                cb.start_fly_in_animation(self.static_viewport_offset, self.direction == PivotAnimationDirection::Left);
                self.manual_animation_in_progress = true;
                if self.pending_viewport_enabled_update {
                    self.programmatic_view_change_in_progress = true;
                    self.pending_viewport_enabled_update = false;
                    cb.set_viewport_enabled(self.total_items > 1);
                    self.programmatic_view_change_in_progress = false;
                }
            } else {
                cb.set_pivot_section_offset(self.static_viewport_offset);
            }
            // The new item is loaded and shown: the user may now override the move.
            self.pending_out_of_sequence_move = false;
        }

        // From the fly-out or the finish transition to idle the block above runs too, by design.
        if new_state == PivotState::Idle || new_state == PivotState::Locked {
            let carousel = self.is_header_items_carousel_enabled(cb);
            // Without the carousel there is no wrapping: the midpoint is not used.
            let midpoint = if carousel { self.midpoint_viewport_offset(cb) } else { 0.0 };

            // Reached only when the fly-out or the finish transition were skipped (a busy UI
            // thread whose scroll viewer coalesced its view changes): the user stopped the move.
            if self.pending_out_of_sequence_move {
                if self.current_index != self.next_index {
                    self.current_index = self.next_index;
                    cb.set_selected_index_with(
                        self.current_index,
                        !self.pending_item_host_validation,
                        false,
                        false,
                        PivotAnimationDirection::Reset,
                    );
                }
                self.pending_out_of_sequence_move = false;
            }

            let should_normalize = carousel
                && (self.static_viewport_offset - midpoint).abs()
                    > SECTION_NORMALIZATION_THRESHOLD * self.pivot_section_width;
            if self.pending_initialization || self.pending_arrange || should_normalize {
                self.static_viewport_offset =
                    if carousel { midpoint } else { self.current_index as f64 * self.pivot_section_width };
                cb.set_pivot_section_offset(self.static_viewport_offset);
                let offset = self.static_viewport_offset;
                self.set_viewport_offset(cb, offset, false);
                // Syncs the items presenter and raises the first section change.
                if self.total_items > 0 {
                    debug_assert!(!(self.pending_item_host_validation && self.pending_initialization));
                    cb.set_selected_index_with(
                        self.current_index,
                        !self.pending_item_host_validation,
                        false,
                        self.pending_initialization,
                        PivotAnimationDirection::Reset,
                    );
                }
                self.pending_initialization = false;
                self.pending_arrange = false;
            }

            if self.snap_point_behavior_relaxed {
                cb.set_snapping_behavior(true);
                self.snap_point_behavior_relaxed = false;
            }
            if self.pending_viewport_enabled_update {
                self.pending_viewport_enabled_update = false;
                cb.set_viewport_enabled(self.total_items > 1 && new_state != PivotState::Locked);
            }
            cb.update_focus_follower();
        }

        if new_state == PivotState::InFlyOutProgrammaticInertial {
            cb.start_fly_out_animation(
                self.static_viewport_offset,
                self.pending_header_shift_offset,
                self.direction == PivotAnimationDirection::Right,
            );
            self.manual_animation_in_progress = true;
            self.pending_header_shift_offset = 0.0;
        }

        if new_state == PivotState::ProgrammaticInertial {
            // The user must not pan during the programmatic change.
            self.programmatic_view_change_in_progress = true;
            self.pending_viewport_enabled_update = true;
            cb.set_viewport_enabled(false);
            self.programmatic_view_change_in_progress = false;
            self.pending_zoom_to_rect = true;
            self.pending_zoom_to_rect_offset = self.predicted_viewport_offset;
            // The index is forced whatever happens (the out of sequence move); the handlers run
            // now, which is simpler for the application.
            cb.set_selected_index_with(self.next_index, false, true, true, self.direction);
            pending_state = Some(PivotState::InFlyOutProgrammaticInertial);
        }

        if new_state == PivotState::Locked {
            debug_assert!(matches!(
                self.state,
                PivotState::Idle | PivotState::LockedPending | PivotState::ArrangePending
            ));
            // Unlocked again while the last manipulation completed: back to idle.
            if !self.is_locked {
                pending_state = Some(PivotState::Idle);
            }
        }

        if new_state == PivotState::LockedPending {
            // The viewport stops and sends a final view change, which moves on to locked.
            self.pending_viewport_enabled_update = false;
            cb.set_viewport_enabled(false);
        }

        self.state = new_state;
        self.within_transition = false;
        if let Some(pending_state) = pending_state {
            self.transition(cb, pending_state);
        }
    }

    fn slide_index(&self, index: &mut i32, direction: PivotAnimationDirection) {
        match direction {
            PivotAnimationDirection::Left => *index -= 1,
            PivotAnimationDirection::Right => *index += 1,
            _ => {}
        }
        *index = positive_mod(*index, self.total_items);
    }

    fn determine_direction(&self, index: i32, is_from_header_tap: bool) -> PivotAnimationDirection {
        if is_from_header_tap {
            // Static headers: left or right of the current header. Dynamic headers: the current
            // header is always first, so a tapped one is on the right.
            if self.using_static_headers && index < self.current_index {
                PivotAnimationDirection::Left
            } else {
                PivotAnimationDirection::Right
            }
        } else if self.total_items == 2 {
            if index < self.current_index {
                PivotAnimationDirection::Left
            } else {
                PivotAnimationDirection::Right
            }
        } else {
            // The section just before moves left, any other right.
            let mut incremented = index;
            self.slide_index(&mut incremented, PivotAnimationDirection::Right);
            if incremented == self.current_index {
                PivotAnimationDirection::Left
            } else {
                PivotAnimationDirection::Right
            }
        }
    }

    fn set_viewport_offset(&mut self, cb: &dyn PivotStateMachineCallbacks, offset: f64, animate: bool) {
        if cb.is_in_manipulation_animation() {
            // Setting the offset now would add to the running animation: it is cancelled, and
            // the offset set when the view comes to rest (`final_view_event`).
            self.deferred_set_viewport_offset =
                DeferredSetViewportOffset { is_pending: true, offset, should_animate: animate };
            self.programmatic_view_change_in_progress = true;
            cb.cancel_manipulation_animations();
            self.programmatic_view_change_in_progress = false;
        } else {
            // At the offset already, nothing moves and nothing waits.
            self.programmatic_view_change_in_progress = !animate;
            cb.set_viewport_offset(offset, animate);
            self.programmatic_view_change_in_progress = false;
        }
    }

    fn revert_to_idle(&mut self, cb: &dyn PivotStateMachineCallbacks) {
        if !matches!(self.state, PivotState::Locked | PivotState::Idle | PivotState::ArrangePending)
            && !self.programmatic_view_change_in_progress
        {
            if self.is_locked {
                self.transition(cb, PivotState::Locked);
            } else {
                self.transition(cb, PivotState::Idle);
            }
        }
    }

    fn in_scrolling_static_headers_mode(&self) -> bool {
        self.using_static_headers
    }

    fn is_header_items_carousel_enabled(&self, cb: &dyn PivotStateMachineCallbacks) -> bool {
        cb.is_header_items_carousel_enabled()
    }
}
