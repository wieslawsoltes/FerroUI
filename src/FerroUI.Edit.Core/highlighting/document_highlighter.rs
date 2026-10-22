use super::{
    HighlightedLine, HighlightingColor, HighlightingEngine, HighlightingError, HighlightingState,
    HighlightingStateChangedEventArgs, IHighlighter,
};
use crate::document::{
    DocumentChangeEventArgs, DocumentSubscription, ILineTracker, LiveDocumentLine, TextDocument,
    WeakLineTracker,
};
use crate::utils::Event;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

// The document owns each live line until BeforeRemoveLine removes its cache
// entry. Its Rc identity is therefore stable and cannot be recycled while the
// entry exists. Coordinates remain the responsibility of the document's tree.
fn line_identity(line: &LiveDocumentLine) -> usize {
    Rc::as_ptr(&line.inner) as usize
}

#[derive(Default)]
struct HighlightCache {
    // Sections are relative to their line start, so an earlier edit does not
    // require rewriting the offsets of every later cached section.
    lines: HashMap<usize, HighlightedLine>,
    frontier: Option<LiveDocumentLine>,
    dirty: HashMap<usize, LiveDocumentLine>,
    dirty_order: Option<BTreeSet<usize>>,
    generation: u64,
    #[cfg(test)]
    scanned_lines: usize,
}

impl HighlightCache {
    fn clear(&mut self) {
        self.lines.clear();
        self.frontier = None;
        self.dirty.clear();
        self.dirty_order = None;
        self.generation = self.generation.wrapping_add(1);
    }

    fn frontier_number(&self) -> usize {
        self.frontier
            .as_ref()
            .and_then(|line| line.line_number().ok())
            .unwrap_or(0)
    }

    fn invalidate_line(&mut self, line: &LiveDocumentLine) {
        if line
            .line_number()
            .is_ok_and(|number| number <= self.frontier_number())
        {
            self.dirty.insert(line_identity(line), line.clone());
            self.dirty_order = None;
        }
    }

    fn next_invalid(&mut self, target: usize) -> Option<usize> {
        if self.dirty_order.is_none() {
            self.dirty_order = Some(
                self.dirty
                    .values()
                    .filter_map(|line| line.line_number().ok())
                    .collect(),
            );
        }
        let first_dirty = self
            .dirty_order
            .as_ref()
            .and_then(|order| order.first())
            .copied();
        let first_uncached = self.frontier_number() + 1;
        let next = first_dirty.map_or(first_uncached, |dirty| dirty.min(first_uncached));
        (next <= target).then_some(next)
    }

    fn store(
        &mut self,
        line: &LiveDocumentLine,
        number: usize,
        highlighted: HighlightedLine,
    ) -> bool {
        let key = line_identity(line);
        let changed = self
            .lines
            .get(&key)
            .is_none_or(|previous| previous.end_state != highlighted.end_state);
        self.lines.insert(key, highlighted);
        self.dirty.remove(&key);
        if let Some(order) = self.dirty_order.as_mut() {
            order.remove(&number);
        }
        if number > self.frontier_number() {
            self.frontier = Some(line.clone());
        }
        if changed && number < self.frontier_number() {
            if let Some(next) = line.next_line() {
                self.dirty.insert(line_identity(&next), next);
                if let Some(order) = self.dirty_order.as_mut() {
                    order.insert(number + 1);
                }
            }
        }
        changed
    }
}

#[derive(Default)]
struct CacheTracker {
    cache: RefCell<HighlightCache>,
}

impl ILineTracker for CacheTracker {
    fn before_remove_line(&self, line: &LiveDocumentLine) {
        let mut cache = self.cache.borrow_mut();
        let key = line_identity(line);
        cache.lines.remove(&key);
        cache.dirty.remove(&key);
        cache.dirty_order = None;
        let previous = line.previous_line();
        if cache.frontier.as_ref() == Some(line) {
            cache.frontier = previous.clone();
        }
        // A merge may replace delimiter units with content without changing
        // the survivor's total length, so SetLineLength need not be raised.
        if let Some(previous) = previous {
            cache.invalidate_line(&previous);
        }
        if let Some(next) = line.next_line() {
            cache.invalidate_line(&next);
        }
    }

    fn set_line_length(&self, line: &LiveDocumentLine, _: usize) {
        self.cache.borrow_mut().invalidate_line(line);
    }

    fn line_inserted(&self, previous: &LiveDocumentLine, new_line: &LiveDocumentLine) {
        let mut cache = self.cache.borrow_mut();
        cache.invalidate_line(previous);
        cache.invalidate_line(new_line);
        // Live-line positions of all existing dirty entries may have shifted,
        // even when the new line is beyond the already cached prefix.
        cache.dirty_order = None;
    }

    fn rebuild_document(&self) {
        self.cache.borrow_mut().clear();
    }

    fn change_complete(&self, _: &DocumentChangeEventArgs) {
        // Structural notifications have already invalidated affected entries.
        // Resolve their current order lazily, after the line index is complete.
        self.cache.borrow_mut().dirty_order = None;
    }
}

struct HighlightingOperation<'a>(&'a Cell<bool>);
impl Drop for HighlightingOperation<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// An incremental line highlighter backed by the document's live line index.
///
/// Changed lines are recomputed lazily. State propagation stops at an unchanged
/// end state, retaining every unaffected cached suffix. Line insertion/deletion
/// updates stable identities instead of shifting a vector of cached lines.
pub struct DocumentHighlighter {
    document: TextDocument,
    engine: HighlightingEngine,
    tracker: Rc<CacheTracker>,
    registration: WeakLineTracker,
    initial_span_stack: RefCell<HighlightingState>,
    state_changed: Event<HighlightingStateChangedEventArgs>,
    is_highlighting: Cell<bool>,
    is_in_highlighting_group: Cell<bool>,
    is_disposed: Cell<bool>,
}

impl DocumentHighlighter {
    pub fn new(document: TextDocument, engine: HighlightingEngine) -> Self {
        let tracker = Rc::new(CacheTracker::default());
        let interface: Rc<dyn ILineTracker> = tracker.clone();
        let registration = WeakLineTracker::register(&document, &interface);
        Self {
            document,
            engine,
            tracker,
            registration,
            initial_span_stack: RefCell::new(HighlightingState::default()),
            state_changed: Event::default(),
            is_highlighting: Cell::new(false),
            is_in_highlighting_group: Cell::new(false),
            is_disposed: Cell::new(false),
        }
    }

    /// The shared document whose lines are cached by this highlighter.
    pub fn document(&self) -> &TextDocument {
        &self.document
    }

    pub fn initial_span_stack(&self) -> HighlightingState {
        self.initial_span_stack.borrow().clone()
    }

    pub fn set_initial_span_stack(
        &self,
        state: HighlightingState,
    ) -> Result<(), HighlightingError> {
        self.check_available()?;
        if self.is_highlighting.get() {
            return Err(HighlightingError::ReentrantOperation);
        }
        self.engine.get_color_stack(&state)?;
        *self.initial_span_stack.borrow_mut() = state;
        self.invalidate_highlighting();
        Ok(())
    }

    pub fn highlight_line(&self, line_number: usize) -> Result<HighlightedLine, HighlightingError> {
        self.validate_line(line_number, false)?;
        let _operation = self.begin_operation()?;
        self.highlight_up_to(line_number)?;
        let line = self
            .document
            .get_live_line_by_number(line_number)
            .map_err(document_error)?;
        let mut highlighted = self
            .tracker
            .cache
            .borrow()
            .lines
            .get(&line_identity(&line))
            .cloned()
            .ok_or(HighlightingError::HighlightingInterrupted)?;
        highlighted.line_number = line_number;
        let offset = line.offset().map_err(document_error)?;
        for section in &mut highlighted.sections {
            section.offset += offset;
        }
        Ok(highlighted)
    }

    /// Invalidates cached states and reports the whole document to subscribers.
    /// If called from a highlighting callback, the current operation returns an
    /// interruption error; a later operation starts from the new cache state.
    pub fn invalidate_highlighting(&self) {
        if self.is_disposed.get() {
            return;
        }
        self.tracker.cache.borrow_mut().clear();
        if !self.is_highlighting.get() {
            self.raise_state_changed(1, self.document.line_count());
        }
    }

    /// Compatibility accessor for the opaque parser state. New consumers can
    /// use `get_span_stack` for state or `get_active_colors` for actual colors.
    pub fn get_color_stack(
        &self,
        line_number: usize,
    ) -> Result<HighlightingState, HighlightingError> {
        self.get_span_stack(line_number)
    }

    /// Returns the state at the end of a line; zero denotes the initial state.
    /// Already computed states can be inspected inside a state-change callback.
    pub fn get_span_stack(
        &self,
        line_number: usize,
    ) -> Result<HighlightingState, HighlightingError> {
        self.check_available()?;
        self.validate_line(line_number, true)?;
        if line_number == 0 {
            return Ok(self.initial_span_stack());
        }
        let needs_update = self
            .tracker
            .cache
            .borrow_mut()
            .next_invalid(line_number)
            .is_some();
        if needs_update {
            self.update_highlighting_state(line_number)?;
        }
        let line = self
            .document
            .get_live_line_by_number(line_number)
            .map_err(document_error)?;
        self.tracker
            .cache
            .borrow()
            .lines
            .get(&line_identity(&line))
            .map(|line| line.end_state.clone())
            .ok_or(HighlightingError::HighlightingInterrupted)
    }

    /// Returns active span colors in inside-out order, omitting uncolored spans.
    pub fn get_active_colors(
        &self,
        line_number: usize,
    ) -> Result<Vec<HighlightingColor>, HighlightingError> {
        self.engine
            .get_color_stack(&self.get_span_stack(line_number)?)
    }

    pub fn update_highlighting_state(&self, line_number: usize) -> Result<(), HighlightingError> {
        self.validate_line(line_number, true)?;
        let _operation = self.begin_operation()?;
        self.highlight_up_to(line_number)
    }

    pub fn highlighting_state_changed(
        &self,
        handler: impl Fn(&HighlightingStateChangedEventArgs) + 'static,
    ) -> DocumentSubscription {
        self.state_changed.subscribe(handler)
    }

    pub fn begin_highlighting(&self) -> Result<(), HighlightingError> {
        self.check_available()?;
        if self.is_in_highlighting_group.replace(true) {
            Err(HighlightingError::HighlightingGroupAlreadyOpen)
        } else {
            Ok(())
        }
    }

    pub fn end_highlighting(&self) -> Result<(), HighlightingError> {
        self.check_available()?;
        if self.is_in_highlighting_group.replace(false) {
            Ok(())
        } else {
            Err(HighlightingError::NoHighlightingGroup)
        }
    }

    pub fn get_named_color(&self, name: &str) -> Option<HighlightingColor> {
        self.engine.definition().colors.get(name).cloned()
    }

    pub fn default_text_color(&self) -> Option<HighlightingColor> {
        None
    }

    pub fn dispose(&self) {
        if !self.is_disposed.replace(true) {
            self.is_in_highlighting_group.set(false);
            self.registration.deregister();
        }
    }

    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    fn check_available(&self) -> Result<(), HighlightingError> {
        if self.is_disposed.get() {
            Err(HighlightingError::Disposed)
        } else {
            Ok(())
        }
    }

    fn begin_operation(&self) -> Result<HighlightingOperation<'_>, HighlightingError> {
        self.check_available()?;
        if self.is_highlighting.replace(true) {
            Err(HighlightingError::ReentrantOperation)
        } else {
            Ok(HighlightingOperation(&self.is_highlighting))
        }
    }

    fn validate_line(&self, line: usize, allow_zero: bool) -> Result<(), HighlightingError> {
        if (!allow_zero && line == 0) || line > self.document.line_count() {
            Err(HighlightingError::InvalidDefinition(
                "Line number lies outside the document".into(),
            ))
        } else {
            Ok(())
        }
    }

    fn highlight_up_to(&self, target: usize) -> Result<(), HighlightingError> {
        let version = self.document.version();
        let generation = self.tracker.cache.borrow().generation;
        loop {
            let next = self.tracker.cache.borrow_mut().next_invalid(target);
            let Some(number) = next else { break };
            let line = self
                .document
                .get_live_line_by_number(number)
                .map_err(document_error)?;
            let initial = if number == 1 {
                self.initial_span_stack()
            } else {
                let previous = line
                    .previous_line()
                    .ok_or(HighlightingError::HighlightingInterrupted)?;
                self.tracker
                    .cache
                    .borrow()
                    .lines
                    .get(&line_identity(&previous))
                    .map(|line| line.end_state.clone())
                    .ok_or(HighlightingError::HighlightingInterrupted)?
            };
            let position = line.snapshot().map_err(document_error)?;
            let text = self
                .document
                .get_text(position.offset, position.length)
                .map_err(document_error)?;
            #[cfg(test)]
            {
                self.tracker.cache.borrow_mut().scanned_lines += 1;
            }
            let highlighted = self.engine.highlight_line(&text, number, 0, &initial)?;
            let changed = self
                .tracker
                .cache
                .borrow_mut()
                .store(&line, number, highlighted);
            if changed && number < self.document.line_count() {
                self.raise_state_changed(number + 1, number + 1);
                self.check_available()?;
                if self.document.version() != version
                    || self.tracker.cache.borrow().generation != generation
                {
                    return Err(HighlightingError::HighlightingInterrupted);
                }
            }
        }
        Ok(())
    }

    fn raise_state_changed(&self, from_line_number: usize, to_line_number: usize) {
        self.state_changed
            .raise(&HighlightingStateChangedEventArgs {
                from_line_number,
                to_line_number,
            });
    }
}

fn document_error(error: crate::document::DocumentError) -> HighlightingError {
    HighlightingError::InvalidDefinition(error.to_string())
}

impl IHighlighter for DocumentHighlighter {
    fn document(&self) -> &TextDocument {
        self.document()
    }
    fn get_color_stack(&self, line: usize) -> Result<Vec<HighlightingColor>, HighlightingError> {
        self.get_active_colors(line)
    }
    fn highlight_line(&self, line: usize) -> Result<HighlightedLine, HighlightingError> {
        self.highlight_line(line)
    }
    fn update_highlighting_state(&self, line: usize) -> Result<(), HighlightingError> {
        self.update_highlighting_state(line)
    }
    fn highlighting_state_changed(
        &self,
        handler: Rc<dyn Fn(&HighlightingStateChangedEventArgs)>,
    ) -> DocumentSubscription {
        self.highlighting_state_changed(move |event| handler(event))
    }
    fn begin_highlighting(&self) -> Result<(), HighlightingError> {
        self.begin_highlighting()
    }
    fn end_highlighting(&self) -> Result<(), HighlightingError> {
        self.end_highlighting()
    }
    fn get_named_color(&self, name: &str) -> Option<HighlightingColor> {
        self.get_named_color(name)
    }
    fn default_text_color(&self) -> Option<HighlightingColor> {
        self.default_text_color()
    }
    fn dispose(&self) {
        self.dispose();
    }
}

impl Drop for DocumentHighlighter {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighting::HighlightingDefinition;

    const DEFINITION: &str = r#"<SyntaxDefinition name="CacheTest">
        <Color name="Keyword" foreground="Red" bold="true"/>
        <Color name="Comment" foreground="Green"/>
        <RuleSet>
            <Keywords color="Keyword"><Word>let</Word><Word>class</Word></Keywords>
            <Span color="Comment" begin="/\*" end="\*/" multiline="true">
                <RuleSet><Span foreground="Blue" begin="\[" end="\]" multiline="true"/></RuleSet>
            </Span>
            <Span foreground="Purple" begin="&quot;" end="&quot;" multiline="true"/>
            <Rule foreground="Orange">\b\d+</Rule>
        </RuleSet>
    </SyntaxDefinition>"#;

    fn engine() -> HighlightingEngine {
        HighlightingEngine::new(Rc::new(HighlightingDefinition::load(DEFINITION).unwrap()))
    }

    fn assert_matches_fresh(highlighter: &DocumentHighlighter) {
        let mut state = highlighter.initial_span_stack();
        for number in 1..=highlighter.document().line_count() {
            let line = highlighter.document().get_line_by_number(number).unwrap();
            let text = highlighter
                .document()
                .get_text(line.offset, line.length)
                .unwrap();
            let expected = highlighter
                .engine
                .highlight_line(&text, number, line.offset, &state)
                .unwrap();
            state = expected.end_state.clone();
            let actual = highlighter.highlight_line(number).unwrap();
            assert_eq!(
                actual,
                expected,
                "fresh highlighting mismatch at line {number}, document {:?}",
                highlighter.document().text()
            );
        }
    }

    fn scanned(highlighter: &DocumentHighlighter) -> usize {
        highlighter.tracker.cache.borrow().scanned_lines
    }

    #[test]
    fn equal_state_reuses_the_entire_suffix_and_projects_current_utf16_offsets() {
        let document = TextDocument::from_text(&"let 😀 = 12;\n".repeat(1000));
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        let last_number = document.line_count() - 1;
        let old = highlighter.highlight_line(last_number).unwrap();
        let before = scanned(&highlighter);
        document.replace(0, 3, "class").unwrap();
        let new = highlighter.highlight_line(last_number).unwrap();
        assert_eq!(scanned(&highlighter) - before, 1);
        assert_eq!(new.sections.len(), old.sections.len());
        for (new, old) in new.sections.iter().zip(&old.sections) {
            assert_eq!(new.offset, old.offset + 2);
            assert_eq!(new.length, old.length);
            assert_eq!(new.color, old.color);
        }
        assert_matches_fresh(&highlighter);
    }

    #[test]
    fn inserted_and_removed_lines_reuse_stable_suffix_identities() {
        let document = TextDocument::from_text(&"let a = 1;\n".repeat(500));
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        highlighter.highlight_line(document.line_count()).unwrap();
        let before = scanned(&highlighter);
        document.insert(0, "new 😀\r\n").unwrap();
        highlighter.highlight_line(document.line_count()).unwrap();
        assert!(scanned(&highlighter) - before <= 3);
        assert_matches_fresh(&highlighter);
        let before = scanned(&highlighter);
        document
            .remove(0, "new 😀\r\n".encode_utf16().count())
            .unwrap();
        highlighter.highlight_line(document.line_count()).unwrap();
        assert!(scanned(&highlighter) - before <= 2);
        assert_matches_fresh(&highlighter);
    }

    #[test]
    fn lazy_frontier_ignores_edits_in_the_uncached_suffix() {
        let document = TextDocument::from_text(&"let a = 1;\n".repeat(100));
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        highlighter.highlight_line(10).unwrap();
        assert_eq!(highlighter.tracker.cache.borrow().lines.len(), 10);
        let before = scanned(&highlighter);
        document
            .insert(document.get_line_by_number(80).unwrap().offset, "/*")
            .unwrap();
        highlighter.highlight_line(10).unwrap();
        assert_eq!(scanned(&highlighter), before);
        highlighter.highlight_line(90).unwrap();
        assert_matches_fresh(&highlighter);
    }

    #[test]
    fn pending_independent_edits_are_not_lost_when_an_earlier_state_converges() {
        let document = TextDocument::from_text("let a\nlet b\nlet c\nlet d\nlet e\nlet f\nlet g");
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        highlighter.highlight_line(7).unwrap();
        let before = scanned(&highlighter);
        document.insert(0, "/* ").unwrap();
        document
            .insert(document.get_line_by_number(5).unwrap().offset, "*/ ")
            .unwrap();
        document
            .replace(document.get_line_by_number(7).unwrap().offset, 3, "class")
            .unwrap();
        highlighter.highlight_line(7).unwrap();
        assert_eq!(scanned(&highlighter) - before, 6);
        assert_matches_fresh(&highlighter);
    }

    #[test]
    fn state_events_stop_at_convergence_and_can_be_explicitly_unsubscribed() {
        let document = TextDocument::from_text("let a\nlet b\n*/ let c\nlet d\nlet e");
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        highlighter.highlight_line(5).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let output = events.clone();
        let subscription =
            highlighter.highlighting_state_changed(move |event| output.borrow_mut().push(*event));
        document.insert(0, "/* ").unwrap();
        highlighter.highlight_line(5).unwrap();
        assert_eq!(
            &*events.borrow(),
            &[
                HighlightingStateChangedEventArgs {
                    from_line_number: 2,
                    to_line_number: 2
                },
                HighlightingStateChangedEventArgs {
                    from_line_number: 3,
                    to_line_number: 3
                },
            ]
        );
        events.borrow_mut().clear();
        document
            .insert(document.get_line_by_number(2).unwrap().offset, "inside ")
            .unwrap();
        highlighter.highlight_line(5).unwrap();
        assert!(events.borrow().is_empty());
        highlighter.invalidate_highlighting();
        assert_eq!(
            &*events.borrow(),
            &[HighlightingStateChangedEventArgs {
                from_line_number: 1,
                to_line_number: 5
            }]
        );
        subscription.dispose();
        highlighter.invalidate_highlighting();
        assert_eq!(events.borrow().len(), 1);
    }

    #[test]
    fn callback_can_read_computed_state_but_cannot_reenter_highlighting() {
        let highlighter = Rc::new(DocumentHighlighter::new(
            TextDocument::from_text("/*\ninside\n*/"),
            engine(),
        ));
        let weak = Rc::downgrade(&highlighter);
        let observations = Rc::new(Cell::new(0));
        let count = observations.clone();
        highlighter.highlighting_state_changed(move |event| {
            let highlighter = weak.upgrade().unwrap();
            assert!(highlighter
                .get_span_stack(event.from_line_number - 1)
                .is_ok());
            assert!(matches!(
                highlighter.highlight_line(1),
                Err(HighlightingError::ReentrantOperation)
            ));
            if event.from_line_number == 2 {
                assert!(matches!(
                    highlighter.get_span_stack(3),
                    Err(HighlightingError::ReentrantOperation)
                ));
            }
            count.set(count.get() + 1);
        });
        highlighter.highlight_line(3).unwrap();
        assert_eq!(observations.get(), 2);
        assert_matches_fresh(&highlighter);
    }

    #[test]
    fn callback_document_edits_interrupt_cleanly_and_the_next_call_recovers() {
        let document = TextDocument::from_text("let a\nlet b\nlet c");
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        let edited = Rc::new(Cell::new(false));
        let once = edited.clone();
        let observed_document = document.clone();
        highlighter.highlighting_state_changed(move |_| {
            if !once.replace(true) {
                observed_document.insert(0, "/*").unwrap();
            }
        });
        assert!(matches!(
            highlighter.highlight_line(3),
            Err(HighlightingError::HighlightingInterrupted)
        ));
        assert!(edited.get());
        assert_matches_fresh(&highlighter);
    }

    #[test]
    fn callback_invalidation_does_not_borrow_panic_or_return_stale_sections() {
        let highlighter = Rc::new(DocumentHighlighter::new(
            TextDocument::from_text("let a\nlet b"),
            engine(),
        ));
        let weak = Rc::downgrade(&highlighter);
        let once = Rc::new(Cell::new(false));
        highlighter.highlighting_state_changed(move |_| {
            if !once.replace(true) {
                weak.upgrade().unwrap().invalidate_highlighting();
            }
        });
        assert!(matches!(
            highlighter.highlight_line(2),
            Err(HighlightingError::HighlightingInterrupted)
        ));
        assert_matches_fresh(&highlighter);
    }

    #[test]
    fn interface_exposes_actual_colors_named_colors_groups_and_disposal() {
        let highlighter =
            DocumentHighlighter::new(TextDocument::from_text("/* [\ninside\n] */"), engine());
        let interface: &dyn IHighlighter = &highlighter;
        assert!(matches!(
            interface.end_highlighting(),
            Err(HighlightingError::NoHighlightingGroup)
        ));
        interface.begin_highlighting().unwrap();
        assert!(matches!(
            interface.begin_highlighting(),
            Err(HighlightingError::HighlightingGroupAlreadyOpen)
        ));
        interface.update_highlighting_state(1).unwrap();
        let colors = interface.get_color_stack(1).unwrap();
        assert_eq!(
            colors
                .iter()
                .map(|color| color.foreground.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("Blue"), Some("Green")]
        );
        assert!(interface.get_color_stack(0).unwrap().is_empty());
        assert_eq!(
            interface
                .get_named_color("Keyword")
                .unwrap()
                .foreground
                .as_deref(),
            Some("Red")
        );
        assert!(interface.get_named_color("missing").is_none());
        assert!(interface.default_text_color().is_none());
        assert_eq!(interface.document(), highlighter.document());
        interface.end_highlighting().unwrap();
        interface.begin_highlighting().unwrap();
        interface.dispose();
        interface.dispose();
        assert!(highlighter.is_disposed());
        assert!(!highlighter.registration.is_registered());
        assert!(matches!(
            interface.highlight_line(1),
            Err(HighlightingError::Disposed)
        ));
        assert!(matches!(
            interface.end_highlighting(),
            Err(HighlightingError::Disposed)
        ));
    }

    #[test]
    fn initial_span_stack_is_used_for_line_zero_and_reinvalidates_cached_lines() {
        let engine = engine();
        let initial = engine
            .highlight_line("/* [", 1, 0, &HighlightingState::default())
            .unwrap()
            .end_state;
        let highlighter =
            DocumentHighlighter::new(TextDocument::from_text("inside\n] */ let x\nlet y"), engine);
        highlighter.highlight_line(3).unwrap();
        highlighter.set_initial_span_stack(initial.clone()).unwrap();
        assert_eq!(highlighter.get_span_stack(0).unwrap(), initial);
        assert_eq!(highlighter.get_active_colors(0).unwrap().len(), 2);
        assert_eq!(highlighter.get_color_stack(1).unwrap(), initial);
        assert_matches_fresh(&highlighter);
        highlighter
            .set_initial_span_stack(HighlightingState::default())
            .unwrap();
        assert!(highlighter.highlight_line(1).unwrap().sections.is_empty());
    }

    #[test]
    fn invalid_initial_stack_and_engine_failure_leave_the_highlighter_usable() {
        let engine = engine();
        let initial = engine
            .highlight_line("/* [", 1, 0, &HighlightingState::default())
            .unwrap()
            .end_state;
        let empty = HighlightingEngine::new(Rc::new(
            HighlightingDefinition::load("<SyntaxDefinition><RuleSet/></SyntaxDefinition>")
                .unwrap(),
        ));
        let highlighter = DocumentHighlighter::new(TextDocument::from_text("x"), empty);
        assert!(matches!(
            highlighter.set_initial_span_stack(initial),
            Err(HighlightingError::InvalidState)
        ));
        assert_eq!(
            highlighter.initial_span_stack(),
            HighlightingState::default()
        );
        highlighter.highlight_line(1).unwrap();
        let mut highlighter =
            DocumentHighlighter::new(TextDocument::from_text("let x\nlet y"), engine);
        highlighter.engine.operation_limit = 0;
        assert!(matches!(
            highlighter.highlight_line(2),
            Err(HighlightingError::MatchBudget(_))
        ));
        highlighter.engine.operation_limit = 100_000;
        assert_matches_fresh(&highlighter);
        assert!(highlighter.highlight_line(0).is_err());
        assert!(highlighter.get_span_stack(3).is_err());
        highlighter.update_highlighting_state(0).unwrap();
    }

    #[test]
    fn line_tracker_registration_does_not_retain_a_dropped_highlighter() {
        let document = TextDocument::from_text("let x\nlet y");
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        highlighter.highlight_line(2).unwrap();
        let tracker = Rc::downgrade(&highlighter.tracker);
        drop(highlighter);
        assert!(tracker.upgrade().is_none());
        document.insert(0, "/*\n").unwrap();
        document.remove(0, 3).unwrap();
    }

    #[test]
    fn crlf_splits_merges_and_surrogate_edits_match_fresh_highlighting() {
        let engine = engine();
        let original: Vec<u16> = "let 😀\r\n/* [a]\n b */\rlet z\n".encode_utf16().collect();
        let insertions: Vec<Vec<u16>> = vec!["\r", "\n", "\r\n", "/*", "*/", "😀"]
            .into_iter()
            .map(|text| text.encode_utf16().collect())
            .chain([vec![0xd800]])
            .collect();
        for offset in 0..=original.len() {
            for insertion in &insertions {
                let document = TextDocument::from_utf16(&original);
                let highlighter = DocumentHighlighter::new(document.clone(), engine.clone());
                highlighter.highlight_line(document.line_count()).unwrap();
                document.replace_utf16(offset, 0, insertion).unwrap();
                highlighter.highlight_line(document.line_count()).unwrap();
                assert_matches_fresh(&highlighter);
                document.undo().unwrap();
                assert_matches_fresh(&highlighter);
            }
            if offset < original.len() {
                let document = TextDocument::from_utf16(&original);
                let highlighter = DocumentHighlighter::new(document.clone(), engine.clone());
                highlighter.highlight_line(document.line_count()).unwrap();
                document
                    .remove(offset, (original.len() - offset).min(2))
                    .unwrap();
                assert_matches_fresh(&highlighter);
            }
        }
    }

    #[test]
    fn randomized_queued_edits_and_history_match_a_fresh_engine() {
        let document = TextDocument::from_text("let a\r\n/* [one\ntwo] */\rlet 😀 = 42;\n");
        let highlighter = DocumentHighlighter::new(document.clone(), engine());
        highlighter.highlight_line(document.line_count()).unwrap();
        let atoms: Vec<Vec<u16>> = [
            "x", "/*", "*/", "[", "]", "\"", "\r", "\n", "\r\n", "😀", "let", "",
        ]
        .into_iter()
        .map(|text| text.encode_utf16().collect())
        .chain([vec![0xd800], vec![0xdc00]])
        .collect();
        let mut seed = 0x14ab_329f_21d5_896bu64;
        let mut random = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as usize
        };
        for batch in 0..180 {
            for _ in 0..3 {
                let offset = random() % (document.text_length() + 1);
                let length = random() % ((document.text_length() - offset).min(6) + 1);
                let insertion = &atoms[random() % atoms.len()];
                document.replace_utf16(offset, length, insertion).unwrap();
            }
            let request = 1 + random() % document.line_count();
            highlighter.highlight_line(request).unwrap();
            assert_matches_fresh(&highlighter);
            if batch % 9 == 0 {
                document.undo().unwrap();
                assert_matches_fresh(&highlighter);
                document.redo().unwrap();
                assert_matches_fresh(&highlighter);
            }
        }
    }
}
