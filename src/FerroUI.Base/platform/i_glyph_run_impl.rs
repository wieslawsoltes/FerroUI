use crate::{Point, Rect};

/// Actual implementation of a glyph run that stores platform dependent resources.
pub trait IGlyphRunImpl {
    /// Gets the em size used for rendering the glyph run.
    fn font_rendering_em_size(&self) -> f64;

    /// Gets the baseline origin of the glyph run.
    fn baseline_origin(&self) -> Point;

    /// Gets the conservative bounding box of the glyph run.
    fn bounds(&self) -> Rect;

    /// Gets the intersections of specified upper and lower limit.
    fn get_intersections(&self, lower_limit: f32, upper_limit: f32) -> Vec<f32>;

    /// Releases the platform resources (C# `IDisposable.Dispose`).
    fn dispose(&self);

    /// Lets the backend recover its concrete type when drawing.
    fn as_any(&self) -> &dyn std::any::Any;
}
