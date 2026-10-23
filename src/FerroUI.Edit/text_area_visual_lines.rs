//! Live native visual-line ownership, construction and extension lifecycle.
use super::{CachedVisualLine, TextArea};
use crate::{
    EndOfLineMarkerOptions, FoldingElementGenerator, LinkElementGenerator,
    MailLinkElementGenerator, PreeditElementGenerator, SingleCharacterElementGenerator,
    TabElementGenerator, VisualLine, VisualLineBuildOptions, VisualLineBuilder,
    VisualLineConstructionStartEventArgs, VisualLineElementGeneratorRef, VisualLineError,
    VisualLineTransformerRef,
};
use ferroui_base::Point;
use ferroui_base::collections::FerroList;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, TextRunProperties,
};
use ferroui_base::media::{
    BaselineAlignment, Color, FontStretch, FontStyle, FontWeight, IBrush, TextAlignment,
    TextDecorationCollection, TextWrapping, Typeface,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_edit_core::document::{
    CaretPositioningMode, LogicalDirection, TextDocument, TextSnapshot,
};
use ferroui_edit_core::editing::{CaretMovement, EditorOptions, Selection, SelectionSegment};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

type ConstructionHandler = dyn Fn(&VisualLineConstructionStartEventArgs);

pub(super) struct VisualLineHost {
    generators: FerroList<VisualLineElementGeneratorRef>,
    transformers: FerroList<VisualLineTransformerRef>,
    connected_generators: RefCell<Vec<VisualLineElementGeneratorRef>>,
    connected_transformers: RefCell<Vec<VisualLineTransformerRef>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    starting: Rc<HandlerList<ConstructionHandler>>,
    batch: RefCell<Option<Rc<VisualLineBatch>>>,
    pub(super) revision: Cell<u64>,
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
            revision: Cell::new(0),
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
    generators: Vec<VisualLineElementGeneratorRef>,
    transformers: Vec<VisualLineTransformerRef>,
    options: EditorOptions,
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
    }
}

impl TextArea {
    pub(super) fn visual_line_construction_in_progress(&self) -> bool {
        self.visual_lines.building.get()
    }
    /// Ordered custom generators. Changes reconnect the view and invalidate its
    /// retained lines. The built-in composition/folding/tab generators are added
    /// by the host after these entries (composition always has first priority).
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
    ) -> Rc<dyn IDisposable> {
        let handlers = self.visual_lines.starting.clone();
        let token = handlers.add(Rc::new(handler));
        Disposable::create(move || {
            handlers.remove(token);
        })
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
        self.cache
            .borrow()
            .get(&index)
            .map(|line| line.display.clone())
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
        Ok(self.try_line_layout(index)?.display.clone())
    }
    /// Requests reformatting after an application mutates a generator's own
    /// settings. Collection membership changes invalidate automatically.
    pub fn redraw(&self) {
        self.invalidate_layout_cache();
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
            }));
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
            }));
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
        self.visual_lines
            .revision
            .set(self.visual_lines.revision.get().wrapping_add(1));
        let old = self.cache.take();
        drop(old);
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
        let _scope = FlagScope(&self.visual_lines.building);
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
        let properties = Rc::new(GenericTextRunProperties::with_all(
            Typeface::new(editor.font_family()),
            editor.font_size(),
            None,
            editor.foreground(),
            None,
            BaselineAlignment::Baseline,
            None,
            None,
        )) as Rc<dyn TextRunProperties>;
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
        generators.extend(self.element_generators().snapshot().iter().cloned());
        if let Some(manager) = editor.folding_manager() {
            generators.push(
                Rc::new(FoldingElementGenerator::from_sections(
                    manager.all_foldings(),
                ))
                .into(),
            );
        }
        let mut tabs = TabElementGenerator::from_options(&options);
        tabs.foreground = editor.line_numbers_foreground();
        generators.push(Rc::new(tabs).into());
        let mut special = SingleCharacterElementGenerator::from_options(&options);
        special.foreground = editor.line_numbers_foreground();
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
            generators,
            transformers,
            options,
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
        if let Some(line) = self.cache.borrow().get(&index).cloned() {
            return Ok(line);
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
        let editor = self
            .editor()
            .ok_or(VisualLineError::ConstructionInvalidated)?;
        let mut builder = VisualLineBuilder::generate(
            batch.snapshot.clone(),
            index + 1,
            batch.properties.clone(),
            &batch.generators,
            VisualLineBuildOptions {
                text_view: Some(self.to_ref().downgrade()),
                wide_space_width: self.space_advance(),
                end_of_line: batch
                    .options
                    .show_end_of_line
                    .then(|| EndOfLineMarkerOptions {
                        foreground: editor.line_numbers_foreground(),
                        ..Default::default()
                    }),
            },
        )?;
        self.apply_visual_line_highlighting(&mut builder, &batch)?;
        builder.run_transformers(&batch.transformers)?;
        let display = builder.finish()?;
        let layout = display.create_text_layout(
            Rc::new(GenericTextParagraphProperties::with_options(
                batch.properties.clone(),
                TextAlignment::Left,
                if editor.word_wrap() {
                    TextWrapping::Wrap
                } else {
                    TextWrapping::NoWrap
                },
                f64::NAN,
                0.0,
            )),
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
        if self.visual_lines.revision.get() != batch.revision
            || editor.document() != batch.document
            || batch.snapshot.version().as_ref() != Some(&batch.document.version())
        {
            self.invalidate_visual();
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
        });
        if last.line_number() > first.line_number() {
            self.visual_lines
                .collapsed
                .borrow_mut()
                .insert(first.line_number(), last.line_number());
            let mut collapsed: Vec<_> = self
                .visual_lines
                .collapsed
                .borrow()
                .iter()
                .map(|(&a, &b)| (a, b))
                .collect();
            // Known folds still hide unconstructed offscreen rows. Generated
            // multiline elements contribute their complete retained span too.
            for fold in self.folds.borrow().iter() {
                if let (Ok(a), Ok(b)) = (
                    batch.document.get_line_by_offset(fold.start_offset()),
                    batch.document.get_line_by_offset(fold.end_offset()),
                ) {
                    collapsed.push((a.line_number(), b.line_number()));
                }
            }
            self.heights
                .borrow_mut()
                .set_collapsed_preserving_heights(&collapsed);
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
        if self.visual_lines.revision.get() != batch.revision
            || editor.document() != batch.document
            || batch.snapshot.version().as_ref() != Some(&batch.document.version())
        {
            self.invalidate_visual();
            return Err(VisualLineError::ConstructionInvalidated);
        }
        self.heights
            .borrow_mut()
            .set_height(index, result.layout.height().max(self.line_height()));
        let old = self.cache.borrow_mut().insert(index, result.clone());
        drop(old);
        Ok(result)
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
            if !installation.advance_to(&batch.document, target, 32)
                && installation.last_error().is_none()
            {
                self.schedule_highlight(target);
            }
            for source in sources {
                for (start, end, style) in installation.styled_tokens(source.line_number()) {
                    let brush = |c: ferroui_textmate::Color| {
                        Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(
                            c.a, c.r, c.g, c.b,
                        ))) as Rc<dyn IBrush>
                    };
                    let foreground = style.foreground.map(brush).or_else(|| editor.foreground());
                    let background = style.background.map(brush);
                    let typeface = Typeface::with_style(
                        editor.font_family(),
                        if style
                            .font_style
                            .contains(ferroui_textmate::FontStyle::ITALIC)
                        {
                            FontStyle::Italic
                        } else {
                            FontStyle::Normal
                        },
                        if style.font_style.contains(ferroui_textmate::FontStyle::BOLD) {
                            FontWeight::Bold
                        } else {
                            FontWeight::Normal
                        },
                        FontStretch::Normal,
                    );
                    let decorations = match (
                        style
                            .font_style
                            .contains(ferroui_textmate::FontStyle::UNDERLINE),
                        style
                            .font_style
                            .contains(ferroui_textmate::FontStyle::STRIKETHROUGH),
                    ) {
                        (true, true) => {
                            TextDecorationCollection::parse("Underline,Strikethrough").ok()
                        }
                        (true, false) => TextDecorationCollection::parse("Underline").ok(),
                        (false, true) => TextDecorationCollection::parse("Strikethrough").ok(),
                        _ => None,
                    };
                    let properties = GenericTextRunProperties::with_all(
                        typeface,
                        editor.font_size(),
                        decorations,
                        foreground,
                        background,
                        BaselineAlignment::Baseline,
                        None,
                        None,
                    );
                    let start = source.offset() + start.min(source.length());
                    let end = source.offset() + end.min(source.length());
                    if end > start {
                        builder.apply_text_run_properties(start, end, &properties)?;
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
                    builder.apply_text_run_properties(
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
        if point.x < self.gutter_width()
            || point.x >= self.viewport.get().width
            || point.y < 0.0
            || point.y >= self.viewport.get().height
        {
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
                    | CaretMovement::LineStart
                    | CaretMovement::LineEnd
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
            CaretMovement::CharacterRight | CaretMovement::WordRight | CaretMovement::LineEnd
        );
        let word = matches!(movement, CaretMovement::WordLeft | CaretMovement::WordRight);
        let allow_virtual = editor.options().enable_virtual_space || selection.is_rectangular();
        let resolve = |segment: &SelectionSegment| -> Result<(usize, usize), VisualLineError> {
            if !extend
                && !segment.is_empty()
                && !word
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
            let logical = document.get_line_by_offset(segment.caret)?;
            let index = self
                .heights
                .borrow()
                .visible_owner(logical.line_number() - 1);
            let line = self.try_line_layout(index)?;
            if movement == CaretMovement::LineStart {
                return Ok((line.offset, 0));
            }
            if movement == CaretMovement::LineEnd {
                return Ok((line.offset + line.length, 0));
            }
            if segment.caret_virtual_space > 0 {
                return Ok((
                    segment.caret,
                    if forward {
                        if word {
                            0
                        } else {
                            segment.caret_virtual_space.saturating_add(1)
                        }
                    } else if word {
                        0
                    } else {
                        segment.caret_virtual_space - 1
                    },
                ));
            }
            let mut visual = line.display.visual_offset(segment.caret - line.offset) as isize;
            if allow_virtual && !forward && !word && visual == 0 {
                return Ok(segment.caret_tuple());
            }
            loop {
                let next = line.display.get_next_caret_position(
                    visual,
                    if forward {
                        LogicalDirection::Forward
                    } else {
                        LogicalDirection::Backward
                    },
                    if word {
                        CaretPositioningMode::WordStart
                    } else {
                        CaretPositioningMode::Normal
                    },
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
                if offset != segment.caret {
                    return Ok((offset, 0));
                }
                // Zero-document-length adornments do not create an editable
                // document position; advance past them without getting stuck.
                visual = next as isize;
            }
            if forward {
                let next = line.display.last_document_line().line_number() + 1;
                Ok((
                    document
                        .get_line_by_number(next)
                        .map_or(document.text_length(), |line| line.offset()),
                    0,
                ))
            } else if index == 0 {
                Ok((0, 0))
            } else {
                let previous = self.heights.borrow().visible_owner(index - 1);
                let previous = self.try_line_layout(previous)?;
                Ok((previous.offset + previous.length, 0))
            }
        };
        let mut segments = Vec::with_capacity(selection.segments().len());
        for segment in selection.segments() {
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
        let selection = if selection.is_rectangular() {
            Selection::from_rectangle_segments(segments, selection.primary_index())
        } else {
            Selection::from_segments(segments, selection.primary_index())
        };
        if let Err(error) = editor.session().set_selection(selection) {
            editor.report_error(error.to_string());
        }
        true
    }
}
