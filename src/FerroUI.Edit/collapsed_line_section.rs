use crate::TextArea;
use ferroui_base::WeakRef;
use ferroui_edit_core::document::{
    DocumentChangeEventArgs, DocumentSubscription, ILineTracker, LiveDocumentLine, TextDocument,
};
use std::cell::RefCell;
use std::rc::Rc;

type Endpoints = (LiveDocumentLine, LiveDocumentLine);

/// An inclusive collapsed range owned by a text area. Cloned handles refer to
/// the same range. Call `uncollapse` to restore it, even if other handles exist.
#[derive(Clone)]
pub struct CollapsedLineSection {
    area: WeakRef<TextArea>,
    endpoints: Rc<RefCell<Option<Endpoints>>>,
}

impl CollapsedLineSection {
    pub fn is_collapsed(&self) -> bool {
        self.endpoints.borrow().is_some()
    }

    pub fn start(&self) -> Option<LiveDocumentLine> {
        self.endpoints
            .borrow()
            .as_ref()
            .map(|(line, _)| line.clone())
    }

    pub fn end(&self) -> Option<LiveDocumentLine> {
        self.endpoints
            .borrow()
            .as_ref()
            .map(|(_, line)| line.clone())
    }

    pub fn uncollapse(&self) {
        if self.endpoints.take().is_some() {
            if let Some(area) = self.area.upgrade() {
                area.refresh_explicit_line_collapses();
            }
        }
    }
}

#[derive(Default)]
struct CollapseTracker {
    sections: RefCell<Vec<CollapsedLineSection>>,
}

impl CollapseTracker {
    fn clear(&self) {
        for section in self.sections.take() {
            section.endpoints.take();
        }
    }
}

impl ILineTracker for CollapseTracker {
    fn before_remove_line(&self, line: &LiveDocumentLine) {
        self.sections.borrow_mut().retain(|section| {
            let mut endpoints = section.endpoints.borrow_mut();
            let Some((first, last)) = endpoints.as_mut() else {
                return false;
            };
            match (first == line, last == line) {
                (true, true) => *endpoints = None,
                (true, false) => {
                    if let Some(next) = line.next_line() {
                        *first = next;
                    } else {
                        *endpoints = None;
                    }
                }
                (false, true) => {
                    if let Some(previous) = line.previous_line() {
                        *last = previous;
                    } else {
                        *endpoints = None;
                    }
                }
                (false, false) => {}
            }
            endpoints.is_some()
        });
    }

    fn set_line_length(&self, _: &LiveDocumentLine, _: usize) {}
    fn line_inserted(&self, _: &LiveDocumentLine, _: &LiveDocumentLine) {}
    fn rebuild_document(&self) {
        self.clear();
    }
    fn change_complete(&self, _: &DocumentChangeEventArgs) {}
}

pub(crate) struct CollapsedLineSections {
    pub(crate) document: TextDocument,
    tracker: Rc<CollapseTracker>,
    _registration: DocumentSubscription,
}

impl CollapsedLineSections {
    pub(crate) fn new(document: TextDocument) -> Self {
        let tracker = Rc::new(CollapseTracker::default());
        let registration = document.add_line_tracker(tracker.clone());
        Self {
            document,
            tracker,
            _registration: registration,
        }
    }

    pub(crate) fn collapse(
        &self,
        area: WeakRef<TextArea>,
        endpoints: Endpoints,
    ) -> CollapsedLineSection {
        let section = CollapsedLineSection {
            area,
            endpoints: Rc::new(RefCell::new(Some(endpoints))),
        };
        self.tracker.sections.borrow_mut().push(section.clone());
        section
    }

    pub(crate) fn extend_intervals(&self, intervals: &mut Vec<(usize, usize)>) {
        self.tracker.sections.borrow_mut().retain(|section| {
            let endpoints = section.endpoints.borrow();
            let Some((first, last)) = endpoints.as_ref() else {
                return false;
            };
            if let (Ok(first), Ok(last)) = (first.line_number(), last.line_number()) {
                intervals.push((first - 1, last));
            }
            true
        });
    }
}

impl Drop for CollapsedLineSections {
    fn drop(&mut self) {
        self.tracker.clear();
    }
}
