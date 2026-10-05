use crate::{Control, ControlImpl, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::immutable::ImmutableTransform;
use ferroui_base::media::{ITransform, MediaExtensions, Stretch, StretchDirection};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Matrix, Nullable, Rect, Ref, RelativePoint, Size, StyledElement,
    StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// A control used to scale a single child.
#[repr(C)]
pub struct Viewbox {
    base: Control,
    container_visual: OnceCell<Ref<ViewboxContainer>>,
}

ferro_class!(Viewbox: Control);
ferroui_base::ferro_class_info!(Viewbox { new: Viewbox::new });
ferro_impl_classes!(Viewbox: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for Viewbox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // The child control is hosted inside a container control so that the
        // transform can be applied independently of the viewbox and child
        // transforms.
        let container_visual = ViewboxContainer::new();
        container_visual.set_render_transform_origin(RelativePoint::TOP_LEFT);
        container_visual.set_parent(this.to_ref());
        this.visual_children().add(container_visual.clone().upcast());
        let _ = this.container_visual.set(container_visual);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::child_property().as_property() {
            let (old_child, new_child) = change.get_old_and_new_value::<Option<Ref<Control>>>();

            if let Some(old_child) = old_child {
                old_child.set_parent(None);
                this.logical_children().remove(&old_child.upcast());
            }

            this.container().set_child(new_child.clone());

            if let Some(new_child) = new_child {
                new_child.set_parent(this.to_ref());
                this.logical_children().add(new_child.upcast());
            }

            this.invalidate_measure();
        } else if change.property() == StyledElement::templated_parent_property().as_property() {
            // Update the container's templated parent, otherwise its
            // descendants aren't reachable during the template's teardown and
            // incorrectly stay attached to the visual tree.
            if let Some(templated_parent) = change.get_new_value::<Option<Ref<FerroObject>>>() {
                this.container().set_templated_parent(templated_parent);
            }
        }
    }
}

impl LayoutableImpl for Viewbox {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let child = this.container();

        child.measure(Size::INFINITY);

        let child_size = child.desired_size();

        MediaExtensions::calculate_size(this.stretch(), available_size, child_size, this.stretch_direction())
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let child = this.container();

        let child_size = child.desired_size();
        let scale =
            MediaExtensions::calculate_scaling(this.stretch(), final_size, child_size, this.stretch_direction());

        this.set_internal_transform(Some(Rc::new(ImmutableTransform::new(Matrix::create_scale(scale.x, scale.y)))));

        child.arrange(Rect::from_size(child_size));

        child_size * scale
    }
}

ferroui_base::ferro_properties! { impl Viewbox {
    ferro_property!(
        /// Defines the `Stretch` property.
        pub fn stretch_property() -> StyledProperty<Stretch> {
            FerroProperty::register::<Viewbox, _>("Stretch", Stretch::Uniform)
        }
    );

    ferro_property!(
        /// Defines the `StretchDirection` property.
        pub fn stretch_direction_property() -> StyledProperty<StretchDirection> {
            FerroProperty::register::<Viewbox, _>("StretchDirection", StretchDirection::Both)
        }
    );

    ferro_property!(
        /// Defines the `Child` property.
        pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
            Decorator::child_property().add_owner::<Viewbox>()
        }
    );
} }

impl Viewbox {
    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<Viewbox>(true);
        Layoutable::use_layout_rounding_property().override_default_value::<Viewbox>(true);
        Layoutable::affects_measure::<Viewbox>(&[
            Self::stretch_property().as_property(),
            Self::stretch_direction_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Control::construct(), container_visual: OnceCell::new() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn container(&self) -> &Ref<ViewboxContainer> {
        self.container_visual.get().expect("the viewbox has been constructed")
    }

    /// The stretch mode, which determines how child fits into the available
    /// space.
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }

    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value)
    }

    /// A value controlling in what direction contents will be stretched.
    pub fn stretch_direction(&self) -> StretchDirection {
        self.get_value(Self::stretch_direction_property())
    }

    pub fn set_stretch_direction(&self, value: StretchDirection) {
        self.set_value(Self::stretch_direction_property(), value)
    }

    /// The child of the viewbox.
    pub fn child(&self) -> Option<Ref<Control>> {
        self.get_value(Self::child_property())
    }

    pub fn set_child(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::child_property(), value.into().0)
    }

    /// The transform applied to the container of the child.
    #[cfg(test)]
    pub(crate) fn internal_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.container().render_transform()
    }

    pub(crate) fn set_internal_transform(&self, value: Option<Rc<dyn ITransform>>) {
        self.container().set_render_transform(value)
    }
}

/// A simple container control which hosts its child as a visual but not
/// logical child.
#[repr(C)]
pub(crate) struct ViewboxContainer {
    base: Control,
    child: RefCell<Option<Ref<Control>>>,
}

ferro_class!(ViewboxContainer: Control);
ferro_impl_classes!(
    ViewboxContainer: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl ViewboxContainer {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), child: RefCell::new(None) })
    }

    fn set_child(&self, value: Option<Ref<Control>>) {
        let old = self.child.borrow().clone();
        if old != value {
            if let Some(old) = old {
                self.visual_children().remove(&old.upcast());
            }

            *self.child.borrow_mut() = value.clone();

            if let Some(value) = value {
                self.visual_children().add(value.upcast());
            }

            self.invalidate_measure();
        }
    }
}
