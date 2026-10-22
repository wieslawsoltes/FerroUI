mod activator_utilities_tests;
mod dependency_injection_specification_tests;
mod disposal_tests;
mod keyed_dependency_injection_specification_tests;
mod service_collection_tests;
mod service_provider_validation_tests;
mod splat_tests;

use ferroui_dependency_injection::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

trait Service {
    fn value(&self) -> usize;
}
struct Value(usize);
impl Service for Value {
    fn value(&self) -> usize {
        self.0
    }
}
fn options() -> ServiceProviderOptions {
    ServiceProviderOptions {
        validate_scopes: true,
        validate_on_build: true,
    }
}
fn descriptor(lifetime: ServiceLifetime, value: usize) -> ServiceDescriptor {
    ServiceDescriptor::new::<dyn Service, _>(lifetime, move |_| Ok(Rc::new(Value(value))))
}

mod upstream;
