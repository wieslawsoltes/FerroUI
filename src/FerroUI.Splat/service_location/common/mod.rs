//! The helpers the resolvers share (`ServiceLocation/Common`).

mod entry;
mod registration;
mod resolver_exception_helpers;

pub(crate) use entry::{materialize_registrations, Entry};
pub(crate) use registration::Registration;
pub(crate) use resolver_exception_helpers::{run_swallowing_exceptions, throw_object_disposed};

use crate::service_location::service_type::ErasedService;
use std::rc::Rc;

/// The registrations of one key, type-erased, as the resolvers keep them.
pub(crate) type ErasedEntry = Rc<Entry<Registration<ErasedService>>>;
