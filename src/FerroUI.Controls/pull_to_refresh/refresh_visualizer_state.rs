/// Defines constants that specify the state of a RefreshVisualizer
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RefreshVisualizerState {
    #[default]
    Idle = 0,
    Peeking = 1,
    Interacting = 2,
    Pending = 3,
    Refreshing = 4,
}
