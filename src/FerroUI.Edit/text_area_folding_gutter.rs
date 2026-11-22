use super::*;
use crate::column_ruler_renderer::pixel_align;
use crate::folding_margin::round_to_odd_pixel;
use crate::FoldingMargin;
use ferroui_base::media::BoxShadows;

const MARKER_SCALE: f64 = 0.7;

struct FoldingMarker {
    section: FoldingSection,
    bounds: Rect,
}

struct FoldingGuide {
    span: Range<f64>,
    elbow: bool,
    highlighted: bool,
}

/// Marker bounds are shared by rendering and pointer hit testing.
struct FoldingGutter {
    bounds: Rect,
    pixel: f64,
    offset_y: f64,
    markers: Vec<FoldingMarker>,
}

impl FoldingGutter {
    fn for_margin(
        margin: &FoldingMargin,
        view: &TextArea,
        frame: &ViewportFrame,
        foldings: &[FoldingSection],
    ) -> Self {
        let offset_y = view
            .translate_point(Point::default(), margin)
            .map_or(0.0, |point| point.y);
        Self::new(
            margin.local_bounds(),
            margin.pixel_size(),
            offset_y,
            frame,
            foldings,
        )
    }

    fn new(
        bounds: Rect,
        pixel: f64,
        offset_y: f64,
        frame: &ViewportFrame,
        foldings: &[FoldingSection],
    ) -> Self {
        let size = round_to_odd_pixel(bounds.width * MARKER_SCALE, pixel);
        let markers = frame
            .lines
            .iter()
            .filter_map(|entry| {
                let line = &entry.line;
                let index =
                    foldings.partition_point(|section| section.start_offset() < line.offset);
                let section = foldings.get(index)?;
                let start = section.start_offset();
                if start > line.display.last_document_line().end_offset() {
                    return None;
                }
                let y = entry.row_middle_at(start)? + offset_y;
                Some(FoldingMarker {
                    section: section.clone(),
                    bounds: Rect::new(
                        round_to_pixel(bounds.center().x - size * 0.5, pixel),
                        round_to_pixel(y - size * 0.5, pixel),
                        size,
                        size,
                    ),
                })
            })
            .collect();
        Self {
            bounds,
            pixel,
            offset_y,
            markers,
        }
    }

    fn marker_at(&self, point: Point) -> Option<&FoldingMarker> {
        if !self.bounds.contains_exclusive(point) {
            return None;
        }
        self.markers
            .iter()
            .find(|marker| marker.bounds.contains_exclusive(point))
    }

    fn guides(
        &self,
        frame: &ViewportFrame,
        foldings: &[FoldingSection],
        hovered: Option<&FoldingMarker>,
    ) -> Vec<FoldingGuide> {
        let Some(first) = frame.lines.first().map(|entry| entry.line.offset) else {
            return Vec::new();
        };
        let mut guides = Vec::new();
        // Foldings starting before the viewport can still bring a guide or
        // end connector into view, even when neither marker is visible.
        for section in foldings
            .iter()
            .take_while(|section| section.start_offset() < first)
        {
            if !section.is_folded() && section.end_offset() >= first {
                guides.extend(self.guide(frame, section, self.bounds.y, false));
            }
        }
        for marker in &self.markers {
            if !marker.section.is_folded() {
                guides.extend(self.guide(
                    frame,
                    &marker.section,
                    marker.bounds.center().y,
                    hovered.is_some_and(|hovered| std::ptr::eq(hovered, marker)),
                ));
            }
        }
        guides
    }

    fn guide(
        &self,
        frame: &ViewportFrame,
        section: &FoldingSection,
        start: f64,
        highlighted: bool,
    ) -> Option<FoldingGuide> {
        let end = frame
            .row_middle_at(section.end_offset())
            .map(|y| pixel_align(y + self.offset_y, self.pixel));
        let bottom = self.bounds.bottom();
        let y = end.unwrap_or(bottom);
        if start >= bottom || y < self.bounds.y {
            return None;
        }
        Some(FoldingGuide {
            span: start.max(self.bounds.y)..y.min(bottom),
            elbow: end.is_some() && y < bottom,
            highlighted,
        })
    }
}

impl ViewportLine {
    fn row_middle_at(&self, offset: usize) -> Option<f64> {
        let column = self
            .line
            .display
            .visual_offset(offset.checked_sub(self.line.offset)?);
        let layout = &self.line.layout;
        let row = usize::try_from(layout.get_line_index_from_character_index(column as i32, false))
            .ok()?;
        Some(
            self.origin.y
                + layout.line_origin(row).y
                + layout.text_lines().get(row)?.height() * 0.5,
        )
    }
}

impl ViewportFrame {
    fn row_middle_at(&self, offset: usize) -> Option<f64> {
        let index = self
            .lines
            .partition_point(|entry| entry.line.offset <= offset)
            .checked_sub(1)?;
        let entry = self.lines.get(index)?;
        let last = entry.line.display.last_document_line();
        (offset < last.end_offset() + last.delimiter_length() || offset == last.end_offset())
            .then(|| entry.row_middle_at(offset))
            .flatten()
    }
}

impl TextArea {
    fn folding_gutter_bounds(&self) -> Rect {
        let width = self.folding_gutter_width();
        Rect::new(
            self.gutter_width() - width,
            0.0,
            width,
            self.viewport.get().height,
        )
    }

    pub(in crate::text_area) fn folding_gutter_contains(&self, point: Point) -> bool {
        self.folding_gutter_bounds().contains_exclusive(point)
    }

    pub(in crate::text_area) fn update_folding_pointer(&self, point: Option<Point>) {
        if let Some(margin) = self.folding_margin() {
            margin.update_pointer(point.and_then(|point| self.translate_point(point, margin)));
        }
    }

    pub(in crate::text_area) fn press_builtin_folding_margin(
        &self,
        event: &PointerPressedEventArgs,
        point: Point,
    ) {
        if let Some(margin) = self.folding_margin() {
            if let Some(local) = self.translate_point(point, margin) {
                self.press_folding_margin(margin, event, local);
            }
        }
        event.set_handled(true);
    }

    pub(crate) fn press_folding_margin(
        &self,
        margin: &FoldingMargin,
        event: &PointerPressedEventArgs,
        point: Point,
    ) {
        if event.handled()
            || !event.properties().is_left_button_pressed
            || self.pointer_selection.borrow().is_some()
            || !self.uses_default_editor_input()
        {
            return;
        }
        let Some(editor) = self.editor() else { return };
        self.focus();
        margin.update_pointer(Some(point));
        match self.folding_marker_at(margin, &editor, point) {
            Ok(Some(section)) => section.set_is_folded(!section.is_folded()),
            Ok(None) => {}
            Err(error) => editor.report_error(error.to_string()),
        }
        event.set_handled(true);
    }

    fn folding_marker_at(
        &self,
        margin: &FoldingMargin,
        editor: &TextEditor,
        point: Point,
    ) -> Result<Option<FoldingSection>, VisualLineError> {
        if !margin.local_bounds().contains_exclusive(point) {
            return Ok(None);
        }
        let document = editor.document();
        let frame = self.collect_viewport_frame()?;
        if frame.document != document || !self.owns_folding_projection(margin) {
            return Ok(None);
        }
        let Some(manager) = margin.manager_for_editor(editor) else {
            return Ok(None);
        };
        let gutter = FoldingGutter::for_margin(margin, self, &frame, &manager.all_foldings());
        Ok(gutter.marker_at(point).map(|marker| marker.section.clone()))
    }

    fn owns_folding_projection(&self, margin: &FoldingMargin) -> bool {
        margin
            .text_view()
            .is_some_and(|view| view.ptr_eq(&self.to_ref()))
    }

    pub(crate) fn render_folding_margin(
        &self,
        margin: &FoldingMargin,
        context: &mut DrawingContext,
    ) {
        if margin.local_bounds().is_empty()
            || self.rendering.get()
            || self.redraw_pending.get() == PendingRedraw::InvalidatedFrame
        {
            return;
        }
        let Some(editor) = self.editor() else { return };
        if margin.manager_for_editor(&editor).is_none() {
            return;
        }
        self.rendering.set(true);
        let _scope = SynchronizeScope(&self.rendering);
        let frame = match self.collect_viewport_frame() {
            Ok(frame) => frame,
            Err(error) => {
                editor.report_error(error.to_string());
                return;
            }
        };
        if !self.owns_folding_projection(margin) {
            return;
        }
        let Some(manager) = margin.manager_for_editor(&editor) else {
            return;
        };
        let foldings = manager.all_foldings();
        if foldings.is_empty() {
            return;
        }
        let gutter = FoldingGutter::for_margin(margin, self, &frame, &foldings);
        let hovered = margin
            .pointer()
            .filter(|_| self.uses_default_editor_input())
            .and_then(|point| gutter.marker_at(point));
        let guides = gutter.guides(&frame, &foldings, hovered);
        let brush = margin
            .get_value(FoldingMargin::folding_marker_brush_property())
            .or_else(|| editor.get_value(TextEditor::folding_marker_brush_property()))
            .or_else(|| editor.line_numbers_foreground());
        let background = margin
            .get_value(FoldingMargin::folding_marker_background_brush_property())
            .or_else(|| editor.get_value(TextEditor::folding_marker_background_brush_property()))
            .or_else(|| editor.background());
        let selected_brush = margin
            .get_value(FoldingMargin::selected_folding_marker_brush_property())
            .or_else(|| editor.get_value(TextEditor::selected_folding_marker_brush_property()))
            .or_else(|| editor.foreground())
            .or_else(|| brush.clone());
        let selected_background = margin
            .get_value(FoldingMargin::selected_folding_marker_background_brush_property())
            .or_else(|| {
                editor.get_value(TextEditor::selected_folding_marker_background_brush_property())
            })
            .or_else(|| background.clone());
        let pen: Rc<dyn IPen> = Pen::with_brush(brush, gutter.pixel).into();
        let selected_pen: Rc<dyn IPen> = Pen::with_brush(selected_brush, gutter.pixel).into();

        let clip = context.push_clip(gutter.bounds);
        draw_guides(context, &gutter, &guides, &pen, &selected_pen);
        for marker in &gutter.markers {
            let active = hovered.is_some_and(|hovered| std::ptr::eq(hovered, marker));
            let (pen, fill) = if active {
                (&selected_pen, &selected_background)
            } else {
                (&pen, &background)
            };
            draw_marker(context, marker, gutter.pixel, pen, fill.as_ref());
        }
        context.pop(clip);
    }
}

fn draw_marker(
    context: &mut DrawingContext,
    marker: &FoldingMarker,
    pixel: f64,
    pen: &Rc<dyn IPen>,
    fill: Option<&Rc<dyn IBrush>>,
) {
    let outline = marker.bounds.deflate(pixel * 0.5);
    if outline.is_empty() {
        if let Some(brush) = pen.brush() {
            context.fill_rectangle(&brush, marker.bounds, 0.0);
        }
        return;
    }
    context.draw_rectangle(fill, Some(pen), outline, 0.0, 0.0, &BoxShadows::default());
    let center = marker.bounds.center();
    let radius = round_to_pixel(outline.width * 0.25, pixel).max(pixel);
    context.draw_line(
        pen,
        Point::new(center.x - radius, center.y),
        Point::new(center.x + radius, center.y),
    );
    if marker.section.is_folded() {
        context.draw_line(
            pen,
            Point::new(center.x, center.y - radius),
            Point::new(center.x, center.y + radius),
        );
    }
}

fn draw_guides(
    context: &mut DrawingContext,
    gutter: &FoldingGutter,
    guides: &[FoldingGuide],
    pen: &Rc<dyn IPen>,
    selected_pen: &Rc<dyn IPen>,
) {
    let x = pixel_align(gutter.bounds.center().x, gutter.pixel);
    let highlighted = guides.iter().find(|guide| guide.highlighted);
    // Overlapping open folds share one rail so translucent theme brushes
    // blend only once. A hovered fold owns the overlapping part.
    let mut spans: Vec<_> = guides
        .iter()
        .filter(|guide| !guide.highlighted)
        .map(|guide| guide.span.clone())
        .collect();
    spans.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut runs: Vec<Range<f64>> = Vec::new();
    for span in spans {
        if let Some(last) = runs.last_mut().filter(|last| last.end >= span.start) {
            last.end = last.end.max(span.end);
        } else {
            runs.push(span);
        }
    }
    let mut vertical = |span: Range<f64>, pen: &Rc<dyn IPen>| {
        let mut start = span.start;
        let draw = |context: &mut DrawingContext, start, end| {
            if start < end {
                context.draw_line(pen, Point::new(x, start), Point::new(x, end));
            }
        };
        // Leave the symbol legible when its brush uses a transparent fill.
        let first = gutter
            .markers
            .partition_point(|marker| marker.bounds.bottom() <= start);
        for marker in gutter.markers[first..]
            .iter()
            .take_while(|marker| marker.bounds.y < span.end)
        {
            draw(context, start, marker.bounds.y);
            start = start.max(marker.bounds.bottom());
        }
        draw(context, start, span.end);
    };
    for span in runs {
        if let Some(active) = highlighted {
            vertical(span.start..span.end.min(active.span.start), pen);
            vertical(span.start.max(active.span.end)..span.end, pen);
        } else {
            vertical(span, pen);
        }
    }
    if let Some(active) = highlighted {
        vertical(active.span.clone(), selected_pen);
    }

    let mut elbows: Vec<_> = guides
        .iter()
        .filter(|guide| guide.elbow)
        .map(|guide| guide.span.end)
        .collect();
    elbows.sort_by(f64::total_cmp);
    elbows.dedup();
    for y in elbows {
        let elbow_pen = if highlighted.is_some_and(|active| active.elbow && active.span.end == y) {
            selected_pen
        } else {
            pen
        };
        let start = gutter
            .marker_at(Point::new(x, y))
            .map_or(x - gutter.pixel * 0.5, |marker| marker.bounds.right());
        context.draw_line(
            elbow_pen,
            Point::new(start, y),
            Point::new(gutter.bounds.right(), y),
        );
    }
}

fn round_to_pixel(value: f64, pixel: f64) -> f64 {
    (value / pixel).round() * pixel
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{laid_out_editor, test_scope};
    use ferroui_base::input::{
        MouseButton, Pointer, PointerPointProperties, PointerType, RawInputModifiers,
    };
    use ferroui_base::layout::ILayoutManager;
    use ferroui_base::media::PlatformDrawingContext;
    use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
    use ferroui_controls::{DockPanel, Window};

    fn render_margin(margin: &FoldingMargin) -> Vec<String> {
        let log = DrawingLog::new();
        let mut platform = MockDrawingContextImpl::new(log.clone());
        let mut core = PlatformDrawingContext::borrowed(&mut platform);
        let mut context = DrawingContext::new(&mut core);
        margin.render(&mut context);
        context.dispose();
        log.entries()
    }

    #[test]
    fn a_folding_margin_in_a_sibling_panel_styles_and_toggles_the_bound_view() {
        let _scope = test_scope();
        let editor = laid_out_editor("{\r\ninside\r\n}\r\nfooter");
        let area = editor.text_area();
        let manager = editor.install_folding();
        let fold = manager.create_folding(0, 13).unwrap();
        let margin = FoldingMargin::new();
        let binding = margin.as_margin();
        binding.set_text_view(Some(&area)).unwrap();
        margin.set_folding_manager(Some(manager.clone()));
        margin.set_value(
            FoldingMargin::folding_marker_brush_property(),
            Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff2548ac)) as Rc<dyn IBrush>),
        );
        let wrong_document = Rc::new(ferroui_edit_core::folding::FoldingManager::new(
            TextDocument::from_text("another document"),
        ));
        margin.set_folding_manager(Some(wrong_document));
        assert!(margin.folding_manager().is_none());
        margin.set_folding_manager(Some(manager.clone()));

        let panel = DockPanel::new();
        panel.children().add(margin.clone());
        panel.children().add(editor.clone());
        let window = Window::new();
        window.set_width(300.0);
        window.set_height(140.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        window.layout_manager().execute_layout_pass();
        assert!(!margin.is_visual_ancestor_of(&area));

        assert!(render_margin(&margin)
            .iter()
            .any(|entry| entry.starts_with("DrawLine") && entry.contains("#ff2548ac")));

        let frame = area.collect_viewport_frame().unwrap();
        let gutter = FoldingGutter::for_margin(&margin, &area, &frame, &manager.all_foldings());
        let center = margin
            .translate_point(gutter.markers[0].bounds.center(), &window)
            .unwrap();
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let press = PointerPressedEventArgs::new(
            margin.clone(),
            pointer,
            &window,
            center,
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        );
        margin.raise_event(&press);
        assert!(press.handled() && fold.is_folded());
        assert_eq!(area.heights.borrow().visible_owner(1), 0);

        let builtin = area.folding_margin().unwrap().clone();
        editor.set_document(TextDocument::from_text("replacement"));
        assert!(area.folding_margin().is_none() && builtin.text_view().is_none());
        assert!(margin.text_view().unwrap().ptr_eq(&area));
        assert!(margin.folding_manager().is_none());
        window.close();
    }

    #[test]
    fn markers_follow_wrapped_fold_starts_and_blank_gutter_preserves_selection() {
        let _scope = test_scope();
        let text = format!("{}{{\nbody\nlast}}\nnext", "word ".repeat(8));
        let editor = laid_out_editor(&text);
        editor.set_show_line_numbers(true);
        editor.set_word_wrap(true);
        let area = editor.text_area();
        let window = Window::new();
        window.set_content(Some(Control::boxed(editor.clone())));
        window.show();
        let viewport = Size::new(155.0, 330.0);
        area.measure(viewport);
        area.arrange(Rect::from_size(viewport));
        let start = text.find('{').unwrap();
        let end = text.find('}').unwrap() + 1;
        let fold = editor.install_folding().create_folding(start, end).unwrap();
        area.measure(viewport);
        area.arrange(Rect::from_size(viewport));
        let margin = area.folding_margin().unwrap().clone();
        assert!(margin.text_view().unwrap().ptr_eq(&area));
        let frame = area.collect_viewport_frame().unwrap();
        let gutter = FoldingGutter::new(
            area.folding_gutter_bounds(),
            margin.pixel_size(),
            0.0,
            &frame,
            &[fold.clone()],
        );
        let marker = gutter.markers.first().unwrap().bounds.center();
        let first = &frame.lines[0];
        let first_row = first.line.layout.text_lines()[0].height();
        assert!(marker.y > first.origin.y + first_row);
        let blank = Point::new(marker.x, first.origin.y + first_row * 0.5);
        assert!(gutter.marker_at(blank).is_none());

        let click = |local| {
            let point = area.translate_point(local, &window).unwrap();
            let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
            let press = PointerPressedEventArgs::new(
                area.clone(),
                pointer.clone(),
                &window,
                point,
                1,
                PointerPointProperties::new(
                    RawInputModifiers::LEFT_MOUSE_BUTTON,
                    PointerUpdateKind::LeftButtonPressed,
                ),
                KeyModifiers::NONE,
                1,
            );
            area.raise_event(&press);
            assert!(press.handled());
            area.raise_event(&PointerReleasedEventArgs::new(
                area.clone(),
                pointer,
                &window,
                point,
                2,
                PointerPointProperties::new(
                    RawInputModifiers::NONE,
                    PointerUpdateKind::LeftButtonReleased,
                ),
                KeyModifiers::NONE,
                MouseButton::Left,
            ));
        };
        editor.select(0, 2).unwrap();
        let selection = editor.selection();
        click(blank);
        assert!(!fold.is_folded());
        assert_eq!(editor.selection(), selection);
        area.invalidate_layout_cache();
        click(marker);
        assert!(fold.is_folded());
        assert_eq!(editor.selection(), selection);
        let frame = area.collect_viewport_frame().unwrap();
        let gutter = FoldingGutter::new(
            area.folding_gutter_bounds(),
            margin.pixel_size(),
            0.0,
            &frame,
            std::slice::from_ref(&fold),
        );
        click(gutter.markers[0].bounds.center());
        assert!(!fold.is_folded());
        assert_eq!(editor.selection(), selection);
        editor.uninstall_folding();
        assert!(area.folding_margin().is_none());
        assert!(margin.visual_parent().is_none() && margin.text_view().is_none());
        window.close();
    }

    #[test]
    fn a_marker_from_a_replaced_document_cannot_toggle_the_new_fold() {
        let _scope = test_scope();
        let editor = laid_out_editor("{\ninside\n}\nfooter");
        let area = editor.text_area();
        let fold = editor.install_folding().create_folding(0, 10).unwrap();
        let size = Size::new(360.0, 180.0);
        area.measure(size);
        area.arrange(Rect::from_size(size));
        let margin = area.folding_margin().unwrap().clone();
        let frame = area.collect_viewport_frame().unwrap();
        let gutter = FoldingGutter::for_margin(&margin, &area, &frame, std::slice::from_ref(&fold));
        let marker = gutter.markers[0].bounds.center();
        let weak = editor.to_ref().downgrade();
        let replaced = Cell::new(false);
        let _subscription = area.visual_line_construction_starting(move |_| {
            if !replaced.replace(true) {
                let editor = weak.upgrade().unwrap();
                editor.set_document(TextDocument::from_text("{\nnew\n}\nfooter"));
                editor.install_folding().create_folding(0, 7).unwrap();
            }
        });
        area.invalidate_layout_cache();

        assert!(area
            .folding_marker_at(&margin, &editor, marker)
            .unwrap()
            .is_none());
        assert_eq!(editor.text(), "{\nnew\n}\nfooter");
        assert!(!editor.folding_manager().unwrap().all_foldings()[0].is_folded());
    }

    #[test]
    fn nested_guides_continue_past_the_top_and_hover_uses_the_styled_brushes() {
        let _scope = test_scope();
        let text = "outer\nbefore\ninner\ninside\ninner end\nouter end\nfooter";
        let editor = laid_out_editor(text);
        let area = editor.text_area();
        let manager = editor.install_folding();
        let outer = manager
            .create_folding(0, text.find("outer end").unwrap() + 9)
            .unwrap();
        let inner = manager
            .create_folding(
                text.find("inner").unwrap(),
                text.find("inner end").unwrap() + 9,
            )
            .unwrap();
        let size = Size::new(360.0, 180.0);
        area.measure(size);
        area.arrange(Rect::from_size(size));
        let margin = area.folding_margin().unwrap();
        let color = |value| Rc::new(ImmutableSolidColorBrush::from_uint32(value)) as Rc<dyn IBrush>;
        editor.set_value(
            TextEditor::folding_marker_brush_property(),
            Some(color(0xff336699)),
        );
        editor.set_value(
            TextEditor::selected_folding_marker_brush_property(),
            Some(color(0xffcc2277)),
        );
        editor.set_value(
            TextEditor::selected_folding_marker_background_brush_property(),
            Some(color(0xff112244)),
        );
        let frame = area.collect_viewport_frame().unwrap();
        let gutter = FoldingGutter::for_margin(margin, &area, &frame, &manager.all_foldings());
        let hit = gutter
            .markers
            .iter()
            .find(|marker| marker.section.start_offset() == inner.start_offset())
            .unwrap();
        let guides = gutter.guides(&frame, &[outer.clone(), inner.clone()], Some(hit));
        assert_eq!(guides.len(), 2);
        assert!(guides[1].highlighted && guides[1].elbow);
        assert!(guides[0].span.start < guides[1].span.start);
        assert!(guides[0].span.end > guides[1].span.end);

        margin.update_pointer(Some(hit.bounds.center()));
        let log = render_margin(margin);
        assert!(log
            .iter()
            .any(|entry| entry.starts_with("DrawLine") && entry.contains("#ff336699")));
        assert!(log
            .iter()
            .any(|entry| entry.starts_with("DrawLine") && entry.contains("#ffcc2277")));
        assert!(log
            .iter()
            .any(|entry| entry.starts_with("DrawRectangle") && entry.contains("#ff112244")));
        margin.update_pointer(None);
        assert!(!render_margin(margin)
            .iter()
            .any(|entry| entry.contains("#ffcc2277")));

        let height = area.line_height() * 4.0;
        area.measure(Size::new(360.0, height));
        area.arrange(Rect::new(0.0, 0.0, 360.0, height));
        area.set_offset(Vector::new(0.0, area.line_height() * 2.0));
        let frame = area.collect_viewport_frame().unwrap();
        let gutter = FoldingGutter::new(
            area.folding_gutter_bounds(),
            margin.pixel_size(),
            0.0,
            &frame,
            &manager.all_foldings(),
        );
        assert!(frame.lines[0].line.offset > outer.start_offset());
        assert!(!gutter
            .markers
            .iter()
            .any(|marker| marker.section.start_offset() == outer.start_offset()));
        assert!(gutter
            .guides(&frame, &[outer, inner], None)
            .iter()
            .any(|guide| { guide.span.start == 0.0 && guide.span.end > 0.0 && guide.elbow }));
    }
}
