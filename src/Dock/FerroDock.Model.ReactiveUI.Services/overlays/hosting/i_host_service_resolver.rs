// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{HostServiceValue, HostTypeRegistry};
use ferroui_reactiveui::IScreen;
use std::any::TypeId;
use std::rc::Rc;
/// Object-safe core of the source generic host service resolver.
pub trait IHostServiceResolver {
    /// Resolves the requested native reference type, preserving its original object identity.
    fn resolve_erased(&self, screen: Rc<dyn IScreen>, target: TypeId) -> Option<HostServiceValue>;
    /// Native projection registry shared by this composition; absent for resolvers
    /// that supply their own type-query mechanism.
    fn native_types(&self) -> Option<Rc<HostTypeRegistry>> {
        None
    }
}
/// Generic source Resolve<TService> on concrete and erased resolver references.
pub trait HostServiceResolverExt: IHostServiceResolver {
    /// Resolves the requested reference type without cloning the service object.
    fn resolve<T: ?Sized + 'static>(&self, screen: Rc<dyn IScreen>) -> Option<Rc<T>> {
        self.resolve_erased(screen, TypeId::of::<T>())?.get()
    }
}
impl<R: IHostServiceResolver + ?Sized> HostServiceResolverExt for R {}
