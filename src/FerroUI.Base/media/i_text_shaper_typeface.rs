/// Represents a typeface as prepared by the text shaper backend (for example
/// the shaper's own font object created from a glyph typeface).
pub trait ITextShaperTypeface: std::any::Any {
    /// Releases the shaper's font object (C# `IDisposable.Dispose`).
    fn dispose(&self);

}

impl dyn ITextShaperTypeface {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
