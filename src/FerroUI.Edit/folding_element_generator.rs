use crate::TextArea;
use crate::formatted_text_element::FormattedTextElement;
use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line_element::{VisualLineElement, VisualLineElementData};
use crate::visual_line_element_generator::VisualLineElementGenerator;
use crate::visual_line_element_text_run_properties::VisualLineElementTextRunProperties;
use crate::visual_line_error::VisualLineError;
use ferroui_base::WeakRef;
use ferroui_base::input::PointerPressedEventArgs;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::media::{BrushExtensions, Color, IBrush, IPen};
use ferroui_edit_core::document::DocumentError;
use ferroui_edit_core::folding::{FoldingManager, FoldingSection, FoldingSubscription};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

struct FoldState {
    start: usize,
    end: usize,
    title: String,
    section: FoldingSection,
    farthest: usize,
    union_end: usize,
}
enum FoldingSource {
    Manager(Rc<FoldingManager>),
    Sections(Vec<FoldingSection>),
    Snapshot(Rc<[FoldState]>),
}

/// Draws folded regions as native atomic text runs and preserves display-column
/// widths for subsequent tabs. Manager-backed generators also track view lifetime.
pub struct FoldingElementGenerator {
    source: FoldingSource,
    pub text_brush: Rc<dyn IBrush>,
    current: RefCell<Option<Rc<[FoldState]>>>,
    connections: RefCell<Vec<(WeakRef<TextArea>, FoldingSubscription)>>,
}
impl FoldingElementGenerator {
    pub fn new(manager: Rc<FoldingManager>) -> Self {
        Self::from_source(FoldingSource::Manager(manager))
    }
    /// Uses a caller-owned folding snapshot. The caller must invalidate its view
    /// when these sections change and supply sections from the same document.
    pub fn from_sections(sections: Vec<FoldingSection>) -> Self {
        Self::from_source(FoldingSource::Sections(sections))
    }
    pub(crate) fn from_snapshot(sections: Vec<FoldingSection>) -> Self {
        Self::from_source(FoldingSource::Snapshot(fold_snapshot(sections)))
    }
    fn from_source(source: FoldingSource) -> Self {
        Self {
            source,
            text_brush: Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                128, 128, 128,
            ))),
            current: RefCell::new(None),
            connections: RefCell::new(Vec::new()),
        }
    }
}

fn fold_snapshot(sections: impl IntoIterator<Item = FoldingSection>) -> Rc<[FoldState]> {
    let mut states: Vec<_> = sections
        .into_iter()
        .filter(|section| section.is_folded())
        .filter_map(|section| {
            let start = section.start_offset();
            let end = section.end_offset();
            (start < end).then(|| FoldState {
                start,
                end,
                title: section
                    .title()
                    .filter(|title| !title.is_empty())
                    .unwrap_or_else(|| "...".into()),
                section,
                farthest: 0,
                union_end: 0,
            })
        })
        .collect();
    states.sort_by_key(|state| state.start);

    // Keep the first section when several containing sections end together.
    let mut farthest = 0;
    for index in 0..states.len() {
        if states[index].end > states[farthest].end {
            farthest = index;
        }
        states[index].farthest = farthest;
    }
    // A connected next entry already knows the end of its entire union.
    for index in (0..states.len()).rev() {
        let end = states[states[index].farthest].end;
        states[index].union_end = match states.get(index + 1) {
            Some(next) if next.start <= end => next.union_end,
            _ => end,
        };
    }
    states.into()
}

impl ITextViewConnect for FoldingElementGenerator {
    fn add_to_text_view(&self, view: &TextArea) {
        let FoldingSource::Manager(manager) = &self.source else {
            return;
        };
        let handle = view.to_ref();
        let mut connections = self.connections.borrow_mut();
        connections.retain(|(view, _)| view.upgrade().is_some());
        if connections
            .iter()
            .any(|(view, _)| view.upgrade().as_ref() == Some(&handle))
        {
            return;
        }
        let weak = handle.downgrade();
        let listener = weak.clone();
        let subscription = manager.changed(move || {
            if let Some(view) = listener.upgrade() {
                view.invalidate_layout_cache();
            }
        });
        connections.push((weak, subscription));
    }
    fn remove_from_text_view(&self, view: &TextArea) {
        // Disconnect also runs while the view is being dropped. Compare weak
        // identities without attempting to recreate a strong view reference.
        let handle = view.to_weak();
        self.connections
            .borrow_mut()
            .retain(|(view, _)| !view.ptr_eq(&handle) && view.upgrade().is_some());
    }
}
impl VisualLineElementGenerator for FoldingElementGenerator {
    fn start_generation(
        &self,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        let snapshot = match &self.source {
            FoldingSource::Manager(manager) => {
                let version = context
                    .document()
                    .version()
                    .ok_or(DocumentError::DifferentDocument)?;
                let current = manager.document().version();
                if !version.belongs_to_same_document_as(&current) {
                    return Err(DocumentError::DifferentDocument.into());
                }
                if version != current {
                    return Err(DocumentError::InvalidVersion.into());
                }
                fold_snapshot(manager.all_foldings())
            }
            FoldingSource::Sections(sections) => fold_snapshot(sections.iter().cloned()),
            FoldingSource::Snapshot(snapshot) => snapshot.clone(),
        };
        *self.current.borrow_mut() = Some(snapshot);
        Ok(())
    }
    fn finish_generation(&self) {
        self.current.borrow_mut().take();
    }
    fn runs_on_long_lines(&self) -> bool {
        true
    }
    fn get_first_interested_offset(
        &self,
        start: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        let snapshot = self.current.borrow();
        let states = snapshot.as_deref().unwrap_or_default();
        let next = states.partition_point(|state| state.start < start);
        // A preceding generator may consume the original start of a fold.
        // Project its remaining range from the next available source offset.
        if next > 0 && states[states[next - 1].farthest].end > start {
            return Ok(Some(start));
        }
        Ok(states.get(next).map(|state| state.start))
    }
    fn construct_element(
        &self,
        offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        let (title, section, end) = {
            let snapshot = self.current.borrow();
            let states = snapshot.as_deref().unwrap_or_default();
            let Some(index) = states
                .partition_point(|state| state.start <= offset)
                .checked_sub(1)
            else {
                return Ok(None);
            };
            let state = &states[index];
            let primary = &states[state.farthest];
            if primary.end <= offset {
                return Ok(None);
            }
            (
                primary.title.clone(),
                primary.section.clone(),
                state.union_end,
            )
        };
        let mut properties =
            VisualLineElementTextRunProperties::new(&**context.global_text_run_properties());
        properties.set_foreground_brush(Some(self.text_brush.clone()));
        let layout = FormattedTextElement::prepare_text(&title, properties.snapshot());
        let mut element = FormattedTextElement::from_text_layout(
            layout,
            end - offset,
            &**context.global_text_run_properties(),
        )?;
        element.set_display_column_length(title.encode_utf16().count());
        Ok(Some(Box::new(FoldingLineElement {
            element,
            section,
            pen: Rc::new(ImmutablePen::with_brush(
                Some(BrushExtensions::to_immutable(&self.text_brush)),
                1.0,
            )),
        })))
    }
}

pub struct FoldingLineElement {
    element: FormattedTextElement,
    section: FoldingSection,
    pen: Rc<dyn IPen>,
}
impl FoldingLineElement {
    pub fn folding_section(&self) -> &FoldingSection {
        &self.section
    }
}
impl VisualLineElement for FoldingLineElement {
    fn data(&self) -> &VisualLineElementData {
        self.element.data()
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        self.element.data_mut()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn display_column_length(&self) -> usize {
        self.element.display_column_length()
    }
    fn create_text_run(
        &self,
        column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        self.data().checked_visual_index(column)?;
        Ok(Rc::new(self.element.create_formatted_run().with_box(
            0.0,
            None,
            Some(self.pen.clone()),
        )))
    }
    fn on_pointer_pressed(&self, _view: &TextArea, event: &PointerPressedEventArgs) {
        if !event.handled() {
            self.section.set_is_folded(false);
            event.set_handled(true);
        }
    }
}
