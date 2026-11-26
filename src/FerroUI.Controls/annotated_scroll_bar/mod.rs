//! WinUI's `AnnotatedScrollBar`, its labels, its event arguments and its scrolling kind (#1268;
//! docs/porting/winui-controls.md, section 10.13).

mod annotated_scroll_bar;
mod annotated_scroll_bar_types;

#[cfg(test)]
mod annotated_scroll_bar_tests;

pub use annotated_scroll_bar::AnnotatedScrollBar;
pub use annotated_scroll_bar_types::{
    AnnotatedScrollBarDetailLabelRequestedEventArgs, AnnotatedScrollBarLabel, AnnotatedScrollBarScrollingEventArgs,
    AnnotatedScrollBarScrollingEventKind,
};
