//! Revision-scoped native include entry points. Only generated dictionary calls
//! that have no copies inlined elsewhere in the group may use this interface.
use crate::XamlLoadError;
use ferroui_base::{controls::ResourceDictionary, metadata::IServiceProvider, Ref};
use std::{collections::BTreeMap, rc::Rc};

type Provider = Rc<dyn IServiceProvider>;
pub type DictionaryBuilder = Rc<dyn Fn(Provider) -> Result<Ref<ResourceDictionary>, XamlLoadError>>;
pub type Includes = BTreeMap<&'static str, DictionaryBuilder>;

#[cfg(all(feature = "binary-resources", feature = "runtime-populate"))]
mod scoped {
    use super::*;
    use std::{cell::RefCell, thread::LocalKey};

    type Slot<T> = LocalKey<RefCell<Option<T>>>;
    #[derive(Default)]
    pub(crate) struct Revision {
        dictionaries: Includes,
        #[cfg(feature = "compiled-updates")]
        blocks: super::updates::Blocks,
    }
    type Pending = (&'static ferroui_base::TypeInfo, Rc<Revision>);
    thread_local! {
        static NEXT: RefCell<Option<Pending>> = const { RefCell::new(None) };
        static ACTIVE: RefCell<Option<Rc<Revision>>> = const { RefCell::new(None) };
    }
    fn scoped<T: 'static, R>(
        slot: &'static Slot<T>,
        value: Option<T>,
        run: impl FnOnce() -> R,
    ) -> R {
        struct Restore<T: 'static>(&'static Slot<T>, Option<T>);
        impl<T> Drop for Restore<T> {
            fn drop(&mut self) {
                self.0.with(|s| s.replace(self.1.take()));
            }
        }
        let _guard = Restore(slot, slot.with(|s| s.replace(value)));
        run()
    }

    /// Offers the replacement plan to precisely the next matching class populate.
    pub fn with_revision<R>(
        root: &'static ferroui_base::TypeInfo,
        plan: Includes,
        run: impl FnOnce() -> R,
    ) -> R {
        scoped(
            &NEXT,
            Some((
                root,
                Rc::new(Revision {
                    dictionaries: plan,
                    ..Revision::default()
                }),
            )),
            run,
        )
    }

    #[cfg(feature = "compiled-updates")]
    pub(super) fn with_blocks<R>(
        root: &'static ferroui_base::TypeInfo,
        blocks: super::updates::Blocks,
        run: impl FnOnce() -> R,
    ) -> R {
        scoped(
            &NEXT,
            Some((
                root,
                Rc::new(Revision {
                    blocks,
                    ..Revision::default()
                }),
            )),
            run,
        )
    }

    #[cfg(feature = "compiled-updates")]
    pub(crate) fn block(
        id: &str,
    ) -> Option<crate::xaml_il::runtime::compiled::binary_resources::SharedBatch> {
        ACTIVE.with(|slot| slot.borrow().as_ref()?.blocks.get(id).cloned())
    }

    pub fn populate<R>(root: &ferroui_base::TypeInfo, run: impl FnOnce() -> R) -> R {
        let plan = NEXT.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot
                .as_ref()
                .is_some_and(|(uri, _)| std::ptr::eq(*uri, root))
            {
                slot.take().map(|(_, plan)| plan)
            } else {
                None
            }
        });
        activate(plan, run)
    }

    pub(crate) fn capture() -> Option<Rc<Revision>> {
        ACTIVE.with(|slot| slot.borrow().clone())
    }

    pub(crate) fn activate<R>(plan: Option<Rc<Revision>>, run: impl FnOnce() -> R) -> R {
        if plan.is_none() && ACTIVE.with(|slot| slot.borrow().is_none()) {
            return run();
        }
        scoped(&ACTIVE, plan, run)
    }

    pub fn dictionary(
        uri: &str,
        provider: Option<&Provider>,
    ) -> Option<Result<Ref<ResourceDictionary>, XamlLoadError>> {
        let build = ACTIVE.with(|slot| slot.borrow().as_ref()?.dictionaries.get(uri).cloned())?;
        Some(
            provider
                .cloned()
                .ok_or_else(|| {
                    XamlLoadError::with_message("Missing compiled include service provider")
                })
                .and_then(|provider| {
                    // An interpreted document is an independent execution unit. Its own
                    // constructions must never accidentally inherit a native revision.
                    activate(None, || build(provider))
                }),
        )
    }
}
#[cfg(feature = "compiled-updates")]
pub(crate) use scoped::block;
#[cfg(all(feature = "binary-resources", feature = "runtime-populate"))]
pub(crate) use scoped::{activate, capture, Revision};
#[cfg(feature = "compiled-updates")]
#[path = "operand_update.rs"]
mod updates;
#[cfg(all(feature = "binary-resources", feature = "runtime-populate"))]
pub use scoped::{dictionary, populate, with_revision};
#[cfg(feature = "compiled-updates")]
pub use updates::load_operand_update;

pub const ENABLED: bool = cfg!(all(
    feature = "binary-resources",
    feature = "runtime-populate"
));

#[cfg(not(all(feature = "binary-resources", feature = "runtime-populate")))]
pub fn dictionary(
    _: &str,
    _: Option<&Provider>,
) -> Option<Result<Ref<ResourceDictionary>, XamlLoadError>> {
    None
}
#[cfg(not(all(feature = "binary-resources", feature = "runtime-populate")))]
pub fn populate<R>(_: &ferroui_base::TypeInfo, run: impl FnOnce() -> R) -> R {
    run()
}
#[cfg(not(all(feature = "binary-resources", feature = "runtime-populate")))]
pub fn with_revision<R>(
    _: &'static ferroui_base::TypeInfo,
    _: Includes,
    run: impl FnOnce() -> R,
) -> R {
    run()
}

/// Only compiler-linked calls within the group consult the active revision.
/// Direct calls to a public document builder retain their ordinary semantics.
#[inline]
pub fn build_dictionary(
    uri: &str,
    provider: Option<Provider>,
    native: fn(Option<Provider>) -> Result<Ref<ResourceDictionary>, XamlLoadError>,
) -> Result<Ref<ResourceDictionary>, XamlLoadError> {
    match dictionary(uri, provider.as_ref()) {
        Some(edited) => edited,
        None => native(provider),
    }
}
