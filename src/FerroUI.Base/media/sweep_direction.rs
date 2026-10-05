/// Defines the direction an elliptical arc is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SweepDirection {
    #[default]
    CounterClockwise = 0,
    Clockwise = 1,
}
