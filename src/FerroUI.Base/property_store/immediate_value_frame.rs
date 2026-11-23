use super::{BindingEntry, BindingSource, BoxedBindingEntry, FrameType, IValueEntry, ValueFrame, ValueFrameBase};
use crate::data::BindingPriority;
use crate::reactive::{IDisposable, IObservable};
use super::effective_value::box_stored;
use crate::{BoxedValue, FerroProperty, StyledProperty, StyledValue};
use std::any::Any;
use std::rc::{Rc, Weak};

/// Holds values with a non-local-value priority that are set directly on an
/// object, as opposed to coming from a style.
pub struct ImmediateValueFrame {
    base: ValueFrameBase,
    this: Weak<ImmediateValueFrame>,
}

impl ImmediateValueFrame {
    pub(crate) fn new(priority: BindingPriority) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { base: ValueFrameBase::new(priority, FrameType::Style), this: this.clone() })
    }

    pub(crate) fn add_binding<T: StyledValue>(
        &self,
        property: &'static StyledProperty<T>,
        source: BindingSource<T>,
    ) -> Rc<BindingEntry<T>> {
        let owner = self.base.owner().expect("frame has no owner");
        let frame: Rc<dyn ValueFrame> = self.this.upgrade().expect("frame is alive");
        let entry = BindingEntry::new(&owner, Rc::downgrade(&frame), property, source);
        self.base.add(entry.clone());
        entry
    }

    pub(crate) fn add_boxed_binding(
        &self,
        property: &'static FerroProperty,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<BoxedBindingEntry> {
        let owner = self.base.owner().expect("frame has no owner");
        let frame: Rc<dyn ValueFrame> = self.this.upgrade().expect("frame is alive");
        let entry = BoxedBindingEntry::new(&owner, Rc::downgrade(&frame), property, source);
        self.base.add(entry.clone());
        entry
    }

    /// Adds a value of `property`, as the store keeps it (`S` is the storage type of the value
    /// type of the property).
    pub(crate) fn add_value<S: StyledValue>(
        &self,
        property: &'static FerroProperty,
        value: S,
    ) -> Rc<ImmediateValueEntry<S>> {
        let entry = Rc::new(ImmediateValueEntry { owner: self.this.clone(), property, value });
        self.base.add(entry.clone());
        entry
    }

    /// Adds a binding expression as an entry of the frame.
    pub(crate) fn add_binding_expression(&self, source: Rc<dyn crate::data::BindingExpressionBase>) {
        let entry: Rc<dyn IValueEntry> = source;
        self.base.add(entry);
    }

    pub(crate) fn on_entry_disposed(&self, property: &'static FerroProperty) {
        self.base.remove(property);
        if let Some(owner) = self.base.owner() {
            let frame: Rc<dyn ValueFrame> = self.this.upgrade().expect("frame is alive");
            owner.values().on_value_entry_removed(&owner, &frame, property);
        }
    }
}

impl ValueFrame for ImmediateValueFrame {
    fn base(&self) -> &ValueFrameBase {
        &self.base
    }

    fn get_is_active(&self) -> (bool, bool) {
        (true, false)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A constant value in an [`ImmediateValueFrame`], kept as the store keeps the
/// values of the property (`T` is the storage type of its value type).
/// Disposing it removes the value.
pub(crate) struct ImmediateValueEntry<T: StyledValue> {
    owner: Weak<ImmediateValueFrame>,
    property: &'static FerroProperty,
    value: T,
}

impl<T: StyledValue> IValueEntry for ImmediateValueEntry<T> {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn has_value(&self) -> bool {
        true
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        match out.downcast_mut::<Option<T>>() {
            Some(out) => {
                *out = Some(self.value.clone());
                true
            }
            None => false,
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        Some(box_stored(self.property, &self.value))
    }

    fn unsubscribe(&self) {}
}

impl<T: StyledValue> IDisposable for ImmediateValueEntry<T> {
    fn dispose(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.on_entry_disposed(self.property);
        }
    }
}
