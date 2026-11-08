use crate::data::BindingMode;
use crate::property_store::{EffectiveValue, UncommonFields};
use crate::{FerroObject, TypeInfo};
use std::cell::RefCell;
use std::marker::PhantomData;
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

/// The small typed operations used by explicit recoercion. These functions
/// already serve ordinary typed writes; keeping them separate lets the cold
/// two-value algorithm require only the public metadata's `Clone` bound.
pub(crate) struct CoercionOperations<T> {
    pub(crate) equals: fn(&T, &T) -> bool,
    pub(crate) notify_value_changed: fn(&EffectiveValue<T>, &FerroObject, T),
    pub(crate) notify_base_value_changed: fn(&EffectiveValue<T>, &FerroObject),
}

pub(crate) struct DefaultCoercionOperations<T> {
    pub(crate) equals: fn(&T, &T) -> bool,
    pub(crate) set_default: fn(&EffectiveValue<T>, &FerroObject, T),
}

/// The factory borrows the store, owner and typed property. Keeping that
/// PropertyValue-specific construction outside the algorithm lets public
/// Clone-only metadata retain its existing bounds.
pub(crate) trait DefaultEffectiveValueFactory<T> {
    fn create_and_add(&self) -> Rc<EffectiveValue<T>>;
}

// Requests contain only borrows and function pointers: no owned T, Rc or
// observable destructor crosses this dispatch frame. One entry covers all
// coercion paths without enlarging metadata for additional algorithm slots.
enum CoercionRequest<'a, T> {
    Recoerce {
        target: &'a EffectiveValue<T>,
        owner: &'a FerroObject,
        operations: CoercionOperations<T>,
    },
    EndReevaluation {
        target: &'a EffectiveValue<T>,
        owner: &'a FerroObject,
        operations: DefaultCoercionOperations<T>,
    },
    Default {
        coercion: &'a Coercion<T>,
        metadata: &'a StyledPropertyMetadata<T>,
        owner: &'a FerroObject,
        factory: &'a dyn DefaultEffectiveValueFactory<T>,
        operations: DefaultCoercionOperations<T>,
    },
    TypeSafeMetadata {
        metadata: &'a Rc<StyledPropertyMetadata<T>>,
        result: &'a mut Option<Rc<StyledPropertyMetadata<T>>>,
    },
    InitializeUncommon {
        callback: &'a CoerceValueCallback<T>,
        value: &'a T,
        result: &'a mut Option<UncommonFields<T>>,
    },
}

/// Constructed only when a caller supplies a coercer. A property without a
/// coercer must not take the address of its cold algorithms merely because
/// its untyped effective-value and property interfaces expose coercion.
pub(crate) struct Coercion<T> {
    callback: CoerceValueCallback<T>,
    apply: for<'a> fn(CoercionRequest<'a, T>),
}

impl<T> Clone for Coercion<T> {
    fn clone(&self) -> Self {
        // Copy the algorithm address without cloning T or wrapping the
        // callback: metadata continues to share the original Rc allocation.
        Self { callback: self.callback.clone(), apply: self.apply }
    }
}

impl<T> Coercion<T> {
    pub(crate) fn callback(&self) -> &CoerceValueCallback<T> {
        &self.callback
    }

    pub(crate) fn apply(&self, target: &EffectiveValue<T>, owner: &FerroObject, operations: CoercionOperations<T>) {
        (self.apply)(CoercionRequest::Recoerce { target, owner, operations });
    }

    pub(crate) fn end_reevaluation(
        &self,
        target: &EffectiveValue<T>,
        owner: &FerroObject,
        operations: DefaultCoercionOperations<T>,
    ) {
        (self.apply)(CoercionRequest::EndReevaluation { target, owner, operations });
    }

    pub(crate) fn coerce_default(
        &self,
        metadata: &StyledPropertyMetadata<T>,
        owner: &FerroObject,
        factory: &dyn DefaultEffectiveValueFactory<T>,
        operations: DefaultCoercionOperations<T>,
    ) {
        (self.apply)(CoercionRequest::Default { coercion: self, metadata, owner, factory, operations });
    }

    fn type_safe_metadata(&self, metadata: &Rc<StyledPropertyMetadata<T>>) -> Rc<StyledPropertyMetadata<T>> {
        let mut result = None;
        (self.apply)(CoercionRequest::TypeSafeMetadata { metadata, result: &mut result });
        result.expect("coercion dispatch returns type-safe metadata")
    }

    pub(crate) fn initialize_uncommon(&self, value: &T) -> UncommonFields<T> {
        let mut result = None;
        // Borrow the callback here. Its original Rc clone belongs inside the
        // initializer, before either value clone, including partial unwind.
        (self.apply)(CoercionRequest::InitializeUncommon {
            callback: &self.callback,
            value,
            result: &mut result,
        });
        result.expect("coercion dispatch initializes uncommon fields")
    }
}

impl<T: Clone + 'static> Coercion<T> {
    pub(crate) fn new(callback: CoerceValueCallback<T>) -> Self {
        Self { callback, apply: Self::dispatch }
    }

    fn dispatch(request: CoercionRequest<'_, T>) {
        match request {
            CoercionRequest::Recoerce { target, owner, operations } => target.apply_coercion(owner, operations),
            CoercionRequest::EndReevaluation { target, owner, operations } => {
                target.apply_coercion_at_end(owner, operations);
            }
            CoercionRequest::Default { coercion, metadata, owner, factory, operations } => {
                coercion.apply_default(metadata, owner, factory, operations);
            }
            CoercionRequest::TypeSafeMetadata { metadata, result } => {
                // The public implementation keeps its original owned copy
                // frame; only explicit coercer construction roots this call.
                *result = Some(StyledPropertyMetadata::generate_type_safe_metadata(metadata));
            }
            CoercionRequest::InitializeUncommon { callback, value, result } => {
                *result = Some(UncommonFields::new(callback, value));
            }
        }
    }

    fn apply_default(
        &self,
        metadata: &StyledPropertyMetadata<T>,
        owner: &FerroObject,
        factory: &dyn DefaultEffectiveValueFactory<T>,
        operations: DefaultCoercionOperations<T>,
    ) {
        let coerce = &self.callback;
        // The outer typed route retains the original metadata Rc. Keep all
        // owned values in this frame through application and cleanup; never
        // return an owned coerced T after releasing default or metadata.
        let default = metadata.default_value().clone();
        let coerced = coerce(owner, default.clone());
        if (operations.equals)(&default, &coerced) {
            return;
        }
        // Construction reloads current metadata, as it originally did after
        // a potentially reentrant coercer/equality implementation.
        let effective = factory.create_and_add();
        (operations.set_default)(&effective, owner, coerced);
    }
}

#[cfg(test)]
mod coercion_request_tests {
    use super::*;
    use std::mem::{align_of, needs_drop, size_of};

    #[test]
    fn borrowed_coercion_requests_keep_the_single_pointer_layout_and_unbounded_types() {
        struct DropOnly;
        impl Drop for DropOnly {
            fn drop(&mut self) { panic!("the request must not own this value"); }
        }
        type Original<T> = (
            CoerceValueCallback<T>,
            fn(&EffectiveValue<T>, &FerroObject, CoercionOperations<T>),
        );
        // Preserve the exact pre-construction request representation as an
        // independent mirror; the two new variants contain only borrows.
        #[allow(dead_code)]
        enum OriginalRequest<'a, T> {
            Recoerce {
                target: &'a EffectiveValue<T>,
                owner: &'a FerroObject,
                operations: CoercionOperations<T>,
            },
            EndReevaluation {
                target: &'a EffectiveValue<T>,
                owner: &'a FerroObject,
                operations: DefaultCoercionOperations<T>,
            },
            Default {
                coercion: &'a OriginalDispatch<T>,
                metadata: &'a StyledPropertyMetadata<T>,
                owner: &'a FerroObject,
                factory: &'a dyn DefaultEffectiveValueFactory<T>,
                operations: DefaultCoercionOperations<T>,
            },
        }
        #[allow(dead_code)]
        struct OriginalDispatch<T> {
            callback: CoerceValueCallback<T>,
            apply: for<'a> fn(OriginalRequest<'a, T>),
        }
        assert_eq!(size_of::<Coercion<DropOnly>>(), size_of::<Original<DropOnly>>());
        assert_eq!(align_of::<Coercion<DropOnly>>(), align_of::<Original<DropOnly>>());
        assert_eq!(size_of::<Option<Coercion<DropOnly>>>(), size_of::<Option<Original<DropOnly>>>());
        assert_eq!(size_of::<Coercion<DropOnly>>(), size_of::<OriginalDispatch<DropOnly>>());
        assert_eq!(align_of::<Coercion<DropOnly>>(), align_of::<OriginalDispatch<DropOnly>>());
        assert_eq!(size_of::<Option<Coercion<DropOnly>>>(), size_of::<Option<OriginalDispatch<DropOnly>>>());
        assert_eq!(align_of::<Option<Coercion<DropOnly>>>(), align_of::<Option<OriginalDispatch<DropOnly>>>());
        assert!(size_of::<CoercionRequest<'_, DropOnly>>() <= size_of::<OriginalRequest<'_, DropOnly>>());
        assert_eq!(align_of::<CoercionRequest<'_, DropOnly>>(), align_of::<OriginalRequest<'_, DropOnly>>());
        assert!(!needs_drop::<OriginalRequest<'_, DropOnly>>());
        assert!(!needs_drop::<CoercionRequest<'_, DropOnly>>());
        assert!(!needs_drop::<CoercionOperations<DropOnly>>());
        assert!(!needs_drop::<DefaultCoercionOperations<DropOnly>>());
        assert_eq!(size_of::<CoercionRequest<'_, DropOnly>>(), size_of::<CoercionRequest<'_, String>>());

        // Naming public metadata must not acquire Clone, PartialEq or 'static
        // bounds just because its private dispatch field names a request.
        let _: Option<StyledPropertyMetadata<DropOnly>> = None;
        fn borrowed<'a>(_: &'a ()) -> Option<StyledPropertyMetadata<&'a ()>> { None }
        let local = ();
        assert!(borrowed(&local).is_none());
    }
}

#[cfg(test)]
#[path = "ferro_property_metadata_construction_oracles.rs"]
pub(crate) mod construction_oracles;

/// Metadata for styled properties.
pub struct StyledPropertyMetadata<T> {
    base: FerroPropertyMetadata,
    default_value: Option<T>,
    coerce: Option<Coercion<T>>,
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

    pub fn with_coerce(self, coerce: impl Fn(&FerroObject, T) -> T + 'static) -> Self {
        self.with_registered_coercion(Coercion::new(Rc::new(coerce)))
    }

    pub(crate) fn with_registered_coercion(mut self, coerce: Coercion<T>) -> Self {
        self.coerce = Some(coerce);
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
        self.coerce.as_ref().map(Coercion::callback)
    }

    pub(crate) fn coercion(&self) -> Option<&Coercion<T>> {
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

// Only MetadataTable::new uses this after freezing its owned metadata. Keep
// the public generation method independent: callers may supply unfrozen
// metadata, which must still make a frozen copy even without a coercer.
pub(crate) trait FrozenPropertyMetadata: PropertyMetadata {
    fn generate_frozen_type_safe_metadata(this: &Rc<Self>) -> Rc<Self>;
}

impl<T: Clone + 'static> FrozenPropertyMetadata for StyledPropertyMetadata<T> {
    fn generate_frozen_type_safe_metadata(this: &Rc<Self>) -> Rc<Self> {
        match this.coercion() {
            Some(coercion) => coercion.type_safe_metadata(this),
            None => this.clone(),
        }
    }
}

impl<T: Clone + 'static> FrozenPropertyMetadata for DirectPropertyMetadata<T> {
    fn generate_frozen_type_safe_metadata(this: &Rc<Self>) -> Rc<Self> {
        this.clone()
    }
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

/// The untyped view of the metadata of a property, whatever its kind and value
/// type: what the shared metadata table and the untyped routes of a property
/// read. Implemented for every metadata type.
pub(crate) trait ErasedMetadata: 'static {
    /// The metadata common to all property kinds.
    fn erased_base(&self) -> &FerroPropertyMetadata;
}

impl<M: PropertyMetadata> ErasedMetadata for M {
    #[inline]
    fn erased_base(&self) -> &FerroPropertyMetadata {
        PropertyMetadata::base(self)
    }
}

/// The metadata of `value`, which must be an `M`, as its type.
///
/// # Safety
///
/// `value` must have been made from an `Rc<M>`.
#[inline]
unsafe fn downcast_metadata<M: 'static>(value: Rc<dyn ErasedMetadata>) -> Rc<M> {
    // SAFETY: the data pointer of the box is that of the `Rc<M>` it was made
    // from (the caller's contract); the cast drops only the vtable.
    unsafe { Rc::from_raw(Rc::into_raw(value) as *const M) }
}

/// Per-host-type metadata storage of a property, with the lookup rules of the
/// property system: the metadata registered for the nearest base class of the
/// queried type wins, falling back to the type-safe default metadata.
///
/// The typed facade of [`MetadataTableCore`]: the lookups, the registrations
/// and their cache are compiled once for every metadata type (#576); only the
/// construction, the merge of an override and the cast of a result are typed.
pub(crate) struct MetadataTable<M> {
    core: MetadataTableCore,
    _metadata: PhantomData<Rc<M>>,
}

/// The metadata of a property per host type, untyped: every value is the
/// metadata type of the [`MetadataTable`] that owns the core.
pub(crate) struct MetadataTableCore {
    default_metadata: Rc<dyn ErasedMetadata>,
    inner: RefCell<MetadataEntries>,
}

// The normal property has only its registration. Do not allocate the host
// index or cache until a second host actually provides different metadata.
// The cache of the metadata of every type it was asked for is a hash table by
// the address of the type, as the reference's `_metadataCache` dictionary
// (DEVIATIONS.md, corrected divergences); its hierarchy traversal is shared by
// every metadata type, and so are the cached values, untyped.
enum MetadataEntries {
    Single(Option<(&'static TypeInfo, Rc<dyn ErasedMetadata>)>),
    Multiple(Box<MetadataOverrides>),
}

struct MetadataOverrides {
    hosts: MetadataHosts,
    values: Vec<Rc<dyn ErasedMetadata>>,
}

// Slots are stable until a registration changes. All metadata types share the
// hierarchy lookup and its cache.
struct MetadataHosts {
    owners: Vec<&'static TypeInfo>,
    // Each key is a non-null, process-lifetime TypeInfo address. The cache grows
    // by powers of two and stays at most half full so every probe terminates.
    // Removing or adding an owner invalidates every inherited slot at once.
    slots: Vec<(usize, usize)>,
    occupied: usize,
}

impl MetadataHosts {
    fn new(first: &'static TypeInfo, second: &'static TypeInfo) -> Self {
        Self { owners: vec![first, second], slots: Vec::new(), occupied: 0 }
    }

    #[inline]
    fn bucket(key: usize, capacity: usize) -> usize {
        // Adjacent records have an odd stride after the shift. Fold higher
        // address bits into the index for independently aligned type descriptors.
        let hash = key >> std::mem::size_of::<TypeInfo>().trailing_zeros();
        (hash ^ (hash >> 7)) & (capacity - 1)
    }

    #[inline]
    fn resolve(&mut self, host: &'static TypeInfo) -> usize {
        let key = host as *const TypeInfo as usize;
        if !self.slots.is_empty() {
            let bucket = Self::bucket(key, self.slots.len());
            let (cached, slot) = self.slots[bucket];
            if cached == key {
                return slot;
            }
        }
        self.resolve_other(host, key)
    }

    #[inline(never)]
    fn resolve_other(&mut self, host: &'static TypeInfo, key: usize) -> usize {
        if !self.slots.is_empty() {
            let bucket = Self::find(&self.slots, key);
            if self.slots[bucket].0 != 0 {
                return self.slots[bucket].1;
            }
        }
        if self.occupied == self.slots.len() / 2 {
            let mut slots = vec![(0, 0); (self.slots.len() * 2).max(4)];
            for &(cached, slot) in &self.slots {
                if cached != 0 {
                    let bucket = Self::find(&slots, cached);
                    slots[bucket] = (cached, slot);
                }
            }
            self.slots = slots;
        }
        let bucket = Self::find(&self.slots, key);
        let slot = inherited_metadata_index(&self.owners, host).unwrap_or(usize::MAX);
        self.slots[bucket] = (key, slot);
        self.occupied += 1;
        slot
    }

    #[inline]
    fn find(slots: &[(usize, usize)], key: usize) -> usize {
        let mut bucket = Self::bucket(key, slots.len());
        while slots[bucket].0 != 0 && slots[bucket].0 != key {
            bucket = (bucket + 1) & (slots.len() - 1);
        }
        bucket
    }

    fn invalidate(&mut self) {
        self.slots.fill((0, 0));
        self.occupied = 0;
    }
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
    pub fn new(host_type: &'static TypeInfo, mut metadata: M) -> Self
    where
        M: FrozenPropertyMetadata,
    {
        metadata.freeze();
        let metadata = Rc::new(metadata);
        let default_metadata = M::generate_frozen_type_safe_metadata(&metadata);
        Self::from_parts(host_type, metadata, default_metadata)
    }

    /// The table of the registration of `metadata` for `host_type`, with
    /// `default_metadata` for every other type.
    fn from_parts(host_type: &'static TypeInfo, metadata: Rc<M>, default_metadata: Rc<M>) -> Self {
        let metadata: Rc<dyn ErasedMetadata> = metadata;
        let default_metadata: Rc<dyn ErasedMetadata> = default_metadata;
        Self {
            core: MetadataTableCore {
                default_metadata,
                inner: RefCell::new(MetadataEntries::Single(Some((host_type, metadata)))),
            },
            _metadata: PhantomData,
        }
    }

    /// A table for a property that shares its identity with `source` (added
    /// owner) and optionally carries metadata for the new owner.
    pub fn for_added_owner(source: &Self, owner_type: &'static TypeInfo, metadata: Option<M>) -> Self {
        let metadata = metadata.map(|m| {
            let m: Rc<dyn ErasedMetadata> = Rc::new(m);
            (owner_type, m)
        });
        Self {
            core: MetadataTableCore {
                default_metadata: source.core.default_metadata.clone(),
                inner: RefCell::new(MetadataEntries::Single(metadata)),
            },
            _metadata: PhantomData,
        }
    }

    /// The untyped table, which the routes of the property read.
    #[inline]
    pub(crate) fn core(&self) -> &MetadataTableCore {
        &self.core
    }

    #[cfg(test)]
    pub fn default_metadata(&self) -> Rc<M> {
        // SAFETY: every value of the core is an `M` (the constructors).
        unsafe { downcast_metadata(self.core.default_metadata.clone()) }
    }

    #[inline]
    pub fn get(&self, type_: &'static TypeInfo) -> Rc<M> {
        // SAFETY: every value of the core is an `M` (the constructors and
        // `override_metadata`).
        unsafe { downcast_metadata(self.core.get(type_)) }
    }

    pub fn override_metadata(&self, property_name: &str, type_: &'static TypeInfo, mut metadata: M) {
        self.core.ensure_not_registered(property_name, type_);
        let base = self.get(type_);
        metadata.merge(&base);
        metadata.freeze();
        self.core.insert(type_, Rc::new(metadata));
    }

    #[inline]
    pub fn unregister(&self, type_: &'static TypeInfo) {
        self.core.unregister(type_);
    }
}

impl MetadataTableCore {
    /// The metadata for `type_`: the registration of its nearest class, else
    /// the default metadata.
    pub(crate) fn get(&self, type_: &'static TypeInfo) -> Rc<dyn ErasedMetadata> {
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
        let slot = table.hosts.resolve(type_);
        table.values.get(slot).unwrap_or(&self.default_metadata).clone()
    }

    /// Panics if metadata is registered for exactly `type_`.
    fn ensure_not_registered(&self, property_name: &str, type_: &'static TypeInfo) {
        let exists = match &*self.inner.borrow() {
            MetadataEntries::Single(existing) => existing.as_ref().is_some_and(|(t, _)| std::ptr::eq(*t, type_)),
            MetadataEntries::Multiple(table) => host_index(&table.hosts.owners, type_).is_some(),
        };
        if exists {
            panic!("Metadata is already set for {property_name} on {type_}.");
        }
    }

    /// Registers `value` for `type_`, which has no registration.
    fn insert(&self, type_: &'static TypeInfo, value: Rc<dyn ErasedMetadata>) {
        let mut inner = self.inner.borrow_mut();
        match &mut *inner {
            MetadataEntries::Single(existing) => {
                if let Some((host, original)) = existing.take() {
                    *inner = MetadataEntries::Multiple(Box::new(MetadataOverrides {
                        hosts: MetadataHosts::new(host, type_),
                        values: vec![original, value],
                    }));
                } else {
                    *existing = Some((type_, value));
                }
            }
            MetadataEntries::Multiple(table) => {
                table.hosts.owners.push(type_);
                table.values.push(value);
                table.hosts.invalidate();
            }
        }
    }

    pub(crate) fn unregister(&self, type_: &'static TypeInfo) {
        let mut inner = self.inner.borrow_mut();
        match &mut *inner {
            MetadataEntries::Single(existing) => {
                if existing.as_ref().is_some_and(|(t, _)| std::ptr::eq(*t, type_)) {
                    *existing = None;
                }
            }
            MetadataEntries::Multiple(table) => {
                if let Some(i) = host_index(&table.hosts.owners, type_) {
                    table.hosts.owners.swap_remove(i);
                    table.values.swap_remove(i);
                    // Descendants may have cached the removed override.
                    table.hosts.invalidate();
                    if table.hosts.owners.len() == 1 {
                        let metadata = table.values.pop().expect("one registration remains");
                        *inner = MetadataEntries::Single(Some((table.hosts.owners[0], metadata)));
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
    fn colliding_cache_entries_follow_replaced_and_removed_registrations() {
        static TYPES: [TypeInfo; 16] = [const { TypeInfo::new("Derived", Some(&CHILD)) }; 16];
        let table = MetadataTable::new(&ROOT, StyledPropertyMetadata::new(Some(1)));
        table.override_metadata("Number", &CHILD, StyledPropertyMetadata::new(Some(2)));
        table.override_metadata("Number", &TYPES[0], StyledPropertyMetadata::new(Some(3)));
        for _ in 0..2 {
            for (i, host) in TYPES.iter().enumerate() {
                assert_eq!(*table.get(host).default_value(), if i == 0 { 3 } else { 2 });
            }
        }
        table.unregister(&CHILD);
        for (i, host) in TYPES.iter().enumerate() {
            assert_eq!(*table.get(host).default_value(), if i == 0 { 3 } else { 1 });
        }
        table.override_metadata("Number", &CHILD, StyledPropertyMetadata::new(Some(4)));
        for (i, host) in TYPES.iter().enumerate().rev() {
            assert_eq!(*table.get(host).default_value(), if i == 0 { 3 } else { 4 });
        }
    }

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
