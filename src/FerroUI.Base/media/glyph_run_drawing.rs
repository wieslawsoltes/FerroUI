use crate::media::{Drawing, DrawingContext, DrawingImpl, GlyphRun, IBrush};
use crate::{ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Rect, Ref, StyledProperty};
use std::rc::Rc;

/// Represents a drawing of a [`GlyphRun`] with a foreground brush.
#[repr(C)]
pub struct GlyphRunDrawing {
    base: Drawing,
}

ferro_class!(GlyphRunDrawing: Drawing);
ferro_impl_classes!(GlyphRunDrawing: FerroObjectImpl);

impl DrawingImpl for GlyphRunDrawing {
    fn draw_core(this: &Self, context: &mut DrawingContext) {
        let Some(glyph_run) = this.glyph_run() else {
            return;
        };

        context.draw_glyph_run(this.foreground().as_ref(), &glyph_run);
    }

    fn get_bounds(this: &Self) -> Rect {
        this.glyph_run().map(|glyph_run| glyph_run.bounds()).unwrap_or_default()
    }
}

impl GlyphRunDrawing {
    ferro_property!(
        /// Defines the `Foreground` property.
        pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<GlyphRunDrawing, _>("Foreground", None)
        }
    );

    ferro_property!(
        /// Defines the `GlyphRun` property.
        pub fn glyph_run_property() -> StyledProperty<Option<Rc<GlyphRun>>> {
            FerroProperty::register::<GlyphRunDrawing, _>("GlyphRun", None)
        }
    );

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Drawing::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The brush the glyph run is drawn with.
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }

    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value)
    }

    /// The glyph run that is drawn.
    pub fn glyph_run(&self) -> Option<Rc<GlyphRun>> {
        self.get_value(Self::glyph_run_property())
    }

    pub fn set_glyph_run(&self, value: Option<Rc<GlyphRun>>) {
        self.set_value(Self::glyph_run_property(), value)
    }
}
