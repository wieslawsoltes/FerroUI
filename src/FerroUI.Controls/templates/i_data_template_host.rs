use super::DataTemplates;

/// Defines an element that has a [`DataTemplates`] collection.
pub trait IDataTemplateHost {
    /// The data templates for the element.
    fn data_templates(&self) -> DataTemplates;

    /// Whether `data_templates` is initialized.
    ///
    /// The `data_templates` property may be lazily initialized; if so this
    /// indicates whether the collection has been created.
    fn is_data_templates_initialized(&self) -> bool;
}
