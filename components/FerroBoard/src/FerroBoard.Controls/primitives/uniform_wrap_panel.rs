//! Derived from ScreenKeyboard.Uno/Primitives/UniformWrapPanel.cs (Apache-2.0).
use ferroui_base::{
    FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl, ferro_class,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};
use std::cell::Cell;

/// Source uniform-cell panel, including the eight-column unbounded-width default.
/// These are ordinary source properties: changing one does not itself invalidate layout.
#[repr(C)]
pub struct UniformWrapPanel {
    base: Panel,
    item_width: Cell<f64>,
    item_height: Cell<f64>,
    max_columns: Cell<i32>,
}
ferro_class!(UniformWrapPanel:Panel);
ferroui_base::ferro_class_info!(UniformWrapPanel {
    new: UniformWrapPanel::new
});
ferro_impl_classes!(UniformWrapPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl LayoutableImpl for UniformWrapPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let columns = this.columns(available.width);
        let cell_width = if available.width.is_infinite() {
            this.item_width()
        } else {
            available.width / f64::from(columns)
        };
        for child in this.children().snapshot().iter() {
            child.measure(Size::new(cell_width, this.item_height()));
        }
        let rows = (this.children().count() as f64 / f64::from(columns)).ceil() as i32;
        let width = if available.width.is_infinite() {
            f64::from(columns) * this.item_width()
        } else {
            available.width
        };
        Size::new(width, f64::from(rows) * this.item_height())
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let columns = this.columns(final_size.width);
        let cell_width = final_size.width / f64::from(columns);
        let mut i = 0;
        // Source Arrange tests live Count every iteration, unlike Measure's collection enumerator.
        while i < this.children().count() {
            let row = i / columns as usize;
            let col = i % columns as usize;
            this.children().get(i).arrange(Rect::new(
                col as f64 * cell_width,
                row as f64 * this.item_height(),
                cell_width,
                this.item_height(),
            ));
            i += 1;
        }
        let rows = (this.children().count() as f64 / f64::from(columns)).ceil() as i32;
        Size::new(
            final_size.width,
            source_max(final_size.height, f64::from(rows) * this.item_height()),
        )
    }
}
pub(crate) fn source_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || a > b || (a == b && !a.is_sign_negative()) {
        a
    } else {
        b
    }
}
pub(crate) fn source_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}
impl UniformWrapPanel {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            item_width: Cell::new(44.0),
            item_height: Cell::new(44.0),
            max_columns: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn item_width(&self) -> f64 {
        self.item_width.get()
    }
    pub fn set_item_width(&self, v: f64) {
        self.item_width.set(v);
    }
    pub fn item_height(&self) -> f64 {
        self.item_height.get()
    }
    pub fn set_item_height(&self, v: f64) {
        self.item_height.set(v);
    }
    pub fn max_columns(&self) -> i32 {
        self.max_columns.get()
    }
    pub fn set_max_columns(&self, v: i32) {
        self.max_columns.set(v);
    }
    pub(crate) fn columns(&self, width: f64) -> i32 {
        let columns = if width.is_infinite() || width <= 0.0 {
            8
        } else {
            ((width / self.item_width()).floor() as i32).max(1)
        };
        if self.max_columns() > 0 {
            columns.min(self.max_columns())
        } else {
            columns
        }
    }
}
#[cfg(test)]
#[path = "uniform_wrap_panel_tests.rs"]
mod tests;
