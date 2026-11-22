//! Live background renderer lifetime, shared range geometry and native paint phases.
use super::*;
use crate::{
    BackgroundGeometryBuilder, BackgroundGeometryLine, BackgroundGeometryViewport,
    BackgroundRendererRef, ColumnRulerRenderer, CurrentLineHighlightRenderer, KnownLayer,
    VisibleEditorLine, VisualLineError, get_rects_for_segment, get_rects_for_selection,
};
use ferroui_base::collections::FerroList;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::text_formatting::{TextLayoutOptions, TextLineImpl};
use ferroui_base::media::{Brush, Geometry, IPen, Pen};
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    FerroProperty, FerroPropertyChangedEventArgs, Matrix, StyledProperty, StyledPropertyOptions,
};
use ferroui_edit_core::document::{DocumentVersion, ISegment, TextDocument};
use ferroui_edit_core::editing::{EditorOptions, Selection, SelectionSegment};
use ferroui_controls::documents::EmbeddedControlRun;
use std::collections::HashSet;
use std::ops::Range;

#[path = "text_area_folding_gutter.rs"]
mod folding_gutter;

pub(super) struct BackgroundRendererHost {
    renderers: FerroList<BackgroundRendererRef>,
    current_line: Rc<CurrentLineHighlightRenderer>,
    column_ruler: Rc<ColumnRulerRenderer>,
    connected: RefCell<Vec<BackgroundRendererRef>>,
    subscription: RefCell<Option<Subscription>>,
    selection_border_subscription: RefCell<Option<Subscription>>,
    selection_foreground_subscription: RefCell<Option<Subscription>>,
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
            selection_border_subscription: RefCell::new(None),
            selection_foreground_subscription: RefCell::new(None),
            painting: Cell::new(false),
            synchronizing: Cell::new(false),
            synchronize_again: Cell::new(false),
        }
    }
}
ferroui_base::ferro_properties! { impl TextArea {
    pub fn watermark_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<TextArea, _>("Watermark", None)
    }
    pub fn overstrike_mode_property() -> StyledProperty<bool> {
        FerroProperty::register::<TextArea, _>("OverstrikeMode", false)
    }
    pub fn right_click_moves_caret_property() -> StyledProperty<bool> {
        FerroProperty::register::<TextArea, _>("RightClickMovesCaret", false)
    }
    /// Foreground for visible whitespace and end-of-line markers.
    pub fn non_printable_character_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextArea, _>(
            "NonPrintableCharacterBrush",
            Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0x91808080)) as Rc<dyn IBrush>),
        )
    }
    pub fn column_ruler_pen_property() -> StyledProperty<Option<Rc<dyn IPen>>> {
        FerroProperty::register::<TextArea, _>("ColumnRulerPen", Some(Rc::new(ImmutablePen::from_uint32(0x5a808080, 1.0)) as Rc<dyn IPen>))
    }
    pub fn current_line_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextArea, _>("CurrentLineBackground", None)
    }
    pub fn current_line_border_property() -> StyledProperty<Option<Rc<dyn IPen>>> {
        FerroProperty::register::<TextArea, _>("CurrentLineBorder", None)
    }
    pub fn selection_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextEditor::selection_brush_property().add_owner::<TextArea>()
    }
    pub fn selection_border_property() -> StyledProperty<Option<Rc<dyn IPen>>> {
        FerroProperty::register::<TextArea, _>("SelectionBorder", None)
    }
    pub fn selection_corner_radius_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<TextArea, _>("SelectionCornerRadius", StyledPropertyOptions::new(3.0_f64).validate(|radius| radius.is_finite() && *radius >= 0.0))
    }
    pub fn selection_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextArea, _>("SelectionForeground", None)
    }
} }
struct SynchronizeScope<'a>(&'a Cell<bool>);
impl Drop for SynchronizeScope<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
struct BackgroundFrameScope<'a>(&'a TextArea);
struct RenderingScope<'a>(&'a TextArea);

impl Drop for RenderingScope<'_> {
    fn drop(&mut self) {
        self.0.rendering.set(false);
        self.0.schedule_automation_format_change();
    }
}

/// The segments that can touch the given lines: a frame costs what is visible, not the size of
/// the selection. The segments of a selection are ordered by offset.
fn visible_segments<'a>(selection: &'a Selection, lines: &[ViewportLine]) -> &'a [SelectionSegment] {
    let segments = selection.segments();
    let (Some(first), Some(last)) = (lines.first(), lines.last()) else {
        return &[];
    };
    let start = first.line.offset;
    let end = last.line.offset + last.line.length;
    let from = segments.partition_point(|segment| segment.end_offset() < start);
    let to = from + segments[from..].partition_point(|segment| segment.start_offset() <= end);
    &segments[from..to]
}

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
    explicit_collapse_revision: u64,
    offset: Vector,
    viewport: Size,
    selection: Selection,
    projected_caret: Option<NavigationCaret>,
    options: EditorOptions,
    metrics: BackgroundGeometryViewport,
}
impl ViewportFrame {
    fn visible_lines(&self, viewport: Rect) -> impl Iterator<Item = (&Rc<CachedVisualLine>, Rect)> + Clone {
        self.lines.iter().filter_map(move |entry| {
            let height = entry.line.layout.height().max(self.metrics.default_line_height);
            let bounds = Rect::new(viewport.x, entry.origin.y, viewport.width, height);
            (bounds.y < viewport.height && bounds.bottom() > 0.0).then_some((&entry.line, bounds))
        })
    }

    fn geometry_lines(&self) -> impl Iterator<Item = BackgroundGeometryLine<'_>> {
        self.lines.iter().map(|entry| BackgroundGeometryLine {
            visual_line: &entry.line.display,
            layout: &entry.line.layout,
            origin: entry.origin,
        })
    }
    fn check(&self, view: &TextArea) -> Result<(), VisualLineError> {
        if view.visual_lines.revision.get() == self.revision
            && view.explicit_collapse_revision.get() == self.explicit_collapse_revision
            && view.offset.get() == self.offset
            && view.viewport.get() == self.viewport
            && view.editor().is_some_and(|editor| {
                editor.document() == self.document
                    && self.document.version() == self.version
                    && editor.selection() == self.selection
                    && view.projected_caret(&self.selection) == self.projected_caret
            })
        {
            Ok(())
        } else {
            view.retry_invalidated_frame();
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
    pub fn selection_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_brush_property())
    }
    pub fn set_selection_brush(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_brush_property(), brush);
    }
    pub fn selection_border(&self) -> Option<Rc<dyn IPen>> {
        self.get_value(Self::selection_border_property())
    }
    pub fn set_selection_border(&self, pen: Option<Rc<dyn IPen>>) {
        self.set_value(Self::selection_border_property(), pen);
    }
    pub fn selection_corner_radius(&self) -> f64 {
        self.get_value(Self::selection_corner_radius_property())
    }
    pub fn set_selection_corner_radius(&self, radius: f64) {
        self.set_value(Self::selection_corner_radius_property(), radius);
    }
    pub fn selection_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_foreground_property())
    }
    pub fn set_selection_foreground(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_foreground_property(), brush);
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
        } else if change.property() == Self::selection_border_property().as_property() {
            let subscription = self.selection_border().and_then(|pen| {
                let pen = pen.as_object()?.downcast_ref::<Pen>()?;
                let weak = self.to_ref().downgrade();
                Some(pen.invalidated(move || {
                    if let Some(view) = weak.upgrade() {
                        view.invalidate_layer(KnownLayer::Selection);
                    }
                }))
            });
            self.background_layers.selection_border_subscription.replace(subscription);
            self.invalidate_layer(KnownLayer::Selection);
        } else if change.property() == Self::selection_brush_property().as_property()
            || change.property() == Self::selection_corner_radius_property().as_property()
        {
            self.invalidate_layer(KnownLayer::Selection);
        } else if change.property() == Self::selection_foreground_property().as_property() {
            let subscription = self.selection_foreground().and_then(|brush| {
                let brush = brush.as_object()?.downcast_ref::<Brush>()?;
                let weak = self.to_ref().downgrade();
                Some(brush.invalidated(move || {
                    if let Some(view) = weak.upgrade() {
                        view.invalidate_layout_cache();
                    }
                }))
            });
            self.background_layers.selection_foreground_subscription.replace(subscription);
            self.invalidate_layout_cache();
        }
    }
    /// Ordered native decoration renderers. Entries connect by shared identity
    /// and draw once, immediately before their selected layer's content.
    pub fn background_renderers(&self) -> FerroList<BackgroundRendererRef> {
        self.background_layers.renderers.clone()
    }
    /// Invalidates painting without discarding native text shaping. A phase
    /// with its own native visual can be repainted independently.
    pub fn invalidate_layer(&self, layer: KnownLayer) {
        if !self.invalidate_native_layer(layer) {
            self.request_redraw();
        }
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
    /// Constructs the visible lines if needed and delivers pending observers
    /// when called outside rendering. A retained viewport is reused.
    pub fn ensure_visual_lines(&self) -> Result<(), VisualLineError> {
        self.ensure_viewport_lines()?;
        self.deliver_visible_lines_changed();
        if !self.visual_lines_valid() {
            self.retry_invalidated_frame();
            return Err(VisualLineError::ConstructionInvalidated);
        }
        Ok(())
    }

    pub(super) fn ensure_viewport_lines(&self) -> Result<(), VisualLineError> {
        if self.visual_line_construction_in_progress() {
            return Err(VisualLineError::RecursiveConstruction);
        }
        if !self.visual_lines_valid() {
            if self.editor().is_none() {
                self.publish_visible_lines(std::iter::empty());
            } else {
                self.collect_viewport_frame()?;
            }
        }
        Ok(())
    }

    pub(super) fn visible_line_at_visual_top(
        &self,
        visual_top: f64,
    ) -> Result<Option<(Rc<CachedVisualLine>, f64)>, VisualLineError> {
        if !visual_top.is_finite() || visual_top < 0.0 {
            return Ok(None);
        }
        let frame = self.collect_viewport_frame()?;
        Ok(frame.lines.into_iter().find_map(|entry| {
            let top = entry.origin.y + frame.offset.y;
            let height = entry.line.layout.height().max(frame.metrics.default_line_height);
            (entry.origin.y + height > 0.0
                && entry.origin.y < frame.viewport.height
                && (top..top + height).contains(&visual_top))
                .then_some((entry.line, top))
        }))
    }
    /// Line projections intersecting the viewport, in display order. A
    /// multi-line fold is represented by the visual line that owns it.
    pub fn visible_editor_lines(&self) -> Result<Vec<VisibleEditorLine>, VisualLineError> {
        self.ensure_viewport_lines()?;
        self.visible_visual_lines().ok_or(VisualLineError::ConstructionInvalidated)
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
    pub(crate) fn automation_visible_text(&self, clip: Rect) -> Result<Vec<Range<usize>>, VisualLineError> {
        let frame = self.collect_viewport_frame()?;
        let clip = clip.intersect(self.content_viewport());
        let mut ranges = Vec::new();
        for entry in &frame.lines {
            let layout = &entry.line.layout;
            let display = &entry.line.display;
            for (index, row) in layout.text_lines().iter().enumerate() {
                let origin = layout.line_origin(index) + Vector::new(entry.origin.x, entry.origin.y);
                let start = row.first_text_source_index();
                for bounds in row.get_text_bounds(start, row.length()) {
                    for run in bounds.text_run_bounds() {
                        let rect = run.rectangle().translate(Vector::new(origin.x, origin.y));
                        let visible = rect.intersect(clip);
                        if visible.is_empty() {
                            continue;
                        }
                        let first = run.text_source_character_index();
                        let end = first + run.length();
                        let columns = if visible.width < rect.width {
                            let (leading, trailing) =
                                if bounds.flow_direction() == ferroui_base::media::FlowDirection::RightToLeft {
                                    (visible.right(), visible.left())
                                } else {
                                    (visible.left(), visible.right())
                                };
                            let left = row.get_character_hit_from_distance(leading - origin.x);
                            let right = row.get_character_hit_from_distance(trailing - origin.x);
                            let start = left.first_character_index().clamp(first, end);
                            let mut stop = right.first_character_index() + right.trailing_length();
                            // A clipped glyph is visible even if the hit is closer
                            // to its leading edge than to its trailing edge.
                            if right.trailing_length() == 0
                                && (row.get_distance_from_character_hit(right) - (trailing - origin.x)).abs() > 0.001
                            {
                                let next = row.get_next_caret_character_hit(right);
                                stop = next.first_character_index() + next.trailing_length();
                            }
                            start..stop.clamp(first, end)
                        } else {
                            first..end
                        };
                        let a = entry.line.offset + display.get_relative_offset(columns.start as usize);
                        let b = entry.line.offset + display.get_relative_offset(columns.end as usize);
                        if a < b {
                            ranges.push(a..b);
                        }
                    }
                }
            }
            let last = display.last_document_line();
            if last.delimiter_length() != 0 {
                let edge = layout.hit_test_text_position(display.visual_length() as i32)
                    .translate(Vector::new(entry.origin.x, entry.origin.y))
                    .with_width(1.0);
                if !edge.intersect(clip).is_empty() {
                    ranges.push(last.end_offset()..last.end_offset() + last.delimiter_length());
                }
            }
        }
        frame.check(self)?;
        Ok(ranges)
    }
    pub(crate) fn automation_inline_objects(&self) -> Result<Vec<(Ref<Control>, Range<usize>)>, VisualLineError> {
        let Some(layer) = self.inline_controls.get() else { return Ok(Vec::new()) };
        if layer.children().is_empty() {
            return Ok(Vec::new());
        }
        let frame = self.collect_viewport_frame()?;
        let mut objects = Vec::new();
        for entry in frame.lines.iter().filter(|entry| entry.line.has_inline_controls) {
            for element in entry.line.display.elements() {
                if let Some(control) = element.inline_control().filter(|child| {
                    child.is_effectively_visible()
                        && child.visual_parent().is_some_and(|parent| parent.ptr_eq(layer))
                }) {
                    let data = element.data();
                    let start = entry.line.offset + data.relative_text_offset();
                    let end = entry.line.offset + data.end_relative_text_offset();
                    objects.push((control, start..end));
                }
            }
        }
        frame.check(self)?;
        Ok(objects)
    }
    pub(super) fn connect_background_renderers(&self) {
        let weak = self.to_ref().downgrade();
        let renderers = self.background_renderers();
        let token = renderers.add_collection_changed(Rc::new(move |_| {
            if let Some(view) = weak.upgrade() {
                view.synchronize_background_renderers();
            }
        }));
        *self.background_layers.subscription.borrow_mut() = Some(
            Disposable::create(move || {
                renderers.remove_collection_changed(token);
            }).into(),
        );
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
            self.request_redraw();
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
            self.request_redraw();
            if !host.synchronize_again.get() {
                break;
            }
        }
    }
    pub(super) fn inline_control_placements(&self) -> Result<Vec<(Ref<Control>, Rect)>, VisualLineError> {
        let frame = self.collect_viewport_frame()?;
        let viewport = self.content_viewport();
        let mut controls = HashSet::new();
        let mut placements = Vec::new();
        for entry in frame.lines.iter().filter(|entry| entry.line.has_inline_controls) {
            let layout = &entry.line.layout;
            for (index, row) in layout.text_lines().iter().enumerate() {
                let origin = layout.line_origin(index);
                let mut x = entry.origin.x + origin.x + row.start() - viewport.x;
                let y = entry.origin.y + origin.y - viewport.y;
                if y >= viewport.height || y + row.height() <= 0.0 {
                    continue;
                }
                // The formatter supplies runs in physical order, including the
                // reordered runs for bidirectional text.
                for run in row.text_runs().iter() {
                    if let Some(drawable) = run.as_drawable() {
                        let size = drawable.size();
                        if let Some(run) = run.downcast_ref::<EmbeddedControlRun>() {
                            let child = run.control();
                            if !controls.insert(child.clone()) {
                                return Err(VisualLineError::Extension(
                                    "An inline control cannot occupy two visible positions".into(),
                                ));
                            }
                            let top = y + TextLineImpl::get_baseline_offset(&**row, drawable);
                            placements.push((child.clone(), Rect::from_position_size(Point::new(x, top), size)));
                        }
                        x += size.width;
                    }
                }
            }
        }
        Ok(placements)
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
            std::iter::successors((first < count).then_some(first), |&index| {
                let next = heights.next_visible(index + 1);
                (next < count).then_some(next)
            })
            .take_while(|&index| heights.top(index) - offset.y <= viewport.height)
            .any(|index| {
                cache.get(&index).is_none_or(|line| {
                    !self.cached_selection_foreground_is_current(&editor, line)
                })
            })
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
        let explicit_collapse_revision = self.explicit_collapse_revision.get();
        let offset = self.offset.get();
        let viewport = self.viewport.get();
        let selection_at_start = self.selection_foreground().map(|_| editor.selection());
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
        if self.visual_lines.width_needs_measure.replace(false) {
            self.update_extent_with_width(0.0);
        } else {
            self.update_extent();
        }
        let selection = editor.selection();
        if selection_at_start.is_some_and(|initial| initial != selection) {
            self.retry_invalidated_frame();
            return Err(VisualLineError::ConstructionInvalidated);
        }
        let frame = ViewportFrame {
            lines,
            document,
            version,
            revision,
            explicit_collapse_revision,
            offset,
            viewport,
            projected_caret: self.projected_caret(&selection),
            selection,
            options: editor.options(),
            metrics: BackgroundGeometryViewport {
                full_width: (self.extent.get().width.max(viewport.width) - gutter).max(0.0),
                default_line_height: self.line_height(),
                wide_space_width: self.wide_space_width(),
                ..BackgroundGeometryViewport::default()
            },
        };
        // Release generator/transformer snapshots before validating; custom
        // destructors may themselves change the document or viewport.
        drop(batch);
        frame.check(self)?;
        self.publish_visible_lines(frame.visible_lines(self.content_viewport()));
        Ok(frame)
    }
    pub(super) fn render_text_view(&self, context: &mut DrawingContext) {
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
        let _scope = RenderingScope(self);
        if self.background_layers.synchronizing.get() {
            // Connection hooks are application callbacks. A reentrant frame
            // must wait until every retained renderer is connected.
            self.retry_invalidated_frame();
            return;
        }
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
            .and_then(|frame| {
                let only = self.uses_native_layers().then_some(KnownLayer::Background);
                self.draw_viewport_frame(context, &frame, only)
            });
        context.pop(clip);
        if let Err(error) = result {
            if matches!(error, VisualLineError::ConstructionInvalidated) {
                self.retry_invalidated_frame();
            }
            editor.report_error(error.to_string());
        }
    }
    pub(super) fn render_editor_plane(&self, context: &mut DrawingContext, layer: KnownLayer) {
        if self.rendering.get() || self.background_layers.synchronizing.get() {
            self.request_redraw();
            return;
        }
        if self.redraw_pending.get() == PendingRedraw::InvalidatedFrame {
            return;
        }
        self.rendering.set(true);
        let _scope = RenderingScope(self);
        // Each phase is a native child of the clipped content viewport. Draw
        // using the same text-area geometry as the standalone render path.
        let transform = context.push_transform(Matrix::create_translation(-self.gutter_width(), 0.0));
        let result = self.collect_viewport_frame()
            .and_then(|frame| self.draw_viewport_frame(context, &frame, Some(layer)));
        context.pop(transform);
        if let Err(error) = result {
            if matches!(error, VisualLineError::ConstructionInvalidated) {
                self.retry_invalidated_frame();
            }
            if let Some(editor) = self.editor() {
                editor.report_error(error.to_string());
            }
        }
    }
    fn draw_viewport_frame(
        &self,
        context: &mut DrawingContext,
        frame: &ViewportFrame,
        only: Option<KnownLayer>,
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
            for layer in [KnownLayer::Background, KnownLayer::Selection, KnownLayer::Text, KnownLayer::Caret] {
                if only.is_some_and(|pass| pass != layer) {
                    continue;
                }
                self.draw_background_layer(layer, &renderers, context, frame)?;
                match layer {
                    KnownLayer::Background => self.draw_element_backgrounds(context, frame),
                    KnownLayer::Selection => self.draw_search_and_selection(context, frame, &editor)?,
                    KnownLayer::Text => {
                        for entry in &frame.lines {
                            entry.line.layout.draw(context, entry.origin);
                            frame.check(self)?;
                        }
                        if frame.document.text_length() == 0 && self.preedit.borrow().is_none() {
                            self.draw_watermark(context, &editor);
                        }
                    }
                    KnownLayer::Caret => self.draw_viewport_carets(context, frame, &editor),
                }
                frame.check(self)?;
            }
            Ok(())
        })();
        context.pop(clip);
        result?;
        frame.check(self)?;
        Ok(())
    }
    fn draw_watermark(&self, context: &mut DrawingContext, editor: &TextEditor) {
        let cached = self.watermark_layout.borrow().clone();
        let Some(layout) = cached.or_else(|| {
            let text = self.watermark().filter(|text| !text.is_empty())?;
            let layout = Rc::new(TextLayout::new(
                &text,
                editor.typeface(),
                TextLayoutOptions {
                    font_size: editor.font_size(),
                    foreground: editor.foreground(),
                    ..Default::default()
                },
            ));
            self.watermark_layout.replace(Some(layout.clone()));
            Some(layout)
        }) else { return };
        let opacity = context.push_opacity(0.5);
        layout.draw(context, self.content_viewport().top_left());
        context.pop(opacity);
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
        if let (Some(panel), Some(brush), Some(first), Some(last)) = (
            editor
                .installed_search_panel()
                .filter(|panel| panel.is_open()),
            editor.search_results_brush(),
            frame.lines.first(),
            frame.lines.last(),
        ) {
            let start = first.line.offset;
            let end = last.line.offset + last.line.length;
            for matched in panel.results_in_range(start, end - start) {
                let rectangles =
                    get_rects_for_segment(frame.geometry_lines(), &matched, frame.metrics, false)?;
                self.draw_background_rectangles(context, &brush, rectangles)?;
            }
        }
        let brush = self.selection_brush();
        let border = self.selection_border().filter(|pen| {
            let thickness = pen.thickness();
            thickness.is_finite() && thickness >= 0.0
        });
        if brush.is_some() || border.is_some() {
            let mut builder = BackgroundGeometryBuilder::new();
            builder.align_to_whole_pixels = true;
            builder.corner_radius = self.selection_corner_radius();
            builder.border_thickness = border.as_ref().map_or(0.0, |pen| pen.thickness());
            builder.extend_to_full_width_at_line_end =
                frame.options.enable_virtual_space || frame.selection.is_rectangular();
            let pixel_size = BackgroundGeometryBuilder::pixel_size(self);
            let primary = frame.selection.primary();
            for segment in visible_segments(&frame.selection, &frame.lines)
                .iter()
                .filter(|segment| !segment.is_empty())
            {
                let lines = frame
                    .lines
                    .iter()
                    .filter(|entry| *segment != primary || entry.line.preedit.is_none())
                    .map(|entry| BackgroundGeometryLine {
                        visual_line: &entry.line.display,
                        layout: &entry.line.layout,
                        origin: entry.origin,
                    });
                builder.add_selection(lines, *segment, frame.metrics, pixel_size)?;
                // Independent carets and rectangle rows have independent outlines.
                builder.close_figure();
            }
            if let Some(geometry) = builder.create_geometry() {
                context.draw_geometry(
                    brush.as_ref(),
                    border.as_ref(),
                    geometry.upcast_ref::<Geometry>(),
                );
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
        let drop_caret = self.drop_caret.get();
        if drop_caret.is_none() && !(self.is_focused() && self.caret_visible.get()) {
            return;
        }
        let brush = editor.foreground().unwrap_or_else(|| {
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xff202020))
        });
        if let Some(caret) = drop_caret {
            context.fill_rectangle(&brush, self.navigation_rectangle(caret, false, false), 0.0);
            return;
        }

        let selection = &frame.selection;
        let primary = selection.primary();
        let overstrike = self.overstrike_mode() && selection.is_empty();
        let composing = self.preedit.borrow().is_some();
        let projected_caret = frame.projected_caret.filter(|_| !composing);
        let is_visible_in_line = |line: &CachedVisualLine, offset| {
            (line.offset..=line.offset + line.length).contains(&offset)
                && line.display.is_document_offset_visible(offset - line.offset)
        };
        if let Some(caret) = projected_caret.filter(|caret| {
            frame.lines.iter().any(|entry| is_visible_in_line(&entry.line, caret.offset))
        }) {
            let rectangle = self.navigation_rectangle(caret, overstrike, false);
            let opacity = overstrike.then(|| context.push_opacity(0.5)).unwrap_or_default();
            context.scope(opacity).fill_rectangle(&brush, rectangle, 0.0);
        }

        for entry in &frame.lines {
            let line = &entry.line;
            for &segment in visible_segments(selection, std::slice::from_ref(entry)) {
                let is_primary = segment == primary;
                let preedit_caret = is_primary && composing && line.preedit.is_some();
                if !preedit_caret
                    && (!segment.is_empty()
                        || (is_primary && projected_caret.is_some())
                        || !is_visible_in_line(line, segment.caret))
                {
                    continue;
                }
                let caret = NavigationCaret::from_segment(
                    segment,
                    is_primary && self.caret_at_end_of_line.get(),
                );
                let block = overstrike && !preedit_caret;
                let bounds = self.line_caret_bounds(line, caret, block, preedit_caret);
                let opacity = block.then(|| context.push_opacity(0.5)).unwrap_or_default();
                context.scope(opacity).fill_rectangle(
                    &brush,
                    Rect::new(
                        entry.origin.x + bounds.x,
                        entry.origin.y + bounds.y,
                        bounds.width,
                        bounds.height.max(self.line_height()),
                    ),
                    0.0,
                );
            }
        }
    }
    pub(crate) fn render_line_number_margin(
        &self,
        margin: &crate::LineNumberMargin,
        context: &mut DrawingContext,
    ) {
        let bounds = margin.bounds();
        if bounds.is_empty()
            || self.rendering.get()
            || self.redraw_pending.get() == PendingRedraw::InvalidatedFrame
        {
            return;
        }
        let Some(editor) = self.editor() else { return };
        self.rendering.set(true);
        let _scope = SynchronizeScope(&self.rendering);
        let result = self.collect_viewport_frame().and_then(|frame| {
            frame.check(self)?;
            let y_offset = self.translate_point(Point::default(), margin).map_or(0.0, |point| point.y);
            let right = bounds.width - crate::LineNumberMargin::RIGHT_PADDING;
            let foreground = margin.foreground();
            for entry in &frame.lines {
                let layout = &entry.line.layout;
                let top = y_offset + entry.origin.y;
                let number = margin.number_layout(&(entry.index + 1).to_string(), foreground.clone());
                let y = layout.text_lines().first().map_or(top, |row| {
                    top + layout.line_origin(0).y + row.baseline() - number.baseline()
                });
                number.draw(context, Point::new(right - number.width(), y));
                frame.check(self)?;
            }
            Ok(())
        });
        if let Err(error) = result {
            editor.report_error(error.to_string());
        }
    }
}
