use super::effective_value::cast_effective_value;
use super::{
    entry_ptr_eq, frame_ptr_eq, BindingSource, DirectBindingObserver, EffectiveValue, EffectiveValueDyn,
    FramePriority, FrameType, ImmediateValueFrame, IValueEntry, LocalValueBindingObserver, ValueFrame,
};
use crate::data::core::SinkRef;
use crate::data::{BindingExpressionBase, BindingPriority, BindingValueType};
use crate::reactive::IDisposable;
use crate::{DirectPropertyBase, FerroObject, FerroProperty, PropertyValue, Ref, StyledProperty, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type EffectiveValueRef = Rc<dyn EffectiveValueDyn>;

/// Stores the styled property values of a [`FerroObject`] and resolves the
/// effective value of each property from the values supplied at different
/// priorities.
///
/// The store lives inside its owner; every operation takes the owner so that
/// change notifications can be raised on it.
pub(crate) struct ValueStore {
    /// Value frames ordered from lowest to highest precedence.
    frames: RefCell<Vec<Rc<dyn ValueFrame>>>,
    local_value_bindings: RefCell<Vec<(u32, Rc<dyn IDisposable>)>>,
    /// The local value bindings that are binding expressions.
    local_value_expressions: RefCell<Vec<(u32, Rc<dyn BindingExpressionBase>)>>,
    /// Effective values sorted by property ID.
    effective_values: RefCell<Vec<(u32, EffectiveValueRef)>>,
    inherited_value_count: Cell<u32>,
    is_evaluating: Cell<u32>,
    frame_generation: Cell<u32>,
    styling: Cell<u32>,
    /// The nearest ancestor that holds inherited values. `Some(Weak)` pointing
    /// at the owner itself when the owner holds inherited values.
    inheritance_ancestor: RefCell<Option<WeakRef<FerroObject>>>,
}

impl ValueStore {
    pub fn new() -> Self {
        Self {
            frames: RefCell::new(Vec::new()),
            local_value_bindings: RefCell::new(Vec::new()),
            local_value_expressions: RefCell::new(Vec::new()),
            effective_values: RefCell::new(Vec::new()),
            inherited_value_count: Cell::new(0),
            is_evaluating: Cell::new(0),
            frame_generation: Cell::new(0),
            styling: Cell::new(0),
            inheritance_ancestor: RefCell::new(None),
        }
    }

    pub fn is_evaluating(&self) -> bool {
        self.is_evaluating.get() > 0
    }

    pub fn frames(&self) -> Vec<Rc<dyn ValueFrame>> {
        self.frames.borrow().clone()
    }

    pub fn inheritance_ancestor(&self) -> Option<Ref<FerroObject>> {
        self.inheritance_ancestor.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub fn begin_styling(&self) {
        self.styling.set(self.styling.get() + 1);
    }

    pub fn end_styling(&self, owner: &FerroObject) {
        let styling = self.styling.get() - 1;
        self.styling.set(styling);
        if styling == 0 {
            self.reevaluate_effective_values(owner, None);
        }
    }

    pub fn add_frame(&self, owner: &FerroObject, frame: Rc<dyn ValueFrame>) {
        self.insert_frame(owner, frame);
        self.reevaluate_effective_values(owner, None);
    }

    /// Adds a binding to a styled property.
    pub fn add_binding<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
        source: BindingSource<T>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        if priority == BindingPriority::LocalValue {
            let observer = LocalValueBindingObserver::new(owner, property);
            self.dispose_existing_local_value_binding(property);
            self.set_local_value_binding(property, observer.clone());
            observer.start(source);
            observer
        } else {
            let effective = self.get_effective_value(property);
            let frame = self.get_or_create_immediate_value_frame(owner, property, priority);
            let result = frame.add_binding(property, source);
            if effective.is_none_or(|e| priority <= e.priority()) {
                result.start();
            }
            result
        }
    }

    /// Adds a binding to a direct property.
    pub fn add_direct_binding<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static DirectPropertyBase<T>,
        source: BindingSource<T>,
    ) -> Rc<dyn IDisposable> {
        let observer = DirectBindingObserver::new(owner, property);
        self.dispose_existing_local_value_binding(property);
        self.set_local_value_binding(property, observer.clone());
        observer.start(source);
        observer
    }

    /// Adds a binding expression to a property. The expression's default
    /// priority decides where it is stored; bindings to direct properties
    /// always have local value priority.
    pub fn add_binding_expression(
        &self,
        owner: &FerroObject,
        property: &'static FerroProperty,
        source: Rc<dyn BindingExpressionBase>,
    ) -> Rc<dyn BindingExpressionBase> {
        let priority = source.default_priority();

        if priority == BindingPriority::LocalValue || property.is_direct() {
            self.dispose_existing_local_value_binding(property);
            let disposable: Rc<dyn IDisposable> = source.clone();
            self.set_local_value_binding(property, disposable);
            self.local_value_expressions.borrow_mut().push((property.id(), source.clone()));
            source.attach(SinkRef::Store(owner.to_weak()), None, owner, property, BindingPriority::LocalValue);
            source.start(true);
        } else {
            let effective = self.get_effective_value(property);
            let frame = self.get_or_create_immediate_value_frame(owner, property, priority);
            source.attach(SinkRef::Store(owner.to_weak()), Some(Rc::downgrade(&frame)), owner, property, priority);
            frame.add_binding_expression(source.clone());
            if effective.is_none_or(|e| priority <= e.priority()) {
                source.start(true);
            }
        }
        source
    }

    /// The binding expression that is currently active on the property, if
    /// any.
    pub fn get_expression(&self, property: &'static FerroProperty) -> Option<Rc<dyn BindingExpressionBase>> {
        let local = || {
            self.local_value_expressions
                .borrow()
                .iter()
                .find(|(id, _)| *id == property.id())
                .map(|(_, e)| e.clone())
        };
        let has_local = self.local_value_bindings.borrow().iter().any(|(id, _)| *id == property.id());

        for frame in self.frames().iter().rev() {
            if frame.base().priority() > BindingPriority::LocalValue && has_local {
                return local();
            }
            if let (Some(entry), _) = frame.try_get_entry_if_active(property) {
                return entry.as_binding_expression();
            }
        }
        local()
    }

    /// Called by a binding expression attached to this store when its value
    /// or error state changes.
    pub fn on_expression_changed(
        &self,
        owner: &FerroObject,
        instance: &Rc<dyn BindingExpressionBase>,
        has_value_changed: bool,
        has_error_changed: bool,
    ) {
        let property = instance.target_property().expect("the binding expression is attached");
        let entry: Rc<dyn IValueEntry> = instance.clone();

        if property.is_direct() {
            if has_value_changed {
                property.routes().route_set_direct_value_unchecked(owner, &*entry);
            }
        } else if has_value_changed {
            let priority = instance.priority();
            if priority == BindingPriority::LocalValue {
                let effective = match self.get_effective_value(property) {
                    Some(existing) => existing,
                    None => {
                        let created = property.routes().create_effective_value(owner);
                        self.add_effective_value(owner, property, created.clone());
                        created
                    }
                };
                effective.set_local_value_and_raise_entry(owner, &*entry);
            } else {
                match self.get_effective_value(property) {
                    Some(existing) => {
                        if priority <= existing.base_priority() {
                            self.reevaluate_effective_value(owner, property, Some(existing), Some(&entry), false);
                        }
                    }
                    None => self.add_effective_value_and_raise(owner, property, &entry, priority),
                }
            }
        }

        if has_error_changed {
            if let Some((state, error)) = entry.get_data_validation_state() {
                owner.on_update_data_validation(property, state, error);
            }
        }
    }

    /// Called by a binding expression attached to this store when it
    /// completes.
    pub fn on_expression_completed(&self, owner: &FerroObject, instance: &Rc<dyn BindingExpressionBase>) {
        let property = instance.target_property().expect("the binding expression is attached");

        if instance.is_data_validation_enabled() {
            owner.on_update_data_validation(property, BindingValueType::UNSET_VALUE, None);
        }

        if instance.priority() == BindingPriority::LocalValue {
            let disposable: Rc<dyn IDisposable> = instance.clone();
            self.on_local_value_binding_completed(owner, property, &disposable);
        }
    }

    pub fn clear_value(&self, owner: &FerroObject, property: &'static FerroProperty) {
        if let Some(effective) = self.get_effective_value(property) {
            if effective.priority() == BindingPriority::LocalValue || effective.is_overriden_current_value() {
                effective.set_is_overriden_current_value(false);
                self.reevaluate_effective_value(owner, property, Some(effective), None, true);
            }
        }
    }

    pub fn set_value<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
        value: T,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        if !property.is_valid(&value) {
            property.value_not_valid();
        }

        if priority != BindingPriority::LocalValue {
            let frame = self.get_or_create_immediate_value_frame(owner, property, priority);
            let result = frame.add_value(property, value);
            let entry: Rc<dyn IValueEntry> = result.clone();
            let effective = match self.get_effective_value(property) {
                Some(existing) => existing,
                None => {
                    let created: EffectiveValueRef = self.create_effective_value(owner, property);
                    self.add_effective_value(owner, property, created.clone());
                    created
                }
            };
            effective.set_and_raise(owner, &entry, priority);
            Some(result)
        } else {
            self.set_local_value(owner, property, value);
            None
        }
    }

    pub fn set_current_value<T: PropertyValue>(&self, owner: &FerroObject, property: &'static StyledProperty<T>, value: T) {
        let effective = self.get_or_add_effective_value(owner, property);
        cast_effective_value::<T>(&*effective).set_current_value_and_raise(owner, value);
    }

    pub fn set_local_value<T: PropertyValue>(&self, owner: &FerroObject, property: &'static StyledProperty<T>, value: T) {
        let effective = self.get_or_add_effective_value(owner, property);
        cast_effective_value::<T>(&*effective).set_local_value_and_raise(owner, value);
    }

    /// The value of a styled property: the effective value, else the inherited
    /// value, else the default value.
    #[inline]
    pub fn get_value<T: PropertyValue>(&self, owner: &FerroObject, property: &'static StyledProperty<T>) -> T {
        // <!> Performance critical method.
        {
            let values = self.effective_values.borrow();
            if let Some(index) = Self::find(&values, property.id()) {
                return cast_effective_value::<T>(&*values[index].1).value();
            }
        }
        if property.inherits() {
            if let Some(v) = self.try_get_inherited_value(property) {
                return cast_effective_value::<T>(&*v).value();
            }
        }
        property.get_default_value_for(owner)
    }

    pub fn is_animating(&self, property: &FerroProperty) -> bool {
        self.get_effective_value(property).is_some_and(|v| v.priority() <= BindingPriority::Animation)
    }

    pub fn is_set(&self, property: &FerroProperty) -> bool {
        Self::find(&self.effective_values.borrow(), property.id()).is_some()
    }

    pub fn coerce_value(&self, owner: &FerroObject, property: &'static FerroProperty) {
        match self.get_effective_value(property) {
            Some(v) => v.coerce_value(owner),
            None => property.routes().route_coerce_default_value(owner),
        }
    }

    pub fn coerce_default_value<T: PropertyValue>(&self, owner: &FerroObject, property: &'static StyledProperty<T>) {
        let metadata = property.get_metadata_for(owner);
        let Some(coerce) = metadata.coerce_value() else { return };
        let default = metadata.default_value().clone();
        let coerced = coerce(owner, default.clone());
        if default == coerced {
            return;
        }
        // The default value isn't valid according to the coerce function, so
        // an effective value entry is needed.
        let effective = self.create_effective_value(owner, property);
        self.add_effective_value(owner, property, effective.clone());
        effective.set_coerced_default_value_and_raise(owner, coerced);
    }

    pub fn get_base_value<T: PropertyValue>(&self, property: &'static StyledProperty<T>) -> Option<T> {
        let effective = self.get_effective_value(property)?;
        cast_effective_value::<T>(&*effective).try_get_base_value()
    }

    /// Finds the effective value of an inherited property on the nearest
    /// ancestor that has one.
    pub fn try_get_inherited_value(&self, property: &FerroProperty) -> Option<EffectiveValueRef> {
        debug_assert!(property.inherits());
        let mut current = self.inheritance_ancestor();
        while let Some(ancestor) = current {
            let store = ancestor.values();
            if let Some(v) = store.get_effective_value(property) {
                return Some(v);
            }
            let next = store.inheritance_ancestor();
            // A store that holds inherited values is its own ancestor marker;
            // never loop on it.
            current = match next {
                Some(n) if n.ptr_eq(&ancestor) => None,
                other => other,
            };
        }
        None
    }

    pub fn create_effective_value<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
    ) -> Rc<EffectiveValue<T>> {
        let inherited = if property.inherits() { self.try_get_inherited_value(property) } else { None };
        let inherited = inherited.as_deref().map(cast_effective_value::<T>);
        Rc::new(EffectiveValue::new(owner, property, inherited))
    }

    /// The ancestor store to use for inherited value lookups starting at
    /// `object`: the object itself if it holds inherited values, otherwise its
    /// own ancestor.
    fn ancestor_for(object: &Ref<FerroObject>) -> Option<Ref<FerroObject>> {
        if object.values().inherited_value_count.get() == 0 {
            object.values().parent_ancestor()
        } else {
            Some(object.clone())
        }
    }

    /// The inheritance ancestor excluding the owner itself.
    fn parent_ancestor(&self) -> Option<Ref<FerroObject>> {
        self.inheritance_ancestor()
    }

    pub fn set_inheritance_parent(&self, owner: &FerroObject, new_parent: Option<&Ref<FerroObject>>) {
        struct OldNew {
            property: &'static FerroProperty,
            old: Option<EffectiveValueRef>,
            new: Option<EffectiveValueRef>,
        }

        let old_ancestor = self.inheritance_ancestor();
        let new_ancestor = new_parent.and_then(Self::ancestor_for);

        // The old and new inheritance ancestors are the same, nothing to do.
        let same = match (&old_ancestor, &new_ancestor) {
            (Some(a), Some(b)) => a.ptr_eq(b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }

        let mut values: Vec<OldNew> = Vec::new();

        fn collect(
            start: Option<Ref<FerroObject>>,
            mut visit: impl FnMut(&'static FerroProperty, &EffectiveValueRef),
        ) {
            let mut current = start;
            while let Some(ancestor) = current {
                for (_, value) in ancestor.values().effective_values.borrow().iter() {
                    if value.property().inherits() {
                        visit(value.property(), value);
                    }
                }
                current = ancestor.values().parent_ancestor_of(&ancestor);
            }
        }

        // First get the old values from the old inheritance ancestor.
        collect(old_ancestor, |property, value| {
            if !values.iter().any(|v| v.property.id() == property.id()) {
                values.push(OldNew { property, old: Some(value.clone()), new: None });
            }
        });

        // Get the new values from the new inheritance ancestor.
        collect(new_ancestor.clone(), |property, value| {
            match values.iter_mut().find(|v| v.property.id() == property.id()) {
                Some(existing) => {
                    if existing.new.is_none() {
                        existing.new = Some(value.clone());
                    }
                }
                None => values.push(OldNew { property, old: None, new: Some(value.clone()) }),
            }
        });

        self.on_inheritance_ancestor_changed(owner, new_ancestor.as_ref());

        // Raise PropertyChanged events where necessary on this object and
        // inheritance children.
        for v in values {
            let same = match (&v.old, &v.new) {
                (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
                _ => false,
            };
            if !same {
                self.inherited_value_changed(owner, v.property, v.old.as_deref(), v.new.as_deref());
            }
        }
    }

    /// The next ancestor up the chain from `this_object`'s store, skipping the
    /// self-marker.
    fn parent_ancestor_of(&self, this_object: &Ref<FerroObject>) -> Option<Ref<FerroObject>> {
        match self.inheritance_ancestor() {
            Some(a) if a.ptr_eq(this_object) => {
                // This store is its own marker; continue from the owner's parent.
                this_object.inheritance_parent().and_then(|p| Self::ancestor_for(&p))
            }
            other => other,
        }
    }

    pub fn on_binding_value_changed(&self, owner: &FerroObject, entry: &Rc<dyn IValueEntry>, priority: BindingPriority) {
        debug_assert!(priority != BindingPriority::LocalValue);
        let property = entry.property();
        match self.get_effective_value(property) {
            Some(existing) => {
                if priority <= existing.base_priority() {
                    self.reevaluate_effective_value(owner, property, Some(existing), Some(entry), false);
                }
            }
            None => self.add_effective_value_and_raise(owner, property, entry, priority),
        }
    }

    pub fn on_frame_activation_changed(&self, owner: &FerroObject, frame: &Rc<dyn ValueFrame>) {
        match frame.base().entry_count() {
            0 => {}
            1 => {
                let property = frame.base().get_entry(0).property();
                let current = self.get_effective_value(property);
                self.reevaluate_effective_value(owner, property, current, None, false);
            }
            _ => self.reevaluate_effective_values(owner, None),
        }
    }

    /// Called when the nearest ancestor holding inherited values changes.
    /// `ancestor` is the new ancestor for the owner's children when the owner
    /// holds no inherited values itself.
    pub fn on_inheritance_ancestor_changed(&self, owner: &FerroObject, ancestor: Option<&Ref<FerroObject>>) {
        let is_self = ancestor.is_some_and(|a| {
            let a: &FerroObject = a;
            std::ptr::eq(a, owner)
        });
        if !is_self {
            *self.inheritance_ancestor.borrow_mut() = ancestor.map(Ref::downgrade);
            if self.inherited_value_count.get() > 0 {
                return;
            }
        }
        for child in owner.inheritance_children() {
            child.values().on_inheritance_ancestor_changed(&child, ancestor);
        }
    }

    pub fn on_inherited_effective_value_changed<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
        old_value: &T,
        new_value: &T,
    ) {
        debug_assert!(property.inherits());
        for child in owner.inheritance_children() {
            child.values().on_ancestor_inherited_value_changed(&child, property, old_value, new_value);
        }
    }

    pub fn on_inherited_effective_value_disposed<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
        old_value: &T,
        new_value: &T,
    ) {
        self.on_inherited_effective_value_changed(owner, property, old_value, new_value)
    }

    pub fn on_local_value_binding_completed(
        &self,
        owner: &FerroObject,
        property: &'static FerroProperty,
        observer: &Rc<dyn IDisposable>,
    ) {
        let removed = {
            let mut bindings = self.local_value_bindings.borrow_mut();
            match bindings.iter().position(|(id, _)| *id == property.id()) {
                Some(index) if std::ptr::addr_eq(Rc::as_ptr(&bindings[index].1), Rc::as_ptr(observer)) => {
                    bindings.remove(index);
                    self.local_value_expressions.borrow_mut().retain(|(id, _)| *id != property.id());
                    true
                }
                _ => false,
            }
        };
        if removed {
            self.clear_value(owner, property);
        }
    }

    fn on_ancestor_inherited_value_changed<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
        old_value: &T,
        new_value: &T,
    ) {
        // If the inherited value is set locally, propagation stops here.
        if self.is_set(property) {
            return;
        }
        let notifying = property.notifying();
        if let Some(notifying) = notifying {
            notifying(owner, true);
        }
        owner.raise_property_changed(property, Some(old_value), new_value, BindingPriority::Inherited, true);
        for child in owner.inheritance_children() {
            child.values().on_ancestor_inherited_value_changed(&child, property, old_value, new_value);
        }
        if let Some(notifying) = notifying {
            notifying(owner, false);
        }
    }

    pub fn on_value_entry_removed(&self, owner: &FerroObject, frame: &Rc<dyn ValueFrame>, property: &'static FerroProperty) {
        if frame.base().entry_count() == 0 {
            let mut frames = self.frames.borrow_mut();
            if let Some(index) = frames.iter().position(|f| frame_ptr_eq(f, frame)) {
                frames.remove(index);
            }
        }
        if let Some(existing) = self.get_effective_value(property) {
            if frame.base().priority() <= existing.priority() {
                self.reevaluate_effective_value(owner, property, Some(existing), None, false);
            }
        }
    }

    pub fn remove_frame(&self, owner: &FerroObject, frame: &Rc<dyn ValueFrame>) -> bool {
        let removed = {
            let mut frames = self.frames.borrow_mut();
            frames.iter().position(|f| frame_ptr_eq(f, frame)).map(|i| frames.remove(i))
        };
        match removed {
            Some(frame) => {
                frame.dispose();
                self.frame_generation.set(self.frame_generation.get().wrapping_add(1));
                self.reevaluate_effective_values(owner, None);
                true
            }
            None => false,
        }
    }

    /// Removes all style/theme frames of the given type (immediate value
    /// frames are kept).
    pub fn remove_frames(&self, owner: &FerroObject, type_: FrameType) {
        self.remove_frames_where(owner, |frame| {
            !frame.as_any().is::<ImmediateValueFrame>() && frame.base().frame_priority().is_type(type_)
        });
    }

    /// Removes all frames matching `predicate`.
    pub fn remove_frames_where(&self, owner: &FerroObject, predicate: impl Fn(&Rc<dyn ValueFrame>) -> bool) {
        let removed: Vec<Rc<dyn ValueFrame>> = {
            let mut frames = self.frames.borrow_mut();
            let mut removed = Vec::new();
            let mut i = frames.len();
            while i > 0 {
                i -= 1;
                if predicate(&frames[i]) {
                    removed.push(frames.remove(i));
                }
            }
            removed
        };
        if !removed.is_empty() {
            for frame in &removed {
                frame.dispose();
            }
            self.frame_generation.set(self.frame_generation.get().wrapping_add(1));
            self.reevaluate_effective_values(owner, None);
        }
    }

    /// The priority the property's effective value currently has.
    pub fn get_priority(&self, property: &FerroProperty) -> BindingPriority {
        match self.get_effective_value(property) {
            Some(v) => v.priority(),
            None if property.inherits() && self.try_get_inherited_value(property).is_some() => {
                BindingPriority::Inherited
            }
            None => BindingPriority::Unset,
        }
    }

    fn insert_frame(&self, owner: &FerroObject, frame: Rc<dyn ValueFrame>) -> usize {
        debug_assert!(!self.frames.borrow().iter().any(|f| frame_ptr_eq(f, &frame)));
        let index = self.binary_search_frame(frame.base().frame_priority());
        frame.base().set_owner(Some(owner.to_weak()));
        self.frames.borrow_mut().insert(index, frame);
        self.frame_generation.set(self.frame_generation.get().wrapping_add(1));
        index
    }

    fn get_or_create_immediate_value_frame(
        &self,
        owner: &FerroObject,
        property: &FerroProperty,
        priority: BindingPriority,
    ) -> Rc<ImmediateValueFrame> {
        debug_assert!(priority != BindingPriority::LocalValue);
        let index = self.binary_search_frame(FramePriority::new(priority, FrameType::Style));
        if index > 0 {
            let frames = self.frames.borrow();
            let candidate = &frames[index - 1];
            if candidate.base().priority() == priority && !candidate.base().contains(property) {
                let any: Rc<dyn std::any::Any> = candidate.clone();
                if let Ok(frame) = any.downcast::<ImmediateValueFrame>() {
                    return frame;
                }
            }
        }
        let result = ImmediateValueFrame::new(priority);
        self.insert_frame(owner, result.clone());
        result
    }

    fn get_or_add_effective_value<T: PropertyValue>(
        &self,
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
    ) -> EffectiveValueRef {
        match self.get_effective_value(property) {
            Some(existing) => existing,
            None => {
                let created: EffectiveValueRef = self.create_effective_value(owner, property);
                self.add_effective_value(owner, property, created.clone());
                created
            }
        }
    }

    fn add_effective_value(&self, owner: &FerroObject, property: &FerroProperty, value: EffectiveValueRef) {
        {
            let mut values = self.effective_values.borrow_mut();
            let index = values.partition_point(|(id, _)| *id < property.id());
            debug_assert!(values.get(index).is_none_or(|(id, _)| *id != property.id()));
            values.insert(index, (property.id(), value));
        }
        if property.inherits() {
            let count = self.inherited_value_count.get();
            self.inherited_value_count.set(count + 1);
            if count == 0 {
                self.on_inheritance_ancestor_changed(owner, Some(&owner.to_ref()));
            }
        }
    }

    fn add_effective_value_and_raise(
        &self,
        owner: &FerroObject,
        property: &'static FerroProperty,
        entry: &Rc<dyn IValueEntry>,
        priority: BindingPriority,
    ) {
        debug_assert!(priority < BindingPriority::Inherited);
        let effective = property.routes().create_effective_value(owner);
        self.add_effective_value(owner, property, effective.clone());
        effective.set_and_raise(owner, entry, priority);
    }

    fn remove_effective_value(&self, owner: &FerroObject, property: &FerroProperty) -> bool {
        let removed = {
            let mut values = self.effective_values.borrow_mut();
            Self::find(&values, property.id()).map(|index| values.remove(index))
        };
        if removed.is_none() {
            return false;
        }
        if property.inherits() {
            let count = self.inherited_value_count.get() - 1;
            self.inherited_value_count.set(count);
            if count == 0 {
                let ancestor = owner.inheritance_parent().and_then(|p| Self::ancestor_for(&p));
                // Detach the self-marker first so the notification is not
                // mistaken for "the owner is the ancestor".
                *self.inheritance_ancestor.borrow_mut() = ancestor.as_ref().map(Ref::downgrade);
                for child in owner.inheritance_children() {
                    child.values().on_inheritance_ancestor_changed(&child, ancestor.as_ref());
                }
            }
        }
        true
    }

    fn inherited_value_changed(
        &self,
        owner: &FerroObject,
        property: &'static FerroProperty,
        old_value: Option<&dyn EffectiveValueDyn>,
        new_value: Option<&dyn EffectiveValueDyn>,
    ) {
        debug_assert!(old_value.is_some() || new_value.is_some());

        // If the value is set locally, propagation ends here.
        if self.is_set(property) {
            return;
        }

        let notifying = property.notifying();
        if let Some(notifying) = notifying {
            notifying(owner, true);
        }

        // Raise PropertyChanged on this object if necessary.
        let raiser = old_value.or(new_value).expect("one of the values is set");
        raiser.raise_inherited_value_changed(owner, old_value, new_value);

        for child in owner.inheritance_children() {
            child.values().inherited_value_changed(&child, property, old_value, new_value);
        }

        if let Some(notifying) = notifying {
            notifying(owner, false);
        }
    }

    fn reevaluate_effective_value(
        &self,
        owner: &FerroObject,
        property: &'static FerroProperty,
        mut current: Option<EffectiveValueRef>,
        changed_value_entry: Option<&Rc<dyn IValueEntry>>,
        ignore_local_value: bool,
    ) {
        self.is_evaluating.set(self.is_evaluating.get() + 1);
        let _guard = EvaluatingGuard(&self.is_evaluating);

        'restart: loop {
            // Don't reevaluate if a styling pass is in effect, reevaluation
            // will be done when it has finished.
            if self.styling.get() > 0 {
                return;
            }

            let generation = self.frame_generation.get();

            // Notify the existing effective value that reevaluation is starting.
            if let Some(current) = &current {
                current.begin_reevaluation(ignore_local_value);
            }

            // Iterate the frames to get the effective value.
            let frames = self.frames();
            for frame in frames.iter().rev() {
                let priority = frame.base().priority();

                // Exit early if the current effective value has higher priority
                // than this frame.
                if let Some(current) = &current {
                    if current.priority() < priority && current.base_priority() < priority {
                        break;
                    }
                }

                // Try to get an entry from the frame for the property.
                let (entry, active_changed) = frame.try_get_entry_if_active(property);

                // If the active state of the frame has changed since the last
                // read, and the frame holds multiple values then we need to
                // re-evaluate the effective values of all properties.
                if active_changed && frame.base().entry_count() > 1 {
                    self.reevaluate_effective_values(owner, changed_value_entry);
                    return;
                }

                // If the frame has an entry for this property with a higher
                // priority than the current effective value (and that entry has
                // a value), then we have a new value for the property. The
                // check for `has_value` must be evaluated last as it can cause
                // bindings to be subscribed.
                if let Some(entry) = entry {
                    if Self::has_higher_priority(&entry, priority, current.as_deref(), changed_value_entry)
                        && entry.has_value()
                    {
                        match &current {
                            Some(current) => current.set_and_raise(owner, &entry, priority),
                            None => {
                                let created = property.routes().create_effective_value(owner);
                                self.add_effective_value(owner, property, created.clone());
                                created.set_and_raise(owner, &entry, priority);
                                current = Some(created);
                            }
                        }
                    }
                }

                if generation != self.frame_generation.get() {
                    continue 'restart;
                }
            }

            if let Some(current) = &current {
                current.end_reevaluation(owner);

                if current.can_remove() {
                    if current.base_priority() == BindingPriority::Unset {
                        self.remove_effective_value(owner, property);
                        current.dispose_and_raise_unset(owner);
                    } else {
                        current.remove_animation_and_raise(owner);
                    }
                }

                current.unsubscribe_if_necessary();
            }
            return;
        }
    }

    fn reevaluate_effective_values(&self, owner: &FerroObject, changed_value_entry: Option<&Rc<dyn IValueEntry>>) {
        self.is_evaluating.set(self.is_evaluating.get() + 1);
        let _guard = EvaluatingGuard(&self.is_evaluating);

        'restart: loop {
            // Don't reevaluate if a styling pass is in effect, reevaluation
            // will be done when it has finished.
            if self.styling.get() > 0 {
                return;
            }

            let generation = self.frame_generation.get();

            // Notify the existing effective values that reevaluation is starting.
            let existing: Vec<EffectiveValueRef> =
                self.effective_values.borrow().iter().map(|(_, v)| v.clone()).collect();
            for value in &existing {
                value.begin_reevaluation(false);
            }

            // Iterate the frames, setting and creating effective values.
            let frames = self.frames();
            for frame in frames.iter().rev() {
                if !frame.is_active() {
                    continue;
                }

                let priority = frame.base().priority();
                let count = frame.base().entry_count();

                for j in 0..count {
                    if j >= frame.base().entry_count() {
                        break;
                    }
                    let entry = frame.base().get_entry(j);
                    let property = entry.property();
                    let effective = self.get_effective_value(property);

                    if !Self::has_higher_priority(&entry, priority, effective.as_deref(), changed_value_entry) {
                        continue;
                    }
                    if !entry.has_value() {
                        continue;
                    }

                    match effective {
                        Some(effective) => effective.set_and_raise(owner, &entry, priority),
                        None => {
                            let created = property.routes().create_effective_value(owner);
                            self.add_effective_value(owner, property, created.clone());
                            created.set_and_raise(owner, &entry, priority);
                        }
                    }

                    if generation != self.frame_generation.get() {
                        continue 'restart;
                    }
                }
            }

            // Remove all effective values that are still unset.
            let values: Vec<EffectiveValueRef> =
                self.effective_values.borrow().iter().map(|(_, v)| v.clone()).collect();
            for value in values.iter().rev() {
                let property = value.property();
                value.end_reevaluation(owner);
                if value.can_remove() {
                    self.remove_effective_value(owner, property);
                    value.dispose_and_raise_unset(owner);
                }
                value.unsubscribe_if_necessary();
            }
            return;
        }
    }

    fn has_higher_priority(
        entry: &Rc<dyn IValueEntry>,
        entry_priority: BindingPriority,
        current: Option<&dyn EffectiveValueDyn>,
        changed_value_entry: Option<&Rc<dyn IValueEntry>>,
    ) -> bool {
        // Set the value if: there is no current effective value; or
        let Some(current) = current else { return true };

        // The value's priority is higher than the current effective value's
        // priority; or
        if entry_priority < current.priority() && entry_priority < current.base_priority() {
            return true;
        }

        // - The value's priority is equal to the current effective value's priority
        // - But the effective value was set via SetCurrentValue
        // - As long as the SetCurrentValue wasn't overriding the value from the
        //   value entry under consideration
        // - Or if it was, the value entry under consideration has changed; or
        if entry_priority == current.priority() && current.is_overriden_current_value() {
            let is_current_entry = current.value_entry().is_some_and(|e| entry_ptr_eq(&e, entry));
            let is_changed_entry = changed_value_entry.is_some_and(|e| entry_ptr_eq(e, entry));
            if !is_current_entry || is_changed_entry {
                return true;
            }
        }

        // The value is a non-animation value and its priority is higher than
        // the current effective value's base priority.
        entry_priority > BindingPriority::Animation && entry_priority < current.base_priority()
    }

    #[inline]
    fn find(values: &[(u32, EffectiveValueRef)], id: u32) -> Option<usize> {
        // Objects typically hold a handful of values: a linear scan over the
        // sorted IDs beats a binary search at these sizes.
        if values.len() <= 8 {
            values.iter().position(|(i, _)| *i == id)
        } else {
            values.binary_search_by_key(&id, |(i, _)| *i).ok()
        }
    }

    #[inline]
    pub fn get_effective_value(&self, property: &FerroProperty) -> Option<EffectiveValueRef> {
        let values = self.effective_values.borrow();
        Self::find(&values, property.id()).map(|index| values[index].1.clone())
    }

    fn set_local_value_binding(&self, property: &FerroProperty, binding: Rc<dyn IDisposable>) {
        // The slot is being taken over: a binding expression that held it is
        // no longer the property's local value binding.
        self.local_value_expressions.borrow_mut().retain(|(id, _)| *id != property.id());
        let mut bindings = self.local_value_bindings.borrow_mut();
        match bindings.iter_mut().find(|(id, _)| *id == property.id()) {
            Some(slot) => slot.1 = binding,
            None => bindings.push((property.id(), binding)),
        }
    }

    fn dispose_existing_local_value_binding(&self, property: &FerroProperty) {
        let existing =
            self.local_value_bindings.borrow().iter().find(|(id, _)| *id == property.id()).map(|(_, b)| b.clone());
        if let Some(existing) = existing {
            existing.dispose();
        }
    }

    fn binary_search_frame(&self, priority: FramePriority) -> usize {
        // Insertion point: after all frames with a priority value greater than
        // or equal to `priority` (the list runs from lowest to highest
        // precedence, i.e. descending priority value).
        self.frames.borrow().partition_point(|f| priority <= f.base().frame_priority())
    }
}

struct EvaluatingGuard<'a>(&'a Cell<u32>);

impl Drop for EvaluatingGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}
