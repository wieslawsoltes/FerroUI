//! Explicit UI-thread source snapshots for native loaders in every build profile.
//! Global reload-session publication is available only in development builds.
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
pub type Sources = BTreeMap<String, String>;
ferroui_base::ferro_thread_local! { static CURRENT: RefCell<Rc<Sources>> = RefCell::new(Rc::new(Sources::new())); }

/// An immutable source view retained by native includes. It records actual native
/// include requests, even when a resource is missing or loaded after construction.
/// It never publishes a document or claims global hot-reload ownership.
#[derive(Clone)]
pub struct SourceSnapshot {
    sources: Rc<Sources>,
    unavailable: Rc<BTreeMap<String, String>>,
    requested: Rc<RefCell<BTreeSet<String>>>,
}

ferroui_base::ferro_thread_local! {
    static SNAPSHOT: RefCell<Option<SourceSnapshot>> = const { RefCell::new(None) };
}

impl SourceSnapshot {
    /// Keys are absolute loader URIs. Unavailable entries prevent fallback to a
    /// stale compiled/disk/published source while an open editor buffer is pending.
    pub fn new(sources: Sources, unavailable: BTreeMap<String, String>) -> Self {
        Self {
            sources: Rc::new(sources),
            unavailable: Rc::new(unavailable),
            requested: Rc::new(RefCell::new(BTreeSet::new())),
        }
    }

    /// Activate this snapshot for construction or deferred native loading. Nested
    /// scopes and unwinding restore the exact previous context.
    pub fn run<T>(&self, action: impl FnOnce() -> T) -> T {
        struct Restore(Option<SourceSnapshot>);
        impl Drop for Restore {
            fn drop(&mut self) {
                SNAPSHOT.with(|slot| *slot.borrow_mut() = self.0.take());
            }
        }
        let _restore = Restore(SNAPSHOT.with(|slot| slot.replace(Some(self.clone()))));
        with_sources(self.sources.clone(), action)
    }

    pub fn requested_uris(&self) -> BTreeSet<String> {
        self.requested.borrow().clone()
    }

    pub(crate) fn record_request(&self, uri: &str) {
        self.requested.borrow_mut().insert(uri.into());
    }
}

pub(crate) fn capture_snapshot() -> Option<SourceSnapshot> {
    SNAPSHOT.with(|slot| slot.borrow().clone())
}

/// Native includes preserve errors as errors instead of silently reading older
/// assets when an explicitly supplied source is temporarily unavailable.
pub(crate) fn try_source(uri: &str) -> Result<Option<String>, String> {
    if let Some(snapshot) = capture_snapshot() {
        snapshot.record_request(uri);
        if let Some(message) = snapshot.unavailable.get(uri) {
            return Err(message.clone());
        }
    }
    Ok(source(uri))
}

pub fn source(uri: &str) -> Option<String> {
    if let Some(snapshot) = capture_snapshot() {
        snapshot.record_request(uri);
        if snapshot.unavailable.contains_key(uri) {
            return None;
        }
    }
    let current = CURRENT.with(|slot| slot.borrow().get(uri).cloned());
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    let current = current.or_else(|| publication::published_source(uri));
    current
}
/// Restores the prior snapshot on normal return and unwind. Never transfers UI objects across threads.
pub fn with_sources<T>(sources: Rc<Sources>, action: impl FnOnce() -> T) -> T {
    struct Restore(Option<Rc<Sources>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|slot| *slot.borrow_mut() = self.0.take().unwrap());
        }
    }
    let _restore = Restore(Some(CURRENT.with(|slot| slot.replace(sources))));
    action()
}

#[cfg(all(feature = "hot-reload", debug_assertions))]
pub use publication::{compiled_source, SourceRegistration};

#[cfg(all(feature = "hot-reload", debug_assertions))]
mod publication {
    use super::*;

    /// Owns publication of one session's accepted documents. Scoped candidate sources always win.
    /// A document can belong to only one live session on this UI thread.
    pub struct SourceRegistration {
        id: u64,
        compiled: Rc<RefCell<Sources>>,
    }
    ferroui_base::ferro_thread_local! {
        static PUBLISHED: RefCell<BTreeMap<u64, std::rc::Weak<RefCell<Rc<Sources>>>>> = RefCell::new(BTreeMap::new());
        static COMPILED: RefCell<BTreeMap<u64, std::rc::Weak<RefCell<Sources>>>> = RefCell::new(BTreeMap::new());
        static NEXT_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
    }
    impl SourceRegistration {
        pub fn new(sources: &Rc<RefCell<Rc<Sources>>>) -> Result<Self, String> {
            let id = NEXT_ID.with(|next| {
                let id = next.get();
                next.set(
                    id.checked_add(1)
                        .expect("source registration IDs exhausted"),
                );
                id
            });
            let registration = Self {
                id,
                compiled: Rc::new(RefCell::new((**sources.borrow()).clone())),
            };
            for uri in sources.borrow().keys() {
                registration.check_document(uri)?;
            }
            PUBLISHED.with(|published| {
                published.borrow_mut().insert(id, Rc::downgrade(sources));
            });
            COMPILED.with(|compiled| {
                compiled
                    .borrow_mut()
                    .insert(id, Rc::downgrade(&registration.compiled));
            });
            Ok(registration)
        }
        /// Keep the build baseline separate from accepted edits. A compiled template
        /// retained by application code still declares its original markup.
        pub fn register_compiled_source(&self, uri: &str, source: &str) {
            self.compiled
                .borrow_mut()
                .entry(uri.to_owned())
                .or_insert_with(|| source.to_owned());
        }
        pub fn check_document(&self, uri: &str) -> Result<(), String> {
            PUBLISHED.with(|published| {
                let mut published = published.borrow_mut();
                published.retain(|_, sources| sources.strong_count() != 0);
                for (id, sources) in published.iter() {
                    if *id != self.id
                        && sources
                            .upgrade()
                            .is_some_and(|sources| sources.borrow().contains_key(uri))
                    {
                        return Err(format!(
                            "Document already belongs to a reload session: {uri}"
                        ));
                    }
                }
                Ok(())
            })
        }
    }
    impl Drop for SourceRegistration {
        fn drop(&mut self) {
            // A session can itself live in TLS and outlive these weak registries.
            let _ = COMPILED.try_with(|compiled| {
                compiled.borrow_mut().remove(&self.id);
            });
            let _ = PUBLISHED.try_with(|published| {
                published.borrow_mut().remove(&self.id);
            });
        }
    }
    pub(super) fn published_source(uri: &str) -> Option<String> {
        PUBLISHED.with(|published| {
            published
                .borrow()
                .values()
                .filter_map(std::rc::Weak::upgrade)
                .find_map(|sources| sources.borrow().get(uri).cloned())
        })
    }

    /// Immutable source from a session's build baseline, never a scoped draft or
    /// a later accepted edit. Used to identify parts made by compiled templates.
    pub fn compiled_source(uri: &str) -> Option<String> {
        COMPILED.with(|compiled| {
            compiled
                .borrow()
                .values()
                .filter_map(std::rc::Weak::upgrade)
                .find_map(|sources| sources.borrow().get(uri).cloned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    #[test]
    fn compiled_baseline_survives_accepted_and_scoped_edits() {
        let uri = "ferres://baseline/View.xaml";
        let sources = Rc::new(RefCell::new(Rc::new(Sources::new())));
        let registration = SourceRegistration::new(&sources).unwrap();
        registration.register_compiled_source(uri, "original");
        *sources.borrow_mut() = Rc::new(Sources::from([(uri.into(), "accepted".into())]));
        registration.register_compiled_source(uri, "replacement");
        with_sources(
            Rc::new(Sources::from([(uri.into(), "candidate".into())])),
            || {
                assert_eq!(source(uri).as_deref(), Some("candidate"));
                assert_eq!(compiled_source(uri).as_deref(), Some("original"));
            },
        );
        assert_eq!(source(uri).as_deref(), Some("accepted"));
        drop(registration);
        assert!(compiled_source(uri).is_none());
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    #[test]
    fn registration_can_outlive_its_thread_local_registry() {
        std::thread::spawn(|| {
            ferroui_base::ferro_thread_local! {
                static OWNER: RefCell<Option<(Rc<RefCell<Rc<Sources>>>, SourceRegistration)>> = const { RefCell::new(None) };
            }
            // Initialize the owner first: its destructor runs after PUBLISHED.
            OWNER.with(|owner| {
                let sources = Rc::new(RefCell::new(Rc::new(Sources::from([
                    ("ferres://teardown/View.xaml".into(), "<Border/>".into()),
                ]))));
                let registration = SourceRegistration::new(&sources).unwrap();
                *owner.borrow_mut() = Some((sources, registration));
            });
            assert_eq!(source("ferres://teardown/View.xaml").as_deref(), Some("<Border/>"));
        }).join().unwrap();
    }

    #[test]
    fn scoped_snapshots_record_missing_requests_and_restore_context_on_unwind() {
        let uri = "ferres://snapshot/Value.xaml";
        let missing = "ferres://snapshot/Missing.xaml";
        let unavailable = "ferres://snapshot/Pending.xaml";
        let outer = SourceSnapshot::new(
            Sources::from([(uri.into(), "outer".into())]),
            BTreeMap::new(),
        );
        let inner = SourceSnapshot::new(
            Sources::from([(uri.into(), "inner".into())]),
            BTreeMap::from([(unavailable.into(), "Apply pending source".into())]),
        );
        outer.run(|| {
            assert_eq!(try_source(uri).unwrap().as_deref(), Some("outer"));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                inner.run(|| {
                    assert_eq!(try_source(uri).unwrap().as_deref(), Some("inner"));
                    assert!(try_source(missing).unwrap().is_none());
                    assert_eq!(try_source(unavailable).unwrap_err(), "Apply pending source");
                    panic!("restore scoped preview");
                })
            }));
            assert!(result.is_err());
            assert_eq!(try_source(uri).unwrap().as_deref(), Some("outer"));
        });
        assert!(source(uri).is_none());
        assert_eq!(outer.requested_uris(), BTreeSet::from([uri.into()]));
        assert_eq!(
            inner.requested_uris(),
            BTreeSet::from([uri.into(), missing.into(), unavailable.into()])
        );
    }
}
