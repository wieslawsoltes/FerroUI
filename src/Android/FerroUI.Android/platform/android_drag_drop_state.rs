//! What the drag events of the system become: the part of
//! `AndroidDragDrop` of the reference that is a function of values, so the
//! host tests run it.

use ferroui_base::input::raw::RawDragEventType;
use ferroui_base::input::DragDropEffects;
use ferroui_base::Point;

/// `DragEvent.ACTION_DRAG_STARTED`.
pub(crate) const ACTION_DRAG_STARTED: i32 = 1;
/// `DragEvent.ACTION_DRAG_LOCATION`.
pub(crate) const ACTION_DRAG_LOCATION: i32 = 2;
/// `DragEvent.ACTION_DROP`.
pub(crate) const ACTION_DROP: i32 = 3;
/// `DragEvent.ACTION_DRAG_ENDED`.
pub(crate) const ACTION_DRAG_ENDED: i32 = 4;
/// `DragEvent.ACTION_DRAG_ENTERED`.
pub(crate) const ACTION_DRAG_ENTERED: i32 = 5;
/// `DragEvent.ACTION_DRAG_EXITED`.
pub(crate) const ACTION_DRAG_EXITED: i32 = 6;

/// What a drag event of the system asks of the backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DragStep {
    /// Nothing to raise.
    None,
    /// Raise an enter, an over or a leave at the point, with the formats of
    /// the description (the system gives no data before the drop).
    Raise(RawDragEventType, Point),
    /// Raise the drop at the point, with the data of the clip, while the
    /// permissions of the drop are held.
    Drop(Point),
    /// A drag this application started ended with the effect.
    Ended(DragDropEffects),
}

/// The state of a drag over the view, from the system's enter to its exit
/// or drop.
#[derive(Default)]
pub(crate) struct DragDropState {
    is_in_active_drag_drop: bool,
    initial_event_sent: bool,
    last_position: Point,
}

impl DragDropState {
    /// The step of a drag event: `action` of the event, its point in the
    /// coordinates of the root (`ACTION_DRAG_LOCATION` and `ACTION_DROP`
    /// carry one), the result an ended drag reports and whether the ended
    /// drag is the one this view started (its local state).
    pub(crate) fn on_drag(&mut self, action: i32, position: Point, result: bool, is_own_drag: bool) -> DragStep {
        match action {
            ACTION_DRAG_ENDED => {
                if !is_own_drag {
                    return DragStep::None;
                }
                // The system has no drag effects: a drag and drop is a copy, though the source
                // may allow the target to change its data. The data of the application is
                // read-only.
                DragStep::Ended(if result { DragDropEffects::COPY } else { DragDropEffects::NONE })
            }
            ACTION_DRAG_ENTERED => {
                self.is_in_active_drag_drop = true;
                self.initial_event_sent = false;
                self.last_position = Point::default();
                // The enter has no point: the first location raises the enter.
                DragStep::None
            }
            ACTION_DRAG_EXITED => {
                self.is_in_active_drag_drop = false;
                let entered = std::mem::replace(&mut self.initial_event_sent, false);
                if entered {
                    DragStep::Raise(RawDragEventType::DragLeave, self.last_position)
                } else {
                    DragStep::None
                }
            }
            ACTION_DRAG_LOCATION => {
                if !self.is_in_active_drag_drop {
                    return DragStep::None;
                }
                self.last_position = position;
                let event_type =
                    if self.initial_event_sent { RawDragEventType::DragOver } else { RawDragEventType::DragEnter };
                self.initial_event_sent = true;
                DragStep::Raise(event_type, position)
            }
            ACTION_DROP => {
                self.initial_event_sent = false;
                if !self.is_in_active_drag_drop {
                    return DragStep::None;
                }
                self.is_in_active_drag_drop = false;
                self.last_position = position;
                DragStep::Drop(position)
            }
            // `ACTION_DRAG_STARTED` reaches every view for every drag of the device, other
            // applications included, wherever the pointer is.
            _ => DragStep::None,
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of its drag and drop.
    use super::*;

    fn at(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn the_first_location_enters_and_the_next_ones_move() {
        let mut state = DragDropState::default();
        assert_eq!(state.on_drag(ACTION_DRAG_STARTED, at(0.0, 0.0), false, false), DragStep::None);
        assert_eq!(state.on_drag(ACTION_DRAG_ENTERED, at(0.0, 0.0), false, false), DragStep::None);
        assert_eq!(
            state.on_drag(ACTION_DRAG_LOCATION, at(10.0, 20.0), false, false),
            DragStep::Raise(RawDragEventType::DragEnter, at(10.0, 20.0))
        );
        assert_eq!(
            state.on_drag(ACTION_DRAG_LOCATION, at(11.0, 21.0), false, false),
            DragStep::Raise(RawDragEventType::DragOver, at(11.0, 21.0))
        );
    }

    #[test]
    fn an_exit_leaves_at_the_last_location_once_entered() {
        let mut state = DragDropState::default();
        state.on_drag(ACTION_DRAG_ENTERED, at(0.0, 0.0), false, false);
        assert_eq!(state.on_drag(ACTION_DRAG_EXITED, at(0.0, 0.0), false, false), DragStep::None);

        state.on_drag(ACTION_DRAG_ENTERED, at(0.0, 0.0), false, false);
        state.on_drag(ACTION_DRAG_LOCATION, at(5.0, 6.0), false, false);
        assert_eq!(
            state.on_drag(ACTION_DRAG_EXITED, at(0.0, 0.0), false, false),
            DragStep::Raise(RawDragEventType::DragLeave, at(5.0, 6.0))
        );
        // Outside the view, locations raise nothing.
        assert_eq!(state.on_drag(ACTION_DRAG_LOCATION, at(7.0, 8.0), false, false), DragStep::None);
    }

    #[test]
    fn a_drop_inside_drops_once_and_outside_does_nothing() {
        let mut state = DragDropState::default();
        assert_eq!(state.on_drag(ACTION_DROP, at(1.0, 1.0), false, false), DragStep::None);

        state.on_drag(ACTION_DRAG_ENTERED, at(0.0, 0.0), false, false);
        state.on_drag(ACTION_DRAG_LOCATION, at(3.0, 4.0), false, false);
        assert_eq!(state.on_drag(ACTION_DROP, at(3.5, 4.5), false, false), DragStep::Drop(at(3.5, 4.5)));
        assert_eq!(state.on_drag(ACTION_DROP, at(3.5, 4.5), false, false), DragStep::None);

        // A new drag enters again.
        state.on_drag(ACTION_DRAG_ENTERED, at(0.0, 0.0), false, false);
        assert_eq!(
            state.on_drag(ACTION_DRAG_LOCATION, at(1.0, 2.0), false, false),
            DragStep::Raise(RawDragEventType::DragEnter, at(1.0, 2.0))
        );
    }

    #[test]
    fn only_the_end_of_an_own_drag_reports_its_effect() {
        let mut state = DragDropState::default();
        assert_eq!(state.on_drag(ACTION_DRAG_ENDED, at(0.0, 0.0), true, false), DragStep::None);
        assert_eq!(state.on_drag(ACTION_DRAG_ENDED, at(0.0, 0.0), true, true), DragStep::Ended(DragDropEffects::COPY));
        assert_eq!(state.on_drag(ACTION_DRAG_ENDED, at(0.0, 0.0), false, true), DragStep::Ended(DragDropEffects::NONE));
    }
}
