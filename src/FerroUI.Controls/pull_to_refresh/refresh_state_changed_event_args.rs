//! WinUI's `RefreshStateChangedEventArgs` (`controls/dev/PullToRefresh/RefreshVisualizer`,
//! `RefreshVisualizer.idl` of microsoft-ui-xaml; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::RefreshVisualizerState;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};

/// Provides event data for the `RefreshStateChanged` event of a
/// [`RefreshVisualizer`](super::RefreshVisualizer): the state it left and the state it entered.
#[derive(Clone)]
pub struct RefreshStateChangedEventArgs {
    base: RoutedEventArgs,
    old_state: RefreshVisualizerState,
    new_state: RefreshVisualizerState,
}

ferro_routed_event_args!(RefreshStateChangedEventArgs: RoutedEventArgs);

impl RefreshStateChangedEventArgs {
    /// Creates args of a change from `old_state` to `new_state`.
    pub fn new<T: ?Sized>(
        old_state: RefreshVisualizerState,
        new_state: RefreshVisualizerState,
        routed_event: Option<&RoutedEvent<T>>,
    ) -> Self {
        let base = match routed_event {
            Some(routed_event) => RoutedEventArgs::with_event(routed_event),
            None => RoutedEventArgs::new(),
        };
        Self { base, old_state, new_state }
    }

    /// Gets the state the visualizer left.
    pub fn old_state(&self) -> RefreshVisualizerState {
        self.old_state
    }

    /// Gets the state the visualizer entered.
    pub fn new_state(&self) -> RefreshVisualizerState {
        self.new_state
    }
}
