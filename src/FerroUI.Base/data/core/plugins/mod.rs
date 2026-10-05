//! Binding plugins: how members are read, written, observed and validated.

mod binding_plugins;
mod data_validation_base;
mod exception_validation_plugin;
mod ferro_property_accessor_plugin;
mod i_property_accessor;
mod i_stream_plugin;
mod indei_validation_plugin;
mod inpc_property_accessor_plugin;
pub(crate) mod markup_members;
mod method_accessor_plugin;
mod property_accessor_base;
mod property_error;
mod property_info_accessor_factory;
mod property_info_accessor_plugin;
mod task_stream_plugin;
mod untyped_accessor_plugin;

pub use binding_plugins::BindingPlugins;
pub use data_validation_base::DataValidationBase;
pub use exception_validation_plugin::ExceptionValidationPlugin;
pub use ferro_property_accessor_plugin::{FerroPropertyAccessor, FerroPropertyAccessorPlugin};
pub(crate) use ferro_property_accessor_plugin::{get_property_value, property_value_type, set_property_value};
pub use i_property_accessor::{AccessorListener, IDataValidationPlugin, IPropertyAccessor, IPropertyAccessorPlugin};
pub use i_stream_plugin::{IStreamPlugin, ObservableStreamPlugin, ObservableValue};
pub use indei_validation_plugin::IndeiValidationPlugin;
pub use inpc_property_accessor_plugin::{
    AsNotifyPropertyChanged, InpcLookup, InpcPropertyAccessor, InpcPropertyAccessorPlugin,
};
pub use method_accessor_plugin::MethodAccessorPlugin;
pub use property_accessor_base::PropertyAccessorBase;
pub use property_error::PropertyError;
pub use property_info_accessor_factory::PropertyInfoAccessorFactory;
pub use property_info_accessor_plugin::{PropertyAccessorFactory, PropertyInfoAccessorPlugin};
pub use task_stream_plugin::{TaskStreamPlugin, TaskValue, TaskValueSource};
pub(crate) use untyped_accessor_plugin::{UntypedAccessorPlugin, UntypedMember};
pub use untyped_accessor_plugin::{
    UntypedCanExecute, UntypedCreateDelegate, UntypedExecute, UntypedGetter, UntypedSetter,
};

#[cfg(test)]
mod markup_members_tests;
