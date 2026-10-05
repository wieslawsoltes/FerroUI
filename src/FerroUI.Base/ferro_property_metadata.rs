use crate::data::BindingMode;
use crate::{FerroObject, TypeInfo};
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
    inner: RefCell<TableInner<M>>,
}

struct TableInner<M> {
    /// Fast path while only the registration metadata exists.
    single: Option<(&'static TypeInfo, Rc<M>)>,
    metadata: Vec<(&'static TypeInfo, Rc<M>)>,
    cache: Vec<(&'static TypeInfo, Rc<M>)>,
}

impl<M: PropertyMetadata> MetadataTable<M> {
    pub fn new(host_type: &'static TypeInfo, mut metadata: M) -> Self {
        metadata.freeze();
        let metadata = Rc::new(metadata);
        let default_metadata = M::generate_type_safe_metadata(&metadata);
        Self {
            default_metadata,
            inner: RefCell::new(TableInner {
                single: Some((host_type, metadata.clone())),
                metadata: vec![(host_type, metadata)],
                cache: Vec::new(),
            }),
        }
    }

    /// A table for a property that shares its identity with `source` (added
    /// owner) and optionally carries metadata for the new owner.
    pub fn for_added_owner(source: &Self, owner_type: &'static TypeInfo, metadata: Option<M>) -> Self {
        let mut entries = Vec::new();
        if let Some(m) = metadata {
            entries.push((owner_type, Rc::new(m)));
        }
        Self {
            default_metadata: source.default_metadata.clone(),
            inner: RefCell::new(TableInner { single: None, metadata: entries, cache: Vec::new() }),
        }
    }

    #[inline]
    pub fn default_metadata(&self) -> &Rc<M> {
        &self.default_metadata
    }

    pub fn get(&self, type_: &'static TypeInfo) -> Rc<M> {
        let mut inner = self.inner.borrow_mut();
        if let Some((host, single)) = &inner.single {
            if Rc::ptr_eq(single, &self.default_metadata) {
                return self.default_metadata.clone();
            }
            return if host.is_assignable_from(type_) { single.clone() } else { self.default_metadata.clone() };
        }
        if let Some((_, m)) = inner.cache.iter().find(|(t, _)| std::ptr::eq(*t, type_)) {
            return m.clone();
        }
        let mut current = Some(type_);
        let mut result = None;
        while let Some(t) = current {
            if let Some((_, m)) = inner.metadata.iter().find(|(mt, _)| std::ptr::eq(*mt, t)) {
                result = Some(m.clone());
                break;
            }
            current = t.base_type();
        }
        let result = result.unwrap_or_else(|| self.default_metadata.clone());
        inner.cache.push((type_, result.clone()));
        result
    }

    pub fn override_metadata(&self, property_name: &str, type_: &'static TypeInfo, mut metadata: M) {
        let already_set = self.inner.borrow().metadata.iter().any(|(t, _)| std::ptr::eq(*t, type_));
        if already_set {
            panic!("Metadata is already set for {property_name} on {type_}.");
        }
        let base = self.get(type_);
        metadata.merge(&base);
        metadata.freeze();
        let mut inner = self.inner.borrow_mut();
        inner.metadata.push((type_, Rc::new(metadata)));
        inner.cache.clear();
        inner.single = None;
    }

    pub fn unregister(&self, type_: &'static TypeInfo) {
        let mut inner = self.inner.borrow_mut();
        inner.metadata.retain(|(t, _)| !std::ptr::eq(*t, type_));
        inner.cache.retain(|(t, _)| !std::ptr::eq(*t, type_));
        inner.single = None;
    }
}
