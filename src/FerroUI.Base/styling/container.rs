use super::{ContainerSizing, VisualQueryProvider};
use crate::layout::Layoutable;
use crate::{ferro_property, AttachedProperty, FerroObject, FerroProperty, StyledPropertyOptions};

/// Provides container attached properties: a layoutable that is a container
/// publishes its size to the container queries of its descendants.
pub struct Container;

crate::ferro_static_type!(Container);

crate::ferro_properties! { impl Container {
    ferro_property!(
        /// Defines the Name attached property.
        pub fn name_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<Container, Layoutable, _>("Name", None)
        }
    );

    ferro_property!(
        /// Defines the Sizing attached property.
        pub fn sizing_property() -> AttachedProperty<ContainerSizing> {
            FerroProperty::register_attached_with::<Container, Layoutable, _>(
                "Sizing",
                StyledPropertyOptions::new(ContainerSizing::Normal).coerce(Container::update_query_provider),
            )
        }
    );

    ferro_property!(
        /// Defines the QueryProvider attached property.
        pub(crate) fn query_provider_property() -> AttachedProperty<Option<VisualQueryProvider>> {
            FerroProperty::register_attached::<Container, Layoutable, _>("QueryProvider", None)
        }
    );
} }

impl Container {
    fn update_query_provider(obj: &FerroObject, sizing: ContainerSizing) -> ContainerSizing {
        if let Some(layoutable) = obj.downcast_ref::<Layoutable>() {
            if sizing != ContainerSizing::Normal {
                if Self::get_query_provider(layoutable).is_none() {
                    layoutable.set_value(Self::query_provider_property(), Some(VisualQueryProvider::new()));
                }
            } else {
                layoutable.set_value(Self::query_provider_property(), None);
            }
        }

        sizing
    }

    /// Gets the name of the container.
    pub fn get_name(layoutable: &Layoutable) -> Option<String> {
        layoutable.get_value(Self::name_property())
    }

    /// Sets the name of the container.
    pub fn set_name(layoutable: &Layoutable, name: Option<String>) {
        layoutable.set_value(Self::name_property(), name);
    }

    /// Gets the container sizing behavior.
    pub fn get_sizing(layoutable: &Layoutable) -> ContainerSizing {
        layoutable.get_value(Self::sizing_property())
    }

    /// Sets the container sizing behavior.
    pub fn set_sizing(layoutable: &Layoutable, sizing: ContainerSizing) {
        layoutable.set_value(Self::sizing_property(), sizing);
    }

    /// Gets the query provider of the container.
    pub(crate) fn get_query_provider(layoutable: &Layoutable) -> Option<VisualQueryProvider> {
        layoutable.get_value(Self::query_provider_property())
    }

    /// Reports the size of a container to its query provider, if it has
    /// one. For layoutables that replace the core measure pass (the roots
    /// of windows), which reports the size otherwise.
    pub fn set_query_provider_size(layoutable: &Layoutable, width: f64, height: f64, sizing: ContainerSizing) {
        if let Some(query_provider) = Self::get_query_provider(layoutable) {
            query_provider.set_size(width, height, sizing);
        }
    }
}
