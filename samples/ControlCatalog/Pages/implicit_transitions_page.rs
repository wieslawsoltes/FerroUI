//! The page of the implicit transitions of an element, built in code.

use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{Brushes, TextWrapping};
use ferroui_base::{BoxedValue, Ref, Thickness, Vector3D};
use ferroui_controls::{Border, Button, ContentPage, Control, StackPanel, TextBlock};
use ferroui_controls_motion::{ImplicitTransitions, ScalarTransition, Vector3Transition};
use std::cell::Cell;
use std::rc::Rc;

/// A square with the transitions of its opacity, rotation, scale and translation.
fn square() -> Ref<Border> {
    let square = Border::new();
    square.set_width(64.0);
    square.set_height(64.0);
    square.set_horizontal_alignment(HorizontalAlignment::Left);
    square.set_background(Some(Brushes::blue()));
    ImplicitTransitions::set_opacity_transition(&square, Some(ScalarTransition::new()));
    ImplicitTransitions::set_rotation_transition(&square, Some(ScalarTransition::new()));
    ImplicitTransitions::set_scale_transition(&square, Some(Vector3Transition::new()));
    ImplicitTransitions::set_translation_transition(&square, Some(Vector3Transition::new()));
    square
}

fn action(text: &str, square: &Ref<Border>, run: fn(&Border, bool)) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(text.to_owned()) as BoxedValue));
    let square = square.downgrade();
    let on = Cell::new(false);
    button.click(move |_, _| {
        if let Some(square) = square.upgrade() {
            on.set(!on.get());
            run(&square, on.get());
        }
    });
    button
}

/// Each button changes one facade, which animates on the composition visual of the square.
pub fn page() -> Ref<ContentPage> {
    let target = square();
    let actions = StackPanel::new();
    actions.set_orientation(Orientation::Horizontal);
    actions.set_spacing(8.0);
    actions.children().add(&action("Opacity", &target, |square, on| square.set_opacity(if on { 0.25 } else { 1.0 })));
    actions.children().add(&action("Rotation", &target, |square, on| {
        ImplicitTransitions::set_rotation(square, if on { 45.0 } else { 0.0 });
    }));
    actions.children().add(&action("Scale", &target, |square, on| {
        let value = if on { 1.5 } else { 1.0 };
        ImplicitTransitions::set_scale(square, Vector3D::new(value, value, 1.0));
    }));
    actions.children().add(&action("Translation", &target, |square, on| {
        square.set_translation(Vector3D::new(if on { 120.0 } else { 0.0 }, 0.0, 0.0));
    }));

    let caption = TextBlock::new();
    caption.set_text(Some("A change of each property animates through the transition set on the element."));
    caption.set_text_wrapping(TextWrapping::Wrap);

    let panel = StackPanel::new();
    panel.set_margin(Thickness::uniform(24.0));
    panel.set_spacing(16.0);
    panel.children().add(&caption);
    panel.children().add(&actions);
    panel.children().add(&target);

    let page = ContentPage::new();
    page.set_header(Some(Rc::new("Implicit transitions".to_owned()) as BoxedValue));
    page.set_content(Some(Control::boxed(&panel)));
    page
}
