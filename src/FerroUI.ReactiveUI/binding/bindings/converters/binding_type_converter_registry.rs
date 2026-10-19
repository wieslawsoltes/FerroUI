//! `BindingTypeConverterRegistry`
//! (`ReactiveUI.Binding/Bindings/Converters/BindingTypeConverterRegistry.cs`).

use crate::{ConverterType, ConverterTypePair, IBindingTypeConverter};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// The converters of one pair of types, in the order of their registration.
type ConverterList = Rc<Vec<Rc<dyn IBindingTypeConverter>>>;

/// The registered converters: a published snapshot that a registration replaces (copy on write).
struct Snapshot {
    /// The pairs in the order of their first registration (upstream's dictionary keeps it).
    pairs: Vec<ConverterTypePair>,
    converters_by_type_pair: HashMap<ConverterTypePair, ConverterList>,
}

/// The typed converters, by the exact pair of their source and target types
/// (`BindingTypeConverterRegistry`). Of the converters of a pair, the one with the highest positive
/// affinity is the one used; on a tie the first registered wins.
///
/// A lookup reads a snapshot of the registrations, so a converter may register others while it is
/// asked for its affinity.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's lock and volatile reads serve readers on other
// threads; the registry is per thread here (`docs/porting/reactive.md`, section 5.1) and keeps the
// copy-on-write snapshot alone.
#[derive(Default)]
pub struct BindingTypeConverterRegistry {
    snapshot: RefCell<Option<Rc<Snapshot>>>,
}

impl BindingTypeConverterRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    fn current(&self) -> Option<Rc<Snapshot>> {
        self.snapshot.borrow().clone()
    }

    /// Registers `converter` under the pair of its source and target types (`Register`).
    pub fn register(&self, converter: Rc<dyn IBindingTypeConverter>) {
        let key = ConverterTypePair::new(converter.from_type(), converter.to_type());
        let current = self.current();
        let mut pairs = current.as_ref().map(|snapshot| snapshot.pairs.clone()).unwrap_or_default();
        let mut converters_by_type_pair = current
            .as_ref()
            .map(|snapshot| Self::clone_registry_shallow(&snapshot.converters_by_type_pair))
            .unwrap_or_default();
        let mut list: Vec<Rc<dyn IBindingTypeConverter>> = match converters_by_type_pair.get(&key) {
            Some(existing) => existing.as_ref().clone(),
            None => {
                pairs.push(key);
                Vec::with_capacity(4)
            }
        };
        list.push(converter);
        converters_by_type_pair.insert(key, Rc::new(list));
        *self.snapshot.borrow_mut() = Some(Rc::new(Snapshot { pairs, converters_by_type_pair }));
    }

    /// The converter of the highest positive affinity from `from_type` to `to_type`, or `None`
    /// (`TryGetConverter`).
    pub fn try_get_converter(&self, from_type: ConverterType, to_type: ConverterType) -> Option<Rc<dyn IBindingTypeConverter>> {
        let snapshot = self.current()?;
        let list = snapshot.converters_by_type_pair.get(&ConverterTypePair::new(from_type, to_type))?;
        let mut best: Option<&Rc<dyn IBindingTypeConverter>> = None;
        let mut best_score = -1;
        for converter in list.iter() {
            let score = converter.get_affinity_for_objects();
            if score <= best_score || score <= 0 {
                continue;
            }
            best_score = score;
            best = Some(converter);
        }
        best.cloned()
    }

    /// Every registered converter, pair by pair in the order the pairs were first registered
    /// (`GetAllConverters`).
    pub fn get_all_converters(&self) -> Vec<Rc<dyn IBindingTypeConverter>> {
        let Some(snapshot) = self.current() else {
            return Vec::new();
        };
        snapshot
            .pairs
            .iter()
            .filter_map(|pair| snapshot.converters_by_type_pair.get(pair))
            .flat_map(|list| list.iter().cloned())
            .collect()
    }

    /// A copy of `source` that shares its lists of converters (`CloneRegistryShallow`).
    #[doc(hidden)]
    pub fn clone_registry_shallow(
        source: &HashMap<ConverterTypePair, Rc<Vec<Rc<dyn IBindingTypeConverter>>>>,
    ) -> HashMap<ConverterTypePair, Rc<Vec<Rc<dyn IBindingTypeConverter>>>> {
        let mut clone = HashMap::with_capacity(source.len());
        for (key, list) in source {
            clone.insert(*key, list.clone());
        }
        clone
    }
}

impl fmt::Debug for BindingTypeConverterRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.current().map_or(0, |snapshot| snapshot.pairs.len());
        write!(f, "BindingTypeConverterRegistry: Typed converters for {count} type pairs")
    }
}
