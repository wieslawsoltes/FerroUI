//! The resolver whose registrations are global (`ServiceLocation/GlobalGenericFirst`): every
//! instance of [`GlobalGenericFirstDependencyResolver`] shares the containers of the thread.

mod container;
mod contract_container;
mod global_generic_first_dependency_resolver;

#[allow(unused_imports)]
pub(crate) use container::{Container, ErasedContainer};
#[allow(unused_imports)]
pub(crate) use contract_container::{ContractContainer, ErasedContractContainer};
pub use global_generic_first_dependency_resolver::GlobalGenericFirstDependencyResolver;
