//! UI-thread source snapshots. A candidate snapshot is scoped until its revision commits.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub type Sources = BTreeMap<String, String>;
ferroui_base::ferro_thread_local! { static CURRENT: RefCell<Rc<Sources>> = RefCell::new(Rc::new(Sources::new())); }
pub fn source(uri: &str) -> Option<String> {
    CURRENT
        .with(|slot| slot.borrow().get(uri).cloned())
        .or_else(|| published_source(uri))
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
        let registration = Self { id, compiled: Rc::new(RefCell::new((**sources.borrow()).clone())) };
        for uri in sources.borrow().keys() {
            registration.check_document(uri)?;
        }
        PUBLISHED.with(|published| {
            published.borrow_mut().insert(id, Rc::downgrade(sources));
        });
        COMPILED.with(|compiled| { compiled.borrow_mut().insert(id, Rc::downgrade(&registration.compiled)); });
        Ok(registration)
    }
    /// Keep the build baseline separate from accepted edits. A compiled template
    /// retained by application code still declares its original markup.
    pub fn register_compiled_source(&self, uri: &str, source: &str) {
        self.compiled.borrow_mut().entry(uri.to_owned()).or_insert_with(|| source.to_owned());
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
        let _ = COMPILED.try_with(|compiled| { compiled.borrow_mut().remove(&self.id); });
        let _ = PUBLISHED.try_with(|published| {
            published.borrow_mut().remove(&self.id);
        });
    }
}
fn published_source(uri: &str) -> Option<String> {
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
    COMPILED.with(|compiled| compiled.borrow().values().filter_map(std::rc::Weak::upgrade)
        .find_map(|sources| sources.borrow().get(uri).cloned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_baseline_survives_accepted_and_scoped_edits() {
        let uri = "ferres://baseline/View.xaml";
        let sources = Rc::new(RefCell::new(Rc::new(Sources::new())));
        let registration = SourceRegistration::new(&sources).unwrap();
        registration.register_compiled_source(uri, "original");
        *sources.borrow_mut() = Rc::new(Sources::from([(uri.into(), "accepted".into())]));
        registration.register_compiled_source(uri, "replacement");
        with_sources(Rc::new(Sources::from([(uri.into(), "candidate".into())])), || {
            assert_eq!(source(uri).as_deref(), Some("candidate"));
            assert_eq!(compiled_source(uri).as_deref(), Some("original"));
        });
        assert_eq!(source(uri).as_deref(), Some("accepted"));
        drop(registration);
        assert!(compiled_source(uri).is_none());
    }

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
}
