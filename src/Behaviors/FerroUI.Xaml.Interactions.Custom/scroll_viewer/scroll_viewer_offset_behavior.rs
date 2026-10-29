//! Port of `ScrollViewer/ScrollViewerOffsetBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ScrollViewer;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Sets the [`offset`](ScrollViewer::offset) of the associated [`ScrollViewer`].
#[repr(C)]
pub struct ScrollViewerOffsetBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ScrollViewerOffsetBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ScrollViewerOffsetBehavior { new: ScrollViewerOffsetBehavior::new });
ferroui_base::ferro_impl_classes!(
    ScrollViewerOffsetBehavior: StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ScrollViewerOffsetBehavior {
        /// Identifies the `HorizontalOffset` property.
        pub fn horizontal_offset_property() -> StyledProperty<Option<f64>> {
            FerroProperty::register::<ScrollViewerOffsetBehavior, _>("HorizontalOffset", None)
        }

        /// Identifies the `VerticalOffset` property.
        pub fn vertical_offset_property() -> StyledProperty<Option<f64>> {
            FerroProperty::register::<ScrollViewerOffsetBehavior, _>("VerticalOffset", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ScrollViewerOffsetBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::horizontal_offset_property().as_property()
                || change.property() == Self::vertical_offset_property().as_property()
            {
                this.set_offset();
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ScrollViewerOffsetBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            this.set_offset();
            DisposableAction::empty()
        }
    }
}

impl ScrollViewerOffsetBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<ScrollViewer as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the horizontal offset value. This is a property of the object model.
    pub fn horizontal_offset(&self) -> Option<f64> {
        self.get_value(Self::horizontal_offset_property())
    }

    pub fn set_horizontal_offset(&self, value: Option<f64>) {
        self.set_value(Self::horizontal_offset_property(), value)
    }

    /// Gets or sets the vertical offset value. This is a property of the object model.
    pub fn vertical_offset(&self) -> Option<f64> {
        self.get_value(Self::vertical_offset_property())
    }

    pub fn set_vertical_offset(&self, value: Option<f64>) {
        self.set_value(Self::vertical_offset_property(), value)
    }

    fn set_offset(&self) {
        let Some(associated_object) = self.associated_object_as::<ScrollViewer>() else {
            return;
        };

        let mut offset = associated_object.offset();
        if let Some(horizontal_offset) = self.horizontal_offset() {
            offset = offset.with_x(horizontal_offset);
        }

        if let Some(vertical_offset) = self.vertical_offset() {
            offset = offset.with_y(vertical_offset);
        }

        associated_object.set_offset(offset);
    }
}
