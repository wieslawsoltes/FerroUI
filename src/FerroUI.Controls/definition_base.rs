// This source file is adapted from the Windows Presentation Foundation project.
// (https://github.com/dotnet/wpf/)

use crate::grid::LayoutTimeSizeType;
use crate::{Control, Grid, GridLength, GridUnitType};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::math_utilities::{max as math_max, MathUtilities};
use ferroui_base::{
    ferro_class, ferro_property, AttachedProperty, FerroObject, FerroObjectImpl, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledPropertyOptions, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// When set, a definition takes into account the minimum of its shared state.
const FLAG_USE_SHARED_MINIMUM: u8 = 0x20;
/// Set every time the parent grid is measured.
const FLAG_LAYOUT_WAS_UPDATED: u8 = 0x40;

/// Provides the core functionality used internally by `Grid` and the column
/// and row definition collections.
///
/// This class is abstract: only its subclasses [`crate::ColumnDefinition`]
/// and [`crate::RowDefinition`] can be created.
#[repr(C)]
pub struct DefinitionBase {
    base: FerroObject,
    /// Flags reflecting various aspects of the internal state.
    flags: Cell<u8>,
    /// The index of this instance in the parent's collection.
    parent_index: Cell<i32>,
    /// The layout-time user size type. It may differ from the unit type of
    /// the user size when calculating "to content".
    size_type: Cell<LayoutTimeSizeType>,
    /// Used during measure to accumulate the size of "Auto" and "Star"
    /// definitions.
    min_size: Cell<f64>,
    /// The size calculated to be the input constraint size for the measure
    /// of a child.
    measure_size: Cell<f64>,
    /// A cache used for various purposes (sorting, caching, etc) during
    /// calculations.
    size_cache: Cell<f64>,
    /// The offset of the definition from the left / top corner.
    offset: Cell<f64>,
    /// The shared state object this instance is registered with.
    shared_state: RefCell<Option<Rc<SharedSizeState>>>,
    parent: RefCell<Option<WeakRef<Grid>>>,
}

ferro_class! {
    DefinitionBase: FerroObject, virtuals DefinitionBaseImpl: FerroObjectImpl {
        /// Internal helper to access the up-to-date user size property value.
        #[doc(hidden)]
        fn user_size_value_cache(this) -> GridLength;
        /// Internal helper to access the up-to-date user minimum size
        /// property value.
        #[doc(hidden)]
        fn user_min_size_value_cache(this) -> f64;
        /// Internal helper to access the up-to-date user maximum size
        /// property value.
        #[doc(hidden)]
        fn user_max_size_value_cache(this) -> f64;
    }
}

ferroui_base::ferro_impl_classes!(DefinitionBase: FerroObjectImpl);

impl DefinitionBaseImpl for DefinitionBase {
    fn user_size_value_cache(_this: &Self) -> GridLength {
        panic!("DefinitionBase is abstract.")
    }

    fn user_min_size_value_cache(_this: &Self) -> f64 {
        panic!("DefinitionBase is abstract.")
    }

    fn user_max_size_value_cache(_this: &Self) -> f64 {
        panic!("DefinitionBase is abstract.")
    }
}

impl Drop for DefinitionBase {
    fn drop(&mut self) {
        // A definition that is dropped while still registered (its grid was
        // dropped without detaching it) leaves its shared state.
        if let Some(shared_state) = self.shared_state.get_mut().take() {
            shared_state.remove_dropped_members();
        }
    }
}

ferroui_base::ferro_properties! { impl DefinitionBase {
    ferro_property!(
        /// Holds the collection of shared state objects of a given shared
        /// size scope; see `on_is_shared_size_scope_property_changed`.
        pub(crate) fn private_shared_size_scope_property() -> AttachedProperty<Option<Rc<SharedSizeScope>>> {
            FerroProperty::register_attached_with::<DefinitionBase, Control, _>(
                "PrivateSharedSizeScope",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `SharedSizeGroup` property, which marks a column / row
        /// definition as belonging to a group "Foo" or "Bar".
        ///
        /// The value must satisfy the following rules:
        ///
        /// * the string must not be empty;
        /// * the string must consist of letters, digits and underscore ('_')
        ///   only;
        /// * the string must not start with a digit.
        pub fn shared_size_group_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached_with::<DefinitionBase, Control, _>(
                "SharedSizeGroup",
                StyledPropertyOptions::new(None).validate(DefinitionBase::shared_size_group_property_value_valid),
            )
        }
    );
} }

impl DefinitionBase {
    fn static_constructor() {
        Self::shared_size_group_property()
            .changed()
            .add_class_handler::<DefinitionBase>(Self::on_shared_size_group_property_changed);
        Self::private_shared_size_scope_property()
            .changed()
            .add_class_handler::<DefinitionBase>(Self::on_private_shared_size_scope_property_changed);
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub(crate) fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            flags: Cell::new(0),
            parent_index: Cell::new(-1),
            size_type: Cell::new(LayoutTimeSizeType::NONE),
            min_size: Cell::new(0.0),
            measure_size: Cell::new(0.0),
            size_cache: Cell::new(0.0),
            offset: Cell::new(0.0),
            shared_state: RefCell::new(None),
            parent: RefCell::new(None),
        }
    }

    /// The shared size group of the definition.
    pub fn shared_size_group(&self) -> Option<String> {
        self.get_value(Self::shared_size_group_property())
    }

    pub fn set_shared_size_group(&self, value: Option<&str>) {
        self.set_value(Self::shared_size_group_property(), value.map(str::to_owned))
    }

    /// Callback to notify about entering the model tree.
    pub(crate) fn on_enter_parent_tree(&self) {
        self.set_inheritance_parent(self.parent().map(Ref::upcast::<FerroObject>));
        if self.shared_state.borrow().is_none() {
            // Start with getting the shared size group value. This property
            // is NOT inherited which should result in better overall
            // performance.
            if let Some(shared_size_group_id) = self.shared_size_group() {
                if let Some(private_shared_size_scope) = self.get_value(Self::private_shared_size_scope_property()) {
                    self.register_with_shared_state(&private_shared_size_scope, &shared_size_group_id);
                }
            }
        }

        if let Some(parent) = self.parent() {
            parent.invalidate_measure();
        }
    }

    /// Callback to notify about exiting the model tree.
    pub(crate) fn on_exit_parent_tree(&self) {
        self.offset.set(0.0);
        self.unregister_from_shared_state();

        if let Some(parent) = self.parent() {
            parent.invalidate_measure();
        }

        // While this link survives the definition still reads the grid's
        // inherited shared size scope, and the grid holds it as an
        // inheritance child.
        self.set_inheritance_parent(None::<Ref<FerroObject>>);
    }

    /// Prepares the definition to enter the layout calculation mode.
    pub(crate) fn on_before_layout(&self, grid: &Grid) {
        // Reset the layout state.
        self.min_size.set(0.0);
        self.set_layout_was_updated(true);

        // Defer the verification for shared definitions.
        let shared_state = self.shared_state.borrow().clone();
        if let Some(shared_state) = shared_state {
            shared_state.ensure_deferred_validation(grid);
        }
    }

    /// Updates the minimum size.
    #[inline]
    pub(crate) fn update_min_size(&self, min_size: f64) {
        self.min_size.set(math_max(self.min_size.get(), min_size));
    }

    /// Sets the minimum size.
    #[inline]
    pub(crate) fn set_min_size(&self, min_size: f64) {
        self.min_size.set(min_size);
    }

    /// Reflects the state of the `IsSharedSizeScope` property of a control
    /// by setting / clearing the private shared size scope property, whose
    /// value is the collection of shared states objects of the scope.
    pub(crate) fn on_is_shared_size_scope_property_changed(d: &Control, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.get_new_value::<bool>() {
            let shared_states_collection = SharedSizeScope::new();
            d.set_value(
                Self::private_shared_size_scope_property(),
                Some(shared_states_collection),
            );
        } else {
            d.clear_value(Self::private_shared_size_scope_property());
        }
    }

    /// Notifies the parent grid or the size scope that the size of a
    /// definition has been changed.
    pub(crate) fn on_user_size_property_changed(definition: &DefinitionBase, e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(parent) = definition.parent() else {
            return;
        };

        let shared_state = definition.shared_state.borrow().clone();
        if let Some(shared_state) = shared_state {
            shared_state.invalidate();
        } else {
            let (old_value, new_value) = e.get_old_and_new_value::<GridLength>();

            if old_value.grid_unit_type() != new_value.grid_unit_type() {
                parent.invalidate();
            } else {
                parent.invalidate_measure();
            }
        }
    }

    /// Whether this definition is a part of a shared group.
    #[inline]
    pub(crate) fn is_shared(&self) -> bool {
        self.shared_state.borrow().is_some()
    }

    /// Internal accessor to the user size.
    pub(crate) fn user_size(&self) -> GridLength {
        let shared_state = self.shared_state.borrow().clone();
        match shared_state {
            Some(shared_state) => shared_state.user_size(),
            None => self.user_size_value_cache(),
        }
    }

    /// Internal accessor to the user minimum size.
    #[inline]
    pub(crate) fn user_min_size(&self) -> f64 {
        self.user_min_size_value_cache()
    }

    /// Internal accessor to the user maximum size.
    #[inline]
    pub(crate) fn user_max_size(&self) -> f64 {
        self.user_max_size_value_cache()
    }

    /// The index of the definition in the parent's collection, or -1.
    #[inline]
    pub(crate) fn index(&self) -> i32 {
        self.parent_index.get()
    }

    #[inline]
    pub(crate) fn set_index(&self, value: i32) {
        debug_assert!(value >= -1);
        self.parent_index.set(value);
    }

    /// The layout-time user size type.
    #[inline]
    pub(crate) fn size_type(&self) -> LayoutTimeSizeType {
        self.size_type.get()
    }

    #[inline]
    pub(crate) fn set_size_type(&self, value: LayoutTimeSizeType) {
        self.size_type.set(value);
    }

    /// The measure size of the definition.
    #[inline]
    pub(crate) fn measure_size(&self) -> f64 {
        self.measure_size.get()
    }

    #[inline]
    pub(crate) fn set_measure_size(&self, value: f64) {
        self.measure_size.set(value);
    }

    /// The layout time type sensitive preferred size of the definition.
    ///
    /// The returned value is guaranteed to be the true preferred size.
    pub(crate) fn preferred_size(&self) -> f64 {
        let mut preferred_size = self.min_size();
        if self.size_type.get() != LayoutTimeSizeType::AUTO && preferred_size < self.measure_size.get() {
            preferred_size = self.measure_size.get();
        }
        preferred_size
    }

    /// The size cache of the definition.
    #[inline]
    pub(crate) fn size_cache(&self) -> f64 {
        self.size_cache.get()
    }

    #[inline]
    pub(crate) fn set_size_cache(&self, value: f64) {
        self.size_cache.set(value);
    }

    /// The minimum size.
    pub(crate) fn min_size(&self) -> f64 {
        let mut min_size = self.min_size.get();
        if self.use_shared_minimum() {
            if let Some(shared_state) = self.shared_state.borrow().as_ref() {
                let shared_min_size = shared_state.min_size();
                if min_size < shared_min_size {
                    min_size = shared_min_size;
                }
            }
        }
        min_size
    }

    /// The minimum size, always taking into account the shared state.
    pub(crate) fn min_size_for_arrange(&self) -> f64 {
        let mut min_size = self.min_size.get();
        if let Some(shared_state) = self.shared_state.borrow().as_ref() {
            if self.use_shared_minimum() || !self.layout_was_updated() {
                let shared_min_size = shared_state.min_size();
                if min_size < shared_min_size {
                    min_size = shared_min_size;
                }
            }
        }
        min_size
    }

    /// The minimum size, never taking into account the shared state.
    ///
    /// This is the definition's own intrinsic contribution to its group. It
    /// is the counterpart of `set_min_size`: code that saves and restores a
    /// minimum size across a measure must round-trip this value, because
    /// feeding the group minimum back into the minimum size would make the
    /// group unable to shrink.
    #[inline]
    pub(crate) fn raw_min_size(&self) -> f64 {
        self.min_size.get()
    }

    /// The final offset.
    #[inline]
    pub(crate) fn final_offset(&self) -> f64 {
        self.offset.get()
    }

    #[inline]
    pub(crate) fn set_final_offset(&self, value: f64) {
        self.offset.set(value);
    }

    /// The grid the definition belongs to.
    pub(crate) fn parent(&self) -> Option<Ref<Grid>> {
        self.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_parent(&self, value: Option<&Ref<Grid>>) {
        *self.parent.borrow_mut() = value.map(Ref::downgrade);
    }

    /// Registers the definition with the shared state of a group.
    fn register_with_shared_state(&self, private_shared_size_scope: &Rc<SharedSizeScope>, shared_size_group: &str) {
        let shared_state = private_shared_size_scope.ensure_shared_state(shared_size_group);
        *self.shared_state.borrow_mut() = Some(shared_state.clone());
        shared_state.add_member(self);
    }

    /// Un-registers the definition from its current shared state, if any.
    fn unregister_from_shared_state(&self) {
        let shared_state = self.shared_state.borrow().clone();
        if let Some(shared_state) = shared_state {
            shared_state.remove_member(self);
            *self.shared_state.borrow_mut() = None;
        }
    }

    fn on_shared_size_group_property_changed(definition: &DefinitionBase, e: &FerroPropertyChangedEventArgs<'_>) {
        if definition.parent().is_some() {
            let shared_size_group_id = e.get_new_value::<Option<String>>();

            // If the definition is already registered AND the shared size
            // group id is changing, then un-register the definition from the
            // current shared size state object.
            definition.unregister_from_shared_state();

            if let Some(shared_size_group_id) = shared_size_group_id {
                if let Some(private_shared_size_scope) =
                    definition.get_value(Self::private_shared_size_scope_property())
                {
                    // If the definition is not registered and both the shared
                    // size group id AND the private shared scope are
                    // available, then register the definition.
                    definition.register_with_shared_state(&private_shared_size_scope, &shared_size_group_id);
                }
            }
        }
    }

    /// Verifies that a shared size group string
    /// a) is not empty;
    /// b) contains only letters, digits and underscore ('_');
    /// c) does not start with a digit.
    fn shared_size_group_property_value_valid(id: &Option<String>) -> bool {
        // None is the default value.
        let Some(id) = id else {
            return true;
        };

        if id.is_empty() {
            return false;
        }

        id.chars().enumerate().all(|(i, c)| {
            let is_digit = c.is_numeric();
            !(i == 0 && is_digit) && (is_digit || c.is_alphabetic() || c == '_')
        })
    }

    /// Called when a new scope enters or the existing scope just left. In
    /// both cases if the definition is already registered in a shared state,
    /// it should un-register and register itself in a new one.
    fn on_private_shared_size_scope_property_changed(
        definition: &DefinitionBase,
        e: &FerroPropertyChangedEventArgs<'_>,
    ) {
        if definition.parent().is_some() {
            // If the definition is already registered AND the shared size
            // scope is changing, then un-register the definition from the
            // current shared size state object.
            definition.unregister_from_shared_state();

            if let Some(private_shared_size_scope) = e.get_new_value::<Option<Rc<SharedSizeScope>>>() {
                if let Some(shared_size_group) = definition.shared_size_group() {
                    // If the definition is not registered and both the shared
                    // size group id AND the private shared scope are
                    // available, then register the definition.
                    definition.register_with_shared_state(&private_shared_size_scope, &shared_size_group);
                }
            }
        }
    }

    #[inline]
    fn set_flags(&self, value: bool, flags: u8) {
        self.flags.set(if value {
            self.flags.get() | flags
        } else {
            self.flags.get() & !flags
        });
    }

    #[inline]
    fn check_flags_and(&self, flags: u8) -> bool {
        (self.flags.get() & flags) == flags
    }

    #[inline]
    fn use_shared_minimum(&self) -> bool {
        self.check_flags_and(FLAG_USE_SHARED_MINIMUM)
    }

    #[inline]
    fn set_use_shared_minimum(&self, value: bool) {
        self.set_flags(value, FLAG_USE_SHARED_MINIMUM)
    }

    #[inline]
    fn layout_was_updated(&self) -> bool {
        self.check_flags_and(FLAG_LAYOUT_WAS_UPDATED)
    }

    #[inline]
    fn set_layout_was_updated(&self, value: bool) {
        self.set_flags(value, FLAG_LAYOUT_WAS_UPDATED)
    }

    /// Marks a property on a definition as affecting the parent grid's
    /// measurement.
    pub(crate) fn affects_parent_measure(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                if let Some(parent) = e
                    .sender()
                    .downcast_ref::<DefinitionBase>()
                    .and_then(DefinitionBase::parent)
                {
                    parent.invalidate_measure();
                }
            });
        }
    }
}

/// The collection of shared states objects of a single scope.
pub(crate) struct SharedSizeScope {
    this: Weak<SharedSizeScope>,
    /// The storage for the shared state objects.
    registry: RefCell<HashMap<String, Rc<SharedSizeState>>>,
}

impl PartialEq for SharedSizeScope {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl SharedSizeScope {
    fn new() -> Rc<SharedSizeScope> {
        Rc::new_cyclic(|this| SharedSizeScope {
            this: this.clone(),
            registry: RefCell::new(HashMap::new()),
        })
    }

    /// Returns the shared state object of a given group, creating it if
    /// necessary.
    fn ensure_shared_state(&self, shared_size_group: &str) -> Rc<SharedSizeState> {
        if let Some(shared_state) = self.registry.borrow().get(shared_size_group) {
            return shared_state.clone();
        }

        let shared_state = SharedSizeState::new(self.this.clone(), shared_size_group);
        self.registry
            .borrow_mut()
            .insert(shared_size_group.to_owned(), shared_state.clone());
        shared_state
    }

    /// Removes an entry in the registry by the given key.
    fn remove(&self, key: &str) {
        self.registry.borrow_mut().remove(key);
    }
}

/// The control for which a layout updated handler is registered, and the
/// registration.
type LayoutUpdatedHost = (WeakRef<Grid>, Rc<dyn IDisposable>);

/// The per shared group state object.
pub(crate) struct SharedSizeState {
    this: Weak<SharedSizeState>,
    /// The scope this state belongs to.
    shared_size_scope: Weak<SharedSizeScope>,
    /// The id of the shared size group this object is servicing.
    shared_size_group_id: String,
    /// The registry of participating definitions.
    registry: RefCell<Vec<WeakRef<DefinitionBase>>>,
    /// Set while a layout updated handler is registered.
    layout_updated_host: RefCell<Option<LayoutUpdatedHost>>,
    /// True when broadcasting of an invalidation is needed.
    broadcast_invalidation: Cell<bool>,
    /// True when the user size is up to date.
    user_size_valid: Cell<bool>,
    user_size: Cell<GridLength>,
    min_size: Cell<f64>,
}

impl SharedSizeState {
    fn new(shared_size_scope: Weak<SharedSizeScope>, shared_size_group_id: &str) -> Rc<Self> {
        Rc::new_cyclic(|this| SharedSizeState {
            this: this.clone(),
            shared_size_scope,
            shared_size_group_id: shared_size_group_id.to_owned(),
            registry: RefCell::new(Vec::new()),
            layout_updated_host: RefCell::new(None),
            broadcast_invalidation: Cell::new(true),
            user_size_valid: Cell::new(false),
            user_size: Cell::new(GridLength::default()),
            min_size: Cell::new(0.0),
        })
    }

    /// The registered definition at `index`, unless it has been dropped.
    fn member(&self, index: usize) -> Option<Ref<DefinitionBase>> {
        self.registry.borrow().get(index).and_then(WeakRef::upgrade)
    }

    fn member_count(&self) -> usize {
        self.registry.borrow().len()
    }

    /// Adds / registers a definition instance.
    fn add_member(&self, member: &DefinitionBase) {
        let member = member.to_ref();
        debug_assert!(!self.registry.borrow().iter().any(|m| m.points_to(&member)));
        self.registry.borrow_mut().push(member.downgrade());
        self.invalidate();
    }

    /// Removes / un-registers a definition instance.
    ///
    /// If the collection of registered definitions becomes empty, the state
    /// removes itself from the collection of its owner.
    fn remove_member(&self, member: &DefinitionBase) {
        self.invalidate();
        let member = member.to_ref();
        self.registry
            .borrow_mut()
            .retain(|m| !m.points_to(&member) && m.upgrade().is_some());
        self.remove_from_scope_if_empty();
    }

    /// Forgets the registered definitions that no longer exist.
    fn remove_dropped_members(&self) {
        self.registry.borrow_mut().retain(|m| m.upgrade().is_some());
        self.remove_from_scope_if_empty();
    }

    fn remove_from_scope_if_empty(&self) {
        if self.registry.borrow().is_empty() {
            if let Some(shared_size_scope) = self.shared_size_scope.upgrade() {
                shared_size_scope.remove(&self.shared_size_group_id);
            }
        }
    }

    /// Propagates invalidations for all registered definitions and resets
    /// its own state.
    fn invalidate(&self) {
        self.user_size_valid.set(false);

        if self.broadcast_invalidation.get() {
            for i in 0..self.member_count() {
                if let Some(parent_grid) = self.member(i).and_then(|m| m.parent()) {
                    parent_grid.invalidate();
                }
            }
            self.broadcast_invalidation.set(false);
        }
    }

    /// Makes sure that one and only one layout updated handler is registered
    /// for this shared state.
    fn ensure_deferred_validation(&self, layout_updated_host: &Grid) {
        let has_host = self
            .layout_updated_host
            .borrow()
            .as_ref()
            .is_some_and(|(host, _)| host.upgrade().is_some());
        if !has_host {
            let this = self.this.clone();
            let subscription = layout_updated_host.layout_updated(move || {
                if let Some(this) = this.upgrade() {
                    this.on_layout_updated();
                }
            });
            *self.layout_updated_host.borrow_mut() = Some((layout_updated_host.to_ref().downgrade(), subscription));
        }
    }

    fn min_size(&self) -> f64 {
        if !self.user_size_valid.get() {
            self.ensure_user_size_valid();
        }
        self.min_size.get()
    }

    fn user_size(&self) -> GridLength {
        if !self.user_size_valid.get() {
            self.ensure_user_size_valid();
        }
        self.user_size.get()
    }

    fn ensure_user_size_valid(&self) {
        let mut user_size = GridLength::new(1.0, GridUnitType::Auto);

        for i in 0..self.member_count() {
            debug_assert!(
                user_size.grid_unit_type() == GridUnitType::Auto || user_size.grid_unit_type() == GridUnitType::Pixel
            );

            let Some(member) = self.member(i) else {
                continue;
            };
            let current_grid_length = member.user_size_value_cache();
            if current_grid_length.grid_unit_type() == GridUnitType::Pixel
                && (user_size.grid_unit_type() == GridUnitType::Auto || user_size.value() < current_grid_length.value())
            {
                user_size = current_grid_length;
            }
        }
        self.user_size.set(user_size);

        // Taking the maximum with the user size effectively prevents
        // squishy-ness. This is a "solution" to avoid shared definitions from
        // being sized to a different final size at arrange time, if / when
        // different grids receive different final sizes.
        self.min_size.set(if user_size.is_absolute() {
            user_size.value()
        } else {
            0.0
        });

        self.user_size_valid.set(true);
    }

    /// The layout updated handler. Validates that all participating
    /// definitions have an updated minimum size value. Forces another layout
    /// update cycle if needed.
    fn on_layout_updated(&self) {
        let mut shared_min_size: f64 = 0.0;

        // Accumulate the minimum size of all participating definitions.
        for i in 0..self.member_count() {
            if let Some(member) = self.member(i) {
                shared_min_size = math_max(shared_min_size, member.min_size.get());
            }
        }

        let shared_min_size_changed = !MathUtilities::are_close(self.min_size.get(), shared_min_size);

        // Compare the accumulated minimum size with the minimum sizes of the
        // individual definitions.
        for i in 0..self.member_count() {
            let Some(definition_base) = self.member(i) else {
                continue;
            };

            // We'll set the definition's "use shared minimum" flag to
            // maintain the invariant:
            //      d.use_shared_minimum iff d.min_size < this.min_size
            // i.e. iff d is not a "long-pole" definition.
            //
            // Measure/Arrange of d's grid uses d.min_size for long-pole
            // definitions, and max(d.min_size, shared size) for short-pole
            // definitions. This distinction allows us to react to changes in
            // "long-pole-ness" more efficiently and correctly, by avoiding
            // remeasures when a long-pole definition changes.
            let use_shared_minimum = !MathUtilities::are_close(definition_base.min_size.get(), shared_min_size);

            // Before doing that, determine whether d's grid needs to be
            // remeasured. It's important _not_ to remeasure if the last
            // measure is still valid, otherwise infinite loops are possible.
            let measure_is_valid = if !definition_base.use_shared_minimum() {
                // d was a long-pole. The measure is valid iff it's still a
                // long-pole, since the previous measure didn't use the shared
                // size.
                !use_shared_minimum
            } else if use_shared_minimum {
                // d was a short-pole, and still is. The measure is valid iff
                // the shared size didn't change.
                !shared_min_size_changed
            } else {
                // d was a short-pole, but is now a long-pole. This can happen
                // in several ways:
                //  a. d's min size increased to or past the old shared size
                //  b. other long-pole definitions decreased, leaving d as the
                //     new winner
                // In the former case, the measure is valid - it used d's new
                // larger min size. In the latter case, the measure is invalid
                // - it used the old shared size, which is larger than d's
                // (possibly changed) min size.
                definition_base.layout_was_updated()
                    && MathUtilities::greater_than_or_close(definition_base.min_size.get(), self.min_size())
            };

            if !measure_is_valid {
                if let Some(parent) = definition_base.parent() {
                    parent.invalidate_measure();
                }
            } else if !MathUtilities::are_close(shared_min_size, definition_base.size_cache()) {
                // If the measure is valid then also need to check arrange.
                // Note: the size cache is volatile but at this point it
                // contains the up-to-date final size.
                if let Some(parent) = definition_base.parent() {
                    parent.invalidate_arrange();
                }
            }

            // Now we can restore the invariant, and clear the layout flag.
            definition_base.set_use_shared_minimum(use_shared_minimum);
            definition_base.set_layout_was_updated(false);
        }

        self.min_size.set(shared_min_size);

        let layout_updated_host = self.layout_updated_host.borrow_mut().take();
        if let Some((_, subscription)) = layout_updated_host {
            subscription.dispose();
        }

        self.broadcast_invalidation.set(true);
    }
}
