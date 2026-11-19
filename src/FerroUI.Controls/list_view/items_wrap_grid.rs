//! WinUI's `ItemsWrapGrid` (`dxaml/xcp/dxaml/lib/ItemsWrapGrid_Partial.cpp` and the metadata of
//! `Microsoft.UI.Xaml.Controls.cs` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), on FerroUI's `WrapPanel` (docs/porting/winui-calculator-gaps.md, LV-2, decision D5).

use crate::{ControlImpl, PanelImpl, WrapPanel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, VisualImpl,
};

/// The items panel of a grid view (WinUI's `ItemsWrapGrid`): the items in cells of one size, in
/// rows (`Orientation` horizontal) or columns, wrapping at the edge of the panel or after
/// [`maximum_rows_or_columns`](Self::maximum_rows_or_columns) items. The cell is `ItemWidth` by
/// `ItemHeight`, or the size of the first item where they are not set, as in WinUI.
///
/// Recorded deviation (decision D5): it derives from FerroUI's [`WrapPanel`] and does not
/// virtualize; WinUI's panel is a modern collection panel that realizes the visible items only.
/// The grouping members (`GroupPadding`, `GroupHeaderPlacement`, `AreStickyGroupHeadersEnabled`)
/// are not ported: FerroUI's items controls have no groups.
#[repr(C)]
pub struct ItemsWrapGrid {
    base: WrapPanel,
}

ferro_class!(ItemsWrapGrid: WrapPanel);
ferroui_base::ferro_class_info!(ItemsWrapGrid {
    new: ItemsWrapGrid::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(
    ItemsWrapGrid: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    FerroObjectImpl
);

ferroui_base::ferro_properties! { impl ItemsWrapGrid {
    ferro_property!(
        /// Defines the `MaximumRowsOrColumns` property: the items of a row (or a column) at most;
        /// -1 (WinUI's default) wraps at the edge only.
        pub fn maximum_rows_or_columns_property() -> StyledProperty<i32> {
            FerroProperty::register::<ItemsWrapGrid, _>("MaximumRowsOrColumns", -1)
        }
    );
} }

impl ItemsWrapGrid {
    fn static_constructor() {
        Layoutable::affects_measure::<ItemsWrapGrid>(&[Self::maximum_rows_or_columns_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: WrapPanel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn maximum_rows_or_columns(&self) -> i32 {
        self.get_value(Self::maximum_rows_or_columns_property())
    }

    pub fn set_maximum_rows_or_columns(&self, value: i32) {
        self.set_value(Self::maximum_rows_or_columns_property(), value)
    }

    /// The size of a cell: `ItemWidth` by `ItemHeight`, the size of the first item where unset.
    fn cell_size(&self, available: Size) -> Size {
        let children = self.children().snapshot();
        let first = children.iter().find(|child| child.is_visible());
        let mut width = self.item_width();
        let mut height = self.item_height();
        if width.is_nan() || height.is_nan() {
            if let Some(first) = first {
                first.measure(available);
                let desired = first.desired_size();
                if width.is_nan() {
                    width = desired.width;
                }
                if height.is_nan() {
                    height = desired.height;
                }
            }
        }
        Size::new(if width.is_nan() { 0.0 } else { width }, if height.is_nan() { 0.0 } else { height })
    }

    /// The cells of a line: as many as fit, at most `MaximumRowsOrColumns`, at least one.
    fn per_line(&self, extent: f64, cell: f64) -> usize {
        let fit = if cell > 0.0 && extent.is_finite() { (extent / cell).floor() as usize } else { usize::MAX };
        let maximum = self.maximum_rows_or_columns();
        let limited = if maximum > 0 { fit.min(maximum as usize) } else { fit };
        limited.max(1)
    }
}

impl LayoutableImpl for ItemsWrapGrid {
    /// The holds of the wrap panel follow its own lines; the cells of this panel hold nothing.
    fn measure_hold_override(_this: &Self, _constraint: Size) -> Option<ferroui_base::layout::MeasureHold> {
        None
    }

    fn measure_override(this: &Self, available_size: Size) -> Size {
        let cell = this.cell_size(available_size);
        let children = this.children().snapshot();
        let count = children.iter().filter(|child| child.is_visible()).count();
        for child in children.iter() {
            child.measure(cell);
        }
        if count == 0 {
            return Size::default();
        }
        let horizontal = this.orientation() == Orientation::Horizontal;
        let per_line = if horizontal {
            this.per_line(available_size.width, cell.width)
        } else {
            this.per_line(available_size.height, cell.height)
        };
        let in_line = per_line.min(count);
        let lines = count.div_ceil(per_line);
        if horizontal {
            Size::new(in_line as f64 * cell.width, lines as f64 * cell.height)
        } else {
            Size::new(lines as f64 * cell.width, in_line as f64 * cell.height)
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let cell = this.cell_size(final_size);
        let horizontal = this.orientation() == Orientation::Horizontal;
        let per_line = if horizontal {
            this.per_line(final_size.width, cell.width)
        } else {
            this.per_line(final_size.height, cell.height)
        };
        let mut position = 0;
        for child in this.children().snapshot().iter() {
            if !child.is_visible() {
                continue;
            }
            let (line, slot) = (position / per_line, position % per_line);
            let rect = if horizontal {
                Rect::new(slot as f64 * cell.width, line as f64 * cell.height, cell.width, cell.height)
            } else {
                Rect::new(line as f64 * cell.width, slot as f64 * cell.height, cell.width, cell.height)
            };
            child.arrange(rect);
            position += 1;
        }
        final_size
    }
}
