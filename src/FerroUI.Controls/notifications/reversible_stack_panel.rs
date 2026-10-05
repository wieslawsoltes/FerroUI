use crate::{ControlImpl, PanelImpl, StackPanel, StackPanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{
    HorizontalAlignment, LayoutExtensions, Layoutable, LayoutableImpl, Orientation, VerticalAlignment,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};

/// Implements a panel which lays out its children horizontally or vertically
/// and can reverse the order of its children.
#[repr(C)]
pub struct ReversibleStackPanel {
    base: StackPanel,
}

ferro_class!(ReversibleStackPanel: StackPanel);
ferroui_base::ferro_class_info!(ReversibleStackPanel { new: ReversibleStackPanel::new });
ferro_impl_classes!(
    ReversibleStackPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    StackPanelImpl
);

impl FerroObjectImpl for ReversibleStackPanel {}

impl LayoutableImpl for ReversibleStackPanel {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let orientation = this.orientation();
        let spacing = this.spacing();
        let final_rect = Rect::from_size(final_size);
        let mut pos = 0.0;

        let children = this.children().snapshot();
        let reverse_order = this.reverse_order();

        for index in 0..children.len() {
            let child = &children[if reverse_order { children.len() - 1 - index } else { index }];

            if !child.is_visible() {
                continue;
            }

            let child_width = child.desired_size().width;
            let child_height = child.desired_size().height;

            if orientation == Orientation::Vertical {
                let rect = LayoutExtensions::align(
                    Rect::new(0.0, pos, child_width, child_height),
                    final_rect,
                    child.horizontal_alignment(),
                    VerticalAlignment::Top,
                );
                this.arrange_child(child, rect, final_size, orientation);
                pos += child_height + spacing;
            } else {
                let rect = LayoutExtensions::align(
                    Rect::new(pos, 0.0, child_width, child_height),
                    final_rect,
                    HorizontalAlignment::Left,
                    child.vertical_alignment(),
                );
                this.arrange_child(child, rect, final_size, orientation);
                pos += child_width + spacing;
            }
        }

        final_size
    }
}

ferroui_base::ferro_properties! { impl ReversibleStackPanel {
    ferro_property!(
        /// Defines the `ReverseOrder` property.
        pub fn reverse_order_property() -> StyledProperty<bool> {
            FerroProperty::register::<ReversibleStackPanel, _>("ReverseOrder", false)
        }
    );
} }

impl ReversibleStackPanel {
    fn static_constructor() {
        Layoutable::affects_arrange::<ReversibleStackPanel>(&[Self::reverse_order_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StackPanel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the order of the children is reversed when they are arranged.
    pub fn reverse_order(&self) -> bool {
        self.get_value(Self::reverse_order_property())
    }

    pub fn set_reverse_order(&self, value: bool) {
        self.set_value(Self::reverse_order_property(), value)
    }
}
