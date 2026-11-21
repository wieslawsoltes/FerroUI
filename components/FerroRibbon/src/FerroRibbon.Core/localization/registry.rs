//! Shared translation tables and their registration transaction. RibbonStrings.cs.
//! Copyright (c) 2026 Wiesław Šoltés. MIT.
use super::{localization_data, ribbon_strings::literal};
use crate::{RibbonString, text::ordinal::OrdinalKey};
use std::{
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError},
    thread::{self, ThreadId},
};

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Rows<K, V> {
    values: HashMap<K, V>,
    keys: Vec<K>,
    version: i32,
}
pub(super) struct Ordered<K, V> {
    rows: Mutex<Rows<K, V>>,
}
impl<K: Eq + Hash + Clone + Send, V: Clone + Send> Ordered<K, V> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rows: Mutex::new(Rows {
                values: HashMap::new(),
                keys: Vec::new(),
                version: 0,
            }),
        })
    }
    pub fn len(&self) -> usize {
        lock(&self.rows).keys.len()
    }
    pub fn get(&self, key: &K) -> Option<V> {
        lock(&self.rows).values.get(key).cloned()
    }
    pub fn insert(&self, key: K, value: V) {
        let old = {
            let mut rows = lock(&self.rows);
            if !rows.values.contains_key(&key) {
                rows.keys.push(key.clone());
                rows.version = rows.version.wrapping_add(1);
            }
            rows.values.insert(key, value)
        };
        drop(old);
    }
    pub fn snapshot(&self) -> Vec<(K, V)> {
        let rows = lock(&self.rows);
        rows.keys
            .iter()
            .map(|key| {
                (
                    key.clone(),
                    rows.values.get(key).expect("registered key").clone(),
                )
            })
            .collect()
    }
    pub fn cursor(self: &Arc<Self>) -> impl Iterator<Item = (K, V)> + use<K, V> {
        let owner = self.clone();
        let version = lock(&self.rows).version;
        let mut index = 0;
        std::iter::from_fn(move || {
            let rows = lock(&owner.rows);
            if rows.version != version {
                drop(rows);
                panic!("InvalidOperation: Collection was modified");
            }
            // Append-only translation keys retain source order. The key index
            // avoids repeatedly walking the dictionary from the beginning.
            let value = rows.keys.get(index).map(|k| {
                (
                    k.clone(),
                    rows.values.get(k).expect("registered key").clone(),
                )
            });
            index += 1;
            value
        })
    }
}
pub(super) type Values = Arc<Ordered<RibbonString, Option<RibbonString>>>;
type Cultures = Arc<Ordered<OrdinalKey, Values>>;

// Source holds its registry lock across user-supplied enumeration and virtual
// culture getters. A short storage mutex alone permits interleaved registrations;
// holding it across callbacks deadlocks on nested Register/ForCulture. This
// private transaction guard serializes those two product operations, reentrantly.
// It is not exposed as a general lock or CLR service.
#[derive(Default)]
struct GateState {
    owner: Option<ThreadId>,
    depth: usize,
}
#[derive(Default)]
pub(super) struct RegistryGate {
    state: Mutex<GateState>,
    available: Condvar,
}
pub(super) struct RegistryScope<'a> {
    gate: &'a RegistryGate,
    _thread: PhantomData<Rc<()>>,
}
impl RegistryGate {
    pub fn enter(&self) -> RegistryScope<'_> {
        let current = thread::current().id();
        let mut state = lock(&self.state);
        while state.owner.is_some_and(|owner| owner != current) {
            state = self
                .available
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        state.owner = Some(current);
        state.depth += 1;
        RegistryScope {
            gate: self,
            _thread: PhantomData,
        }
    }
}
impl Drop for RegistryScope<'_> {
    fn drop(&mut self) {
        let mut state = lock(&self.gate.state);
        state.depth -= 1;
        if state.depth == 0 {
            state.owner = None;
            self.gate.available.notify_one();
        }
    }
}
enum KeySource {
    Cultures(Cultures),
    English(Values),
}
pub struct RibbonLocalizationKeys {
    source: KeySource,
}
impl RibbonLocalizationKeys {
    pub fn count(&self) -> i32 {
        i32::try_from(match &self.source {
            KeySource::Cultures(v) => v.len(),
            KeySource::English(v) => v.len(),
        })
        .expect("source Int32 Count")
    }
    pub fn iter(&self) -> Box<dyn Iterator<Item = RibbonString> + Send> {
        match &self.source {
            KeySource::Cultures(v) => Box::new(v.cursor().map(|(k, _)| k.0)),
            KeySource::English(v) => Box::new(v.cursor().map(|(k, _)| k)),
        }
    }
}
pub(super) struct Registry {
    pub cultures: Cultures,
    pub culture_keys: Arc<RibbonLocalizationKeys>,
    pub english_keys: Arc<RibbonLocalizationKeys>,
    pub gate: RegistryGate,
}
pub(super) fn registry() -> &'static Registry {
    static SHARED: OnceLock<Registry> = OnceLock::new();
    SHARED.get_or_init(|| {
        let cultures = Ordered::new();
        for &(name, rows) in localization_data::CULTURES {
            let values = Ordered::new();
            for &(key, value) in rows {
                values.insert(literal(key), Some(literal(value)));
            }
            cultures.insert(OrdinalKey(literal(name)), values);
        }
        let english = cultures
            .get(&OrdinalKey(literal("en")))
            .expect("English table");
        Registry {
            culture_keys: Arc::new(RibbonLocalizationKeys {
                source: KeySource::Cultures(cultures.clone()),
            }),
            english_keys: Arc::new(RibbonLocalizationKeys {
                source: KeySource::English(english),
            }),
            cultures,
            gate: RegistryGate::default(),
        }
    })
}
