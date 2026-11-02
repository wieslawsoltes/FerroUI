use crate::data::BindingMode;
use crate::{FerroObject, TypeInfo};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Metadata common to all property kinds.
#[derive(Clone, Debug)]
pub struct FerroPropertyMetadata {
    default_binding_mode: BindingMode,
    enable_data_validation: Option<bool>,
    is_read_only: bool,
}

impl FerroPropertyMetadata {
    pub fn new(default_binding_mode: BindingMode, enable_data_validation: Option<bool>) -> Self {
        Self { default_binding_mode, enable_data_validation, is_read_only: false }
    }

    /// The default binding mode for the property.
    pub fn default_binding_mode(&self) -> BindingMode {
        if self.default_binding_mode == BindingMode::Default {
            BindingMode::OneWay
        } else {
            self.default_binding_mode
        }
    }

    /// Whether the property is interested in data validation.
    ///
    /// Data validation is validation performed at the target of a binding, for
    /// example in a view model. To enable it on a binding, the property must
    /// also enable it here.
    pub fn enable_data_validation(&self) -> Option<bool> {
        self.enable_data_validation
    }

    /// Whether the metadata has been frozen.
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// Merges unset members from the base metadata.
    pub fn merge(&mut self, base: &FerroPropertyMetadata) {
        assert!(!self.is_read_only, "The metadata is read-only.");
        if self.default_binding_mode == BindingMode::Default {
            self.default_binding_mode = base.default_binding_mode();
        }
        if self.enable_data_validation.is_none() {
            self.enable_data_validation = base.enable_data_validation;
        }
    }

    /// Makes the metadata read-only.
    pub fn freeze(&mut self) {
        self.is_read_only = true;
    }
}

/// The coercion callback of a styled property.
pub type CoerceValueCallback<T> = Rc<dyn Fn(&FerroObject, T) -> T>;

/// Metadata for styled properties.
pub struct StyledPropertyMetadata<T> {
    base: FerroPropertyMetadata,
    default_value: Option<T>,
    coerce: Option<CoerceValueCallback<T>>,
}

impl<T: Clone> Clone for StyledPropertyMetadata<T> {
    fn clone(&self) -> Self {
        Self { base: self.base.clone(), default_value: self.default_value.clone(), coerce: self.coerce.clone() }
    }
}

impl<T> std::ops::Deref for StyledPropertyMetadata<T> {
    type Target = FerroPropertyMetadata;
    fn deref(&self) -> &FerroPropertyMetadata {
        &self.base
    }
}

impl<T: Clone + 'static> StyledPropertyMetadata<T> {
    /// Creates metadata. A `default_value` of `None` inherits the default
    /// value of the base metadata when merged.
    pub fn new(default_value: Option<T>) -> Self {
        Self {
            base: FerroPropertyMetadata::new(BindingMode::Default, Some(false)),
            default_value,
            coerce: None,
        }
    }

    pub fn with_default_binding_mode(mut self, mode: BindingMode) -> Self {
        self.base.default_binding_mode = mode;
        self
    }

    pub fn with_coerce(mut self, coerce: impl Fn(&FerroObject, T) -> T + 'static) -> Self {
        self.coerce = Some(Rc::new(coerce));
        self
    }

    pub fn with_enable_data_validation(mut self, enable: bool) -> Self {
        self.base.enable_data_validation = Some(enable);
        self
    }

    /// The default value of the property. Panics if the metadata has not been
    /// merged with a base that provides one.
    #[inline]
    pub fn default_value(&self) -> &T {
        self.default_value.as_ref().expect("metadata has no default value")
    }

    pub(crate) fn has_default_value(&self) -> bool {
        self.default_value.is_some()
    }

    /// The value coercion callback, if any.
    #[inline]
    pub fn coerce_value(&self) -> Option<&CoerceValueCallback<T>> {
        self.coerce.as_ref()
    }
}

/// Metadata for direct properties.
#[derive(Clone)]
pub struct DirectPropertyMetadata<T> {
    base: FerroPropertyMetadata,
    unset_value: Option<T>,
}

impl<T> std::ops::Deref for DirectPropertyMetadata<T> {
    type Target = FerroPropertyMetadata;
    fn deref(&self) -> &FerroPropertyMetadata {
        &self.base
    }
}

impl<T: Clone + 'static> DirectPropertyMetadata<T> {
    pub fn new(unset_value: Option<T>) -> Self {
        Self { base: FerroPropertyMetadata::new(BindingMode::Default, None), unset_value }
    }

    pub fn with_default_binding_mode(mut self, mode: BindingMode) -> Self {
        self.base.default_binding_mode = mode;
        self
    }

    pub fn with_enable_data_validation(mut self, enable: bool) -> Self {
        self.base.enable_data_validation = Some(enable);
        self
    }

    /// The value to use when the property is set to the unset value.
    #[inline]
    pub fn unset_value(&self) -> &T {
        self.unset_value.as_ref().expect("metadata has no unset value")
    }
}

/// Behaviour shared by the typed metadata kinds so that [`MetadataTable`] can
/// manage them generically.
pub trait PropertyMetadata: Sized + 'static {
    fn base(&self) -> &FerroPropertyMetadata;
    fn merge(&mut self, base: &Self);
    fn freeze(&mut self);
    /// Returns metadata that is safe to use for any host type: a copy without
    /// host-specific callbacks.
    fn generate_type_safe_metadata(this: &Rc<Self>) -> Rc<Self>;
}

impl<T: Clone + 'static> PropertyMetadata for StyledPropertyMetadata<T> {
    fn base(&self) -> &FerroPropertyMetadata {
        &self.base
    }

    fn merge(&mut self, base: &Self) {
        self.base.merge(&base.base);
        if self.default_value.is_none() {
            self.default_value = base.default_value.clone();
        }
        if self.coerce.is_none() {
            self.coerce = base.coerce.clone();
        }
    }

    fn freeze(&mut self) {
        self.base.freeze();
    }

    fn generate_type_safe_metadata(this: &Rc<Self>) -> Rc<Self> {
        if this.is_read_only() && this.coerce.is_none() {
            return this.clone();
        }
        let mut copy = StyledPropertyMetadata::new(this.default_value.clone())
            .with_default_binding_mode(this.default_binding_mode())
            .with_enable_data_validation(this.enable_data_validation().unwrap_or(false));
        copy.freeze();
        Rc::new(copy)
    }
}

impl<T: Clone + 'static> PropertyMetadata for DirectPropertyMetadata<T> {
    fn base(&self) -> &FerroPropertyMetadata {
        &self.base
    }

    fn merge(&mut self, base: &Self) {
        self.base.merge(&base.base);
        if self.unset_value.is_none() {
            self.unset_value = base.unset_value.clone();
        }
    }

    fn freeze(&mut self) {
        self.base.freeze();
    }

    fn generate_type_safe_metadata(this: &Rc<Self>) -> Rc<Self> {
        this.clone()
    }
}

/// Per-host-type metadata storage of a property, with the lookup rules of the
/// property system: the metadata registered for the nearest base class of the
/// queried type wins, falling back to the type-safe default metadata.
pub(crate) struct MetadataTable<M> {
    default_metadata: Rc<M>,
    hosts: RefCell<MetadataHosts>,
}

type ErasedMetadata = Rc<dyn std::any::Any>;

/// Most properties use the same metadata for all hosts. Their registration
/// needs only its host, with no allocation for an override list or cache.
struct MetadataHosts {
    registration: Option<(&'static TypeInfo, Option<ErasedMetadata>)>,
    overrides: Option<Box<MetadataOverrides>>,
}

#[derive(Default)]
struct MetadataOverrides {
    entries: Vec<(&'static TypeInfo, ErasedMetadata)>,
    /// A miss resolves to the registration (when applicable) or to the default.
    cache: FxHashMap<usize, MetadataSlot>,
}

#[derive(Clone, Copy)]
enum MetadataSlot {
    Default,
    Registration,
    Override(usize),
}

impl MetadataHosts {
    fn new(registration: Option<(&'static TypeInfo, Option<ErasedMetadata>)>) -> Self {
        Self { registration, overrides: None }
    }

    fn contains(&self, host: &'static TypeInfo) -> bool {
        self.registration.as_ref().is_some_and(|(t, _)| std::ptr::eq(*t, host))
            || self.overrides.as_ref().is_some_and(|overrides| {
                overrides.entries.iter().any(|(t, _)| std::ptr::eq(*t, host))
            })
    }

    fn get(&mut self, host: &'static TypeInfo) -> Option<&ErasedMetadata> {
        let registration = self.registration.as_ref();
        let Some(overrides) = self.overrides.as_deref_mut() else {
            let (owner, value) = registration?;
            return value.as_ref().filter(|_| owner.is_assignable_from(host));
        };

        let slot = overrides.cache.entry(host as *const TypeInfo as usize).or_insert_with(|| {
            let mut current = Some(host);
            while let Some(t) = current {
                if registration.is_some_and(|(owner, _)| std::ptr::eq(*owner, t)) {
                    return MetadataSlot::Registration;
                }
                if let Some(index) = overrides.entries.iter().position(|(owner, _)| std::ptr::eq(*owner, t)) {
                    return MetadataSlot::Override(index);
                }
                current = t.base_type();
            }
            MetadataSlot::Default
        });
        match *slot {
            MetadataSlot::Default => None,
            MetadataSlot::Registration => registration.and_then(|(_, value)| value.as_ref()),
            MetadataSlot::Override(index) => Some(&overrides.entries[index].1),
        }
    }

    fn add_override(&mut self, host: &'static TypeInfo, value: ErasedMetadata) {
        let overrides = self.overrides.get_or_insert_with(Box::default);
        overrides.entries.push((host, value));
        overrides.cache.clear();
    }

    fn unregister(&mut self, host: &'static TypeInfo) {
        if self.registration.as_ref().is_some_and(|(t, _)| std::ptr::eq(*t, host)) {
            self.registration = None;
        } else if let Some(overrides) = self.overrides.as_mut() {
            let Some(index) = overrides.entries.iter().position(|(t, _)| std::ptr::eq(*t, host)) else { return };
            overrides.entries.swap_remove(index);
            if overrides.entries.is_empty() {
                self.overrides = None;
                return;
            }
        }
        // Descendants may have cached this host or a slot that moved.
        if let Some(overrides) = self.overrides.as_mut() {
            overrides.cache.clear();
        }
    }
}

impl<M: PropertyMetadata> MetadataTable<M> {
    pub fn new(host_type: &'static TypeInfo, mut metadata: M) -> Self {
        metadata.freeze();
        let metadata = Rc::new(metadata);
        let default_metadata = M::generate_type_safe_metadata(&metadata);
        let host_value = (!Rc::ptr_eq(&metadata, &default_metadata)).then_some(metadata as ErasedMetadata);
        Self { default_metadata, hosts: RefCell::new(MetadataHosts::new(Some((host_type, host_value)))) }
    }

    /// A table for a property that shares its identity with `source` (added
    /// owner) and optionally carries metadata for the new owner.
    pub fn for_added_owner(source: &Self, owner_type: &'static TypeInfo, metadata: Option<M>) -> Self {
        let registration = metadata.map(|m| (owner_type, Some(Rc::new(m) as ErasedMetadata)));
        Self { default_metadata: source.default_metadata.clone(), hosts: RefCell::new(MetadataHosts::new(registration)) }
    }

    #[inline]
    pub fn default_metadata(&self) -> &Rc<M> {
        &self.default_metadata
    }

    pub fn get(&self, type_: &'static TypeInfo) -> Rc<M> {
        match self.hosts.borrow_mut().get(type_) {
            Some(value) => value.clone().downcast::<M>().expect("metadata belongs to this property"),
            None => self.default_metadata.clone(),
        }
    }

    pub fn override_metadata(&self, property_name: &str, type_: &'static TypeInfo, mut metadata: M) {
        if self.hosts.borrow().contains(type_) {
            panic!("Metadata is already set for {property_name} on {type_}.");
        }
        let base = self.get(type_);
        metadata.merge(&base);
        metadata.freeze();
        self.hosts.borrow_mut().add_override(type_, Rc::new(metadata));
    }

    pub fn unregister(&self, type_: &'static TypeInfo) {
        self.hosts.borrow_mut().unregister(type_);
    }
}

#[cfg(test)]
mod metadata_table_tests {
    use super::*;

    static ROOT: TypeInfo = TypeInfo::new("Root", None);
    static CHILD: TypeInfo = TypeInfo::new("Child", Some(&ROOT));
    static LEAF: TypeInfo = TypeInfo::new("Leaf", Some(&CHILD));
    static SIBLING: TypeInfo = TypeInfo::new("Sibling", Some(&ROOT));

    #[test]
    fn unregister_invalidates_inherited_and_relocated_overrides() {
        let table = MetadataTable::new(&ROOT, StyledPropertyMetadata::new(Some(1)));
        table.override_metadata("Number", &CHILD, StyledPropertyMetadata::new(Some(2)));
        table.override_metadata("Number", &SIBLING, StyledPropertyMetadata::new(Some(3)));
        assert_eq!(*table.get(&LEAF).default_value(), 2);
        assert_eq!(*table.get(&SIBLING).default_value(), 3);

        table.unregister(&CHILD);
        assert_eq!(*table.get(&LEAF).default_value(), 1);
        assert_eq!(*table.get(&SIBLING).default_value(), 3);
        table.override_metadata("Number", &CHILD, StyledPropertyMetadata::new(Some(4)));
        assert_eq!(*table.get(&LEAF).default_value(), 4);
    }
}
