//! Port of `XamlIl/Runtime/IFerroXamlIlParentStackProvider.cs`.

use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::Rc;

/// Provides the objects being built above the current one.
///
/// A parent is an untyped value in its canonical form: the handle of a
/// class instance (`Ref<T>`), the object itself for a shared plain type.
pub trait IFerroXamlIlParentStackProvider {
    /// The parents of the object being built, the immediate parent first.
    fn parents(&self) -> Vec<BoxedValue>;

    /// The provider as an eager provider, if it is one (the `as` cast to
    /// [`IFerroXamlIlEagerParentStackProvider`]).
    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        None
    }
}

/// A parent stack provider that exposes its parents without enumerating:
/// its own stack as a list plus the provider of the enclosing scope.
pub trait IFerroXamlIlEagerParentStackProvider: IFerroXamlIlParentStackProvider {
    /// The parents this provider holds itself, the immediate parent last.
    ///
    /// The list is shared, not copied: an implementation that keeps its
    /// stack in an `Rc<Vec<_>>` returns it as it is (and mutates it with
    /// `Rc::make_mut`, which copies only while a caller still holds a
    /// snapshot).
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>>;

    /// The provider of the enclosing scope, whose parents follow the ones of
    /// this provider.
    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>>;
}

impl PartialEq for dyn IFerroXamlIlParentStackProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

impl PartialEq for dyn IFerroXamlIlEagerParentStackProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

ferro_markup_type!(interface dyn IFerroXamlIlParentStackProvider as "IFerroXamlIlParentStackProvider" {
    this: Rc<dyn IFerroXamlIlParentStackProvider>,
    handles: [Rc<dyn IFerroXamlIlParentStackProvider>, Option<Rc<dyn IFerroXamlIlParentStackProvider>>],
    properties: [
        Parents: Vec<BoxedValue> { get: |t: &Rc<dyn IFerroXamlIlParentStackProvider>| t.parents() },
    ],
});

ferro_markup_type!(interface dyn IFerroXamlIlEagerParentStackProvider as "IFerroXamlIlEagerParentStackProvider" {
    this: Rc<dyn IFerroXamlIlEagerParentStackProvider>,
    handles: [
        Rc<dyn IFerroXamlIlEagerParentStackProvider>,
        Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>>,
    ],
    interfaces: [Rc<dyn IFerroXamlIlParentStackProvider>],
    properties: [
        DirectParentsStack: Rc<Vec<BoxedValue>> {
            get: |t: &Rc<dyn IFerroXamlIlEagerParentStackProvider>| t.direct_parents_stack()
        },
        ParentProvider: Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
            get: |t: &Rc<dyn IFerroXamlIlEagerParentStackProvider>| t.parent_provider()
        },
    ],
});
