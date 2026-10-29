//! Port of `ScrollViewer/ScrollToOffsetAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ScrollViewer;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Scrolls a [`ScrollViewer`] to the specified offsets when executed.
#[repr(C)]
pub struct ScrollToOffsetAction {
    base: StyledElementAction,
}

ferro_class!(ScrollToOffsetAction: StyledElementAction);
ferro_class_info!(ScrollToOffsetAction {
    new: ScrollToOffsetAction::new,
    markup: {
        property_attributes: [
            ScrollViewer: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ScrollToOffsetAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ScrollToOffsetAction {
        /// Identifies the `ScrollViewer` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the scroll viewer, so the
        /// value is an element reference.
        pub fn scroll_viewer_property() -> StyledProperty<Option<ElementRef<ScrollViewer>>> {
            ValueTypes::register_element_ref::<ScrollViewer>();
            FerroProperty::register::<ScrollToOffsetAction, _>("ScrollViewer", None)
        }

        /// Identifies the `HorizontalOffset` property.
        pub fn horizontal_offset_property() -> StyledProperty<Option<f64>> {
            FerroProperty::register::<ScrollToOffsetAction, _>("HorizontalOffset", None)
        }

        /// Identifies the `VerticalOffset` property.
        pub fn vertical_offset_property() -> StyledProperty<Option<f64>> {
            FerroProperty::register::<ScrollToOffsetAction, _>("VerticalOffset", None)
        }
    }
}

impl StyledElementActionImpl for ScrollToOffsetAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let scroller = this.scroll_viewer().or_else(|| crate::part_c::sender_as::<ScrollViewer>(sender));
        let Some(scroller) = scroller else {
            return Some(Rc::new(false));
        };

        let mut offset = scroller.offset();
        if let Some(horizontal_offset) = this.horizontal_offset() {
            offset = offset.with_x(horizontal_offset);
        }

        if let Some(vertical_offset) = this.vertical_offset() {
            offset = offset.with_y(vertical_offset);
        }

        scroller.set_offset(offset);
        Some(Rc::new(true))
    }
}

impl ScrollToOffsetAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the [`ScrollViewer`] instance to scroll. This is a property of the object
    /// model; markup resolves a name given for it to the named scroll viewer.
    pub fn scroll_viewer(&self) -> Option<Ref<ScrollViewer>> {
        ElementRef::resolve(&self.get_value(Self::scroll_viewer_property()))
    }

    pub fn set_scroll_viewer(&self, value: impl Into<Nullable<ScrollViewer>>) {
        self.set_value(Self::scroll_viewer_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the horizontal offset to scroll to. This is a property of the object model.
    pub fn horizontal_offset(&self) -> Option<f64> {
        self.get_value(Self::horizontal_offset_property())
    }

    pub fn set_horizontal_offset(&self, value: Option<f64>) {
        self.set_value(Self::horizontal_offset_property(), value)
    }

    /// Gets or sets the vertical offset to scroll to. This is a property of the object model.
    pub fn vertical_offset(&self) -> Option<f64> {
        self.get_value(Self::vertical_offset_property())
    }

    pub fn set_vertical_offset(&self, value: Option<f64>) {
        self.set_value(Self::vertical_offset_property(), value)
    }
}
