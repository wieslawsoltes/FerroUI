use super::{BindingEntry, BindingSource, IValueEntry, ValueFrame};
use crate::reactive::{IDisposable, IObservable};
use crate::{BoxedValue, FerroObject, PropertyValue, StyledProperty};
use std::rc::{Rc, Weak};

/// The shared installer needs only the entry's startup operation. The typed
/// entry, observer and value reads are the same ones used by typed sources.
pub(crate) trait BoxedBindingValue: IValueEntry + IDisposable {
    fn start(&self);
}

impl<T: PropertyValue> BoxedBindingValue for BindingEntry<T> {
    fn start(&self) {
        BindingEntry::start(self);
    }
}

pub(crate) fn create_boxed_binding_entry<T: PropertyValue>(
    target: &FerroObject,
    frame: Weak<dyn ValueFrame>,
    property: &'static StyledProperty<T>,
    source: Rc<dyn IObservable<BoxedValue>>,
) -> Rc<dyn BoxedBindingValue> {
    BindingEntry::new(target, frame, property, BindingSource::untyped(source))
}
