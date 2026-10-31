use super::{CollectionChange, Event, INotifyCollectionChanged, INotifyPropertyChanged, ModelTypes};
use crate::collections::{FerroDictionary, NotifyCollectionChangedEventArgs};
use crate::data::core::{PropertyKind, Value, ValueType};
use crate::data::BindingError;
use crate::{BoxedValue, PropertyValue};
use std::any::TypeId;
use std::cell::{OnceCell, RefCell};
use std::fmt::Display;
use std::hash::Hash;
use std::rc::Rc;

/// The untyped form of an indexer (`this[..]`), for bindings with an indexer
/// in their path (`Items[key]`, `Grid[1,2]`).
///
/// A model type declares its indexer with
/// [`ModelTypeBuilder::indexer`](super::ModelTypeBuilder::indexer). The
/// binding converts the textual arguments of the path to the declared
/// parameter types before calling [`get`](Self::get) or [`set`](Self::set).
///
/// A binding re-reads the indexer when the object raises a collection change
/// notification, or a property change notification for the indexer
/// ([`INDEXER_NAME`](crate::data::core::INDEXER_NAME)).
pub trait IBindableIndexer {
    /// The types of the index parameters, in order.
    fn index_parameter_types(&self) -> Vec<ValueType>;

    /// The type of the values of the indexer.
    fn value_type(&self) -> ValueType;

    /// Whether the indexer has a setter.
    fn can_set(&self) -> bool {
        true
    }

    /// Reads the value at `arguments`, which hold exactly the declared
    /// parameter types. An error is what the getter of a managed indexer
    /// throws: a missing key, an index out of range.
    fn get(&self, arguments: &[BoxedValue]) -> Result<Option<BoxedValue>, BindingError>;

    /// Writes the value at `arguments`. Returns `Ok(false)` if the indexer
    /// has no setter.
    fn set(&self, arguments: &[BoxedValue], value: Option<&BoxedValue>) -> Result<bool, BindingError>;
}

/// Untyped access to an array: a fixed-size collection accessed with integer
/// indexes, one per dimension.
pub trait IBindableArray {
    /// The number of dimensions.
    fn rank(&self) -> usize;

    /// The type of the elements.
    fn element_type(&self) -> ValueType;

    /// The element at `indexes`.
    fn get_value(&self, indexes: &[i32]) -> Result<Option<BoxedValue>, BindingError>;

    /// Replaces the element at `indexes`.
    fn set_value(&self, indexes: &[i32], value: Option<&BoxedValue>) -> Result<(), BindingError>;
}

fn invalid_value(value: Option<&BoxedValue>, target: ValueType) -> BindingError {
    BindingError::message(format!(
        "Object of type '{}' cannot be converted to type '{}'.",
        value.map_or("null", |v| (**v).type_name()),
        target
    ))
}

fn argument<'a, T: 'static>(arguments: &'a [BoxedValue], index: usize) -> Result<&'a T, BindingError> {
    arguments.get(index).and_then(|a| a.downcast_ref::<T>()).ok_or_else(|| {
        BindingError::message(format!(
            "Object of type '{}' cannot be converted to type '{}'.",
            arguments.get(index).map_or("null", |v| (**v).type_name()),
            std::any::type_name::<T>()
        ))
    })
}

/// An array that bindings can index into: one or more dimensions of fixed
/// lengths, stored in row-major order.
///
/// As a model object it has identity equality.
pub struct BindableArray<T: PropertyValue> {
    items: ferroui_system::SharedArray<T>,
    lengths: Vec<usize>,
}

impl<T: PropertyValue> BindableArray<T> {
    /// Creates a one-dimensional array.
    pub fn new(items: impl IntoIterator<Item = T>) -> Rc<Self> {
        let items: Vec<T> = items.into_iter().collect();
        let lengths = vec![items.len()];
        Self::create(items, lengths)
    }

    /// Creates an array with the given length of each dimension, from its
    /// elements in row-major order. Panics if the number of elements is not
    /// the product of the lengths.
    pub fn with_lengths(lengths: &[usize], items: impl IntoIterator<Item = T>) -> Rc<Self> {
        let items: Vec<T> = items.into_iter().collect();
        if lengths.is_empty() || lengths.iter().product::<usize>() != items.len() {
            panic!("The number of elements does not match the lengths of the array.");
        }
        Self::create(items, lengths.to_vec())
    }

    fn create(items: Vec<T>, lengths: Vec<usize>) -> Rc<Self> {
        if !ModelTypes::is_registered(TypeId::of::<Self>()) {
            ModelTypes::register::<Self>(|b| b.array().indexer());
        }
        register_bindable_array::<T>();
        Rc::new(Self { items: ferroui_system::SharedArray::new(items), lengths })
    }

    /// View the original shared storage without copying its elements.
    pub fn from_shared(items:ferroui_system::SharedArray<T>)->Rc<Self> {
        let lengths=vec![items.len()];
        Self::from_shared_lengths(items,lengths)
    }
    fn from_shared_lengths(items:ferroui_system::SharedArray<T>,lengths:Vec<usize>)->Rc<Self> {
        if !ModelTypes::is_registered(TypeId::of::<Self>()) {ModelTypes::register::<Self>(|b| b.array().indexer());}
        register_bindable_array::<T>();
        Rc::new(Self{items,lengths})
    }
    pub fn shared(&self)->ferroui_system::SharedArray<T>{self.items.clone()}

    /// The length of each dimension.
    pub fn lengths(&self) -> &[usize] {
        &self.lengths
    }

    /// The total number of elements.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The element at `indexes`. Panics if out of range.
    pub fn get(&self, indexes: &[usize]) -> T {
        let offset = self.offset(indexes).expect("Index was outside the bounds of the array.");
        self.items.get(offset as i32).expect("checked array offset")
    }

    /// Replaces the element at `indexes`. Panics if out of range.
    pub fn set(&self, indexes: &[usize], value: T) {
        let offset = self.offset(indexes).expect("Index was outside the bounds of the array.");
        self.items.set(offset as i32,value).expect("checked array offset");
    }

    /// The elements in row-major order.
    pub fn to_vec(&self) -> Vec<T> {
        self.items.to_vec()
    }

    fn offset(&self, indexes: &[usize]) -> Option<usize> {
        if indexes.len() != self.lengths.len() {
            return None;
        }
        let mut offset = 0;
        for (index, length) in indexes.iter().zip(&self.lengths) {
            if index >= length {
                return None;
            }
            offset = offset * length + index;
        }
        Some(offset)
    }

    fn checked_offset(&self, indexes: &[i32]) -> Result<usize, BindingError> {
        if indexes.len() != self.lengths.len() {
            return Err(BindingError::message("Indices length does not match the array rank."));
        }
        let indexes: Option<Vec<usize>> = indexes.iter().map(|i| usize::try_from(*i).ok()).collect();
        indexes
            .and_then(|i| self.offset(&i))
            .ok_or_else(|| BindingError::message("Index was outside the bounds of the array."))
    }
}

impl<T: PropertyValue> PartialEq for BindableArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items.ptr_eq(&other.items) && self.lengths==other.lengths
    }
}

impl<T: PropertyValue> IBindableArray for BindableArray<T> {
    fn rank(&self) -> usize {
        self.lengths.len()
    }

    fn element_type(&self) -> ValueType {
        ValueType::of::<T>()
    }

    fn get_value(&self, indexes: &[i32]) -> Result<Option<BoxedValue>, BindingError> {
        let offset = self.checked_offset(indexes)?;
        Ok(Some(Rc::new(self.items.get(offset as i32).expect("checked array offset"))))
    }

    fn set_value(&self, indexes: &[i32], value: Option<&BoxedValue>) -> Result<(), BindingError> {
        let offset = self.checked_offset(indexes)?;
        let value = Value::<T>::from_untyped(value).ok_or_else(|| invalid_value(value, ValueType::of::<T>()))?;
        self.items.set(offset as i32,value).expect("checked array offset");
        Ok(())
    }
}

impl<T: PropertyValue> IBindableIndexer for BindableArray<T> {
    fn index_parameter_types(&self) -> Vec<ValueType> {
        vec![ValueType::of::<i32>(); self.lengths.len()]
    }

    fn value_type(&self) -> ValueType {
        ValueType::of::<T>()
    }

    fn get(&self, arguments: &[BoxedValue]) -> Result<Option<BoxedValue>, BindingError> {
        let indexes: Result<Vec<i32>, BindingError> =
            (0..arguments.len()).map(|i| argument::<i32>(arguments, i).copied()).collect();
        self.get_value(&indexes?)
    }

    fn set(&self, arguments: &[BoxedValue], value: Option<&BoxedValue>) -> Result<bool, BindingError> {
        let indexes: Result<Vec<i32>, BindingError> =
            (0..arguments.len()).map(|i| argument::<i32>(arguments, i).copied()).collect();
        self.set_value(&indexes?, value).map(|()| true)
    }
}

/// A notifying dictionary that bindings can index into (`Items[key]`): a
/// [`FerroDictionary`] together with the untyped change event that binding
/// indexers subscribe to.
///
/// As a model object it has identity equality.
pub struct BindableDictionary<K: Eq + Hash + Clone + Display + 'static, V: PropertyValue> {
    items: FerroDictionary<K, V>,
    changed: Event<CollectionChange>,
    forwarder: OnceCell<u64>,
}

impl<K: Eq + Hash + Clone + Display + 'static, V: PropertyValue> BindableDictionary<K, V> {
    /// Creates a dictionary with the given entries.
    pub fn new(entries: impl IntoIterator<Item = (K, V)>) -> Rc<Self> {
        let this = Rc::new(Self {
            items: FerroDictionary::from_dictionary(entries),
            changed: Event::new(),
            forwarder: OnceCell::new(),
        });
        let weak = Rc::downgrade(&this);
        let token =
            this.items.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, (K, V)>| {
                if let Some(this) = weak.upgrade() {
                    this.changed.raise(&CollectionChange {
                        action: e.action,
                        new_starting_index: e.new_starting_index,
                        new_count: e.new_items.len(),
                        old_starting_index: e.old_starting_index,
                        old_count: e.old_items.len(),
                    });
                }
            }));
        let _ = this.forwarder.set(token);
        if !ModelTypes::is_registered(TypeId::of::<Self>()) {
            ModelTypes::register::<Self>(|b| b.indexer().notify_collection_changed().notify_property_changed());
        }
        this
    }

    /// The typed dictionary.
    pub fn items(&self) -> &FerroDictionary<K, V> {
        &self.items
    }
}

impl<K: Eq + Hash + Clone + Display + 'static, V: PropertyValue> PartialEq for BindableDictionary<K, V> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<K: Eq + Hash + Clone + Display + 'static, V: PropertyValue> INotifyCollectionChanged
    for BindableDictionary<K, V>
{
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.changed
    }
}

impl<K: Eq + Hash + Clone + Display + 'static, V: PropertyValue> INotifyPropertyChanged for BindableDictionary<K, V> {
    fn property_changed(&self) -> &Event<str> {
        self.items.property_changed()
    }
}

impl<K: Eq + Hash + Clone + Display + 'static, V: PropertyValue> IBindableIndexer for BindableDictionary<K, V> {
    fn index_parameter_types(&self) -> Vec<ValueType> {
        vec![ValueType::of::<K>()]
    }

    fn value_type(&self) -> ValueType {
        ValueType::of::<V>()
    }

    fn get(&self, arguments: &[BoxedValue]) -> Result<Option<BoxedValue>, BindingError> {
        let key = argument::<K>(arguments, 0)?;
        match self.items.try_get_value(key) {
            Some(value) => Ok(Some(Rc::new(value))),
            None => Err(BindingError::message(format!("The given key '{key}' was not present in the dictionary."))),
        }
    }

    fn set(&self, arguments: &[BoxedValue], value: Option<&BoxedValue>) -> Result<bool, BindingError> {
        let key = argument::<K>(arguments, 0)?;
        let value = Value::<V>::from_untyped(value).ok_or_else(|| invalid_value(value, ValueType::of::<V>()))?;
        self.items.set(key.clone(), value);
        Ok(true)
    }
}

// Native array handles are reference types. The type-system hosts read this process table.
static ARRAY_HANDLES:std::sync::Mutex<Vec<(ValueType,ValueType)>>=std::sync::Mutex::new(Vec::new());
pub fn bindable_array_element(handle:TypeId)->Option<ValueType>{ARRAY_HANDLES.lock().expect("array handle table poisoned").iter().find(|(known,_)|known.id()==handle).map(|(_,element)|*element)}
pub fn register_bindable_array<T:PropertyValue>() {
    {
        let mut handles=ARRAY_HANDLES.lock().expect("array handle table poisoned");
        for handle in [ValueType::of::<BindableArray<T>>(),ValueType::of::<Rc<BindableArray<T>>>(),ValueType::of::<Option<Rc<BindableArray<T>>>>()] {
            if !handles.iter().any(|(known,_)|known.id()==handle.id()){handles.push((handle,ValueType::of::<T>()));}
        }
    }
    crate::data::core::ValueTypes::register_reference::<BindableArray<T>>();
    crate::data::core::ValueTypes::register_projected_weak::<BindableArray<T>>(|array|Rc::new(WeakBindableArray{items:array.items.downgrade(),lengths:array.lengths.clone()}));
}
struct WeakBindableArray<T:PropertyValue>{items:ferroui_system::WeakSharedArray<T>,lengths:Vec<usize>}
impl<T:PropertyValue>crate::data::core::ProjectedWeakReference for WeakBindableArray<T>{
    fn upgrade(&self)->Option<BoxedValue>{Some(BindableArray::from_shared_lengths(self.items.upgrade()?,self.lengths.clone()))}
    fn ptr_eq(&self,other:&dyn crate::data::core::ProjectedWeakReference)->bool{(other as &dyn std::any::Any).downcast_ref::<Self>().is_some_and(|other|self.items.ptr_eq(&other.items)&&self.lengths==other.lengths)}
}

#[cfg(test)]mod shared_storage_tests {
    use super::*;
    #[test]fn original_shared_array_aliases_mutation_and_model_access() {
        let original=ferroui_system::SharedArray::new(vec![97i32]);let view=BindableArray::from_shared(original.clone());
        view.set(&[0],98);assert_eq!(original.get(0),Ok(98));original.set(0,99).unwrap();assert_eq!(view.get(&[0]),99);
        assert!(view.shared().ptr_eq(&original));assert_eq!(view.get_value(&[0]).unwrap().unwrap().downcast_ref::<i32>(),Some(&99));
    }
    #[test]fn weak_view_survives_wrapper_release_but_does_not_own_backing() {
        let original=ferroui_system::SharedArray::new(vec![97i32]);let view=BindableArray::from_shared(original.clone());
        let boxed:BoxedValue=view;let weak=crate::data::core::WeakValue::new(&boxed);assert!(matches!(weak,crate::data::core::WeakValue::Projected(_)));drop(boxed);
        let restored=weak.upgrade().unwrap();assert!(restored.downcast_ref::<BindableArray<i32>>().unwrap().shared().ptr_eq(&original));drop(restored);drop(original);assert!(weak.upgrade().is_none());
    }
}
