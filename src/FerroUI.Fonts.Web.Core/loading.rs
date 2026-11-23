//! The states of the CSS Font Loading API (`FontFace`, `FontFaceSet`) and the `font-display`
//! timeline of CSS Fonts Level 4, as plain state machines: no I/O, no clock of their own, and
//! no callbacks. Whoever drives them feeds in what happened and reads what to do.

use crate::css::{FontDisplay, FontFaceDescriptors};
use crate::matching::{family_matches, match_font_faces, FontMatchRequest};

/// Identifies a face among those a driver knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontFaceId(pub u32);

/// The `status` of a `FontFace`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFaceLoadStatus {
    #[default]
    Unloaded,
    Loading,
    Loaded,
    Error,
}

impl FontFaceLoadStatus {
    /// Whether a face may go from this status to `next`. A face loads once: it never goes back,
    /// and a finished face stays as it is.
    pub const fn can_become(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Unloaded, Self::Loading | Self::Error) | (Self::Loading, Self::Loaded | Self::Error)
        )
    }
}

/// The `status` of a `FontFaceSet`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFaceSetLoadStatus {
    Loading,
    #[default]
    Loaded,
}

/// The kind of an event of a `FontFaceSet`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFaceSetEventKind {
    Loading,
    LoadingDone,
    LoadingError,
}

/// An event of a `FontFaceSet`, with the faces it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFaceSetEvent {
    pub kind: FontFaceSetEventKind,
    pub font_faces: Vec<FontFaceId>,
}

/// Why a face could not be added to a set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontFaceSetError {
    /// A face that comes from a style sheet rule belongs to that rule's set.
    InvalidModification,
}

impl std::fmt::Display for FontFaceSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a face connected to a style sheet rule cannot be added to a set")
    }
}

impl std::error::Error for FontFaceSetError {}

/// The state of a `FontFaceSet`: its members, the three lists of the standard, its status and
/// its `ready` promise.
#[derive(Clone, Debug, Default)]
pub struct FontFaceSetState {
    /// The members in order, each with whether it is connected to a style sheet rule.
    members: Vec<(FontFaceId, bool)>,
    loading: Vec<FontFaceId>,
    loaded: Vec<FontFaceId>,
    failed: Vec<FontFaceId>,
    status: FontFaceSetLoadStatus,
    /// Counts the `ready` promises: a new one replaces a fulfilled one when loading starts.
    ready_generation: u64,
    ready_fulfilled: bool,
    pending_on_environment: bool,
    stuck: bool,
    /// Whether a `loading` event was sent that its `loadingdone` has not answered yet.
    in_cycle: bool,
    events: Vec<FontFaceSetEvent>,
}

impl FontFaceSetState {
    /// A set. While `pending_on_environment` (the first layout has not run), `ready` waits.
    pub fn new(pending_on_environment: bool) -> Self {
        Self {
            ready_fulfilled: !pending_on_environment,
            pending_on_environment,
            stuck: pending_on_environment,
            ..Self::default()
        }
    }

    pub fn status(&self) -> FontFaceSetLoadStatus {
        self.status
    }

    /// Whether the current `ready` promise is fulfilled.
    pub fn is_ready(&self) -> bool {
        self.ready_fulfilled
    }

    /// Which `ready` promise is current: it changes when a fulfilled one is replaced.
    pub fn ready_generation(&self) -> u64 {
        self.ready_generation
    }

    /// The members, in order.
    pub fn members(&self) -> impl Iterator<Item = FontFaceId> + '_ {
        self.members.iter().map(|(id, _)| *id)
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn contains(&self, id: FontFaceId) -> bool {
        self.members.iter().any(|(member, _)| *member == id)
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<FontFaceSetEvent> {
        std::mem::take(&mut self.events)
    }

    /// Adds a face that comes from a style sheet rule.
    pub fn add_css_connected(&mut self, id: FontFaceId, status: FontFaceLoadStatus) {
        if !self.contains(id) {
            self.members.push((id, true));
            self.track_new_member(id, status);
        }
    }

    /// `add`: nothing happens for a member, and a face of a style sheet rule is refused.
    pub fn add(
        &mut self,
        id: FontFaceId,
        status: FontFaceLoadStatus,
        css_connected: bool,
    ) -> Result<(), FontFaceSetError> {
        if self.contains(id) {
            return Ok(());
        }
        if css_connected {
            return Err(FontFaceSetError::InvalidModification);
        }
        self.members.push((id, false));
        self.track_new_member(id, status);
        Ok(())
    }

    fn track_new_member(&mut self, id: FontFaceId, status: FontFaceLoadStatus) {
        if status == FontFaceLoadStatus::Loading {
            self.start_loading(id);
        }
    }

    /// `delete`: whether the face was removed. A face of a style sheet rule stays.
    pub fn delete(&mut self, id: FontFaceId) -> bool {
        let Some(position) = self.members.iter().position(|(member, _)| *member == id) else {
            return false;
        };
        if self.members[position].1 {
            return false;
        }
        self.members.remove(position);
        self.loaded.retain(|face| *face != id);
        self.failed.retain(|face| *face != id);
        let was_loading = self.loading.contains(&id);
        self.loading.retain(|face| *face != id);
        if was_loading && self.loading.is_empty() {
            self.switch_to_loaded();
        }
        true
    }

    /// `clear`: removes every face that does not come from a style sheet rule.
    pub fn clear(&mut self) {
        self.members.retain(|(_, css_connected)| *css_connected);
        let members = &self.members;
        self.loaded.retain(|face| members.iter().any(|(member, _)| member == face));
        self.failed.retain(|face| members.iter().any(|(member, _)| member == face));
        if !self.loading.is_empty() {
            self.loading.clear();
            self.switch_to_loaded();
        }
    }

    /// A member changed status.
    pub fn face_status_changed(&mut self, id: FontFaceId, status: FontFaceLoadStatus) {
        if !self.contains(id) {
            return;
        }
        match status {
            FontFaceLoadStatus::Loading => self.start_loading(id),
            FontFaceLoadStatus::Loaded | FontFaceLoadStatus::Error => {
                let Some(position) = self.loading.iter().position(|face| *face == id) else {
                    return;
                };
                self.loading.remove(position);
                if status == FontFaceLoadStatus::Loaded {
                    self.loaded.push(id);
                } else {
                    self.failed.push(id);
                }
                if self.loading.is_empty() {
                    self.switch_to_loaded();
                }
            }
            FontFaceLoadStatus::Unloaded => {}
        }
    }

    /// Whether the document still has work that can ask for fonts.
    pub fn set_pending_on_environment(&mut self, pending: bool) {
        self.pending_on_environment = pending;
        if !pending && self.stuck {
            self.stuck = false;
            if self.loading.is_empty() {
                self.switch_to_loaded();
            }
        }
    }

    fn start_loading(&mut self, id: FontFaceId) {
        if self.loading.contains(&id) {
            return;
        }
        if self.loading.is_empty() {
            self.switch_to_loading();
        }
        self.loading.push(id);
    }

    fn switch_to_loading(&mut self) {
        self.status = FontFaceSetLoadStatus::Loading;
        if self.ready_fulfilled {
            self.ready_fulfilled = false;
            self.ready_generation += 1;
        }
        self.stuck = false;
        self.in_cycle = true;
        self.events.push(FontFaceSetEvent { kind: FontFaceSetEventKind::Loading, font_faces: Vec::new() });
    }

    fn switch_to_loaded(&mut self) {
        if self.pending_on_environment {
            self.stuck = true;
            return;
        }
        self.status = FontFaceSetLoadStatus::Loaded;
        self.ready_fulfilled = true;
        let (loaded, failed) = (std::mem::take(&mut self.loaded), std::mem::take(&mut self.failed));
        if !std::mem::take(&mut self.in_cycle) {
            return;
        }
        self.events.push(FontFaceSetEvent { kind: FontFaceSetEventKind::LoadingDone, font_faces: loaded });
        if !failed.is_empty() {
            self.events.push(FontFaceSetEvent { kind: FontFaceSetEventKind::LoadingError, font_faces: failed });
        }
    }
}

/// A face as the queries of a set see it.
#[derive(Clone, Copy, Debug)]
pub struct FontFaceView<'a> {
    pub id: FontFaceId,
    pub descriptors: &'a FontFaceDescriptors,
    pub status: FontFaceLoadStatus,
}

/// "Find the matching font faces": the faces of the composite a request selects whose
/// `unicode-range` includes a character of `text`.
pub fn find_matching_faces(
    faces: &[FontFaceView<'_>],
    family: &str,
    request: &FontMatchRequest,
    text: &str,
) -> Vec<FontFaceId> {
    let family_faces: Vec<&FontFaceView<'_>> =
        faces.iter().filter(|face| family_matches(face.descriptors, family)).collect();
    let descriptors: Vec<&FontFaceDescriptors> = family_faces.iter().map(|face| face.descriptors).collect();
    match_font_faces(&descriptors, request)
        .into_iter()
        .filter(|index| descriptors[*index].unicode_range.intersects_text(text))
        .map(|index| family_faces[index].id)
        .collect()
}

/// `check`: whether a text can be drawn with what is loaded. True when no declared face applies,
/// as the text then falls back without any load.
pub fn check_faces(faces: &[FontFaceView<'_>], family: &str, request: &FontMatchRequest, text: &str) -> bool {
    find_matching_faces(faces, family, request, text).iter().all(|id| {
        faces.iter().find(|face| face.id == *id).is_some_and(|face| face.status == FontFaceLoadStatus::Loaded)
    })
}

/// Where a face is on its timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontDisplayPhase {
    /// Text is laid out with the fallback's metrics and drawn invisibly.
    Block,
    /// Text is drawn with the fallback, and the font replaces it when it arrives.
    Swap,
    /// The font is no longer waited for. Text keeps the fallback.
    Failure,
}

/// The block period of `swap` and `fallback`, in milliseconds.
pub const SHORT_BLOCK_PERIOD_MS: u64 = 100;
/// The block period of `block` and the swap period of `fallback`, in milliseconds.
pub const LONG_PERIOD_MS: u64 = 3000;

/// The periods of a value of `font-display`: the block period, and the swap period, `None` when
/// it does not end. `auto` is `swap`, so text is readable while a font loads.
pub const fn display_periods(display: FontDisplay) -> (u64, Option<u64>) {
    match display {
        FontDisplay::Auto | FontDisplay::Swap => (SHORT_BLOCK_PERIOD_MS, None),
        FontDisplay::Block => (LONG_PERIOD_MS, None),
        FontDisplay::Fallback => (SHORT_BLOCK_PERIOD_MS, Some(LONG_PERIOD_MS)),
        FontDisplay::Optional => (SHORT_BLOCK_PERIOD_MS, Some(0)),
    }
}

/// The `font-display` timeline of one face. Times are milliseconds of the driver's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontDisplayTimeline {
    display: FontDisplay,
    started_at: Option<u64>,
    painted_with_fallback: bool,
}

impl FontDisplayTimeline {
    pub const fn new(display: FontDisplay) -> Self {
        Self { display, started_at: None, painted_with_fallback: false }
    }

    /// The timer starts the first time the face is used; later calls change nothing.
    pub fn start(&mut self, now: u64) {
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
    }

    pub fn is_started(&self) -> bool {
        self.started_at.is_some()
    }

    /// Text was painted with the fallback. `optional` then never swaps: it must not move
    /// text that is already on the screen.
    pub fn mark_painted_with_fallback(&mut self) {
        self.painted_with_fallback = true;
    }

    /// The phase at a time. Before the timer starts, the face is in its block period.
    pub fn phase(&self, now: u64) -> FontDisplayPhase {
        if self.display == FontDisplay::Optional && self.painted_with_fallback {
            return FontDisplayPhase::Failure;
        }
        let Some(started_at) = self.started_at else {
            return FontDisplayPhase::Block;
        };
        let elapsed = now.saturating_sub(started_at);
        let (block, swap) = display_periods(self.display);
        if elapsed < block {
            return FontDisplayPhase::Block;
        }
        match swap {
            Some(swap) if elapsed >= block.saturating_add(swap) => FontDisplayPhase::Failure,
            _ => FontDisplayPhase::Swap,
        }
    }

    /// Whether a font that arrives at this time replaces the fallback. After the failure
    /// period it does not: the text stays as it is.
    pub fn accepts_load(&self, now: u64) -> bool {
        self.phase(now) != FontDisplayPhase::Failure
    }

    /// When the phase next changes, for a timer; `None` when it never will.
    pub fn next_transition(&self, now: u64) -> Option<u64> {
        let started_at = self.started_at?;
        if self.phase(now) == FontDisplayPhase::Failure {
            return None;
        }
        let (block, swap) = display_periods(self.display);
        let block_end = started_at.saturating_add(block);
        if now < block_end {
            return Some(block_end);
        }
        swap.map(|swap| block_end.saturating_add(swap)).filter(|end| now < *end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::parse_font_face_block;
    use FontFaceLoadStatus::{Error, Loaded, Loading, Unloaded};
    use FontFaceSetEventKind as Kind;

    const A: FontFaceId = FontFaceId(1);
    const B: FontFaceId = FontFaceId(2);
    const C: FontFaceId = FontFaceId(3);

    fn kinds(set: &mut FontFaceSetState) -> Vec<Kind> {
        set.take_events().into_iter().map(|event| event.kind).collect()
    }

    #[test]
    fn status_only_moves_forward() {
        assert!(Unloaded.can_become(Loading));
        assert!(Unloaded.can_become(Error), "a source that does not parse");
        assert!(Loading.can_become(Loaded));
        assert!(Loading.can_become(Error));
        for next in [Unloaded, Loading, Loaded, Error] {
            assert!(!Loaded.can_become(next));
            assert!(!Error.can_become(next));
        }
        assert!(!Unloaded.can_become(Loaded), "a face passes through loading");
    }

    #[test]
    fn a_new_set_is_loaded_and_ready() {
        let set = FontFaceSetState::new(false);
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loaded);
        assert!(set.is_ready() && set.is_empty());
    }

    #[test]
    fn a_load_cycle_sends_its_events_in_order() {
        let mut set = FontFaceSetState::new(false);
        set.add(A, Unloaded, false).unwrap();
        set.add(B, Unloaded, false).unwrap();
        assert!(kinds(&mut set).is_empty(), "adding an unloaded face loads nothing");
        let first_ready = set.ready_generation();

        set.face_status_changed(A, Loading);
        set.face_status_changed(B, Loading);
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loading);
        assert!(!set.is_ready());
        assert_ne!(set.ready_generation(), first_ready, "a fulfilled promise is replaced");
        assert_eq!(kinds(&mut set), [Kind::Loading], "one event for the whole cycle");

        set.face_status_changed(A, Loaded);
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loading);
        assert!(kinds(&mut set).is_empty());

        set.face_status_changed(B, Error);
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loaded);
        assert!(set.is_ready());
        let events = set.take_events();
        assert_eq!(events[0], FontFaceSetEvent { kind: Kind::LoadingDone, font_faces: vec![A] });
        assert_eq!(events[1], FontFaceSetEvent { kind: Kind::LoadingError, font_faces: vec![B] });
    }

    #[test]
    fn no_error_event_without_a_failure() {
        let mut set = FontFaceSetState::new(false);
        set.add(A, Loading, false).unwrap();
        set.face_status_changed(A, Loaded);
        assert_eq!(kinds(&mut set), [Kind::Loading, Kind::LoadingDone]);
    }

    #[test]
    fn the_lists_start_empty_for_the_next_cycle() {
        let mut set = FontFaceSetState::new(false);
        set.add(A, Loading, false).unwrap();
        set.face_status_changed(A, Loaded);
        set.take_events();
        set.add(B, Loading, false).unwrap();
        set.face_status_changed(B, Loaded);
        let events = set.take_events();
        assert_eq!(events[1].font_faces, vec![B]);
    }

    #[test]
    fn ready_waits_for_the_environment() {
        let mut set = FontFaceSetState::new(true);
        assert!(!set.is_ready());
        set.add(A, Loading, false).unwrap();
        set.face_status_changed(A, Loaded);
        assert!(!set.is_ready(), "layout may still ask for fonts");
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loading);
        assert_eq!(kinds(&mut set), [Kind::Loading]);
        set.set_pending_on_environment(false);
        assert!(set.is_ready());
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loaded);
        assert_eq!(kinds(&mut set), [Kind::LoadingDone]);
    }

    #[test]
    fn ready_fulfills_with_nothing_to_load_and_sends_no_event() {
        let mut set = FontFaceSetState::new(true);
        set.set_pending_on_environment(false);
        assert!(set.is_ready());
        assert!(set.take_events().is_empty());
    }

    #[test]
    fn the_end_of_the_environment_does_not_end_a_running_load() {
        let mut set = FontFaceSetState::new(true);
        set.add(A, Loading, false).unwrap();
        set.set_pending_on_environment(false);
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loading);
        set.face_status_changed(A, Loaded);
        assert!(set.is_ready());
    }

    #[test]
    fn add_and_delete() {
        let mut set = FontFaceSetState::new(false);
        set.add(A, Unloaded, false).unwrap();
        set.add(A, Unloaded, false).unwrap();
        assert_eq!(set.len(), 1, "adding twice does nothing");
        assert_eq!(set.add(B, Unloaded, true), Err(FontFaceSetError::InvalidModification));
        assert!(set.delete(A));
        assert!(!set.delete(A));
        set.add_css_connected(C, Unloaded);
        assert_eq!(set.add(C, Unloaded, true), Ok(()), "already a member");
        assert!(!set.delete(C), "a face of a rule stays");
        assert!(set.contains(C));
    }

    #[test]
    fn deleting_the_last_loading_face_ends_the_cycle() {
        let mut set = FontFaceSetState::new(false);
        set.add(A, Loading, false).unwrap();
        set.take_events();
        assert!(set.delete(A));
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loaded);
        assert_eq!(kinds(&mut set), [Kind::LoadingDone]);
    }

    #[test]
    fn clear_keeps_faces_of_rules() {
        let mut set = FontFaceSetState::new(false);
        set.add_css_connected(C, Unloaded);
        set.add(A, Loading, false).unwrap();
        set.add(B, Unloaded, false).unwrap();
        set.take_events();
        set.clear();
        assert_eq!(set.members().collect::<Vec<_>>(), [C]);
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loaded);
        assert_eq!(kinds(&mut set), [Kind::LoadingDone]);
    }

    #[test]
    fn a_face_outside_the_set_is_ignored() {
        let mut set = FontFaceSetState::new(false);
        set.face_status_changed(A, Loading);
        assert_eq!(set.status(), FontFaceSetLoadStatus::Loaded);
        assert!(set.take_events().is_empty());
    }

    fn descriptors(text: &str) -> FontFaceDescriptors {
        parse_font_face_block(&format!("font-family: F; src: url(a); {text}")).unwrap()
    }

    #[test]
    fn check_and_find() {
        let (latin, greek, bold) = (
            descriptors("unicode-range: U+0-FF"),
            descriptors("unicode-range: U+370-3FF"),
            descriptors("font-weight: 700"),
        );
        let view = |status_a, status_b| {
            [
                FontFaceView { id: A, descriptors: &latin, status: status_a },
                FontFaceView { id: B, descriptors: &greek, status: status_b },
                FontFaceView { id: C, descriptors: &bold, status: Unloaded },
            ]
        };
        let request = FontMatchRequest::default();
        let faces = view(Unloaded, Unloaded);
        assert_eq!(find_matching_faces(&faces, "f", &request, "a"), vec![A], "family names ignore case");
        assert_eq!(find_matching_faces(&faces, "F", &request, "a\u{3b1}"), vec![B, A]);
        assert!(!check_faces(&faces, "F", &request, "a"));
        assert!(check_faces(&faces, "F", &request, "\u{3042}"), "excluded by unicode-range");
        assert!(check_faces(&faces, "Other", &request, "a"), "no such family");
        assert!(check_faces(&faces, "F", &request, ""), "no text");
        let faces = view(Loaded, Unloaded);
        assert!(check_faces(&faces, "F", &request, "a"));
        assert!(!check_faces(&faces, "F", &request, "a\u{3b1}"));
        let faces = view(Error, Unloaded);
        assert!(!check_faces(&faces, "F", &request, "a"), "a failed face is not loaded");
    }

    fn timeline(display: FontDisplay) -> FontDisplayTimeline {
        let mut timeline = FontDisplayTimeline::new(display);
        timeline.start(1000);
        timeline
    }

    #[test]
    fn the_periods_of_each_value() {
        use FontDisplayPhase::{Block, Failure, Swap};
        let cases = [
            (FontDisplay::Auto, [(1000, Block), (1099, Block), (1100, Swap), (9_000_000, Swap)]),
            (FontDisplay::Swap, [(1000, Block), (1099, Block), (1100, Swap), (9_000_000, Swap)]),
            (FontDisplay::Block, [(1000, Block), (3999, Block), (4000, Swap), (9_000_000, Swap)]),
            (FontDisplay::Fallback, [(1099, Block), (1100, Swap), (4099, Swap), (4100, Failure)]),
            (FontDisplay::Optional, [(1000, Block), (1099, Block), (1100, Failure), (9_000_000, Failure)]),
        ];
        for (display, phases) in cases {
            let timeline = timeline(display);
            for (now, phase) in phases {
                assert_eq!(timeline.phase(now), phase, "{display:?} at {now}");
            }
        }
    }

    #[test]
    fn the_timer_starts_once() {
        let mut timeline = FontDisplayTimeline::new(FontDisplay::Swap);
        assert_eq!(timeline.phase(5000), FontDisplayPhase::Block, "not started");
        assert!(!timeline.is_started());
        timeline.start(5000);
        timeline.start(9000);
        assert_eq!(timeline.phase(5100), FontDisplayPhase::Swap);
    }

    #[test]
    fn a_late_font_does_not_replace_the_fallback() {
        assert!(timeline(FontDisplay::Swap).accepts_load(100_000));
        assert!(timeline(FontDisplay::Fallback).accepts_load(4099));
        assert!(!timeline(FontDisplay::Fallback).accepts_load(4100));
        assert!(timeline(FontDisplay::Optional).accepts_load(1099));
        assert!(!timeline(FontDisplay::Optional).accepts_load(1100));
    }

    #[test]
    fn optional_never_swaps_after_a_fallback_was_painted() {
        let mut optional = timeline(FontDisplay::Optional);
        optional.mark_painted_with_fallback();
        assert_eq!(optional.phase(1001), FontDisplayPhase::Failure);
        assert!(!optional.accepts_load(1001));
        let mut swap = timeline(FontDisplay::Swap);
        swap.mark_painted_with_fallback();
        assert!(swap.accepts_load(1001), "swap replaces what was painted");
    }

    #[test]
    fn timers_wake_at_each_transition() {
        let fallback = timeline(FontDisplay::Fallback);
        assert_eq!(fallback.next_transition(1000), Some(1100));
        assert_eq!(fallback.next_transition(1100), Some(4100));
        assert_eq!(fallback.next_transition(4100), None);
        let swap = timeline(FontDisplay::Swap);
        assert_eq!(swap.next_transition(1000), Some(1100));
        assert_eq!(swap.next_transition(1100), None, "the swap period does not end");
        assert_eq!(FontDisplayTimeline::new(FontDisplay::Swap).next_transition(0), None, "not started");
    }
}
