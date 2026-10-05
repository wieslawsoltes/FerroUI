use crate::{ControlImpl, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl};
use crate::utils::BorderRenderHelper;
use ferroui_base::media::{BackgroundSizing, BoxShadows, DrawingContext, IBrush};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, CornerRadius, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Thickness, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A control which decorates a child with a border and background.
#[repr(C)]
pub struct Border {
    base: Decorator,
    layout_thickness: Cell<Option<Thickness>>,
    scale: Cell<f64>,
    border_render_helper: RefCell<BorderRenderHelper>,
    border_visual: RefCell<Option<crate::border_visual::CompositionBorderVisual>>,
}

ferro_class!(Border: Decorator);
ferroui_base::ferro_class_info!(Border { new: Border::new });
ferro_impl_classes!(Border: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl VisualImpl for Border {
    fn render(this: &Self, context: &mut DrawingContext) {
        this.border_render_helper.borrow_mut().render(
            context,
            this.bounds().size(),
            this.layout_thickness(),
            this.corner_radius(),
            this.background_sizing(),
            this.background().as_ref(),
            this.border_brush().as_ref(),
            &this.box_shadow(),
        );
    }

    // The rounded clip of the immediate renderer (`IVisualWithRoundRectClip`
    // upstream).
    fn clip_to_bounds_radius(this: &Self) -> Option<CornerRadius> {
        Some(this.corner_radius())
    }

    fn create_composition_visual(
        this: &Self,
        compositor: &Rc<ferroui_base::rendering::composition::Compositor>,
    ) -> ferroui_base::rendering::composition::CompositionDrawListVisual {
        let border_visual = crate::border_visual::CompositionBorderVisual::new(compositor, this);
        border_visual.set_corner_radius(this.corner_radius());
        let result = border_visual.as_draw_list_visual().clone();
        *this.border_visual.borrow_mut() = Some(border_visual);
        result
    }
}

impl FerroObjectImpl for Border {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "UseLayoutRounding" | "BorderThickness" => this.layout_thickness.set(None),
            "CornerRadius" => {
                if let Some(border_visual) = &*this.border_visual.borrow() {
                    border_visual.set_corner_radius(this.corner_radius());
                }
            }
            _ => {}
        }
    }
}

impl LayoutableImpl for Border {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let child = this.child();
        LayoutHelper::measure_child_with_border(
            child.as_deref().map(|c| -> &Layoutable { c }),
            available_size,
            this.padding(),
            this.border_thickness(),
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let child = this.child();
        LayoutHelper::arrange_child_with_border(
            child.as_deref().map(|c| -> &Layoutable { c }),
            final_size,
            this.padding(),
            this.border_thickness(),
        )
    }
}

ferroui_base::ferro_properties! { impl Border {
    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<Border, _>("Background", None)
        }
    );

    ferro_property!(
        /// Defines the `BackgroundSizing` property.
        pub fn background_sizing_property() -> StyledProperty<BackgroundSizing> {
            FerroProperty::register::<Border, _>("BackgroundSizing", BackgroundSizing::CenterBorder)
        }
    );

    ferro_property!(
        /// Defines the `BorderBrush` property.
        pub fn border_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<Border, _>("BorderBrush", None)
        }
    );

    ferro_property!(
        /// Defines the `BorderThickness` property.
        pub fn border_thickness_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<Border, _>(
                "BorderThickness",
                StyledPropertyOptions::new(Thickness::default())
                    .validate(|value| Layoutable::margin_property().validate_value().is_none_or(|validate| validate(value))),
            )
        }
    );

    ferro_property!(
        /// Defines the `CornerRadius` property.
        pub fn corner_radius_property() -> StyledProperty<CornerRadius> {
            FerroProperty::register::<Border, _>("CornerRadius", CornerRadius::default())
        }
    );

    ferro_property!(
        /// Defines the `BoxShadow` property.
        pub fn box_shadow_property() -> StyledProperty<BoxShadows> {
            FerroProperty::register::<Border, _>("BoxShadow", BoxShadows::default())
        }
    );
} }

impl Border {
    fn static_constructor() {
        Visual::affects_render::<Border>(&[
            Self::background_property().as_property(),
            Self::background_sizing_property().as_property(),
            Self::border_brush_property().as_property(),
            Self::border_thickness_property().as_property(),
            Self::corner_radius_property().as_property(),
            Self::box_shadow_property().as_property(),
        ]);
        Layoutable::affects_measure::<Border>(&[Self::border_thickness_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Decorator::construct(), layout_thickness: Cell::new(None), scale: Cell::new(0.0), border_render_helper: RefCell::new(BorderRenderHelper::new()), border_visual: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A brush with which to paint the background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// How the background is drawn relative to the border.
    pub fn background_sizing(&self) -> BackgroundSizing {
        self.get_value(Self::background_sizing_property())
    }

    pub fn set_background_sizing(&self, value: BackgroundSizing) {
        self.set_value(Self::background_sizing_property(), value)
    }

    /// A brush with which to paint the border.
    pub fn border_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::border_brush_property())
    }

    pub fn set_border_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::border_brush_property(), value)
    }

    /// The thickness of the border.
    pub fn border_thickness(&self) -> Thickness {
        self.get_value(Self::border_thickness_property())
    }

    pub fn set_border_thickness(&self, value: Thickness) {
        self.set_value(Self::border_thickness_property(), value)
    }

    /// The radius of the border rounded corners.
    pub fn corner_radius(&self) -> CornerRadius {
        self.get_value(Self::corner_radius_property())
    }

    pub fn set_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::corner_radius_property(), value)
    }

    /// The box shadow effect parameters.
    pub fn box_shadow(&self) -> BoxShadows {
        self.get_value(Self::box_shadow_property())
    }

    pub fn set_box_shadow(&self, value: BoxShadows) {
        self.set_value(Self::box_shadow_property(), value)
    }

    /// The border thickness used for rendering: rounded to device pixels
    /// when layout rounding is in use.
    pub fn layout_thickness(&self) -> Thickness {
        self.verify_scale();

        match self.layout_thickness.get() {
            Some(thickness) => thickness,
            None => {
                let mut border_thickness = self.border_thickness();

                if self.use_layout_rounding() {
                    border_thickness = LayoutHelper::round_layout_thickness(border_thickness, self.scale.get());
                }

                self.layout_thickness.set(Some(border_thickness));
                border_thickness
            }
        }
    }

    fn verify_scale(&self) {
        let current_scale = LayoutHelper::get_layout_scale(self);
        if MathUtilities::are_close(current_scale, self.scale.get()) {
            return;
        }

        self.scale.set(current_scale);
        self.layout_thickness.set(None);
    }

    /// The corner radius with which the visual is clipped when it clips to
    /// its bounds.
    pub fn clip_to_bounds_radius(&self) -> CornerRadius {
        self.corner_radius()
    }
}
