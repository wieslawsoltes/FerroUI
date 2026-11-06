//! WinUI StackPanel padding/layout adapter for source programmatic and compiled views.
//! Geometry derives from Uno da217df2 StackPanel.mux.cs (Microsoft, MIT) and
//! Border.Helpers.cs / LayoutHelper.cs (Uno, Apache-2.0). See reference/uno-layout.
use super::uniform_wrap_panel::source_max;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl, StyledProperty, Thickness,
    VisualImpl, ferro_class, ferro_impl_classes, ferro_property,
    input::InputElementImpl,
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventArgs},
    layout::{LayoutHelper, Layoutable, LayoutableImpl, Orientation},
};
use ferroui_controls::{ControlImpl, PanelImpl, StackPanel, StackPanelImpl};

/// Additional native adapter: retains StackPanel inheritance and Children identity.
/// Implements source zero-border geometry. Source border drawing, insertion and
/// scroll snap contracts are not established by this adapter; native interfaces remain inherited.
#[repr(C)]
pub struct PaddedStackPanel {
    base: StackPanel,
}
ferro_class!(PaddedStackPanel:StackPanel);
ferroui_base::ferro_class_info!(PaddedStackPanel {
    new: PaddedStackPanel::new
});
ferro_impl_classes!(PaddedStackPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
ferroui_base::ferro_properties! {impl PaddedStackPanel {
    ferro_property!(pub fn padding_property() -> StyledProperty<Thickness> {
        FerroProperty::register::<PaddedStackPanel, _>("Padding", Thickness::default())
    });
}}
impl PaddedStackPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<PaddedStackPanel>(&[Self::padding_property().as_property()]);
    }
    pub fn construct() -> Self {
        Self {
            base: StackPanel::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn padding(&self) -> Thickness {
        self.get_value(Self::padding_property())
    }
    pub fn set_padding(&self, value: Thickness) {
        self.set_value(Self::padding_property(), value);
    }
    fn inner_rect(&self, size: Size) -> Rect {
        let mut rect = Rect::from_size(size);
        if self.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(self);
            rect.width = LayoutHelper::round_layout_value(rect.width, scale);
            rect.height = LayoutHelper::round_layout_value(rect.height, scale);
        }
        // Source deflates the zero border first, then padding. Keep the operation order.
        deflate(deflate(rect, Thickness::default()), self.padding())
    }
}
fn deflate(rect: Rect, padding: Thickness) -> Rect {
    Rect::new(
        rect.x - (-padding.left),
        rect.y - (-padding.top),
        source_max((-padding.left + rect.width) + (-padding.right), 0.0),
        source_max((-padding.top + rect.height) + (-padding.bottom), 0.0),
    )
}
impl LayoutableImpl for PaddedStackPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let mut desired = Size::default();
        let padding = this.padding();
        let combined = Size::new(
            0.0 + (padding.left + padding.right),
            0.0 + (padding.top + padding.bottom),
        );
        let constraint = if this.orientation() == Orientation::Vertical {
            Size::new(available.width - combined.width, f64::INFINITY)
        } else {
            Size::new(f64::INFINITY, available.height - combined.height)
        };
        let count = this.children().count();
        let mut visible_count = 0;
        // Source captures Count, but retrieves live children by index. Callbacks may mutate them.
        for i in 0..count {
            let child = this.children().get(i);
            child.measure(constraint);
            let size = child.desired_size();
            if this.orientation() == Orientation::Vertical {
                desired.width = source_max(desired.width, size.width);
                desired.height += size.height;
            } else {
                desired.width += size.width;
                desired.height = source_max(desired.height, size.height);
            }
            if child.is_visible() {
                visible_count += 1;
            }
        }
        desired.width += combined.width;
        desired.height += combined.height;
        if visible_count > 1 {
            let spacing = this.spacing() * f64::from(visible_count - 1);
            if this.orientation() == Orientation::Vertical {
                desired.height += spacing;
            } else {
                desired.width += spacing;
            }
        }
        desired
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut rect = this.inner_rect(final_size);
        let spacing = this.spacing();
        let count = this.children().count();
        for i in 0..count {
            let child = this.children().get(i);
            let desired = child.desired_size();
            let orientation = this.orientation();
            if orientation == Orientation::Vertical {
                rect.height = desired.height;
                rect.width = source_max(rect.width, desired.width);
            } else {
                rect.width = desired.width;
                rect.height = source_max(rect.height, desired.height);
            }
            this.arrange_child(&child, rect, final_size, orientation);
            if child.is_visible() {
                if this.orientation() == Orientation::Vertical {
                    rect.y += rect.height + spacing;
                } else {
                    rect.x += rect.width + spacing;
                }
            }
        }
        // Retain native inherited snap notification behavior. Source snap keys/interfaces differ;
        // this is not the source NotifySnapPointsChanges algorithm (see provenance limitations).
        let event = if this.orientation() == Orientation::Horizontal {
            StackPanel::horizontal_snap_points_changed_event()
        } else {
            StackPanel::vertical_snap_points_changed_event()
        };
        this.raise_event(&RoutedEventArgs::with_event(event));
        final_size
    }
}

#[cfg(test)]
#[path = "padded_stack_panel_tests.rs"]
mod tests;
