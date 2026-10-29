//! CardFieldBehavior: routed pointer input, capture, click-to-move and drag runs.
use crate::{controls::CardStackPlacementControl, view_models::*, views::casino_view::CasinoView};
use ferroui::base::input::InputElement;
use std::rc::Rc;

pub fn attach(field: &Rc<CardStackPlacementControl>, casino: &Rc<CasinoView>) {
    let weak_field = Rc::downgrade(field);
    let weak_casino = Rc::downgrade(casino);
    field.canvas.pointer_pressed(move |_, event| {
        let (Some(field), Some(casino)) = (weak_field.upgrade(), weak_casino.upgrade()) else {
            return;
        };
        if casino.input_blocked() {
            return;
        }
        let point = event.get_position(Some(&field.canvas));
        let target = field.pile_at(point);
        if target == Some(Pile::Stock) {
            casino.draw();
            event.set_handled(true);
            return;
        }
        let source = field.card_at(point);
        if let Some(selected) = field.selected.get()
            && target.is_some_and(|p| p != selected.pile)
            && casino.move_card(selected, target.unwrap())
        {
            event.set_handled(true);
            return;
        }
        if let Some(source) = source {
            let playable = casino
                .model
                .borrow()
                .game()
                .is_some_and(|g| g.can_select(source));
            if playable {
                // Keep the selection when a double-click cannot reach a foundation.
                field.select(Some(source));
                if event.click_count() >= 2
                    || event
                        .get_current_point(Some(&field.canvas))
                        .properties
                        .is_right_button_pressed
                {
                    casino.move_to_foundation(source);
                } else {
                    field.select(Some(source));
                    field.begin_drag(source, point);
                    event
                        .pointer()
                        .capture(Some(&field.canvas.clone().upcast::<InputElement>()));
                    field.canvas.focus();
                }
                event.set_handled(true);
            }
        } else {
            field.select(None);
        }
    });
    let weak_field = Rc::downgrade(field);
    field.canvas.pointer_moved(move |_, event| {
        if let Some(field) = weak_field.upgrade()
            && field.drag.get().is_some()
        {
            field.move_drag(event.get_position(Some(&field.canvas)));
            event.set_handled(true);
        }
    });
    let weak_field = Rc::downgrade(field);
    let weak_casino = Rc::downgrade(casino);
    field.canvas.pointer_released(move |_, event| {
        let (Some(field), Some(casino)) = (weak_field.upgrade(), weak_casino.upgrade()) else {
            return;
        };
        let drag = field.drag.take();
        event.pointer().capture(None);
        let Some(drag) = drag else { return };
        let point = event.get_position(Some(&field.canvas));
        let moved = (point.x - drag.start.x).abs() + (point.y - drag.start.y).abs() > 8.0;
        if moved {
            if let Some(to) = field.pile_at(point)
                && casino.move_card(drag.from, to)
            {
                return;
            }
            field.restore_positions();
            casino
                .set_message("That move is not allowed. Select a card or run, then a destination.");
        } else {
            field.restore_positions();
        }
    });
    let weak_field = Rc::downgrade(field);
    field.canvas.pointer_capture_lost(move |_, _| {
        if let Some(field) = weak_field.upgrade()
            && field.drag.get().is_some()
        {
            field.restore_positions();
        }
    });
}
