//! The page of the theme transitions of a panel, built in code.

use ferroui_base::layout::Orientation;
use ferroui_base::media::{Brushes, TextWrapping};
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{Border, Button, ContentPage, Control, StackPanel, TextBlock};
use ferroui_controls_motion::{
    AddDeleteThemeTransition, EntranceThemeTransition, ThemeTransition, ThemeTransitions, TransitionCollection,
};
use std::rc::Rc;

fn square() -> Ref<Border> {
    let square = Border::new();
    square.set_width(48.0);
    square.set_height(48.0);
    square.set_background(Some(Brushes::blue()));
    square
}

fn action(text: &str, run: impl Fn() + 'static) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(text.to_owned()) as BoxedValue));
    button.click(move |_, _| run());
    button
}

/// Squares enter from an offset, leave with a fade and the others slide to their places.
pub fn page() -> Ref<ContentPage> {
    let squares = StackPanel::new();
    squares.set_orientation(Orientation::Horizontal);
    squares.set_spacing(8.0);
    for _ in 0..4 {
        squares.children().add(&square());
    }
    let entrance = EntranceThemeTransition::new();
    entrance.set_is_staggering_enabled(true);
    let transitions: [Ref<ThemeTransition>; 2] = [entrance.upcast(), AddDeleteThemeTransition::new().upcast()];
    ThemeTransitions::set_children_transitions(&squares, Some(TransitionCollection::from_items(transitions)));

    let actions = StackPanel::new();
    actions.set_orientation(Orientation::Horizontal);
    actions.set_spacing(8.0);
    let target = squares.downgrade();
    actions.children().add(&action("Add", move || {
        if let Some(squares) = target.upgrade() {
            squares.children().insert(0, square().upcast::<Control>());
        }
    }));
    let target = squares.downgrade();
    actions.children().add(&action("Remove", move || {
        if let Some(squares) = target.upgrade() {
            if squares.children().count() > 0 {
                squares.children().remove_at(0);
            }
        }
    }));

    let caption = TextBlock::new();
    caption.set_text(Some("The theme transitions of the panel animate the squares added and removed, on the compositor."));
    caption.set_text_wrapping(TextWrapping::Wrap);

    let panel = StackPanel::new();
    panel.set_margin(Thickness::uniform(24.0));
    panel.set_spacing(16.0);
    panel.children().add(&caption);
    panel.children().add(&actions);
    panel.children().add(&squares);

    let page = ContentPage::new();
    page.set_header(Some(Rc::new("Theme transitions".to_owned()) as BoxedValue));
    page.set_content(Some(Control::boxed(&panel)));
    page
}
