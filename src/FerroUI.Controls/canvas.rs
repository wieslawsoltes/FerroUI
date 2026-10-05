use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::{INavigableContainer, InputElement, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, FerroObject, FerroObjectImpl, FerroProperty, Point, Rect, Ref, Size, StyledElementImpl, Visual, VisualImpl,
};

/// A panel that displays child controls at arbitrary locations.
///
/// Unlike other [`Panel`] implementations, the canvas doesn't lay out its
/// children in any particular layout. Instead, the positioning of each child
/// control is defined by the `Canvas.Left`, `Canvas.Top`, `Canvas.Right` and
/// `Canvas.Bottom` attached properties.
#[repr(C)]
pub struct Canvas {
    base: Panel,
}

ferro_class! {
    Canvas: Panel, virtuals CanvasImpl: PanelImpl {
        /// Arranges a single child within the size allocated to the canvas.
        fn arrange_child(this, child: &Ref<Control>, final_size: Size);
    }
}
ferroui_base::ferro_class_info!(Canvas { new: Canvas::new });

ferro_impl_classes!(Canvas: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);

impl FerroObjectImpl for Canvas {}

impl LayoutableImpl for Canvas {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        let available_size = Size::new(f64::INFINITY, f64::INFINITY);

        for child in this.children().snapshot().iter() {
            child.measure(available_size);
        }

        Size::default()
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        for child in this.children().snapshot().iter() {
            this.arrange_child(child, final_size);
        }

        final_size
    }
}

impl CanvasImpl for Canvas {
    fn arrange_child(_this: &Self, child: &Ref<Control>, final_size: Size) {
        let mut x = 0.0;
        let mut y = 0.0;
        let element_left = Self::get_left(child);

        if !element_left.is_nan() {
            x = element_left;
        } else {
            // Arrange with right.
            let element_right = Self::get_right(child);
            if !element_right.is_nan() {
                x = final_size.width - child.desired_size().width - element_right;
            }
        }

        let element_top = Self::get_top(child);
        if !element_top.is_nan() {
            y = element_top;
        } else {
            let element_bottom = Self::get_bottom(child);
            if !element_bottom.is_nan() {
                y = final_size.height - child.desired_size().height - element_bottom;
            }
        }

        child.arrange(Rect::from_position_size(Point::new(x, y), child.desired_size()));
    }
}

ferroui_base::ferro_properties! { impl Canvas {
    ferro_property!(
        /// Defines the `Left` attached property.
        pub fn left_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Canvas, Control, _>("Left", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `Top` attached property.
        pub fn top_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Canvas, Control, _>("Top", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `Right` attached property.
        pub fn right_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Canvas, Control, _>("Right", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `Bottom` attached property.
        pub fn bottom_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Canvas, Control, _>("Bottom", f64::NAN)
        }
    );
} }

impl Canvas {
    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<Canvas>(false);
        Panel::affects_parent_arrange::<Canvas>(&[
            Self::left_property().as_property(),
            Self::top_property().as_property(),
            Self::right_property().as_property(),
            Self::bottom_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the value of the `Left` attached property for a control.
    pub fn get_left(element: &FerroObject) -> f64 {
        element.get_value(Self::left_property())
    }

    /// Sets the value of the `Left` attached property for a control.
    pub fn set_left(element: &FerroObject, value: f64) {
        element.set_value(Self::left_property(), value)
    }

    /// Gets the value of the `Top` attached property for a control.
    pub fn get_top(element: &FerroObject) -> f64 {
        element.get_value(Self::top_property())
    }

    /// Sets the value of the `Top` attached property for a control.
    pub fn set_top(element: &FerroObject, value: f64) {
        element.set_value(Self::top_property(), value)
    }

    /// Gets the value of the `Right` attached property for a control.
    pub fn get_right(element: &FerroObject) -> f64 {
        element.get_value(Self::right_property())
    }

    /// Sets the value of the `Right` attached property for a control.
    pub fn set_right(element: &FerroObject, value: f64) {
        element.set_value(Self::right_property(), value)
    }

    /// Gets the value of the `Bottom` attached property for a control.
    pub fn get_bottom(element: &FerroObject) -> f64 {
        element.get_value(Self::bottom_property())
    }

    /// Sets the value of the `Bottom` attached property for a control.
    pub fn set_bottom(element: &FerroObject, value: f64) {
        element.set_value(Self::bottom_property(), value)
    }
}

impl INavigableContainer for Canvas {
    /// Gets the next control in the specified direction.
    fn get_control(
        &self,
        _direction: NavigationDirection,
        _from: Option<&Ref<InputElement>>,
        _wrap: bool,
    ) -> Option<Ref<InputElement>> {
        // Not implemented by the reference either.
        None
    }
}
