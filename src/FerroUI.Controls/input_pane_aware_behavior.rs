/// Specifies the behavior the
/// [`InputPaneAwareDecorator`](crate::InputPaneAwareDecorator) takes when the
/// input pane state changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InputPaneAwareBehavior {
    /// The view doesn't resize or move the content when the input pane state
    /// changes.
    #[default]
    None = 0,

    /// The view moves the content vertically when the input pane state
    /// changes. The content is not resized.
    Pan = 1,

    /// The view resizes the content when the input pane state changes.
    Resize = 2,
}
