use crate::background_renderer_connections::BackgroundRendererConnections;
use crate::{
    BackgroundGeometryBuilder, BackgroundRendererRef, IBackgroundRenderer, ITextViewConnect,
    KnownLayer, TextArea, VisualLineError,
};
use ferroui_base::WeakRef;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    Color, DashStyle, DrawingContext, Geometry, IBrush, IPen, PenLineCap, PenLineJoin,
};
use ferroui_edit_core::snippets::{SnippetSession, WeakSnippetSession};
use std::cell::RefCell;
use std::rc::Rc;

/// Native snippet field decoration. The background instance fills every
/// replaceable field; the text-layer instance outlines fields containing the
/// caret and all of those fields' bound occurrences.
///
/// [`crate::SnippetController`] installs and owns both instances automatically.
/// Manually connected instances keep only weak references to their session and
/// views; their owner must invalidate the layer when anchors or the caret move.
pub struct SnippetBackgroundRenderer {
    session: WeakSnippetSession,
    layer: KnownLayer,
    background: RefCell<Option<Rc<dyn IBrush>>>,
    border: RefCell<Option<Rc<dyn IPen>>>,
    connections: BackgroundRendererConnections,
}

impl SnippetBackgroundRenderer {
    pub const DEFAULT_BACKGROUND: Color = Color::from_rgb(50, 205, 50);
    pub const DEFAULT_BACKGROUND_OPACITY: f64 = 0.4;

    pub fn background(session: &SnippetSession) -> Self {
        Self::new(session, KnownLayer::Background)
    }

    pub fn foreground(session: &SnippetSession) -> Self {
        Self::new(session, KnownLayer::Text)
    }

    fn new(session: &SnippetSession, layer: KnownLayer) -> Self {
        Self {
            session: session.downgrade(),
            layer,
            background: RefCell::new(Some(Rc::new(ImmutableSolidColorBrush::with_opacity(
                Self::DEFAULT_BACKGROUND,
                Self::DEFAULT_BACKGROUND_OPACITY,
            )))),
            border: RefCell::new(Some(Rc::new(ImmutablePen::new(
                Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff000000))),
                1.0,
                Some(DashStyle::dot()),
                PenLineCap::Flat,
                PenLineJoin::Miter,
                10.0,
            )))),
            connections: BackgroundRendererConnections::default(),
        }
    }

    pub fn background_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.background.borrow().clone()
    }
    pub fn border_pen(&self) -> Option<Rc<dyn IPen>> {
        self.border.borrow().clone()
    }

    pub fn set_background_brush(&self, brush: Option<Rc<dyn IBrush>>) {
        self.background.replace(brush);
        self.connections.invalidate(self.layer);
    }

    /// The border thickness also controls whole-pixel alignment of field fills,
    /// keeping the fill and foreground outline on the same pixel boundaries.
    pub fn set_border_pen(&self, pen: Option<Rc<dyn IPen>>) {
        self.border.replace(pen);
        self.connections.invalidate(self.layer);
    }
}

impl ITextViewConnect for SnippetBackgroundRenderer {
    fn add_to_text_view(&self, view: &TextArea) {
        self.connections.add(view);
    }
    fn remove_from_text_view(&self, view: &TextArea) {
        self.connections.remove(view);
    }
}

impl IBackgroundRenderer for SnippetBackgroundRenderer {
    fn layer(&self) -> KnownLayer {
        self.layer
    }

    fn draw(&self, view: &TextArea, context: &mut DrawingContext) -> Result<(), VisualLineError> {
        let Some(session) = self
            .session
            .upgrade()
            .filter(|session| !session.is_closed())
        else {
            return Ok(());
        };
        let Some(editor) = view
            .editor()
            .filter(|editor| editor.document() == session.document())
        else {
            return Ok(());
        };
        let document = session.document();
        let version = document.version();
        let caret = editor.caret_offset();
        let background = self.background_brush();
        let border = self.border_pen();
        let foreground = self.layer == KnownLayer::Text;
        if (foreground && border.is_none()) || (!foreground && background.is_none()) {
            return Ok(());
        }
        for field in session.fields() {
            if foreground && !(field.segment.offset <= caret && caret <= field.segment.end_offset())
            {
                continue;
            }
            let mut builder = BackgroundGeometryBuilder::new();
            builder.align_to_whole_pixels = true;
            builder.border_thickness = border.as_ref().map_or(0.0, |pen| pen.thickness());
            for rect in view.segment_rectangles(&field.segment, false)? {
                builder.add_rectangle_for_view(view, rect)?;
            }
            if foreground {
                for segment in field.bound_segments {
                    for rect in view.segment_rectangles(&segment, false)? {
                        builder.add_rectangle_for_view(view, rect)?;
                    }
                }
                builder.close_figure();
            }
            if let Some(geometry) = builder.create_geometry() {
                // Public drawing can construct previously unseen lines. An
                // extension callback may close the session or move its anchors
                // during that construction; never paint an old field snapshot.
                if session.is_closed() || editor.document() != document {
                    return Ok(());
                }
                if document.version() != version || editor.caret_offset() != caret {
                    return Err(VisualLineError::ConstructionInvalidated);
                }
                context.draw_geometry(
                    if foreground {
                        None
                    } else {
                        background.as_ref()
                    },
                    if foreground { border.as_ref() } else { None },
                    geometry.upcast_ref::<Geometry>(),
                );
            }
        }
        Ok(())
    }
}

/// Owns exactly the two collection identities installed for a snippet. The
/// view and renderers point back weakly, so retaining either the session or an
/// externally removed renderer cannot keep a controller or editor alive.
pub(crate) struct SnippetBackgroundRegistration {
    view: WeakRef<TextArea>,
    pub(crate) background: Rc<SnippetBackgroundRenderer>,
    pub(crate) foreground: Rc<SnippetBackgroundRenderer>,
}

impl SnippetBackgroundRegistration {
    pub(crate) fn attach(view: &TextArea, session: &SnippetSession) -> Option<Self> {
        if session.fields().is_empty() {
            return None;
        }
        let registration = Self {
            view: view.to_ref().downgrade(),
            background: Rc::new(SnippetBackgroundRenderer::background(session)),
            foreground: Rc::new(SnippetBackgroundRenderer::foreground(session)),
        };
        for renderer in [&registration.background, &registration.foreground] {
            view.background_renderers()
                .add(BackgroundRendererRef::from(renderer.clone()));
            // A collection observer can synchronously close the session. The
            // local registration removes any entries added before that callback.
            if session.is_closed() {
                return None;
            }
        }
        Some(registration)
    }
}

impl Drop for SnippetBackgroundRegistration {
    fn drop(&mut self) {
        if let Some(view) = self.view.upgrade() {
            let renderers = view.background_renderers();
            for renderer in [&self.background, &self.foreground] {
                renderers.remove(&BackgroundRendererRef::from(renderer.clone()));
            }
        }
    }
}
