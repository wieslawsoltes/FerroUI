//! Control-related base types that live in the base library: style classes,
//! name scopes and resources.

mod child_name_scope;
mod classes;
mod i_deferred_content;
mod i_name_scope;
mod i_resource_dictionary;
mod i_resource_host;
mod i_resource_node;
mod i_resource_provider;
mod i_theme_variant_provider;
mod name_scope;
mod name_scope_extensions;
mod name_scope_locator;
mod pseudo_classes_extensions;
mod resource_dictionary;
mod resource_key;
mod resource_node_extensions;
mod resource_provider;
mod resources_changed_event_args;

pub use child_name_scope::ChildNameScope;
pub use classes::{Classes, IPseudoClasses};
pub use i_deferred_content::IDeferredContent;
pub use i_name_scope::{INameScope, NameScopeError, NameScopeRef};
pub use i_resource_dictionary::IResourceDictionary;
pub use i_resource_host::{IResourceHost, ResourceHostRef, WeakResourceHost};
pub use i_resource_node::{IResourceNode, ResourceValue};
pub use i_resource_provider::{resource_provider_ptr_eq, IResourceProvider};
pub use i_theme_variant_provider::IThemeVariantProvider;
pub use name_scope::NameScope;
pub use name_scope_extensions::NameScopeExtensions;
pub use name_scope_locator::NameScopeLocator;
pub use resource_dictionary::ResourceDictionary;
pub use resource_key::{ResourceKey, ResourceKeyObject};
pub use resource_node_extensions::{get_floating_resource_observable, ResourceConverter};
pub use resource_provider::{ResourceProvider, ResourceProviderImpl, ResourceProviderImplExt, ResourceProviderVTable};
pub use resources_changed_event_args::ResourcesChangedEventArgs;

#[cfg(test)]
mod resource_tests;
