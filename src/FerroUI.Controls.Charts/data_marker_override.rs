use crate::MarkerShape;
use ferroui_base::media::IBrush;
use std::rc::Rc;

/// Immutable presentation for a marker at one zero-based source sample index.
/// A missing brush uses the series' effective data-marker brush.
pub struct DataMarkerOverride {
    shape: MarkerShape,
    brush: Option<Rc<dyn IBrush>>,
}

impl DataMarkerOverride {
    pub fn new(shape: MarkerShape, brush: Option<Rc<dyn IBrush>>) -> Rc<Self> {
        Rc::new(Self { shape, brush })
    }

    pub fn shape(&self) -> MarkerShape {
        self.shape
    }

    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.brush.clone()
    }
}

impl std::fmt::Debug for DataMarkerOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataMarkerOverride")
            .field("shape", &self.shape)
            .field("has_brush", &self.brush.is_some())
            .finish()
    }
}

// Descriptors are immutable reference objects. Identity equality preserves the
// observable-map replacement semantics when their fields happen to be equal.
impl PartialEq for DataMarkerOverride {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
