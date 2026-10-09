use crate::definition_base::DefinitionBaseImpl;
use crate::{DefinitionBase, GridLength, GridUnitType};
use ferroui_base::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// Holds a row definition for a `Grid`.
#[repr(C)]
pub struct RowDefinition {
    base: DefinitionBase,
}

ferro_class!(RowDefinition: DefinitionBase);
ferroui_base::ferro_class_info!(RowDefinition { new: RowDefinition::new });

ferroui_base::ferro_impl_classes!(RowDefinition: FerroObjectImpl);

impl DefinitionBaseImpl for RowDefinition {
    fn user_size_value_cache(this: &Self) -> GridLength {
        this.height()
    }

    fn user_min_size_value_cache(this: &Self) -> f64 {
        this.min_height()
    }

    fn user_max_size_value_cache(this: &Self) -> f64 {
        this.max_height()
    }
}

ferroui_base::ferro_properties! { impl RowDefinition {
    ferro_property!(
        /// Defines the `MaxHeight` property.
        pub fn max_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<RowDefinition, _>("MaxHeight", f64::INFINITY)
        }
    );

    ferro_property!(
        /// Defines the `MinHeight` property.
        pub fn min_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<RowDefinition, _>("MinHeight", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Height` property.
        pub fn height_property() -> StyledProperty<GridLength> {
            FerroProperty::register::<RowDefinition, _>("Height", GridLength::new(1.0, GridUnitType::Star))
        }
    );
} }

impl RowDefinition {
    fn static_constructor() {
        DefinitionBase::affects_parent_measure(&[
            Self::max_height_property().as_property(),
            Self::min_height_property().as_property(),
        ]);

        Self::height_property()
            .changed()
            .add_class_handler::<DefinitionBase>(DefinitionBase::on_user_size_property_changed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: DefinitionBase::construct(),
        }
    }

    /// Creates a row definition.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a row definition with the given height value and unit.
    pub fn with_value(value: f64, type_: GridUnitType) -> Ref<Self> {
        Self::with_height(GridLength::new(value, type_))
    }

    /// Creates a row definition with the given height.
    pub fn with_height(height: GridLength) -> Ref<Self> {
        let result = Self::new();
        result.set_current_value(Self::height_property(), height);
        result
    }

    /// The actual calculated height of the row.
    pub fn actual_height(&self) -> f64 {
        match self.parent() {
            Some(parent) => parent.get_final_row_definition_height(self.index()),
            None => 0.0,
        }
    }

    /// The maximum height of the row in DIPs.
    pub fn max_height(&self) -> f64 {
        self.get_value(Self::max_height_property())
    }

    pub fn set_max_height(&self, value: f64) {
        self.set_value(Self::max_height_property(), value)
    }

    /// The minimum height of the row in DIPs.
    pub fn min_height(&self) -> f64 {
        self.get_value(Self::min_height_property())
    }

    pub fn set_min_height(&self, value: f64) {
        self.set_value(Self::min_height_property(), value)
    }

    /// The height of the row.
    pub fn height(&self) -> GridLength {
        self.get_value(Self::height_property())
    }

    pub fn set_height(&self, value: GridLength) {
        self.set_value(Self::height_property(), value)
    }
}
