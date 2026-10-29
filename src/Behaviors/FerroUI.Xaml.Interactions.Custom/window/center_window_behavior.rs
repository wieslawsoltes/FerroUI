//! Port of `Window/CenterWindowBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl, FerroProperty,
    Nullable, PixelPoint, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, TopLevel, Window};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};

/// Centers the attached window on the current screen when it is attached to the visual tree.
#[repr(C)]
pub struct CenterWindowBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(CenterWindowBehavior: StyledElementBehaviorOfT);
ferro_class_info!(CenterWindowBehavior {
    new: CenterWindowBehavior::new,
    markup: {
        property_attributes: [
            Window: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CenterWindowBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl CenterWindowBehavior {
        /// Identifies the `Window` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the window, so the
        /// value is an element reference.
        pub fn window_property() -> StyledProperty<Option<ElementRef<Window>>> {
            ValueTypes::register_element_ref::<Window>();
            FerroProperty::register::<CenterWindowBehavior, _>("Window", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for CenterWindowBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            this.center();
        }
    }
}

impl CenterWindowBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the window to center. If not set, the visual root window is used. Markup
    /// resolves a name given for it to the named window.
    pub fn window(&self) -> Option<Ref<Window>> {
        ElementRef::resolve(&self.get_value(Self::window_property()))
    }

    pub fn set_window(&self, value: impl Into<Nullable<Window>>) {
        self.set_value(Self::window_property(), ElementRef::from_nullable(value.into().0))
    }

    fn center(&self) {
        let window = match self.window() {
            Some(window) => Some(window),
            None => {
                let associated_object = self.associated_object_as::<Control>();
                TopLevel::get_top_level(associated_object.as_deref().map(crate::part_c::control_as_visual))
                    .and_then(|top_level| top_level.cast::<Window>())
            }
        };
        let Some(window) = window else {
            return;
        };

        let Some(screen) = window.screens().screen_from_window(&window) else {
            return;
        };

        let rect = screen.working_area();
        let x = rect.x as f64 + (rect.width as f64 - window.width()) / 2.0;
        let y = rect.y as f64 + (rect.height as f64 - window.height()) / 2.0;
        window.set_position(PixelPoint::new(x as i32, y as i32));
    }
}
