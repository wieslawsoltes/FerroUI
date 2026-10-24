use ferroui_base::media::IBrush;
use std::rc::Rc;

/// Immutable presentation for a label at one zero-based source sample index.
/// A missing brush uses the series' effective data-label brush.
pub struct DataLabelOverride {
    text: String,
    brush: Option<Rc<dyn IBrush>>,
}

impl DataLabelOverride {
    pub fn new(text: impl Into<String>, brush: Option<Rc<dyn IBrush>>) -> Rc<Self> {
        Rc::new(Self {
            text: text.into(),
            brush,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.brush.clone()
    }
}

impl std::fmt::Debug for DataLabelOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataLabelOverride")
            .field("text", &self.text)
            .field("has_brush", &self.brush.is_some())
            .finish()
    }
}

// Descriptors are immutable reference objects. Identity equality preserves the
// observable-map replacement semantics when their fields happen to be equal.
impl PartialEq for DataLabelOverride {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
