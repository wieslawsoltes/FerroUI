/// Represents a typeface as prepared by the text shaper backend (for example
/// the shaper's own font object created from a glyph typeface).
pub trait ITextShaperTypeface {
    /// Releases the shaper's font object (C# `IDisposable.Dispose`).
    fn dispose(&self);

    /// Lets the backend recover its concrete type.
    fn as_any(&self) -> &dyn std::any::Any;
}
