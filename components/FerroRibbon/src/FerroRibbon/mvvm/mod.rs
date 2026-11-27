pub mod model_values;

mod ribbon_model_bindings;
pub use ribbon_model_bindings::{
    RibbonAction, RibbonBindingList, RibbonCollectionMirror, RibbonItemCreator,
    RibbonModelBindings, RibbonReference,
};

mod ribbon_element_factory;
pub use ribbon_element_factory::{
    RibbonElementFactory, RibbonElementFactoryImpl, RibbonElementFactoryImplExt,
    RibbonItemModelReference,
};

#[cfg(test)]
mod backstage_factory_tests;
#[cfg(test)]
mod element_factory_tests;

mod ribbon_element_factory_menus;

#[cfg(test)]
mod menu_factory_tests;

#[cfg(test)]
mod combo_factory_tests;

mod ribbon_element_factory_palette;

mod ribbon_element_factory_gallery;
mod ribbon_element_factory_parameters;

#[cfg(test)]
mod flyout_picker_tests;

mod ribbon_element_factory_custom;

#[cfg(test)]
mod custom_factory_tests;

mod custom_template_resources;
mod custom_template_theme;

#[cfg(test)]
mod custom_resource_tests;
