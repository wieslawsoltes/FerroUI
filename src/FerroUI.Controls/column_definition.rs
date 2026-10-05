use crate::definition_base::DefinitionBaseImpl;
use crate::{DefinitionBase, GridLength, GridUnitType};
use ferroui_base::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// Holds a column definition for a `Grid`.
#[repr(C)]
pub struct ColumnDefinition {
    base: DefinitionBase,
}

ferro_class!(ColumnDefinition: DefinitionBase);
ferroui_base::ferro_class_info!(ColumnDefinition { new: ColumnDefinition::new });

impl FerroObjectImpl for ColumnDefinition {}

impl DefinitionBaseImpl for ColumnDefinition {
    fn user_size_value_cache(this: &Self) -> GridLength {
        this.width()
    }

    fn user_min_size_value_cache(this: &Self) -> f64 {
        this.min_width()
    }

    fn user_max_size_value_cache(this: &Self) -> f64 {
        this.max_width()
    }
}

ferroui_base::ferro_properties! { impl ColumnDefinition {
    ferro_property!(
        /// Defines the `MaxWidth` property.
        pub fn max_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<ColumnDefinition, _>("MaxWidth", f64::INFINITY)
        }
    );

    ferro_property!(
        /// Defines the `MinWidth` property.
        pub fn min_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<ColumnDefinition, _>("MinWidth", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Width` property.
        pub fn width_property() -> StyledProperty<GridLength> {
            FerroProperty::register::<ColumnDefinition, _>("Width", GridLength::new(1.0, GridUnitType::Star))
        }
    );
} }

impl ColumnDefinition {
    fn static_constructor() {
        DefinitionBase::affects_parent_measure(&[
            Self::min_width_property().as_property(),
            Self::max_width_property().as_property(),
        ]);

        Self::width_property()
            .changed()
            .add_class_handler::<DefinitionBase>(DefinitionBase::on_user_size_property_changed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: DefinitionBase::construct(),
        }
    }

    /// Creates a column definition.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a column definition with the given width value and unit.
    pub fn with_value(value: f64, type_: GridUnitType) -> Ref<Self> {
        Self::with_width(GridLength::new(value, type_))
    }

    /// Creates a column definition with the given width.
    pub fn with_width(width: GridLength) -> Ref<Self> {
        let result = Self::new();
        result.set_value(Self::width_property(), width);
        result
    }

    /// The actual calculated width of the column.
    pub fn actual_width(&self) -> f64 {
        match self.parent() {
            Some(parent) => parent.get_final_column_definition_width(self.index()),
            None => 0.0,
        }
    }

    /// The maximum width of the column in DIPs.
    pub fn max_width(&self) -> f64 {
        self.get_value(Self::max_width_property())
    }

    pub fn set_max_width(&self, value: f64) {
        self.set_value(Self::max_width_property(), value)
    }

    /// The minimum width of the column in DIPs.
    pub fn min_width(&self) -> f64 {
        self.get_value(Self::min_width_property())
    }

    pub fn set_min_width(&self, value: f64) {
        self.set_value(Self::min_width_property(), value)
    }

    /// The width of the column.
    pub fn width(&self) -> GridLength {
        self.get_value(Self::width_property())
    }

    pub fn set_width(&self, value: GridLength) {
        self.set_value(Self::width_property(), value)
    }
}
