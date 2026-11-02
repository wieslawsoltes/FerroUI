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
    inner: RefCell<MetadataEntries<M>>,
}

// The normal property has only its registration. Do not allocate the host
// index or cache until a second host actually provides different metadata.
enum MetadataEntries<M> {
    Single(Option<(&'static TypeInfo, Rc<M>)>),
    Multiple(Box<MetadataOverrides<M>>),
}

struct MetadataOverrides<M> {
    hosts: Vec<&'static TypeInfo>,
    values: Vec<Rc<M>>,
    // A hit returns the typed Rc directly, including a cached fallback.
    cache: FxHashMap<usize, Rc<M>>,
}

fn host_index(hosts: &[&'static TypeInfo], host: &'static TypeInfo) -> Option<usize> {
    hosts.iter().position(|owner| std::ptr::eq(*owner, host))
}

fn inherited_metadata_index(hosts: &[&'static TypeInfo], host: &'static TypeInfo) -> Option<usize> {
    let mut current = Some(host);
    while let Some(t) = current {
        if let Some(index) = host_index(hosts, t) { return Some(index); }
        current = t.base_type();
    }
    None
}

impl<M: PropertyMetadata> MetadataTable<M> {
    pub fn new(host_type: &'static TypeInfo, mut metadata: M) -> Self {
        metadata.freeze();
        let metadata = Rc::new(metadata);
        let default_metadata = M::generate_type_safe_metadata(&metadata);
        Self {
            default_metadata,
            inner: RefCell::new(MetadataEntries::Single(Some((host_type, metadata)))),
        }
    }

    /// A table for a property that shares its identity with `source` (added
    /// owner) and optionally carries metadata for the new owner.
    pub fn for_added_owner(source: &Self, owner_type: &'static TypeInfo, metadata: Option<M>) -> Self {
        Self {
            default_metadata: source.default_metadata.clone(),
            inner: RefCell::new(MetadataEntries::Single(metadata.map(|m| (owner_type, Rc::new(m))))),
        }
    }

    #[inline]
    pub fn default_metadata(&self) -> &Rc<M> {
        &self.default_metadata
    }

    pub fn get(&self, type_: &'static TypeInfo) -> Rc<M> {
        let mut inner = self.inner.borrow_mut();
        let table = match &mut *inner {
            MetadataEntries::Single(Some((host, value))) => {
                return if Rc::ptr_eq(value, &self.default_metadata) || host.is_assignable_from(type_) {
                    value.clone()
                } else {
                    self.default_metadata.clone()
                };
            }
            MetadataEntries::Single(None) => return self.default_metadata.clone(),
            MetadataEntries::Multiple(table) => table,
        };
        let key = type_ as *const TypeInfo as usize;
        if let Some(value) = table.cache.get(&key) {
            return value.clone();
        }
        let value = match inherited_metadata_index(&table.hosts, type_) {
            Some(i) => table.values[i].clone(),
            None => self.default_metadata.clone(),
        };
        table.cache.insert(key, value.clone());
        value
    }

    pub fn override_metadata(&self, property_name: &str, type_: &'static TypeInfo, mut metadata: M) {
        let exists = match &*self.inner.borrow() {
            MetadataEntries::Single(existing) => existing.as_ref().is_some_and(|(t, _)| std::ptr::eq(*t, type_)),
            MetadataEntries::Multiple(table) => host_index(&table.hosts, type_).is_some(),
        };
        if exists {
            panic!("Metadata is already set for {property_name} on {type_}.");
        }
        let base = self.get(type_);
        metadata.merge(&base);
        metadata.freeze();
        let value = Rc::new(metadata);
        let mut inner = self.inner.borrow_mut();
        match &mut *inner {
            MetadataEntries::Single(existing) => {
                if let Some((host, original)) = existing.take() {
                    *inner = MetadataEntries::Multiple(Box::new(MetadataOverrides {
                        hosts: vec![host, type_],
                        values: vec![original, value],
                        cache: FxHashMap::default(),
                    }));
                } else {
                    *existing = Some((type_, value));
                }
            }
            MetadataEntries::Multiple(table) => {
                table.hosts.push(type_);
                table.values.push(value);
                table.cache.clear();
            }
        }
    }

    pub fn unregister(&self, type_: &'static TypeInfo) {
        let mut inner = self.inner.borrow_mut();
        match &mut *inner {
            MetadataEntries::Single(existing) => {
                if existing.as_ref().is_some_and(|(t, _)| std::ptr::eq(*t, type_)) {
                    *existing = None;
                }
            }
            MetadataEntries::Multiple(table) => {
                if let Some(i) = host_index(&table.hosts, type_) {
                    table.hosts.swap_remove(i);
                    table.values.swap_remove(i);
                    // Descendants may have cached the removed override.
                    table.cache.clear();
                    if table.hosts.len() == 1 {
                        let metadata = table.values.pop().expect("one registration remains");
                        *inner = MetadataEntries::Single(Some((table.hosts[0], metadata)));
                    }
                }
            }
        }
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
