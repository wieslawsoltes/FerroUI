//! WinUI's `PivotHeaderPanel` (`dxaml/phone/lib/PivotHeaderPanel_Partial.cpp` of microsoft-ui-xaml
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::pivot_common::positive_mod;
use super::Pivot;
use crate::{Canvas, CanvasImpl, ControlImpl, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl,
    WeakRef,
};
use std::cell::{Cell, RefCell};

/// WinUI's `PivotHeaderDesiredSizeRoundingTolerance`.
const DESIRED_SIZE_ROUNDING_TOLERANCE: f64 = 0.001;

/// The panel of the headers of a [`Pivot`] (WinUI's `PivotHeaderPanel`, a `Canvas`): the headers
/// side by side, starting at the header of the selected item at the current offset, so that the
/// selected header comes first and the headers before it follow at the end. The pivot moves the
/// panel with the parallax of its scroll viewer.
#[repr(C)]
pub struct PivotHeaderPanel {
    base: Canvas,
    pivot: RefCell<Option<WeakRef<Pivot>>>,
    current_offset: Cell<f64>,
    current_index: Cell<usize>,
    clipped_desired_width: Cell<f64>,
    unclipped_desired_width: Cell<f64>,
}

ferro_class!(PivotHeaderPanel: Canvas);
ferroui_base::ferro_class_info!(PivotHeaderPanel {
    new: PivotHeaderPanel::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(
    PivotHeaderPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl,
    FerroObjectImpl
);

impl LayoutableImpl for PivotHeaderPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut desired = Size::default();
        for child in this.children().snapshot().iter() {
            child.measure(available_size);
            let size = child.desired_size();
            desired = Size::new(desired.width + size.width, desired.height.max(size.height));
        }
        this.unclipped_desired_width.set(desired.width);
        this.clipped_desired_width.set(available_size.width.min(desired.width));
        desired
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(pivot) = this.pivot() {
            pivot.on_header_panel_measure(this.clipped_desired_width.get() as f32);
        }
        let children = this.children().snapshot();
        let count = children.len();
        let mut current_offset = this.current_offset.get();
        let mut current_index = this.current_index.get();
        let mut ghost_offset = this.current_offset.get();
        let mut ghost_vertical_offset = 0.0;
        for child_index in 0..count {
            let index = positive_mod(current_index as i32, count as i32) as usize;
            current_index += 1;
            let child = &children[index];
            let desired = child.desired_size();
            if child_index == count - 1 {
                // The last header is also shown before the first (WinUI's ghost item).
                let margin = child.margin();
                ghost_offset += -desired.width + margin.left;
                ghost_vertical_offset = margin.top;
            }
            let height = final_size.height.max(desired.height);
            // WinUI also writes the position into `Canvas.Left` and `Canvas.Top` of the header
            // (`SetCanvasTopLeftOnChild`); the port arranges it there (the attached properties
            // would arrange the panel again).
            child.arrange(Rect::new(current_offset, 0.0, desired.width, height));
            current_offset += desired.width;
        }
        if let Some(pivot) = this.pivot() {
            pivot.on_header_panel_ghost_offset(this, ghost_offset, ghost_vertical_offset);
        }
        final_size
    }
}

impl PivotHeaderPanel {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Canvas::construct(),
            pivot: RefCell::new(None),
            current_offset: Cell::new(0.0),
            current_index: Cell::new(0),
            clipped_desired_width: Cell::new(0.0),
            unclipped_desired_width: Cell::new(0.0),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn pivot(&self) -> Option<Ref<Pivot>> {
        self.pivot.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// WinUI's `SetHeaderManagerCallbacks`.
    pub(crate) fn set_pivot(&self, pivot: Option<&Pivot>) {
        *self.pivot.borrow_mut() = pivot.map(|pivot| pivot.to_ref().downgrade());
    }

    /// The header arranged first (WinUI's `SetCurrentIndex`).
    pub(crate) fn set_current_index(&self, index: usize) {
        if self.current_index.replace(index) != index {
            self.invalidate_arrange();
        }
    }

    /// Where the first header is arranged (WinUI's `SetCurrentOffset`).
    pub(crate) fn set_current_offset(&self, offset: f64) {
        if self.current_offset.replace(offset) != offset {
            self.invalidate_arrange();
        }
    }

    /// Whether the headers are wider than the room they have (WinUI's `IsContentClipped`).
    pub fn is_content_clipped(&self) -> bool {
        let unclipped = self.unclipped_desired_width.get();
        let clipped = self.clipped_desired_width.get();
        unclipped > clipped && (unclipped - clipped).abs() > DESIRED_SIZE_ROUNDING_TOLERANCE
    }
}
