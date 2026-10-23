//! Live background renderer lifetime, shared range geometry and native paint phases.
use super::*;
use crate::{
    BackgroundGeometryBuilder, BackgroundGeometryLine, BackgroundGeometryViewport,
    BackgroundRendererRef, ColumnRulerRenderer, CurrentLineHighlightRenderer, KnownLayer,
    VisualLineError, get_rects_for_segment, get_rects_for_selection,
};
use ferroui_base::collections::FerroList;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{Geometry, IPen};
use ferroui_base::{FerroProperty, FerroPropertyChangedEventArgs, StyledProperty};
use ferroui_edit_core::document::{DocumentVersion, ISegment, TextDocument};
use ferroui_edit_core::editing::EditorOptions;

pub(super) struct BackgroundRendererHost {
    renderers: FerroList<BackgroundRendererRef>,
    current_line: Rc<CurrentLineHighlightRenderer>,
    column_ruler: Rc<ColumnRulerRenderer>,
    connected: RefCell<Vec<BackgroundRendererRef>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    painting: Cell<bool>,
    synchronizing: Cell<bool>,
    synchronize_again: Cell<bool>,
}
impl BackgroundRendererHost {
    pub(super) fn new() -> Self {
        Self {
            renderers: FerroList::new(),
            current_line: Rc::new(CurrentLineHighlightRenderer::new()),
            column_ruler: Rc::new(ColumnRulerRenderer::new()),
            connected: RefCell::new(Vec::new()),
            subscription: RefCell::new(None),
            painting: Cell::new(false),
            synchronizing: Cell::new(false),
            synchronize_again: Cell::new(false),
        }
    }
}
ferroui_base::ferro_properties! { impl TextArea {
    pub fn column_ruler_pen_property() -> StyledProperty<Option<Rc<dyn IPen>>> {
        FerroProperty::register::<TextArea, _>("ColumnRulerPen", Some(Rc::new(ImmutablePen::from_uint32(0x5a808080, 1.0)) as Rc<dyn IPen>))
    }
    pub fn current_line_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextArea, _>("CurrentLineBackground", None)
    }
    pub fn current_line_border_property() -> StyledProperty<Option<Rc<dyn IPen>>> {
        FerroProperty::register::<TextArea, _>("CurrentLineBorder", None)
    }
} }
struct SynchronizeScope<'a>(&'a Cell<bool>);
impl Drop for SynchronizeScope<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
struct BackgroundFrameScope<'a>(&'a TextArea);
impl Drop for BackgroundFrameScope<'_> {
    fn drop(&mut self) {
        self.0.background_layers.painting.set(false);
        if self.0.background_layers.synchronize_again.get() {
            self.0.synchronize_background_renderers();
        }
    }
}
struct ViewportLine {
    index: usize,
    line: Rc<CachedVisualLine>,
    origin: Point,
}
struct ViewportFrame {
    lines: Vec<ViewportLine>,
    document: TextDocument,
    version: DocumentVersion,
    revision: u64,
    offset: Vector,
    viewport: Size,
    selection: Selection,
    options: EditorOptions,
    metrics: BackgroundGeometryViewport,
}
impl ViewportFrame {
    fn geometry_lines(&self) -> impl Iterator<Item = BackgroundGeometryLine<'_>> {
        self.lines.iter().map(|entry| BackgroundGeometryLine {
            visual_line: &entry.line.display,
            layout: &entry.line.layout,
            origin: entry.origin,
        })
    }
    fn check(&self, view: &TextArea) -> Result<(), VisualLineError> {
        if view.visual_lines.revision.get() == self.revision
            && view.offset.get() == self.offset
            && view.viewport.get() == self.viewport
            && view.editor().is_some_and(|editor| {
                editor.document() == self.document
                    && self.document.version() == self.version
                    && editor.selection() == self.selection
            })
        {
            Ok(())
        } else {
            view.invalidate_visual();
            Err(VisualLineError::ConstructionInvalidated)
        }
    }
}
impl TextArea {
    pub fn column_ruler_pen(&self) -> Option<Rc<dyn IPen>> {
        self.get_value(Self::column_ruler_pen_property())
    }
    pub fn set_column_ruler_pen(&self, pen: Option<Rc<dyn IPen>>) {
        self.set_value(Self::column_ruler_pen_property(), pen);
    }
    pub fn current_line_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::current_line_background_property())
    }
    pub fn set_current_line_background(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::current_line_background_property(), brush);
    }
    pub fn current_line_border(&self) -> Option<Rc<dyn IPen>> {
        self.get_value(Self::current_line_border_property())
    }
    pub fn set_current_line_border(&self, pen: Option<Rc<dyn IPen>>) {
        self.set_value(Self::current_line_border_property(), pen);
    }
    /// Restores the pinned renderer colors without assigning style values.
    pub fn set_default_highlight_line_colors(&self) {
        self.background_layers.current_line.set_default_colors();
    }
    /// The default renderer's enablement and line follow the editor options and
    /// primary caret at paint time. Its brush and border use native properties.
    pub fn current_line_highlight_renderer(&self) -> Rc<CurrentLineHighlightRenderer> {
        self.background_layers.current_line.clone()
    }
    /// The default renderer's columns follow the editor options at paint time.
    pub fn column_ruler_renderer(&self) -> Rc<ColumnRulerRenderer> {
        self.background_layers.column_ruler.clone()
    }
    pub(super) fn on_background_property_changed(
        &self,
        change: &FerroPropertyChangedEventArgs<'_>,
    ) {
        if change.property() == Self::column_ruler_pen_property().as_property() {
            self.background_layers
                .column_ruler
                .set_pen(self.column_ruler_pen());
        } else if change.property() == Self::current_line_background_property().as_property() {
            self.background_layers
                .current_line
                .set_background_brush(self.current_line_background());
        } else if change.property() == Self::current_line_border_property().as_property() {
            self.background_layers
                .current_line
                .set_border_pen(self.current_line_border());
        }
    }
    /// Ordered native decoration renderers. Entries connect by shared identity
    /// and draw once, immediately before their selected layer's content.
    pub fn background_renderers(&self) -> FerroList<BackgroundRendererRef> {
        self.background_layers.renderers.clone()
    }
    /// Invalidates painting without discarding native text shaping. FerroUI's
    /// compositor invalidates the view as a whole; layer ordering is retained.
    pub fn invalidate_layer(&self, _layer: KnownLayer) {
        self.invalidate_visual();
    }
    pub fn content_viewport(&self) -> Rect {
        let viewport = self.viewport.get();
        let gutter = self.gutter_width();
        Rect::new(
            gutter,
            0.0,
            (viewport.width - gutter).max(0.0),
            viewport.height,
        )
    }
    pub fn wide_space_width(&self) -> f64 {
        self.space_advance()
    }
    /// Constructs the visible lines if needed. A fully retained viewport does
    /// not raise another construction-start event.
    pub fn ensure_visual_lines(&self) -> Result<(), VisualLineError> {
        self.collect_viewport_frame().map(|_| ())
    }
    /// Full content-width bounds of the visible owner of a logical line. A
    /// folded continuation reports its owning row; offscreen lines return None.
    pub fn visual_line_bounds(&self, line_number: usize) -> Option<Rect> {
        let frame = self.collect_viewport_frame().ok()?;
        let viewport = self.content_viewport();
        let bounds = frame.lines.iter().find(|entry| {
            entry.line.display.first_document_line().line_number() <= line_number
                && line_number <= entry.line.display.last_document_line().line_number()
        });
        frame.check(self).ok()?;
        bounds.map(|entry| {
            Rect::new(
                viewport.x,
                entry.origin.y,
                viewport.width,
                entry.line.layout.height().max(self.line_height()),
            )
        })
    }
    pub fn segment_rectangles(
        &self,
        segment: &dyn ISegment,
        extend_to_full_width: bool,
    ) -> Result<Vec<Rect>, VisualLineError> {
        let frame = self.collect_viewport_frame()?;
        let rectangles = get_rects_for_segment(
            frame.geometry_lines(),
            segment,
            frame.metrics,
            extend_to_full_width,
        )?;
        frame.check(self)?;
        Ok(rectangles)
    }
    pub fn selection_rectangles(
        &self,
        selection: SelectionSegment,
        extend_to_full_width: bool,
    ) -> Result<Vec<Rect>, VisualLineError> {
        let frame = self.collect_viewport_frame()?;
        let rectangles = get_rects_for_selection(
            frame.geometry_lines(),
            selection,
            frame.metrics,
            extend_to_full_width,
        )?;
        frame.check(self)?;
        Ok(rectangles)
    }
    pub(super) fn connect_background_renderers(&self) {
        let weak = self.to_ref().downgrade();
        let renderers = self.background_renderers();
        let token = renderers.add_collection_changed(Rc::new(move |_| {
            if let Some(view) = weak.upgrade() {
                view.synchronize_background_renderers();
            }
        }));
        *self.background_layers.subscription.borrow_mut() = Some(Disposable::create(move || {
            renderers.remove_collection_changed(token);
        }));
        self.background_layers
            .column_ruler
            .set_pen(self.column_ruler_pen());
        self.background_renderers()
            .add(self.background_layers.current_line.clone().into());
        self.background_renderers()
            .add(self.background_layers.column_ruler.clone().into());
    }
    pub(super) fn disconnect_background_renderers(&self) {
        let subscription = self.background_layers.subscription.take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let connected = self.background_layers.connected.take();
        for renderer in connected {
            renderer.remove_from_text_view(self);
        }
    }
    fn synchronize_background_renderers(&self) {
        let host = &self.background_layers;
        if host.painting.get() || host.synchronizing.replace(true) {
            host.synchronize_again.set(true);
            self.invalidate_visual();
            return;
        }
        let _scope = SynchronizeScope(&host.synchronizing);
        loop {
            host.synchronize_again.set(false);
            let mut current = Vec::new();
            for renderer in host.renderers.snapshot().iter() {
                if !current.contains(renderer) {
                    current.push(renderer.clone());
                }
            }
            let previous = host.connected.replace(current.clone());
            for renderer in previous.iter().filter(|item| !current.contains(item)) {
                renderer.remove_from_text_view(self);
            }
            for renderer in current.iter().filter(|item| !previous.contains(item)) {
                renderer.add_to_text_view(self);
            }
            drop(previous);
            drop(current);
            self.invalidate_visual();
            if !host.synchronize_again.get() {
                break;
            }
        }
    }
    fn collect_viewport_frame(&self) -> Result<ViewportFrame, VisualLineError> {
        let editor = self
            .editor()
            .ok_or_else(|| VisualLineError::Extension("The text area has no document".into()))?;
        let offset = self.offset.get();
        let viewport = self.viewport.get();
        let first = self.heights.borrow().line_at(offset.y);
        let needs_construction = viewport.width > 0.0 && viewport.height > 0.0 && {
            let heights = self.heights.borrow();
            let cache = self.cache.borrow();
            let count = editor.document().line_count();
            std::iter::successors(Some(first), |&index| {
                let next = heights.next_visible(index + 1);
                (next < count).then_some(next)
            })
            .take_while(|&index| heights.top(index) - offset.y <= viewport.height)
            .any(|index| !cache.contains_key(&index))
        };
        let batch = if needs_construction {
            Some(self.begin_visual_line_batch(first)?)
        } else {
            None
        };
        // Start-event callbacks may change the document, viewport or selection.
        let document = editor.document();
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let offset = self.offset.get();
        let viewport = self.viewport.get();
        let first = self.heights.borrow().line_at(offset.y);
        let gutter = self.gutter_width();
        let mut lines = Vec::new();
        let mut index = first;
        let mut last = first;
        while viewport.width > 0.0 && viewport.height > 0.0 && index < document.line_count() {
            let top = self.heights.borrow().top(index) - offset.y;
            if top > viewport.height {
                break;
            }
            let line = self.try_line_layout(index)?;
            lines.push(ViewportLine {
                index,
                line,
                origin: Point::new(gutter - offset.x, top),
            });
            last = index;
            // New generated multiline spans have updated the height tree now.
            index = self.heights.borrow().next_visible(index + 1);
        }
        let evicted = {
            let mut cache = self.cache.borrow_mut();
            let keys: Vec<_> = cache
                .keys()
                .copied()
                .filter(|index| *index < first.saturating_sub(8) || *index > last.saturating_add(8))
                .collect();
            keys.into_iter()
                .filter_map(|key| cache.remove(&key))
                .collect::<Vec<_>>()
        };
        drop(evicted);
        self.update_extent();
        let frame = ViewportFrame {
            lines,
            document,
            version,
            revision,
            offset,
            viewport,
            selection: editor.selection(),
            options: editor.options(),
            metrics: BackgroundGeometryViewport {
                full_width: (self.extent.get().width.max(viewport.width) - gutter).max(0.0),
                default_line_height: self.line_height(),
                wide_space_width: self.space_advance(),
                empty_line_selection_width: self.space_advance(),
            },
        };
        // Release generator/transformer snapshots before validating; custom
        // destructors may themselves change the document or viewport.
        drop(batch);
        frame.check(self)?;
        Ok(frame)
    }
    pub(super) fn render_text_view(&self, context: &mut DrawingContext) {
        if self.background_layers.synchronizing.get() {
            // Connection hooks are application callbacks. A reentrant frame
            // must wait until every retained renderer is connected.
            self.invalidate_visual();
            return;
        }
        let Some(editor) = self.editor() else {
            return;
        };
        let size = self.viewport.get();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        if self.rendering.replace(true) {
            editor.report_error("Recursive editor rendering is not supported".into());
            return;
        }
        let _scope = SynchronizeScope(&self.rendering);
        let clip = context.push_clip(Rect::new(0.0, 0.0, size.width, size.height));
        if let Some(background) = editor.background() {
            context.fill_rectangle(
                &background,
                Rect::new(0.0, 0.0, size.width, size.height),
                0.0,
            );
        }
        let result = self
            .collect_viewport_frame()
            .and_then(|frame| self.draw_viewport_frame(context, &frame));
        context.pop(clip);
        if let Err(error) = result {
            editor.report_error(error.to_string());
        }
    }
    fn draw_viewport_frame(
        &self,
        context: &mut DrawingContext,
        frame: &ViewportFrame,
    ) -> Result<(), VisualLineError> {
        let editor = self
            .editor()
            .ok_or(VisualLineError::ConstructionInvalidated)?;
        self.background_layers
            .current_line
            .set_enabled(frame.options.highlight_current_line);
        self.background_layers.current_line.set_line(
            frame
                .document
                .get_line_by_offset(frame.selection.primary().caret)
                .map_or(0, |line| line.line_number()),
        );
        self.background_layers.column_ruler.set_columns(
            frame
                .options
                .column_ruler_positions
                .iter()
                .copied()
                .filter(|_| frame.options.show_column_rulers),
        );
        self.background_layers.painting.set(true);
        let _scope = BackgroundFrameScope(self);
        let renderers: Vec<_> = self
            .background_renderers()
            .snapshot()
            .iter()
            .map(|renderer| (renderer.clone(), renderer.layer()))
            .collect();
        frame.check(self)?;
        let clip = context.push_clip(self.content_viewport());
        let result: Result<(), VisualLineError> = (|| {
            self.draw_background_layer(KnownLayer::Background, &renderers, context, frame)?;
            self.draw_element_backgrounds(context, frame);
            self.draw_background_layer(KnownLayer::Selection, &renderers, context, frame)?;
            self.draw_search_and_selection(context, frame, &editor)?;
            self.draw_background_layer(KnownLayer::Text, &renderers, context, frame)?;
            for entry in &frame.lines {
                entry.line.layout.draw(context, entry.origin);
                frame.check(self)?;
            }
            self.draw_background_layer(KnownLayer::Caret, &renderers, context, frame)?;
            self.draw_viewport_carets(context, frame, &editor);
            frame.check(self)?;
            Ok(())
        })();
        context.pop(clip);
        result?;
        frame.check(self)?;
        self.draw_viewport_margins(context, frame, &editor);
        Ok(())
    }
    fn draw_background_layer(
        &self,
        layer: KnownLayer,
        renderers: &[(BackgroundRendererRef, KnownLayer)],
        context: &mut DrawingContext,
        frame: &ViewportFrame,
    ) -> Result<(), VisualLineError> {
        frame.check(self)?;
        for (renderer, _) in renderers.iter().filter(|(_, selected)| *selected == layer) {
            renderer.draw(self, context)?;
            frame.check(self)?;
        }
        Ok(())
    }
    fn draw_element_backgrounds(&self, context: &mut DrawingContext, frame: &ViewportFrame) {
        for entry in &frame.lines {
            let line = &entry.line;
            let origin = entry.origin;
            for element in line.display.elements() {
                if let Some(brush) = element.data().background_brush() {
                    for rect in line.layout.hit_test_text_range(
                        element.data().visual_column() as i32,
                        element.data().visual_length() as i32,
                    ) {
                        context.fill_rectangle(
                            brush,
                            Rect::new(
                                origin.x + rect.x,
                                origin.y + rect.y,
                                rect.width,
                                rect.height,
                            ),
                            0.0,
                        );
                    }
                }
            }
        }
    }
    fn draw_search_and_selection(
        &self,
        context: &mut DrawingContext,
        frame: &ViewportFrame,
        editor: &TextEditor,
    ) -> Result<(), VisualLineError> {
        if let (Some(panel), Some(first), Some(last)) = (
            editor
                .installed_search_panel()
                .filter(|panel| panel.is_open()),
            frame.lines.first(),
            frame.lines.last(),
        ) {
            let brush =
                Rc::new(ImmutableSolidColorBrush::from_uint32(0x70e7b83a)) as Rc<dyn IBrush>;
            let start = first.line.offset;
            let end = last.line.offset + last.line.length;
            for matched in panel.results_in_range(start, end - start) {
                let rectangles =
                    get_rects_for_segment(frame.geometry_lines(), &matched, frame.metrics, false)?;
                self.draw_background_rectangles(context, &brush, rectangles)?;
            }
        }
        if let Some(brush) = editor.selection_brush() {
            for segment in frame
                .selection
                .segments()
                .iter()
                .filter(|segment| !segment.is_empty())
            {
                let lines = frame
                    .lines
                    .iter()
                    .filter(|entry| entry.line.preedit.is_none())
                    .map(|entry| BackgroundGeometryLine {
                        visual_line: &entry.line.display,
                        layout: &entry.line.layout,
                        origin: entry.origin,
                    });
                let rectangles = get_rects_for_selection(
                    lines,
                    *segment,
                    frame.metrics,
                    frame.options.enable_virtual_space || frame.selection.is_rectangular(),
                )?;
                self.draw_background_rectangles(context, &brush, rectangles)?;
            }
        }
        Ok(())
    }
    fn draw_background_rectangles(
        &self,
        context: &mut DrawingContext,
        brush: &Rc<dyn IBrush>,
        rectangles: Vec<Rect>,
    ) -> Result<(), VisualLineError> {
        let mut builder = BackgroundGeometryBuilder::new();
        builder.align_to_whole_pixels = true;
        let pixel_size = BackgroundGeometryBuilder::pixel_size(self);
        for rectangle in rectangles {
            builder.add_rectangle_aligned(rectangle, pixel_size)?;
        }
        if let Some(geometry) = builder.create_geometry() {
            context.draw_geometry(Some(brush), None, geometry.upcast_ref::<Geometry>());
        }
        Ok(())
    }
    fn draw_viewport_carets(
        &self,
        context: &mut DrawingContext,
        frame: &ViewportFrame,
        editor: &TextEditor,
    ) {
        let this = self;
        let selection = &frame.selection;
        for entry in &frame.lines {
            let line = &entry.line;
            let origin = entry.origin;
            if this.is_focused() && this.caret_visible.get() {
                for segment in selection.segments().iter().filter(|s| {
                    s.is_empty() || line.preedit.is_some() && **s == selection.primary()
                }) {
                    if line.preedit.is_some() && *segment == selection.primary()
                        || segment.caret >= line.offset
                            && segment.caret <= line.offset + line.length
                            && line
                                .display
                                .is_document_offset_visible(segment.caret - line.offset)
                    {
                        let position = if let Some((start, length)) = line.preedit {
                            start
                                + this
                                    .preedit
                                    .borrow()
                                    .as_ref()
                                    .and_then(|p| p.1)
                                    .map_or(length, |c| (c.max(0) as usize).min(length))
                        } else {
                            line.display.visual_offset(segment.caret - line.offset)
                        };
                        let r = if line.preedit.is_none() && segment.caret_virtual_space > 0 {
                            Self::virtual_space_bounds(&line.layout)
                        } else {
                            Self::caret_bounds(
                                &line.layout,
                                position,
                                line.preedit.is_none()
                                    && *segment == selection.primary()
                                    && this.caret_at_end_of_line.get(),
                            )
                        };
                        let brush = editor.foreground().unwrap_or_else(|| {
                            Rc::new(ImmutableSolidColorBrush::from_uint32(0xff202020))
                        });
                        context.fill_rectangle(
                            &brush,
                            Rect::new(
                                origin.x
                                    + r.x
                                    + if line.preedit.is_none() {
                                        segment.caret_virtual_space as f64 * this.space_advance()
                                    } else {
                                        0.0
                                    },
                                origin.y + r.y,
                                1.0,
                                r.height.max(this.line_height()),
                            ),
                            0.0,
                        );
                    }
                }
            }
        }
    }
    fn draw_viewport_margins(
        &self,
        context: &mut DrawingContext,
        frame: &ViewportFrame,
        editor: &TextEditor,
    ) {
        let this = self;
        let gutter = self.gutter_width();
        let folding_sections = editor
            .folding_manager()
            .map(|manager| manager.all_foldings())
            .unwrap_or_default();
        for entry in &frame.lines {
            let index = entry.index;
            let line = &entry.line;
            let top = entry.origin.y;
            if editor.show_line_numbers() {
                let number = TextLayout::new(
                    &(index + 1).to_string(),
                    Typeface::new(editor.font_family()),
                    TextLayoutOptions {
                        font_size: editor.font_size(),
                        foreground: editor.line_numbers_foreground(),
                        ..Default::default()
                    },
                );
                number.draw(
                    context,
                    Point::new(
                        gutter - number.width() - this.folding_gutter_width() - 8.0,
                        top,
                    ),
                );
            }
            if !folding_sections.is_empty() {
                let first = folding_sections.partition_point(|s| s.start_offset() < line.offset);
                if let Some(section) = folding_sections.get(first).filter(|s| {
                    editor
                        .document()
                        .get_line_by_offset(s.start_offset())
                        .is_ok_and(|l| l.line_number() == index + 1)
                }) {
                    let marker = TextLayout::new(
                        if section.is_folded() { "+" } else { "−" },
                        Typeface::new(editor.font_family()),
                        TextLayoutOptions {
                            font_size: editor.font_size(),
                            foreground: editor.line_numbers_foreground(),
                            ..Default::default()
                        },
                    );
                    marker.draw(context, Point::new(gutter - 16.0, top));
                }
            }
        }
    }
}
