//! Product interaction patterns built with real WinUI-themed FerroUI controls.
//!
//! Each page owns its animation clock and pointer capture, and tears both down
//! when it leaves the visual tree. Physics changes only render transforms.

mod details;
mod floating_panel;
mod inbox;
mod workspace;

use crate::icons::{icon_label, Icons};
use crate::showcase_style::{page_header, selection_button, subtle_text, surface};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, MatrixTransform, TextWrapping};
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{Matrix, Rect, Ref, StyledElement, Visual};
use ferroui_controls::{Border, Canvas, Dock, StackPanel};
use ferroui_physics::prelude::*;
use ferroui_view::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Interaction {
    FloatingPanel,
    Inbox,
    Details,
    Workspace,
}

/// A cohesive set of reusable, keyboard-accessible application interactions.
pub fn live_ui_tree_showcase() -> Element {
    let selected = signal(match std::env::var("FERROUI_UI_SAMPLE").as_deref() {
        Ok("inbox") => Interaction::Inbox,
        Ok("details") => Interaction::Details,
        Ok("workspace") => Interaction::Workspace,
        _ => Interaction::FloatingPanel,
    });
    let mut tabs = wrap_panel().item_spacing(8.0).line_spacing(8.0);
    for (page, label, icon) in [
        (
            Interaction::FloatingPanel,
            "Floating inspector",
            Icons::UI_MOTION,
        ),
        (Interaction::Inbox, "Task inbox", Icons::LAYERS),
        (Interaction::Details, "Details drawer", Icons::SLIDER),
        (Interaction::Workspace, "Workspace actions", Icons::PERSON),
    ] {
        tabs = tabs
            .child(selection_button(selected, page, || {}).content(icon_label(icon, label, 16.0)));
    }
    dock_panel()
        .clip_to_bounds(true)
        .child(
            stack_panel()
                .dock_panel_dock(Dock::Top)
                .margin((24.0, 24.0, 24.0, 0.0))
                .spacing(16.0)
                .child(page_header(
                    "Interactive workspace",
                    "Use motion where it helps: position an inspector, triage tasks, reveal details, and confirm an action.",
                ))
                .child(tabs),
        )
        .child(dynamic(move || match selected.get() {
            Interaction::FloatingPanel => floating_panel::build(),
            Interaction::Inbox => inbox::build(),
            Interaction::Details => details::build(),
            Interaction::Workspace => workspace::build(),
        }))
        .into()
}
