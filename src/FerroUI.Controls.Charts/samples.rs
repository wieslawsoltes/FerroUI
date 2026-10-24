//! Observable, shared one-dimensional chart data using FerroUI's items contracts.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::utilities::{DateTimeOffset, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    PropertyValue, Ref, StyledProperty,
};
use ferroui_controls::{ItemsSource, ItemsSourceView};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A decoded sample, with source positions preserved even for unsupported data.
#[derive(Clone, Debug, PartialEq)]
pub enum SampleValue {
    Number(f64),
    Category(String),
    DateTime(DateTimeOffset),
}

impl SampleValue {
    /// Decode supported data without imposing chart-specific restrictions on ItemsSource.
    pub fn from_boxed(value: &BoxedValue) -> Option<Self> {
        if let Some(value) = value.downcast_ref::<Self>() {
            return Some(value.clone());
        }
        if let Some(value) = value.downcast_ref::<String>() {
            return Some(Self::Category(value.clone()));
        }
        if let Some(value) = value.downcast_ref::<&'static str>() {
            return Some(Self::Category((*value).to_owned()));
        }
        if let Some(value) = value.downcast_ref::<DateTimeOffset>() {
            return Some(Self::DateTime(*value));
        }
        macro_rules! number {
            ($($ty:ty),* $(,)?) => { $(
                if let Some(value) = value.downcast_ref::<$ty>() {
                    return Some(Self::Number(*value as f64));
                }
            )* };
        }
        number!(f64, f32, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
        None
    }
}

/// One dimension's values. The same instance can be shared by multiple series.
///
/// `ItemsSource` preserves arbitrary boxed values. Supported collections use the
/// existing FerroUI adapters, including live `Rc<FerroList<T>>`; unsupported
/// objects produce an empty view. Source changes invalidate a lazily decoded
/// snapshot. No chart owns a Samples object, and change subscriptions capture
/// only weak references.
#[repr(C)]
pub struct Samples {
    base: FerroObject,
    view: RefCell<Rc<ItemsSourceView>>,
    subscription: Cell<Option<u64>>,
    revision: Cell<u64>,
    snapshot: RefCell<Option<Rc<[Option<SampleValue>]>>>,
    changed: HandlerList<dyn Fn()>,
}

ferro_class!(Samples: FerroObject);
ferro_class_info!(Samples {
    new: Samples::new,
    markup: { content: ItemsSource },
});

impl FerroObjectImpl for Samples {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::items_source_property().as_property() {
            this.replace_view();
        }
    }
}

ferro_properties! {
    impl Samples {
        /// The underlying, optionally observable FerroUI item source.
        pub fn items_source_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Samples, _>("ItemsSource", None)
        }
    }
}

impl Samples {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            view: RefCell::new(ItemsSourceView::empty()),
            subscription: Cell::new(None),
            revision: Cell::new(0),
            snapshot: RefCell::new(None),
            changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn from_values<T: PropertyValue>(values: impl IntoIterator<Item = T>) -> Ref<Self> {
        Self::from_source(ItemsSource::from_values(values))
    }

    pub fn from_source(source: ItemsSource) -> Ref<Self> {
        let this = Self::new();
        this.set_source(Some(source));
        this
    }

    /// The original source object, including unsupported values. Unsupported
    /// scalar objects remain readable here and produce an empty chart view.
    pub fn items_source(&self) -> Option<BoxedValue> {
        self.get_value(Self::items_source_property())
    }

    pub fn set_items_source(&self, source: Option<BoxedValue>) {
        self.set_value(Self::items_source_property(), source);
    }

    /// Assign an existing FerroUI collection adapter without boxing it at the call site.
    pub fn set_source(&self, source: Option<ItemsSource>) {
        self.set_items_source(source.map(|value| Rc::new(value) as BoxedValue));
    }

    /// Resolve the source through FerroUI's normal collection conversions. This
    /// also permits model collection types registered with
    /// `ItemsSource::register_binding_conversion` to be used in bindings.
    pub fn source(&self) -> Option<ItemsSource> {
        let value = self.items_source()?;
        ValueTypes::try_convert(Some(&value), ValueType::of::<ItemsSource>())
            .flatten()
            .and_then(|converted| converted.downcast_ref::<ItemsSource>().cloned())
    }

    /// Standard FerroUI view of the current source.
    pub fn view(&self) -> Rc<ItemsSourceView> {
        self.view.borrow().clone()
    }

    pub fn count(&self) -> usize {
        self.view().count()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Raw source item; `None` for an out-of-range index or a null item.
    pub fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let view = self.view();
        if index < view.count() {
            view.get_at(index)
        } else {
            None
        }
    }

    /// Decoded item. Unsupported/null values remain gaps rather than shifting indexes.
    pub fn value_at(&self, index: usize) -> Option<SampleValue> {
        self.get_at(index)
            .as_ref()
            .and_then(SampleValue::from_boxed)
    }

    /// A shared snapshot, decoded once per revision and reused by every consumer.
    pub fn snapshot(&self) -> Rc<[Option<SampleValue>]> {
        if let Some(snapshot) = self.snapshot.borrow().as_ref() {
            return snapshot.clone();
        }
        let revision = self.revision();
        let snapshot: Rc<[Option<SampleValue>]> = self
            .view()
            .iter()
            .map(|value| value.as_ref().and_then(SampleValue::from_boxed))
            .collect::<Vec<_>>()
            .into();
        // A custom item source can mutate while read. Never cache its stale generation.
        if self.revision() == revision {
            *self.snapshot.borrow_mut() = Some(snapshot.clone());
        }
        snapshot
    }

    pub fn revision(&self) -> u64 {
        self.revision.get()
    }

    /// Observe both source replacement and changes within a notifying source.
    pub fn add_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.changed.add(handler)
    }

    pub fn remove_changed(&self, token: u64) -> bool {
        self.changed.remove(token)
    }

    fn replace_view(&self) {
        if let Some(token) = self.subscription.take() {
            self.view().remove_collection_changed(token);
        }
        let source = self.source();
        let view = ItemsSourceView::get_or_create(source.as_ref());
        let weak = self.to_ref().downgrade();
        let token = view.add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.invalidate();
            }
        }));
        *self.view.borrow_mut() = view;
        self.subscription.set(Some(token));
        self.invalidate();
    }

    fn invalidate(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
        self.snapshot.borrow_mut().take();
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }
}

impl Drop for Samples {
    fn drop(&mut self) {
        if let Some(token) = self.subscription.take() {
            self.view.get_mut().remove_collection_changed(token);
        }
    }
}
