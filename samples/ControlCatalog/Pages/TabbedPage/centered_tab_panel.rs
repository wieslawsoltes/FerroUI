//! Port of `Pages/TabbedPage/CenteredTabPanel.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};

/// A custom panel that arranges N children in N+1 equally-sized slots,
/// leaving the middle slot empty. Intended for tab bars that need a central
/// action button overlaid on the gap.
///
/// For N children the split is N/2 (rounded down) items on the left and the
/// rest on the right. Example, 4 tabs: `[0][1][ gap ][2][3]` (5 equal
/// columns, the center is free).
#[repr(C)]
pub struct CenteredTabPanel {
    base: Panel,
}

ferro_class!(CenteredTabPanel: Panel);
ferro_class_info!(CenteredTabPanel { new: CenteredTabPanel::new });
ferro_impl_classes!(
    CenteredTabPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl LayoutableImpl for CenteredTabPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let children = this.children().snapshot();
        let count = children.len();
        if count == 0 {
            return Size::default();
        }

        let slots = (count + 1) as f64;
        let infinite_width = available_size.width.is_infinite();
        let slot_width = if infinite_width { 60.0 } else { available_size.width / slots };
        let mut max_height: f64 = 0.0;

        for child in children.iter() {
            child.measure(Size::new(slot_width, available_size.height));
            // `Math.Max`: a NaN operand gives NaN.
            let height = child.desired_size().height;
            max_height = if max_height.is_nan() || height.is_nan() { f64::NAN } else { max_height.max(height) };
        }

        // When given finite width, fill it. When infinite (inside a ScrollViewer),
        // return a small positive width so the parent allocates real space.
        let desired_width = if infinite_width { slot_width * slots } else { available_size.width };
        if max_height.is_nan() || max_height.is_infinite() {
            max_height = 0.0;
        }

        Size::new(desired_width, max_height)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        let count = children.len();
        if count == 0 {
            return final_size;
        }

        let slots = (count + 1) as f64;
        let slot_w = final_size.width / slots;
        // The items placed to the left of the gap.
        let left_count = count / 2;

        for (i, child) in children.iter().enumerate() {
            // The center slot (`left_count`) is skipped: it is reserved for the action button.
            let slot = if i < left_count { i } else { i + 1 };
            child.arrange(Rect::new(slot as f64 * slot_w, 0.0, slot_w, final_size.height));
        }

        final_size
    }
}

impl CenteredTabPanel {
    pub fn construct() -> Self {
        Self { base: Panel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_controls::Border;

    #[test]
    fn four_children_leave_the_middle_slot_empty() {
        let panel = CenteredTabPanel::new();
        let children: Vec<Ref<Border>> = (0..4).map(|_| Border::new()).collect();
        for child in &children {
            child.set_height(10.0);
            panel.children().add(child.clone());
        }

        panel.measure(Size::new(500.0, 100.0));
        assert_eq!(Size::new(500.0, 10.0), panel.desired_size());
        panel.arrange(Rect::new(0.0, 0.0, 500.0, 10.0));

        let lefts: Vec<f64> = children.iter().map(|child| child.bounds().x).collect();
        assert_eq!(vec![0.0, 100.0, 300.0, 400.0], lefts);
        assert!(children.iter().all(|child| child.bounds().width == 100.0));
    }

    #[test]
    fn an_empty_panel_measures_to_nothing() {
        let panel = CenteredTabPanel::new();
        panel.measure(Size::new(500.0, 100.0));
        assert_eq!(Size::default(), panel.desired_size());
    }

    #[test]
    fn an_infinite_width_gives_slots_of_sixty() {
        let panel = CenteredTabPanel::new();
        for _ in 0..2 {
            panel.children().add(Border::new());
        }
        panel.measure(Size::new(f64::INFINITY, 100.0));
        assert_eq!(180.0, panel.desired_size().width);
    }
}
