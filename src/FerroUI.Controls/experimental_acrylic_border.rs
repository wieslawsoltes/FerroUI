use crate::{Border, ControlImpl, Decorator, TopLevel, WindowTransparencyLevel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl};
use ferroui_base::media::{ExperimentalAcrylicMaterial, ImmutableExperimentalAcrylicMaterial};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::rendering::composition::{
    CompositionDrawListVisual, CompositionExperimentalAcrylicVisual, Compositor,
};
use ferroui_base::{FerroObject, FerroObjectExtensions, FerroObjectImplExt, FerroPropertyChangedEventArgs};
use std::cell::RefCell;
use std::rc::Rc;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, CornerRadius, FerroObjectImpl,
    FerroProperty, Nullable, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};

/// A control which decorates a child with a border and an acrylic material
/// background.
///
/// The material is drawn by the composition visual of the border, an
/// acrylic visual that receives the material and the corner radius; the
/// platform transparency compensation of the material follows the
/// transparency level of the top level the border is in.
#[repr(C)]
pub struct ExperimentalAcrylicBorder {
    base: Decorator,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    material_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ExperimentalAcrylicBorder: Decorator);
ferroui_base::ferro_class_info!(ExperimentalAcrylicBorder { new: ExperimentalAcrylicBorder::new });
ferro_impl_classes!(ExperimentalAcrylicBorder: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for ExperimentalAcrylicBorder {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::material_property().as_property() {
            this.update_material_subscription();
        }
        if change.property() == Self::corner_radius_property().as_property() {
            this.sync_material(this.composition_visual());
        }
        Self::parent_on_property_changed(this, change);
    }
}

impl VisualImpl for ExperimentalAcrylicBorder {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let tl = TopLevel::get_top_level(Some(this));
        if let Some(tl) = tl {
            // The subscription lives in the border, which the top level
            // holds: the handler refers to both weakly.
            let weak_tl = tl.downgrade();
            let weak_this = this.to_ref().downgrade();
            let object: &FerroObject = &tl;
            let subscription =
                FerroObjectExtensions::get_observable(object, TopLevel::actual_transparency_level_property())
                    .subscribe_fn(move |x| {
                        let (Some(tl), Some(this)) = (weak_tl.upgrade(), weak_this.upgrade()) else {
                            return;
                        };
                        let (Some(platform_impl), Some(material)) = (tl.platform_impl(), this.material()) else {
                            return;
                        };
                        if x == WindowTransparencyLevel::transparent() || x == WindowTransparencyLevel::none() {
                            material.set_platform_transparency_compensation_level(
                                platform_impl.acrylic_compensation_levels().transparent_level(),
                            );
                        } else if x == WindowTransparencyLevel::blur() {
                            material.set_platform_transparency_compensation_level(
                                platform_impl.acrylic_compensation_levels().blur_level(),
                            );
                        } else if x == WindowTransparencyLevel::acrylic_blur() {
                            material.set_platform_transparency_compensation_level(
                                platform_impl.acrylic_compensation_levels().acrylic_blur_level(),
                            );
                        }
                    });
            *this.subscription.borrow_mut() = Some(subscription);
        } else if let Some(material) = this.material() {
            material.set_platform_transparency_compensation_level(1.0);
        }

        this.update_material_subscription();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.update_material_subscription();
        let subscription = this.subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn create_composition_visual(this: &Self, compositor: &Rc<Compositor>) -> CompositionDrawListVisual {
        let v = CompositionExperimentalAcrylicVisual::new(compositor, this);
        this.sync_material(Some(v.as_draw_list_visual().clone()));

        v.as_draw_list_visual().clone()
    }
}

impl LayoutableImpl for ExperimentalAcrylicBorder {
    /// Measures the control.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let child = this.child();
        LayoutHelper::measure_child(child.as_deref().map(|c| -> &Layoutable { c }), available_size, this.padding())
    }

    /// Arranges the control's child.
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let child = this.child();
        LayoutHelper::arrange_child(child.as_deref().map(|c| -> &Layoutable { c }), final_size, this.padding())
    }
}

ferroui_base::ferro_properties! { impl ExperimentalAcrylicBorder {
    ferro_property!(
        /// Defines the `CornerRadius` property.
        pub fn corner_radius_property() -> StyledProperty<CornerRadius> {
            Border::corner_radius_property().add_owner::<ExperimentalAcrylicBorder>()
        }
    );

    ferro_property!(
        /// Defines the `Material` property.
        pub fn material_property() -> StyledProperty<Option<Ref<ExperimentalAcrylicMaterial>>> {
            FerroProperty::register::<ExperimentalAcrylicBorder, _>("Material", None)
        }
    );
} }

impl ExperimentalAcrylicBorder {
    fn static_constructor() {
        Visual::affects_render::<ExperimentalAcrylicBorder>(&[
            Self::material_property().as_property(),
            Self::corner_radius_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            subscription: RefCell::new(None),
            material_subscription: RefCell::new(None),
        }
    }

    fn update_material_subscription(&self) {
        let material_subscription = self.material_subscription.borrow_mut().take();
        if let Some(material_subscription) = material_subscription {
            material_subscription.dispose();
        }
        let Some(composition_visual) = self.composition_visual() else {
            return;
        };
        let Some(material) = self.material() else {
            return;
        };
        let weak = self.to_ref().downgrade();
        let material_subscription = material.property_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.update_material_subscription();
            }
        });
        *self.material_subscription.borrow_mut() = Some(material_subscription);
        self.sync_material(Some(composition_visual));
    }

    fn sync_material(&self, visual: Option<CompositionDrawListVisual>) {
        if let Some(v) = visual.and_then(|visual| CompositionExperimentalAcrylicVisual::from_visual(&visual)) {
            v.set_corner_radius(self.corner_radius());
            let material = self
                .material()
                .and_then(|material| {
                    material.to_immutable().as_any().downcast_ref::<ImmutableExperimentalAcrylicMaterial>().copied()
                })
                .unwrap_or_default();
            v.set_material(material);
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the radius of the border rounded corners.
    pub fn corner_radius(&self) -> CornerRadius {
        self.get_value(Self::corner_radius_property())
    }

    /// Sets the radius of the border rounded corners.
    pub fn set_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::corner_radius_property(), value)
    }

    /// Gets the material used to paint the background of the border.
    pub fn material(&self) -> Option<Ref<ExperimentalAcrylicMaterial>> {
        self.get_value(Self::material_property())
    }

    /// Sets the material used to paint the background of the border.
    pub fn set_material(&self, value: impl Into<Nullable<ExperimentalAcrylicMaterial>>) {
        self.set_value(Self::material_property(), value.into().0)
    }
}
