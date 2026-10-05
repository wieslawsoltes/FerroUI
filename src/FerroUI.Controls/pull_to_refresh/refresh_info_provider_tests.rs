use super::RefreshInfoProvider;
use crate::test_support::test_scope;
use ferroui_base::input::{PullDirection, PullGestureEndedEventArgs, PullGestureEventArgs};
use ferroui_base::{Size, Vector};

// Repro for the "entered desync" bug.
//
// Real-world flow that triggers it:
//   1. User starts pulling. The scrollable pull gesture recognizer raises the pull
//      gesture event -> `interacting_state_entered` sets `IsInteractingForRefresh` and
//      `entered`.
//   2. The pull motion produces a small scroll change that pushes the scroll offset
//      past the threshold. The scroll changed handler of the adapter clears
//      `IsInteractingForRefresh` directly, bypassing the end of the pull gesture.
//      (The pointer released handler of the adapter does the same.)
//   3. `entered` stays set.
//   4. The user is still pulling, more pull gesture events arrive.
//      `interacting_state_entered` short-circuits because `entered` is already set,
//      so `IsInteractingForRefresh` is NOT reasserted.
//   5. The refresh visualizer never re-enters the Interacting state -> the spinner
//      does not appear.
#[test]
fn is_interacting_for_refresh_is_reasserted_after_being_cleared_externally() {
    let _scope = test_scope();
    let provider = RefreshInfoProvider::new(PullDirection::TopToBottom, Some(Size::new(100.0, 100.0)), None);

    let pull_args = PullGestureEventArgs::new(0, Vector::new(0.0, 50.0), PullDirection::TopToBottom);

    // 1. First pull gesture event of a gesture
    provider.interacting_state_entered(&pull_args);
    assert!(
        provider.is_interacting_for_refresh(),
        "IsInteractingForRefresh should be true after the first PullGestureEvent"
    );

    // 2. Adapter clears the flag directly (simulating the scroll changed or the
    //    pointer released handler)
    provider.set_is_interacting_for_refresh(false);
    assert!(!provider.is_interacting_for_refresh());

    // 3. Pull is still in progress, the next pull gesture event arrives
    provider.interacting_state_entered(&pull_args);

    assert!(
        provider.is_interacting_for_refresh(),
        "PullGestureEvent must re-assert IsInteractingForRefresh after it was cleared by something other than PullGestureEnded"
    );
}

// Repro for the typo where horizontal pulls checked the height instead of the width
// for zero. With a zero width, value.x / width produces infinity or not-a-number,
// which then breaks every downstream consumer of the interaction ratio.
#[test]
fn horizontal_pull_with_zero_width_produces_safe_interaction_ratio() {
    let _scope = test_scope();
    let provider = RefreshInfoProvider::new(PullDirection::LeftToRight, Some(Size::new(0.0, 100.0)), None);

    provider.values_changed(Vector::new(50.0, 0.0));

    assert!(!provider.interaction_ratio().is_nan());
    assert!(!provider.interaction_ratio().is_infinite());
}

// Sanity check for the existing happy-path: a complete gesture lifecycle
// (Entered -> Exited -> Entered) must toggle IsInteractingForRefresh correctly.
#[test]
fn normal_gesture_lifecycle_toggles_is_interacting_for_refresh_correctly() {
    let _scope = test_scope();
    let provider = RefreshInfoProvider::new(PullDirection::TopToBottom, Some(Size::new(100.0, 100.0)), None);

    let pull_args = PullGestureEventArgs::new(0, Vector::new(0.0, 50.0), PullDirection::TopToBottom);
    let end_args = PullGestureEndedEventArgs::new(0, PullDirection::TopToBottom);

    provider.interacting_state_entered(&pull_args);
    assert!(provider.is_interacting_for_refresh());

    provider.interacting_state_exited(&end_args);
    assert!(!provider.is_interacting_for_refresh());

    // Next gesture should work
    provider.interacting_state_entered(&pull_args);
    assert!(provider.is_interacting_for_refresh());
}

// --- Additional tests (not upstream); expectations derived from the reference source. ---

#[test]
fn values_changed_computes_the_ratio_along_the_pull_axis_capped_at_one() {
    let _scope = test_scope();

    let vertical = RefreshInfoProvider::new(PullDirection::BottomToTop, Some(Size::new(50.0, 200.0)), None);
    vertical.values_changed(Vector::new(25.0, 50.0));
    assert_eq!(vertical.interaction_ratio(), 0.25);
    vertical.values_changed(Vector::new(0.0, 500.0));
    assert_eq!(vertical.interaction_ratio(), 1.0);

    let horizontal = RefreshInfoProvider::new(PullDirection::RightToLeft, Some(Size::new(50.0, 200.0)), None);
    horizontal.values_changed(Vector::new(25.0, 50.0));
    assert_eq!(horizontal.interaction_ratio(), 0.5);

    // A zero dimension along the pull axis counts as fully pulled.
    let zero_height = RefreshInfoProvider::new(PullDirection::TopToBottom, Some(Size::new(100.0, 0.0)), None);
    zero_height.values_changed(Vector::new(0.0, 5.0));
    assert_eq!(zero_height.interaction_ratio(), 1.0);

    // No size given: the default (empty) size.
    let no_size = RefreshInfoProvider::new(PullDirection::TopToBottom, None, None);
    assert_eq!(no_size.refresh_visualizer_size(), Size::default());
    assert_eq!(no_size.execution_ratio(), RefreshInfoProvider::DEFAULT_EXECUTION_RATIO);
}

#[test]
fn peeking_mode_keeps_is_interacting_for_refresh_cleared() {
    let _scope = test_scope();
    let provider = RefreshInfoProvider::new(PullDirection::TopToBottom, Some(Size::new(100.0, 100.0)), None);

    provider.set_peeking_mode(true);
    provider.set_is_interacting_for_refresh(true);
    assert!(!provider.is_interacting_for_refresh());

    // The interaction ratio still follows the pull.
    provider.interacting_state_entered(&PullGestureEventArgs::new(0, Vector::new(0.0, 30.0), PullDirection::TopToBottom));
    assert!(!provider.is_interacting_for_refresh());
    assert_eq!(provider.interaction_ratio(), 0.3);

    provider.interacting_state_exited(&PullGestureEndedEventArgs::new(0, PullDirection::TopToBottom));
    assert_eq!(provider.interaction_ratio(), 0.0);

    provider.set_peeking_mode(false);
    provider.interacting_state_entered(&PullGestureEventArgs::new(1, Vector::new(0.0, 30.0), PullDirection::TopToBottom));
    assert!(provider.is_interacting_for_refresh());
}
