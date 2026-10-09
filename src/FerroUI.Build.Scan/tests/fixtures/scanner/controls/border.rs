use super::decorator::Decorator;
use crate::media::{Brush, IBrush, Thickness};
use ferroui_base::{ferro_class, ferro_property, BoxedValue, DirectProperty, FerroProperty, Ref, StyledProperty};
use std::rc::Rc;

/// A control which decorates a child with a border.
#[repr(C)]
pub struct Border {
    base: Decorator,
}

ferro_class!(Border: Decorator);
ferroui_base::ferro_class_info!(Border { new: Border::new, interfaces: [Rc<dyn IBrush>, Rc<dyn ferroui_base::IOther> => Border::as_other] });

ferroui_base::ferro_properties! { impl Border, also [Border::register_more] {
    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<Border, _>("Background", None)
        }
    );

    ferro_property!(
        pub(crate) fn thickness_property() -> DirectProperty<Border, f64> {
            FerroProperty::register_direct::<Border, _>("Thickness", |border| border.thickness(1.0), None, 0.0)
        }
    );

    ferro_property!(
        pub fn child_property() -> StyledProperty<Option<Ref<ferroui_base::Control>>> {
            Decorator::child_property().add_owner::<Border>()
        }
    );

    ferro_property!(
        fn content_property() -> StyledProperty<Option<Ref<ferroui_base::Control>>> {
            Self::child_property()
        }
    );
} }

impl Border {
    pub fn new() -> Ref<Border> {
        ferroui_base::instantiate(Border { base: Decorator::construct() })
    }

    ferro_property!(for Border; pub fn brush_property() -> StyledProperty<Option<Ref<Brush>>> {
        FerroProperty::register::<Border, _>("Brush", None)
    });

    ferroui_base::ferro_routed_event!(pub fn pressed_event() -> RoutedEvent<RoutedEventArgs> {
        RoutedEvent::register::<Border, _>("Pressed", RoutingStrategies::BUBBLE)
    });

    pub(crate) fn thickness(&self, scale: f64) -> f64 {
        // A class declared inside a function: the scanner reports it and does not read it.
        ferro_class!(Local: Border);
        scale
    }

    pub fn with_name(name: String) -> Ref<Border> {
        let _ = name;
        Border::new()
    }

    pub fn tag(&self) -> Option<BoxedValue> {
        None
    }

    pub(crate) fn set_tag(&self, _value: Option<BoxedValue>) {}

    pub fn default_thickness() -> Thickness {
        Thickness::uniform(0.0)
    }

    fn set_default_thickness(_value: Thickness) -> Result<(), String> {
        Ok(())
    }

    /// Takes no index: not what the indexer of the metadata passes.
    pub fn brush_at(&self) -> Ref<Brush> {
        Brush::new()
    }
}
