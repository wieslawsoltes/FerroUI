//! The properties `Control` names for its `on_property_changed` (`PropertyChangeHandler`,
//! desktop-performance.md section 9.59).

use crate::{Border, TextBlock};
use ferroui_base::layout::Layoutable;
use ferroui_base::{FerroProperty, PropertyChangeHandler as Handler, Rect, Size, Visual};
use std::cell::RefCell;
use std::rc::Rc;

const HANDLERS: [Handler; 6] = [
    Handler::Animatable,
    Handler::StyledElement,
    Handler::Visual,
    Handler::Layoutable,
    Handler::InputElement,
    Handler::Control,
];

fn handlers_of(property: &'static FerroProperty) -> Vec<Handler> {
    HANDLERS.into_iter().filter(|handler| property.has_change_handler(*handler)).collect()
}

#[test]
fn control_names_the_bounds_for_its_handler_and_no_class_names_a_text() {
    // The first instance of a class runs the static constructors of the class and its bases.
    let _text = TextBlock::new();

    assert_eq!(handlers_of(Visual::bounds_property().as_property()), [Handler::Control]);
    assert!(handlers_of(TextBlock::text_property().as_property()).is_empty());
    assert!(handlers_of(Layoutable::width_property().as_property()).is_empty());
    assert_eq!(
        handlers_of(Visual::is_visible_property().as_property()),
        [Handler::Visual, Handler::Layoutable, Handler::InputElement]
    );
}

#[test]
fn a_change_of_the_bounds_still_raises_size_changed() {
    let target = Border::new();
    let sizes = Rc::new(RefCell::new(Vec::new()));
    let seen = sizes.clone();
    let _token = target.size_changed(move |_, e| {
        seen.borrow_mut().push((e.previous_size(), e.new_size()));
    });

    target.measure(Size::new(100.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(*sizes.borrow(), [(Size::new(0.0, 0.0), Size::new(100.0, 50.0))]);

    // The same size at another position is a change of the bounds and no change of the size.
    target.arrange(Rect::new(10.0, 20.0, 100.0, 50.0));
    assert_eq!(sizes.borrow().len(), 1);

    target.arrange(Rect::new(10.0, 20.0, 60.0, 50.0));
    assert_eq!(
        *sizes.borrow(),
        [(Size::new(0.0, 0.0), Size::new(100.0, 50.0)), (Size::new(100.0, 50.0), Size::new(60.0, 50.0))]
    );
}
