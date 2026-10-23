use crate::background_renderer_connections::BackgroundRendererConnections;
use crate::{
    BackgroundGeometryBuilder, IBackgroundRenderer, ITextViewConnect, KnownLayer, TextArea,
    VisualLineError,
};
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{Color, DrawingContext, Geometry, IBrush, IPen};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Highlights the full measured visual line containing a logical document line.
///
/// Line numbers are one-based; zero suppresses the highlight. Folded logical
/// lines share their visible owner's rectangle and wrapped lines include all
/// their native text rows. The host controls `enabled` and `line` as its current
/// caret and editor options change.
pub struct CurrentLineHighlightRenderer {
    line: Cell<usize>,
    enabled: Cell<bool>,
    background: RefCell<Option<Rc<dyn IBrush>>>,
    border: RefCell<Option<Rc<dyn IPen>>>,
    connections: BackgroundRendererConnections,
}

impl Default for CurrentLineHighlightRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentLineHighlightRenderer {
    pub const DEFAULT_BACKGROUND: Color = Color::from_argb(22, 20, 220, 224);
    pub const DEFAULT_BORDER: Color = Color::from_argb(52, 0, 255, 110);

    pub fn new() -> Self {
        Self {
            line: Cell::new(0),
            enabled: Cell::new(false),
            background: RefCell::new(Some(Rc::new(ImmutableSolidColorBrush::new(
                Self::DEFAULT_BACKGROUND,
            )))),
            border: RefCell::new(Some(Rc::new(ImmutablePen::from_uint32(0x3400ff6e, 1.0)))),
            connections: BackgroundRendererConnections::default(),
        }
    }

    pub fn line(&self) -> usize {
        self.line.get()
    }
    pub fn enabled(&self) -> bool {
        self.enabled.get()
    }
    pub fn background_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.background.borrow().clone()
    }
    pub fn border_pen(&self) -> Option<Rc<dyn IPen>> {
        self.border.borrow().clone()
    }

    pub fn set_line(&self, line: usize) {
        if self.line.replace(line) != line {
            self.connections.invalidate(KnownLayer::Background);
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        if self.enabled.replace(enabled) != enabled {
            self.connections.invalidate(KnownLayer::Background);
        }
    }

    pub fn set_background_brush(&self, brush: Option<Rc<dyn IBrush>>) {
        self.background.replace(brush);
        self.connections.invalidate(KnownLayer::Background);
    }

    pub fn set_border_pen(&self, pen: Option<Rc<dyn IPen>>) {
        self.border.replace(pen);
        self.connections.invalidate(KnownLayer::Background);
    }

    pub fn set_default_colors(&self) {
        self.background
            .replace(Some(Rc::new(ImmutableSolidColorBrush::new(
                Self::DEFAULT_BACKGROUND,
            ))));
        self.border
            .replace(Some(Rc::new(ImmutablePen::from_uint32(0x3400ff6e, 1.0))));
        self.connections.invalidate(KnownLayer::Background);
    }
}

impl ITextViewConnect for CurrentLineHighlightRenderer {
    fn add_to_text_view(&self, view: &TextArea) {
        self.connections.add(view);
    }
    fn remove_from_text_view(&self, view: &TextArea) {
        self.connections.remove(view);
    }
}

impl IBackgroundRenderer for CurrentLineHighlightRenderer {
    fn layer(&self) -> KnownLayer {
        KnownLayer::Background
    }

    fn draw(&self, view: &TextArea, context: &mut DrawingContext) -> Result<(), VisualLineError> {
        if !self.enabled() || self.line() == 0 {
            return Ok(());
        }
        let background = self.background_brush();
        let border = self.border_pen();
        if background.is_none() && border.is_none() {
            return Ok(());
        }
        let Some(bounds) = view.visual_line_bounds(self.line()) else {
            return Ok(());
        };
        let mut builder = BackgroundGeometryBuilder::new();
        builder.add_rectangle_for_view(view, bounds)?;
        if let Some(geometry) = builder.create_geometry() {
            context.draw_geometry(
                background.as_ref(),
                border.as_ref(),
                geometry.upcast_ref::<Geometry>(),
            );
        }
        Ok(())
    }
}
