//! `...BindingBuilderExtensionsTests` (the tests of ReactiveUI.Binding's integration).
//!
//! Each call goes through one trait object on purpose: the builder implements both, and
//! `with_ferro` is offered on each.

use crate::{FerroAppBuilderExtensions, FerroBindingBuilderExtensions};
use ferroui_reactiveui::{IReactiveUIBindingBuilder, ReactiveUIBindingBuilder};
use ferroui_splat::{IAppBuilder, IMutableDependencyResolver, ModernDependencyResolver};
use std::rc::Rc;

fn builder() -> Rc<ReactiveUIBindingBuilder> {
    ReactiveUIBindingBuilder::new(Rc::new(ModernDependencyResolver::new()) as Rc<dyn IMutableDependencyResolver>, None)
}

#[test]
fn with_ferro_on_binding_builder_returns_the_same_builder() {
    let builder = builder();

    let result = (&*builder as &dyn IReactiveUIBindingBuilder).with_ferro();

    assert!(std::ptr::addr_eq(result as *const dyn IReactiveUIBindingBuilder, Rc::as_ptr(&builder)));
}

#[test]
fn with_ferro_on_app_builder_returns_the_same_builder() {
    let builder = builder();

    let result = (&*builder as &dyn IAppBuilder).with_ferro();

    assert!(Rc::ptr_eq(&result, &builder));
}

// Not ported: WithFerro_NullBuilder_ThrowsArgumentNullException - argument validation
