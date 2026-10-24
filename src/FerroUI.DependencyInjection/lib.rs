//! A UI-independent, thread-local port of Microsoft.Extensions.DependencyInjection.
//!
//! Services (including trait objects) are identified by `TypeId` and returned as `Rc<S>`.
//! Collections are mutable; built providers are immutable snapshots. Dispose providers and
//! scopes explicitly. Constructor factories declare dependencies for build-time validation;
//! arbitrary closures are opaque, as delegate factories are upstream.
//!
//! ```
//! use ferroui_dependency_injection::*;
//! use std::rc::Rc;
//! let mut services = ServiceCollection::new();
//! services.add_singleton::<String, _>(|_| Ok(Rc::new("hello".into())));
//! let provider = services.build_service_provider(Default::default())?;
//! assert_eq!(&*provider.get_required_service::<String>()?, "hello");
//! provider.dispose()?;
//! # Ok::<(), ServiceError>(())
//! ```

mod activator_utilities;
mod default_service_provider_factory;
mod service_collection;
mod service_descriptor;
mod service_provider;
mod service_type;
#[cfg(feature = "splat")]
pub mod splat;

pub use activator_utilities::*;
pub use default_service_provider_factory::*;
pub use service_collection::*;
pub use service_descriptor::*;
pub use service_provider::*;
pub use service_type::*;
