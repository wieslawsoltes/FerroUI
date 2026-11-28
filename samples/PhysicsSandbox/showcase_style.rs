//! Shared Fluent surfaces that follow the app's theme without rebuilding state.

use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Border, Button, TextBlock};
use ferroui_view::prelude::*;

/// A stock toggle used for exclusive selection: selecting it again preserves the current page.
pub(crate) fn selection_button<T: Copy + PartialEq + 'static>(
    selected: Signal<T>,
    option: T,
    on_selected: impl Fn() + 'static,
) -> El<ToggleButton> {
    toggle_button()
        .is_checked(move || Some(selected.get() == option))
        .on(Button::click_event(), move |button, _| {
            // ToggleButton toggles before raising Click. Restore its native checked state
            // directly; notifying an unchanged selection would rebuild its dynamic page.
            button.set_current_value(ToggleButton::is_checked_property(), Some(true));
            if selected.get_untracked() == option {
                return;
            }
            selected.set(option);
            on_selected();
        })
}

pub(crate) fn surface() -> El<Border> {
    border()
        .corner_radius(8.0)
        .border_thickness(1.0)
        .dynamic_resource(
            Border::background_property(),
            "CardBackgroundFillColorDefaultBrush",
        )
        .dynamic_resource(
            Border::border_brush_property(),
            "CardStrokeColorDefaultBrush",
        )
}

pub(crate) fn subtle_text() -> El<TextBlock> {
    text_block()
        .font_size(14.0)
        .text_wrapping(TextWrapping::Wrap)
        .dynamic_resource(
            TextBlock::foreground_property(),
            "TextFillColorSecondaryBrush",
        )
}

pub(crate) fn page_header(title: &'static str, description: &'static str) -> Element {
    stack_panel()
        .spacing(6.0)
        .child(
            text_block()
                .text(title)
                .font_size(24.0)
                .font_weight(FontWeight::SemiBold),
        )
        .child(subtle_text().text(description))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn selecting_current_toggle_keeps_checked_without_notifying_or_rebuilding() {
        let callbacks = Rc::new(Cell::new(0));
        let notifications = Rc::new(Cell::new(0));
        let owner = Owner::new_root();
        let (selected, first, second) = owner.with(|| {
            let selected = signal(0u8);
            let notifications = notifications.clone();
            effect(move || {
                selected.get();
                notifications.set(notifications.get() + 1);
            });
            let make = |option| {
                let callbacks = callbacks.clone();
                selection_button(selected, option, move || callbacks.set(callbacks.get() + 1))
                    .into_element()
                    .mount()
                    .cast::<ToggleButton>()
                    .unwrap()
            };
            (selected, make(0), make(1))
        });

        assert_eq!(first.is_checked(), Some(true));
        assert_eq!(second.is_checked(), Some(false));
        first.on_click();
        assert_eq!(first.is_checked(), Some(true));
        assert_eq!(selected.get_untracked(), 0);
        assert_eq!(callbacks.get(), 0);
        assert_eq!(notifications.get(), 1);

        second.on_click();
        assert_eq!(first.is_checked(), Some(false));
        assert_eq!(second.is_checked(), Some(true));
        assert_eq!(selected.get_untracked(), 1);
        assert_eq!(callbacks.get(), 1);
        assert_eq!(notifications.get(), 2);

        second.on_click();
        assert_eq!(second.is_checked(), Some(true));
        assert_eq!(selected.get_untracked(), 1);
        assert_eq!(callbacks.get(), 1);
        assert_eq!(notifications.get(), 2);
        owner.dispose();
    }
}
