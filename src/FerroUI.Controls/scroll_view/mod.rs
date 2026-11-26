//! WinUI's `ScrollView`, its scroll bar controller and the scrolling enumerations, options and
//! event arguments (#1268; docs/porting/winui-controls.md, section 10.12).

mod scroll_bar_controller;
mod scroll_view;
mod scrolling_types;

#[cfg(test)]
mod scroll_view_tests;

pub use scroll_view::ScrollView;
pub use scrolling_types::{
    ScrollingAnimationMode, ScrollingChainMode, ScrollingContentOrientation, ScrollingInputKinds,
    ScrollingInteractionState, ScrollingRailMode, ScrollingScrollAnimationStartingEventArgs,
    ScrollingScrollBarVisibility, ScrollingScrollCompletedEventArgs, ScrollingScrollMode,
    ScrollingScrollOptions, ScrollingScrollStartingEventArgs, ScrollingSnapPointsMode,
    ScrollingZoomAnimationStartingEventArgs, ScrollingZoomCompletedEventArgs, ScrollingZoomMode,
    ScrollingZoomOptions, ScrollingZoomStartingEventArgs,
};
