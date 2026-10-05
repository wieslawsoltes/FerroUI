use crate::data::{BindingMode, BindingPriority};
use crate::reactive::{IDisposable, IObservable, IObserver};
use crate::{BoxedValue, FerroObject, FerroObjectExtensions, FerroProperty, Ref};
use std::rc::Rc;

/// Holds a description of a binding for the binding indexer of
/// [`FerroObject`] ([`FerroObject::indexer`] and
/// [`FerroObject::bind_indexer`]).
#[derive(Clone, Default)]
pub struct IndexerDescriptor {
    /// The binding mode.
    pub mode: BindingMode,
    /// The binding priority.
    pub priority: BindingPriority,
    /// The source property.
    pub property: Option<&'static FerroProperty>,
    /// The source object.
    pub source: Option<Ref<FerroObject>>,
    /// The source observable. If `None`, then `source`.`property` is used.
    pub source_observable: Option<Rc<dyn IObservable<BoxedValue>>>,
}

impl IndexerDescriptor {
    /// Creates an empty description.
    pub fn new() -> Self {
        Self::default()
    }

    /// A description of the binding.
    pub fn description(&self) -> String {
        format!(
            "{}.{}",
            self.source.as_ref().map_or("", |s| s.get_type().name()),
            self.property.map_or("", |p| p.name())
        )
    }

    /// Modifies the binding mode.
    pub fn with_mode(mut self, mode: BindingMode) -> Self {
        self.mode = mode;
        self
    }

    /// Modifies the binding priority.
    pub fn with_priority(mut self, priority: BindingPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Makes a two-way binding (the `!` and `~` operators of the description).
impl std::ops::Not for IndexerDescriptor {
    type Output = IndexerDescriptor;

    fn not(self) -> IndexerDescriptor {
        self.with_mode(BindingMode::TwoWay)
    }
}

impl IObservable<BoxedValue> for IndexerDescriptor {
    /// Panics if the description has neither a source observable nor a
    /// source object and property.
    fn subscribe(&self, observer: Rc<dyn IObserver<BoxedValue>>) -> Rc<dyn IDisposable> {
        if self.source_observable.is_none() && self.source.is_none() {
            panic!("Cannot subscribe to IndexerDescriptor.");
        }
        let Some(property) = self.property else {
            panic!("Cannot subscribe to IndexerDescriptor.");
        };
        match (&self.source_observable, &self.source) {
            (Some(observable), _) => observable.subscribe(observer),
            (None, Some(source)) => source.get_observable_untyped(property).subscribe(observer),
            (None, None) => unreachable!("checked above"),
        }
    }
}

impl FerroProperty {
    /// Gets a description of a binding to the property, for the binding
    /// indexer of [`FerroObject`]. This is the `!` operator of a property:
    /// `!Border.BackgroundProperty` is `Border::background_property().bind()`,
    /// and the two-way forms `!!Property` and `~Property` are
    /// `!Property.bind()`.
    pub fn bind(&'static self) -> IndexerDescriptor {
        IndexerDescriptor { priority: BindingPriority::LocalValue, property: Some(self), ..IndexerDescriptor::default() }
    }
}
