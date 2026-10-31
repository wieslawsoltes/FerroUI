//! A QML Row/Column places an item along one axis. On the other axis its own
//! origin and size are retained. A native StackPanel normally gives its child
//! the whole cross-axis slot, which centers a fixed-size Stretch control.
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, Orientation},
    AttachedProperty, FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::{
    Canvas, Control, ControlImpl, Panel, PanelImpl, StackPanel, StackPanelImpl, StackPanelImplExt,
};

#[repr(C)]
pub struct Positioner {
    base: StackPanel,
}
ferro_class!(Positioner: StackPanel);
ferro_impl_classes!(Positioner: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, PanelImpl);

ferro_properties! { impl Positioner {
    fn quick_item_property() -> AttachedProperty<bool> {
        FerroProperty::register_attached::<Positioner, Control, _>("QuickItem",false)
    }
}}

impl Positioner {
    fn static_constructor() {
        Panel::affects_parent_arrange::<Self>(&[
            Canvas::left_property().as_property(),
            Canvas::top_property().as_property(),
        ]);
    }

    pub(super) fn create(orientation: Orientation) -> Ref<StackPanel> {
        let panel = instantiate(Self {
            base: StackPanel::construct(),
        });
        panel.set_orientation(orientation);
        panel.upcast()
    }

    pub(super) fn mark_item(child: &Control) {
        child.set_value(Self::quick_item_property(), true);
    }
}

impl StackPanelImpl for Positioner {
    fn arrange_child(
        this: &Self,
        child: &Ref<Control>,
        slot: Rect,
        available: Size,
        orientation: Orientation,
    ) {
        if !child.get_value(Self::quick_item_property()) {
            // Reflected and application-native children keep their own alignment.
            Self::parent_arrange_child(this, child, slot, available, orientation);
            return;
        }
        let desired = child.desired_size();
        let rect = match orientation {
            Orientation::Horizontal => {
                Rect::new(slot.x, Canvas::get_top(child), slot.width, desired.height)
            }
            Orientation::Vertical => {
                Rect::new(Canvas::get_left(child), slot.y, desired.width, slot.height)
            }
        };
        child.arrange(rect);
    }
}
