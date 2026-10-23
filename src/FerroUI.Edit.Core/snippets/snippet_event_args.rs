/// Why the interactive part of a snippet ended. The original six reasons map
/// directly to AvaloniaEdit's `Snippets/SnippetEventArgs.cs`; completion and
/// validation failure are explicit additional Rust-host outcomes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeactivateReason {
    #[default]
    Unknown,
    Deleted,
    NoActiveElements,
    InputHandlerDetached,
    ReturnPressed,
    EscapePressed,
    Completed,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnippetEventArgs {
    pub reason: DeactivateReason,
}

impl SnippetEventArgs {
    pub const fn new(reason: DeactivateReason) -> Self {
        Self { reason }
    }

    /// Only accepting or completing a snippet requests its final caret.
    /// Cancelling, deleting or replacing the host preserves the user's caret.
    pub fn moves_to_final_caret(self) -> bool {
        matches!(
            self.reason,
            DeactivateReason::ReturnPressed
                | DeactivateReason::NoActiveElements
                | DeactivateReason::Completed
        )
    }
}
