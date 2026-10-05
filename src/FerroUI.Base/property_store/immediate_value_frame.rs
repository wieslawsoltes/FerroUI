use super::{BindingEntry, BindingSource, FrameType, IValueEntry, ValueFrame, ValueFrameBase};
use crate::data::BindingPriority;
use crate::reactive::IDisposable;
use crate::{BoxedValue, FerroProperty, PropertyValue, StyledProperty};
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

    pub(crate) fn add_binding<T: PropertyValue>(
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

    pub(crate) fn add_value<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        value: T,
    ) -> Rc<ImmediateValueEntry<T>> {
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

/// A constant value in an [`ImmediateValueFrame`]. Disposing it removes the
/// value.
pub(crate) struct ImmediateValueEntry<T: PropertyValue> {
    owner: Weak<ImmediateValueFrame>,
    property: &'static StyledProperty<T>,
    value: T,
}

impl<T: PropertyValue> IValueEntry for ImmediateValueEntry<T> {
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
        Some(Rc::new(self.value.clone()))
    }

    fn unsubscribe(&self) {}
}

impl<T: PropertyValue> IDisposable for ImmediateValueEntry<T> {
    fn dispose(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.on_entry_disposed(self.property);
        }
    }
}
