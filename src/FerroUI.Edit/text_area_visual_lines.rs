//! Live native visual-line ownership, construction and extension lifecycle.
use super::{CachedVisualLine, NavigationCaret, TextArea, TextEditor};
use crate::visual_line::EndOfLineLayouts;
use crate::{
    EndOfLineMarkerOptions, FoldingElementGenerator, LinkElementGenerator,
    MailLinkElementGenerator, PreeditElementGenerator, SingleCharacterElementGenerator,
    TabElementGenerator, VisualLine, VisualLineBuildOptions, VisualLineBuilder,
    VisualLineConstructionStartEventArgs, VisualLineElementGeneratorRef, VisualLineError,
    VisualLineTransformerRef,
};
use ferroui_base::Point;
use ferroui_base::collections::FerroList;
use ferroui_base::media::text_formatting::{GenericTextParagraphProperties, TextRunProperties};
use ferroui_base::media::{IBrush, TextAlignment, TextWrapping};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_controls::documents::EmbeddedControlRun;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_edit_core::document::{
    CaretPositioningMode, LogicalDirection, TextDocument, TextSnapshot,
};
use ferroui_edit_core::editing::{CaretMovement, Selection, SelectionSegment};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

type ConstructionHandler = dyn Fn(&VisualLineConstructionStartEventArgs);

fn apply_syntax_properties(
    builder: &mut VisualLineBuilder,
    start: usize,
    end: usize,
    properties: &dyn TextRunProperties,
) -> Result<(), VisualLineError> {
    builder.change_document_part(start, end, |element| {
        let data = element.data_mut();
        data.set_text_run_properties(properties);
        if let Some(background) = properties.background_brush() {
            // The editor paints element backgrounds before selection geometry.
            data.set_background_brush(Some(background.clone()));
            data.text_run_properties_mut().set_background_brush(None);
        }
    })
}

pub(super) struct VisualLineHost {
    generators: FerroList<VisualLineElementGeneratorRef>,
    transformers: FerroList<VisualLineTransformerRef>,
    connected_generators: RefCell<Vec<VisualLineElementGeneratorRef>>,
    connected_transformers: RefCell<Vec<VisualLineTransformerRef>>,
    subscriptions: RefCell<Vec<Subscription>>,
    starting: Rc<HandlerList<ConstructionHandler>>,
    batch: RefCell<Option<Rc<VisualLineBatch>>>,
    pub(super) end_of_line_layouts: RefCell<Rc<EndOfLineLayouts>>,
    pub(super) revision: Cell<u64>,
    pub(super) width_needs_measure: Cell<bool>,
    building: Cell<bool>,
    synchronizing: Cell<bool>,
    synchronize_again: Cell<bool>,
    measure_pending: Cell<bool>,
    pub(super) collapsed: RefCell<BTreeMap<usize, usize>>,
}
impl VisualLineHost {
    pub(super) fn new() -> Self {
        Self {
            generators: FerroList::new(),
            transformers: FerroList::new(),
            connected_generators: RefCell::new(Vec::new()),
            connected_transformers: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
            starting: Rc::new(HandlerList::new()),
            batch: RefCell::new(None),
            end_of_line_layouts: RefCell::default(),
            revision: Cell::new(0),
            width_needs_measure: Cell::new(false),
            building: Cell::new(false),
            synchronizing: Cell::new(false),
            synchronize_again: Cell::new(false),
            measure_pending: Cell::new(false),
            collapsed: RefCell::new(BTreeMap::new()),
        }
    }
}

struct VisualLineBatch {
    document: TextDocument,
    snapshot: TextSnapshot,
    revision: u64,
    properties: Rc<dyn TextRunProperties>,
    indentation_size: usize,
    generators: Vec<VisualLineElementGeneratorRef>,
    transformers: Vec<VisualLineTransformerRef>,
    end_of_line: Option<EndOfLineMarkerOptions>,
    end_of_line_layouts: Rc<EndOfLineLayouts>,
}

#[derive(Clone)]
pub(super) struct SelectionForegroundStyle {
    brush: Rc<dyn IBrush>,
    ranges: Vec<Range<usize>>,
}

impl PartialEq for SelectionForegroundStyle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.brush, &other.brush) && self.ranges == other.ranges
    }
}

impl SelectionForegroundStyle {
    fn apply(&self, line: &mut VisualLineBuilder) -> Result<(), VisualLineError> {
        let first = line.first_document_line().offset();
        let end = line.last_document_line().end_offset();
        for range in &self.ranges {
            // A programmatic selection can end between UTF-16 surrogates.
            // Keep the scalar in one formatter run while applying its color.
            let split_scalar = |offset| {
                let document = line.context().document();
                offset > first
                    && offset < end
                    && matches!(document.get_char_at(offset - 1), Ok(0xd800..=0xdbff))
                    && matches!(document.get_char_at(offset), Ok(0xdc00..=0xdfff))
            };
            let start = range.start - usize::from(split_scalar(range.start));
            let end = range.end + usize::from(split_scalar(range.end));
            line.change_document_part(start, end, |element| {
                element.data_mut().set_selection_foreground(self.brush.clone());
            })?;
        }
        Ok(())
    }
}

pub(super) struct VisualLineBatchScope<'a> {
    host: &'a VisualLineHost,
    owns_batch: bool,
}
impl Drop for VisualLineBatchScope<'_> {
    fn drop(&mut self) {
        if self.owns_batch {
            let old = self.host.batch.borrow_mut().take();
            drop(old);
        }
    }
}
struct FlagScope<'a>(&'a Cell<bool>);
impl Drop for FlagScope<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// A retained batch must remain connected until all generator finish hooks and
/// transformer callbacks have returned, even if a callback removes an entry.
struct ConstructionScope<'a>(&'a TextArea);
impl Drop for ConstructionScope<'_> {
    fn drop(&mut self) {
        let view = self.0;
        view.visual_lines.building.set(false);
        if view.visual_lines.synchronize_again.get() {
            let retired_batch = view.visual_lines.batch.borrow_mut().take();
            view.synchronize_visual_line_extensions();
            // Destructors can themselves change extension membership. Release
            // the snapshot after synchronization, outside any state borrow.
            drop(retired_batch);
        }
        view.schedule_automation_format_change();
    }
}

impl TextArea {
    pub fn non_printable_character_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::non_printable_character_brush_property())
    }

    pub fn set_non_printable_character_brush(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::non_printable_character_brush_property(), brush);
    }

    pub(crate) fn has_custom_visual_line_construction(&self) -> bool {
        let host = &self.visual_lines;
        !host.generators.is_empty() || !host.transformers.is_empty() || !host.starting.is_empty()
    }

    pub(super) fn visual_line_construction_in_progress(&self) -> bool {
        self.visual_lines.building.get()
    }
    /// Ordered custom generators. Changes reconnect the view and invalidate its
    /// retained lines. Composition and installed folds run before these
    /// entries. Tabs and special-character generators run after them.
    pub fn element_generators(&self) -> FerroList<VisualLineElementGeneratorRef> {
        self.visual_lines.generators.clone()
    }
    /// Application transformers run after syntax highlighting and before native
    /// shaping. The collection and its items retain identity across calls.
    pub fn line_transformers(&self) -> FerroList<VisualLineTransformerRef> {
        self.visual_lines.transformers.clone()
    }
    pub fn visual_line_construction_starting(
        &self,
        handler: impl Fn(&VisualLineConstructionStartEventArgs) + 'static,
    ) -> Subscription {
        let handlers = self.visual_lines.starting.clone();
        let token = handlers.add(Rc::new(handler));
        Disposable::create(move || {
            handlers.remove(token);
        }).into()
    }
    /// Immutable line models currently retained by the viewport, in document order.
    pub fn visual_lines(&self) -> Vec<Rc<VisualLine>> {
        self.cache
            .borrow()
            .values()
            .map(|line| line.display.clone())
            .collect()
    }
    /// Returns a retained line that includes this one-based logical line number.
    pub fn get_visual_line(&self, line_number: usize) -> Option<Rc<VisualLine>> {
        let index = self
            .heights
            .borrow()
            .visible_owner(line_number.checked_sub(1)?);
        let line = self.cache.borrow().get(&index).cloned()?;
        let editor = self.editor()?;
        let document_lines = line.display.first_document_line().line_number()
            ..=line.display.last_document_line().line_number();
        (document_lines.contains(&line_number)
            && self.cached_selection_foreground_is_current(&editor, &line))
            .then(|| line.display.clone())
    }
    /// Explicit construction reports extension failures to its caller. Ordinary
    /// input/render paths publish the same errors through TextEditor's error event.
    pub fn get_or_construct_visual_line(
        &self,
        line_number: usize,
    ) -> Result<Rc<VisualLine>, VisualLineError> {
        let editor = self
            .editor()
            .ok_or_else(|| VisualLineError::Extension("The text area has no document".into()))?;
        editor.document().get_line_by_number(line_number)?;
        let index = self.heights.borrow().visible_owner(line_number - 1);
        if self.heights.borrow().is_hidden(index) {
            return Err(VisualLineError::NoVisibleLine { line_number });
        }
        Ok(self.try_line_layout(index)?.display.clone())
    }
    /// Requests reformatting after an application changes settings or data
    /// used by a generator or transformer. Collection edits invalidate automatically.
    pub fn redraw(&self) {
        let document = self.editor().map(|editor| editor.document());
        self.invalidate_layout_cache();
        if let Some(document) = document {
            self.notify_automation_format_change(document);
        }
    }

    /// Requests reformatting for visual lines touching a UTF-16 document range.
    /// A point at a line boundary touches both adjacent lines.
    pub fn redraw_range(&self, range: Range<usize>) -> Result<(), VisualLineError> {
        let editor = self.editor().ok_or(VisualLineError::ConstructionInvalidated)?;
        let document = editor.document();
        let length = document.text_length();
        if range.start > range.end || range.end > length {
            return Err(VisualLineError::InvalidRange {
                start: range.start,
                end: range.end,
                length,
            });
        }
        let first = document.get_line_by_offset(range.start)?;
        let first_number = (first.line_number() - usize::from(first.offset() == range.start)).max(1);
        let last_number = document.get_line_by_offset(range.end)?.line_number();
        let (mut indices, before_retained_line) = {
            let cache = self.cache.borrow();
            let indices = cache
                .range(..last_number)
                .filter(|(_, line)| line.display.last_document_line().line_number() >= first_number)
                .map(|(&index, _)| index)
                .collect::<Vec<_>>();
            let before_retained_line = cache.last_key_value().is_some_and(|(_, line)| {
                line.display.last_document_line().line_number() >= first_number
            });
            (indices, before_retained_line)
        };
        // A generated collapse can outlive its cached visual owner when scrolled away.
        indices.extend(
            self.visual_lines
                .collapsed
                .borrow()
                .range(..=last_number)
                .filter(|&(_, &last)| last >= first_number)
                .map(|(&first, _)| first - 1),
        );
        if !indices.is_empty() || before_retained_line || self.visual_line_construction_in_progress() {
            self.redraw_cached_lines(&document, indices);
        }
        self.notify_automation_format_change(document);
        Ok(())
    }

    /// Requests reformatting for a retained visual line from this view.
    /// Returns false if the handle is stale or belongs to another view.
    pub fn redraw_visual_line(&self, line: &VisualLine) -> bool {
        let index = line.first_document_line().line_number() - 1;
        let matches = self.cache.borrow().get(&index).is_some_and(|cached| {
            std::ptr::eq(line, cached.display.as_ref())
        });
        if !matches {
            return false;
        }
        let Some(editor) = self.editor() else {
            return false;
        };
        let document = editor.document();
        self.redraw_cached_lines(&document, vec![index]);
        self.notify_automation_format_change(document);
        true
    }

    fn redraw_cached_lines(&self, document: &TextDocument, indices: Vec<usize>) {
        self.invalidate_viewport_lines();
        if !indices.is_empty() {
            self.visual_lines.width_needs_measure.set(true);
            let mut changed_collapses = false;
            let removed = {
                let mut cache = self.cache.borrow_mut();
                let mut collapsed = self.visual_lines.collapsed.borrow_mut();
                indices
                    .into_iter()
                    .filter_map(|index| {
                        changed_collapses |= collapsed.remove(&(index + 1)).is_some();
                        cache.remove(&index)
                    })
                    .collect::<Vec<_>>()
            };
            if changed_collapses {
                self.sync_collapsed_line_heights(document);
            }
            // Element destructors can reenter the editor.
            drop(removed);
        }
        self.invalidate_line_measure();
        self.request_redraw();
    }
    pub(super) fn connect_visual_line_extensions(&self) {
        let weak = self.to_ref().downgrade();
        let generators = self.element_generators();
        let token = generators.add_collection_changed(Rc::new(move |_| {
            if let Some(view) = weak.upgrade() {
                view.synchronize_visual_line_extensions();
            }
        }));
        self.visual_lines
            .subscriptions
            .borrow_mut()
            .push(Disposable::create(move || {
                generators.remove_collection_changed(token);
            }).into());
        let weak = self.to_ref().downgrade();
        let transformers = self.line_transformers();
        let token = transformers.add_collection_changed(Rc::new(move |_| {
            if let Some(view) = weak.upgrade() {
                view.synchronize_visual_line_extensions();
            }
        }));
        self.visual_lines
            .subscriptions
            .borrow_mut()
            .push(Disposable::create(move || {
                transformers.remove_collection_changed(token);
            }).into());
    }
    fn synchronize_visual_line_extensions(&self) {
        let host = &self.visual_lines;
        if host.building.get() && host.batch.borrow().is_some() {
            host.synchronize_again.set(true);
            self.invalidate_layout_cache();
            return;
        }
        if host.synchronizing.replace(true) {
            host.synchronize_again.set(true);
            return;
        }
        let _scope = FlagScope(&host.synchronizing);
        loop {
            host.synchronize_again.set(false);
            let mut generators = Vec::new();
            for generator in host.generators.snapshot().iter() {
                if !generators.contains(generator) {
                    generators.push(generator.clone());
                }
            }
            let mut transformers = Vec::new();
            for transformer in host.transformers.snapshot().iter() {
                if !transformers.contains(transformer) {
                    transformers.push(transformer.clone());
                }
            }
            let old_generators = host.connected_generators.replace(generators.clone());
            let old_transformers = host.connected_transformers.replace(transformers.clone());
            // No collection, state or cache borrow survives an application hook.
            for generator in old_generators
                .iter()
                .filter(|item| !generators.contains(item))
            {
                generator.remove_from_text_view(self);
            }
            for transformer in old_transformers
                .iter()
                .filter(|item| !transformers.contains(item))
            {
                transformer.remove_from_text_view(self);
            }
            for generator in generators
                .iter()
                .filter(|item| !old_generators.contains(item))
            {
                generator.add_to_text_view(self);
            }
            for transformer in transformers
                .iter()
                .filter(|item| !old_transformers.contains(item))
            {
                transformer.add_to_text_view(self);
            }
            drop(old_generators);
            drop(old_transformers);
            drop(generators);
            drop(transformers);
            self.invalidate_layout_cache();
            if !host.synchronize_again.get() {
                break;
            }
        }
        if let Some(editor) = self.editor() {
            self.notify_automation_format_change(editor.document());
        }
    }
    pub(super) fn disconnect_visual_line_extensions(&self) {
        let subscriptions = self.visual_lines.subscriptions.take();
        for subscription in subscriptions {
            subscription.dispose();
        }
        let generators = self.visual_lines.connected_generators.take();
        let transformers = self.visual_lines.connected_transformers.take();
        for generator in generators {
            generator.remove_from_text_view(self);
        }
        for transformer in transformers {
            transformer.remove_from_text_view(self);
        }
    }
    pub(super) fn clear_visual_line_cache(&self) {
        self.invalidate_viewport_lines();
        let old = self.cache.take();
        drop(old);
    }

    fn invalidate_viewport_lines(&self) {
        self.visual_lines
            .revision
            .set(self.visual_lines.revision.get().wrapping_add(1));
        self.discard_visible_viewport();
    }
    pub(super) fn invalidate_line_measure(&self) {
        if !self.rendering.get() {
            self.invalidate_measure();
        } else if !self.visual_lines.measure_pending.replace(true) {
            let weak = self.to_ref().downgrade();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(view) = weak.upgrade() {
                        view.visual_lines.measure_pending.set(false);
                        view.invalidate_measure();
                    }
                },
                DispatcherPriority::NORMAL,
            );
        }
    }
    pub(super) fn begin_visual_line_batch(
        &self,
        first: usize,
    ) -> Result<VisualLineBatchScope<'_>, VisualLineError> {
        if self.visual_lines.building.get() {
            return Err(VisualLineError::RecursiveConstruction);
        }
        if self.visual_lines.batch.borrow().is_some() {
            return Ok(VisualLineBatchScope {
                host: &self.visual_lines,
                owns_batch: false,
            });
        }
        self.visual_lines.building.set(true);
        let _scope = ConstructionScope(self);
        let editor = self
            .editor()
            .ok_or_else(|| VisualLineError::Extension("The text area has no document".into()))?;
        let document = editor.document();
        let first_line = document.get_line_by_number((first + 1).min(document.line_count()))?;
        let event = VisualLineConstructionStartEventArgs::new(first_line);
        for (_, handler) in self.visual_lines.starting.snapshot().iter() {
            handler(&event);
        }
        // Start hooks may change the document, options and extension collections.
        let document = editor.document();
        let snapshot = document.create_snapshot();
        let options = editor.options();
        let properties = Rc::new(editor.default_text_run_properties()) as Rc<dyn TextRunProperties>;
        let marker_brush = self.non_printable_character_brush();
        let end_of_line = options.show_end_of_line.then(|| EndOfLineMarkerOptions {
            crlf_glyph: options.end_of_line_crlf_glyph.clone(),
            cr_glyph: options.end_of_line_cr_glyph.clone(),
            lf_glyph: options.end_of_line_lf_glyph.clone(),
            foreground: marker_brush.clone(),
        });
        let end_of_line_layouts = {
            let mut layouts = self.visual_lines.end_of_line_layouts.borrow_mut();
            let locale = properties.culture_info();
            if end_of_line.is_some() && layouts.locale.as_ref() != locale {
                *layouts = Rc::new(EndOfLineLayouts::new(locale.cloned()));
            }
            layouts.clone()
        };
        let mut generators = Vec::<VisualLineElementGeneratorRef>::new();
        let preedit = self.preedit.borrow().clone();
        if let Some((text, _)) = preedit {
            let selected = editor.selection().primary();
            generators.push(
                Rc::new(PreeditElementGenerator::new(
                    selected.start_offset(),
                    selected.end_offset(),
                    &text,
                    selected.start_virtual_space(),
                )?)
                .into(),
            );
        }
        if let Some(manager) = editor.folding_manager() {
            generators.push(
                Rc::new(FoldingElementGenerator::from_snapshot(
                    manager.all_foldings(),
                ))
                .into(),
            );
        }
        generators.extend(self.element_generators().snapshot().iter().cloned());
        let mut tabs = TabElementGenerator::from_options(&options);
        let indentation_size = tabs.indentation_size;
        tabs.foreground = marker_brush.clone();
        generators.push(Rc::new(tabs).into());
        let mut special = SingleCharacterElementGenerator::from_options(&options);
        special.foreground = marker_brush;
        generators.push(Rc::new(special).into());
        if options.enable_hyperlinks {
            let mut links = LinkElementGenerator::new();
            links.require_control_modifier_for_click =
                options.require_control_modifier_for_hyperlink_click;
            generators.push(Rc::new(links).into());
        }
        if options.enable_email_hyperlinks {
            let mut links = MailLinkElementGenerator::new();
            links.0.require_control_modifier_for_click =
                options.require_control_modifier_for_hyperlink_click;
            generators.push(Rc::new(links).into());
        }
        let transformers = self
            .line_transformers()
            .snapshot()
            .iter()
            .cloned()
            .collect();
        *self.visual_lines.batch.borrow_mut() = Some(Rc::new(VisualLineBatch {
            document,
            snapshot,
            revision: self.visual_lines.revision.get(),
            properties,
            indentation_size,
            generators,
            transformers,
            end_of_line,
            end_of_line_layouts,
        }));
        Ok(VisualLineBatchScope {
            host: &self.visual_lines,
            owns_batch: true,
        })
    }
    pub(super) fn try_line_layout(
        &self,
        index: usize,
    ) -> Result<Rc<CachedVisualLine>, VisualLineError> {
        if self.heights.borrow().is_hidden(index) {
            return Err(VisualLineError::NoVisibleLine { line_number: index + 1 });
        }
        let editor = self.editor().ok_or(VisualLineError::ConstructionInvalidated)?;
        let cached = self.cache.borrow().get(&index).cloned();
        if let Some(line) = &cached {
            let needs_inline_layout = self.preparing_inline_controls.get()
                && line.has_inline_controls
                && line.layout.text_lines().iter().any(|row| {
                    row.text_runs().iter().any(|run| {
                        run.downcast_ref::<EmbeddedControlRun>().is_some_and(|run| {
                            !run.measured_metrics_are_current()
                                || !self.inline_controls.get().is_some_and(|layer| {
                                    run.control().visual_parent().is_some_and(|parent| parent.ptr_eq(layer))
                                })
                        })
                    })
                });
            if !needs_inline_layout && self.cached_selection_foreground_is_current(&editor, line) {
                return Ok(line.clone());
            }
        }
        let _batch_scope = self.begin_visual_line_batch(index)?;
        if self.visual_lines.building.replace(true) {
            return Err(VisualLineError::RecursiveConstruction);
        }
        let _building_scope = ConstructionScope(self);
        let batch = self
            .visual_lines
            .batch
            .borrow()
            .clone()
            .ok_or(VisualLineError::ConstructionInvalidated)?;
        let retained = cached.filter(|line| {
            self.cache.borrow().get(&index).is_some_and(|current| Rc::ptr_eq(current, line))
                && self.cached_selection_foreground_is_current(&editor, line)
        });
        let (display, selection_foreground) = if let Some(line) = retained {
            (line.display.clone(), line.selection_foreground.clone())
        } else {
            let mut builder = VisualLineBuilder::generate(
                batch.snapshot.clone(),
                index + 1,
                batch.properties.clone(),
                &batch.generators,
                VisualLineBuildOptions {
                    text_view: Some(self.to_ref().downgrade()),
                    wide_space_width: self.wide_space_width(),
                    end_of_line: batch.end_of_line.clone(),
                },
            )?;
            if batch.end_of_line.is_some() {
                builder.use_end_of_line_layouts(batch.end_of_line_layouts.clone());
            }
            self.apply_visual_line_highlighting(&mut builder, &batch)?;
            builder.run_transformers(&batch.transformers)?;
            let composing = builder.elements().iter().any(|element| element.composition_range().is_some());
            let foreground = self.selection_foreground_style(
                &editor,
                builder.first_document_line().offset()..builder.last_document_line().end_offset(),
                composing,
            );
            if let Some(foreground) = &foreground {
                foreground.apply(&mut builder)?;
            }
            (builder.finish()?, foreground)
        };
        let has_inline_controls = self.prepare_inline_line(&display)?;
        let paragraph = GenericTextParagraphProperties::with_options(
            batch.properties.clone(),
            TextAlignment::Left,
            if editor.word_wrap() {
                TextWrapping::Wrap
            } else {
                TextWrapping::NoWrap
            },
            f64::NAN,
            0.0,
        )
        .with_minimum_line_height(self.line_height())
        .with_fixed_tab_width(batch.indentation_size as f64 * self.wide_space_width());
        let layout = display.create_text_layout(
            Rc::new(paragraph),
            if editor.word_wrap() {
                (self.viewport.get().width - self.gutter_width() - 4.0).max(1.0)
            } else {
                f64::INFINITY
            },
            f64::INFINITY,
            0,
            None,
            None,
        )?;
        if self.heights.borrow().is_hidden(index)
            || self.visual_lines.revision.get() != batch.revision
            || editor.document() != batch.document
            || batch.snapshot.version().as_ref() != Some(&batch.document.version())
        {
            self.request_redraw();
            return Err(VisualLineError::ConstructionInvalidated);
        }
        let first = display.first_document_line();
        let last = display.last_document_line();
        let preedit = display
            .composition_range()
            .map(|range| (range.start, range.len()));
        let result = Rc::new(CachedVisualLine {
            offset: first.offset(),
            length: display.document_length(),
            display,
            layout,
            preedit,
            has_inline_controls,
            selection_foreground,
        });
        if last.line_number() > first.line_number() {
            self.visual_lines
                .collapsed
                .borrow_mut()
                .insert(first.line_number(), last.line_number());
            self.sync_collapsed_line_heights(&batch.document);
            let removed: Vec<_> = {
                let mut cache = self.cache.borrow_mut();
                let keys: Vec<_> = cache
                    .range(first.line_number()..last.line_number())
                    .map(|(&key, _)| key)
                    .collect();
                keys.into_iter()
                    .filter_map(|key| cache.remove(&key))
                    .collect()
            };
            drop(removed);
        }
        // Evicted custom elements may release application objects whose Drop
        // callbacks change the document or collections. Never publish stale
        // geometry after those callbacks, even though all borrows were released.
        if self.heights.borrow().is_hidden(index)
            || self.visual_lines.revision.get() != batch.revision
            || editor.document() != batch.document
            || batch.snapshot.version().as_ref() != Some(&batch.document.version())
            || !self.cached_selection_foreground_is_current(&editor, &result)
        {
            self.request_redraw();
            return Err(VisualLineError::ConstructionInvalidated);
        }
        self.heights
            .borrow_mut()
            .set_height(index, result.layout.height().max(self.line_height()));
        let old = self.cache.borrow_mut().insert(index, result.clone());
        drop(old);
        Ok(result)
    }

    fn selection_foreground_style(
        &self,
        editor: &TextEditor,
        line: Range<usize>,
        composing: bool,
    ) -> Option<SelectionForegroundStyle> {
        let brush = self.selection_foreground()?;
        let selection = editor.selection();
        let primary = selection.primary();
        let segments = selection.segments();
        let start = segments.partition_point(|segment| segment.end_offset() <= line.start);
        let ranges: Vec<_> = segments[start..]
            .iter()
            .take_while(|segment| segment.start_offset() < line.end)
            .filter(|segment| !composing || **segment != primary)
            .filter_map(|segment| {
                let range = segment.start_offset().max(line.start)..segment.end_offset().min(line.end);
                (!range.is_empty()).then_some(range)
            })
            .collect();
        (!ranges.is_empty()).then_some(SelectionForegroundStyle { brush, ranges })
    }

    fn selection_foreground_for_line(
        &self,
        editor: &TextEditor,
        line: &CachedVisualLine,
    ) -> Option<SelectionForegroundStyle> {
        self.selection_foreground_style(
            editor,
            line.offset..line.offset + line.length,
            line.preedit.is_some(),
        )
    }

    pub(super) fn cached_selection_foreground_is_current(
        &self,
        editor: &TextEditor,
        line: &CachedVisualLine,
    ) -> bool {
        line.selection_foreground == self.selection_foreground_for_line(editor, line)
    }
    fn apply_visual_line_highlighting(
        &self,
        builder: &mut VisualLineBuilder,
        batch: &VisualLineBatch,
    ) -> Result<(), VisualLineError> {
        let editor = self
            .editor()
            .ok_or(VisualLineError::ConstructionInvalidated)?;
        let sources = builder.source_lines()?;
        if let Some(installation) = editor.textmate_installation() {
            let target = sources.last().map_or(1, |line| line.line_number());
            installation.advance_to(&batch.document, target, 32);
            let background = installation.background_target(&batch.document, target);
            if !installation.cached_through(&batch.document, background)
                && installation.last_error().is_none()
            {
                self.schedule_highlight(target);
            }
            for source in sources {
                for (start, end, style) in installation.token_style_overrides(source.line_number()) {
                    let start = source.offset() + start.min(source.length());
                    let end = source.offset() + end.min(source.length());
                    if end > start {
                        let style = crate::NativeHighlightingStyle::from_textmate(style);
                        builder.change_document_part(start, end, |element| {
                            let data = element.data_mut();
                            let properties = style.text_run_properties(data.text_run_properties());
                            data.set_text_run_properties(properties.as_ref());
                            if let Some(background) = &style.background {
                                data.set_background_brush(Some(background.clone()));
                                data.text_run_properties_mut().set_background_brush(None);
                            }
                        })?;
                    }
                }
            }
        } else if let Some(highlighter) = editor.document_highlighter() {
            for source in sources {
                let highlighted = highlighter
                    .highlight_line(source.line_number())
                    .map_err(|error| VisualLineError::Extension(error.to_string()))?;
                for section in highlighted.sections {
                    let style =
                        crate::NativeHighlightingStyle::resolve(&section.color, Some(&editor))
                            .map_err(|error| VisualLineError::Extension(error.to_string()))?;
                    let properties = style.text_run_properties(batch.properties.as_ref());
                    apply_syntax_properties(
                        builder,
                        section.offset,
                        section.offset + section.length,
                        properties.as_ref(),
                    )?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn element_at_point(&self, point: Point) -> Option<(Rc<CachedVisualLine>, usize)> {
        if !self.text_viewport_contains(point) {
            return None;
        }
        let y = point.y + self.offset.get().y;
        let index = self.heights.borrow().line_at(y);
        let line = self.line_layout(index)?;
        let top = self.heights.borrow().top(index);
        let hit = line.layout.hit_test_point(Point::new(
            point.x + self.offset.get().x - self.gutter_width(),
            y - top,
        ));
        if !hit.is_inside() {
            return None;
        }
        let column = hit.character_hit().first_character_index().max(0) as usize;
        let element_index = line.display.elements().iter().position(|element| {
            element.data().visual_column() <= column && column < element.data().end_visual_column()
        })?;
        Some((line, element_index))
    }

    pub(crate) fn automation_line_at_point(&self, point: Point) -> Option<Rc<crate::VisualLine>> {
        if !self.text_viewport_contains(point) {
            return None;
        }
        let index = self.heights.borrow().line_at(point.y + self.offset.get().y);
        self.line_layout(index).map(|line| line.display.clone())
    }

    /// Horizontal movement visits generated element boundaries. In particular a
    /// closed fold is one caret step and must not be unfolded by stepping into
    /// the hidden document range. Unarranged views keep the core-only behavior.
    pub(super) fn move_caret_over_visual_elements(
        &self,
        movement: CaretMovement,
        extend: bool,
    ) -> bool {
        if self.viewport.get().width <= 0.0
            || !matches!(
                movement,
                CaretMovement::CharacterLeft
                    | CaretMovement::CharacterRight
                    | CaretMovement::WordLeft
                    | CaretMovement::WordRight
            )
        {
            return false;
        }
        let Some(editor) = self.editor() else {
            return false;
        };
        let document = editor.document();
        let version = document.version();
        let selection = editor.selection();
        let forward = matches!(
            movement,
            CaretMovement::CharacterRight | CaretMovement::WordRight
        );
        let word = matches!(movement, CaretMovement::WordLeft | CaretMovement::WordRight);
        let single_caret = selection.is_rectangular() && !extend;
        let allow_virtual = editor.options().enable_virtual_space || selection.is_rectangular() && extend;
        let resolve = |segment: &SelectionSegment| -> Result<(usize, usize), VisualLineError> {
            if !extend
                && !single_caret
                && !segment.is_empty()
                && !word
                && !(*segment == selection.primary() && self.active_context_caret(&selection).is_some())
                && matches!(
                    movement,
                    CaretMovement::CharacterLeft | CaretMovement::CharacterRight
                )
            {
                return Ok(if forward {
                    segment.end_tuple()
                } else {
                    segment.start_tuple()
                });
            }
            let mut caret = if *segment == selection.primary() {
                self.navigation_caret(&selection)
            } else {
                NavigationCaret::from_segment(*segment, false)
            };
            if single_caret && !allow_virtual {
                caret.virtual_space = 0;
            }
            let target = self.horizontal_caret(&document, caret, movement, allow_virtual)?;
            Ok((target.offset, target.virtual_space))
        };
        let mut segments = Vec::with_capacity(selection.segments().len());
        for (index, segment) in selection.segments().iter().enumerate() {
            if single_caret && index != selection.primary_index() {
                continue;
            }
            let target = match resolve(segment) {
                Ok(target) => target,
                Err(error) => {
                    editor.report_error(error.to_string());
                    return true;
                }
            };
            let anchor = if extend {
                segment.anchor_tuple()
            } else {
                target
            };
            segments.push(SelectionSegment::with_virtual_space(
                anchor.0, anchor.1, target.0, target.1,
            ));
        }
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
        {
            editor.report_error(ferroui_edit_core::editing::EditError::StaleEditPlan.to_string());
            return true;
        }
        let selection = if single_caret {
            Selection::from_segments(segments, 0)
        } else if selection.is_rectangular() {
            Selection::from_rectangle_segments(segments, selection.primary_index())
        } else {
            Selection::from_segments(segments, selection.primary_index())
        };
        if let Err(error) = editor.session().set_selection(selection) {
            editor.report_error(error.to_string());
        }
        true
    }

    pub(super) fn horizontal_caret(
        &self,
        document: &TextDocument,
        caret: NavigationCaret,
        movement: CaretMovement,
        allow_virtual: bool,
    ) -> Result<NavigationCaret, VisualLineError> {
        let forward = matches!(movement, CaretMovement::CharacterRight | CaretMovement::WordRight);
        let word = matches!(movement, CaretMovement::WordLeft | CaretMovement::WordRight);
        let mode = if word { CaretPositioningMode::WordStart } else { CaretPositioningMode::Normal };
        let (offset, virtual_space) = self.horizontal_destination(document, caret, forward, mode, allow_virtual)?;
        Ok(NavigationCaret { offset, virtual_space, at_end_of_row: false })
    }

    pub(super) fn visual_deletion_offset(
        &self,
        document: &TextDocument,
        segment: SelectionSegment,
        backward: bool,
        by_word: bool,
    ) -> Result<Option<usize>, VisualLineError> {
        if self.viewport.get().width <= 0.0 {
            return Ok(None);
        }
        let mode = if by_word {
            CaretPositioningMode::WordStart
        } else if backward {
            CaretPositioningMode::EveryCodepoint
        } else {
            CaretPositioningMode::Normal
        };
        let caret = NavigationCaret::from_segment(SelectionSegment::empty(segment.caret), false);
        self.horizontal_destination(document, caret, !backward, mode, false)
            .map(|(offset, _)| Some(offset))
    }

    fn horizontal_destination(
        &self,
        document: &TextDocument,
        caret: NavigationCaret,
        forward: bool,
        mode: CaretPositioningMode,
        allow_virtual: bool,
    ) -> Result<(usize, usize), VisualLineError> {
        let word = mode == CaretPositioningMode::WordStart;
        let logical = document.get_line_by_offset(caret.offset)?;
        let index = self
            .heights
            .borrow()
            .visible_owner(logical.line_number() - 1);
        let line = self.try_line_layout(index)?;
        if caret.virtual_space > 0 {
            return Ok((
                caret.offset,
                if forward {
                    if word {
                        0
                    } else {
                        caret.virtual_space.saturating_add(1)
                    }
                } else if word {
                    0
                } else {
                    caret.virtual_space - 1
                },
            ));
        }
        let mut visual = line.display.visual_offset(caret.offset - line.offset) as isize;
        if allow_virtual && !forward && !word && visual == 0 {
            return Ok((caret.offset, caret.virtual_space));
        }
        loop {
            let next = line.display.get_next_caret_position(
                visual,
                if forward {
                    LogicalDirection::Forward
                } else {
                    LogicalDirection::Backward
                },
                mode,
                allow_virtual && !word,
            )?;
            let Some(next) = next else {
                break;
            };
            if next > line.display.visual_length() {
                return Ok((
                    line.offset + line.length,
                    next - line.display.visual_length(),
                ));
            }
            let offset = line.offset + line.display.get_relative_offset(next);
            if offset != caret.offset {
                return Ok((offset, 0));
            }
            // Zero-document-length adornments do not create an editable
            // document position; advance past them without getting stuck.
            visual = next as isize;
        }
        if forward {
            let last = line.display.last_document_line().line_number();
            let next = self.heights.borrow().next_visible(last) + 1;
            let Ok(next) = document.get_line_by_number(next) else {
                return Ok((document.text_length(), 0));
            };
            if !word {
                return Ok((next.offset(), 0));
            }
            let next = self.try_line_layout(next.line_number() - 1)?;
            let column = next.display.get_next_caret_position(
                -1, LogicalDirection::Forward, mode, false,
            )?.unwrap_or(next.display.visual_length());
            Ok((next.offset + next.display.get_relative_offset(column), 0))
        } else if index == 0 {
            Ok((0, 0))
        } else {
            let previous = self.heights.borrow().visible_owner(index - 1);
            if previous >= index {
                return Ok((line.offset, 0));
            }
            let previous = self.try_line_layout(previous)?;
            Ok((previous.offset + previous.length, 0))
        }
    }
}
