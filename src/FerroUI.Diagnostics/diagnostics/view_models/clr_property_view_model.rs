//! Port of `Diagnostics/ViewModels/ClrPropertyViewModel.cs`.
//!
//! The managed original reflects over the public properties of the object's type. Their
//! counterpart here is the markup metadata of the type (`ferroui_base::metadata`): the plain
//! properties a type declares ([`MarkupProperty`]), read and written through their untyped
//! accessors.

use super::property_view_model::runtime_type_of;
use super::{PropertyViewModel, PropertyViewModelKey, PropertyViewModelRef};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::metadata::{MarkupProperty, MarkupType, MarkupTypeKind};
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// The view model of a plain property of an object: its value, read through the metadata of the
/// type that declares it.
pub struct ClrPropertyViewModel {
    base: PropertyViewModel,
    target: BoxedValue,
    assigned_type: RefCell<ValueType>,
    value: RefCell<Option<BoxedValue>>,
    property_type: ValueType,
    property: &'static MarkupProperty,
    declaring: &'static MarkupType,
    name: String,
    declaring_type: Option<ValueType>,
}

impl PartialEq for ClrPropertyViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Deref for ClrPropertyViewModel {
    type Target = PropertyViewModel;

    fn deref(&self) -> &PropertyViewModel {
        &self.base
    }
}

impl INotifyPropertyChanged for ClrPropertyViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ClrPropertyViewModel {
    /// The view model of the property `property` of `o`, which the type `declaring_type`
    /// declares (the `PropertyInfo` of the managed original, with its `DeclaringType`).
    pub fn new(o: BoxedValue, property: &'static MarkupProperty, declaring_type: &'static MarkupType) -> Rc<Self> {
        let name = if declaring_type.kind != MarkupTypeKind::Interface {
            property.name.to_string()
        } else {
            format!("{}.{}", declaring_type.name, property.name)
        };
        let property_type = (property.type_)();

        let this = Rc::new(Self {
            base: PropertyViewModel::construct(),
            target: o,
            assigned_type: RefCell::new(property_type),
            value: RefCell::new(None),
            property_type,
            property,
            declaring: declaring_type,
            name,
            declaring_type: declaring_type.handle(),
        });

        this.update();

        this
    }

    /// The handle of the view model typed with the abstract class.
    pub fn to_property_view_model(self: &Rc<Self>) -> PropertyViewModelRef {
        PropertyViewModelRef::Clr(self.clone())
    }

    /// The property (`PropertyInfo Property`).
    pub fn property(&self) -> &'static MarkupProperty {
        self.property
    }

    /// The metadata of the type that declares the property.
    pub fn declaring_markup_type(&self) -> &'static MarkupType {
        self.declaring
    }

    pub fn key(&self) -> PropertyViewModelKey {
        PropertyViewModelKey::Name(self.name.clone())
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn group(&self) -> String {
        if self.is_pinned() {
            "Pinned".to_string()
        } else {
            "CLR Properties".to_string()
        }
    }

    pub fn assigned_type(&self) -> ValueType {
        *self.assigned_type.borrow()
    }

    pub fn property_type(&self) -> ValueType {
        self.property_type
    }

    /// `!Property.CanWrite`.
    pub fn is_readonly(&self) -> bool {
        self.property.set.is_none()
    }

    pub fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().clone()
    }

    /// Sets the property of the object to `value` and reads it back. A value the setter does
    /// not take is ignored (the managed original swallows the exception).
    pub fn set_value(&self, value: Option<BoxedValue>) {
        let Some(set) = self.property.set else {
            return;
        };
        if set(&[Some(self.target.clone()), value]).is_ok() {
            self.update();
        }
    }

    pub fn priority(&self) -> String {
        String::new()
    }

    pub fn is_attached(&self) -> Option<bool> {
        None
    }

    pub fn declaring_type(&self) -> Option<ValueType> {
        self.declaring_type
    }

    pub fn update(&self) {
        let base = self.base.view_model_base();

        // `Property.GetValue(_target)`: the exception of a getter is shown as the value (its
        // message here).
        let value: Option<BoxedValue> = match self.property.get {
            Some(get) => match get(&[Some(self.target.clone())]) {
                Ok(value) => value.and_then(ValueTypes::normalize),
                Err(error) => Some(Rc::new(error.to_string()) as BoxedValue),
            },
            None => None,
        };
        let value_type = runtime_type_of(value.as_ref());

        base.raise_and_set_if_changed(&self.value, value, "Value");
        base.raise_and_set_if_changed(&self.assigned_type, value_type.unwrap_or(self.property_type), "AssignedType");
        base.raise_property_changed("Type");
    }

    /// The override of `OnPropertyChanged`: pinning the property moves it to another group.
    pub(crate) fn on_property_changed(&self, property_name: &str) {
        if property_name == "IsPinned" {
            self.base.raise_property_changed("Group");
        }
    }
}

ferro_markup_type!(class ClrPropertyViewModel {
    this: Rc<ClrPropertyViewModel>,
    handles: [ClrPropertyViewModel, Rc<ClrPropertyViewModel>, Option<Rc<ClrPropertyViewModel>>],
    properties: [
        Name: String { get: ClrPropertyViewModel::name },
        Group: String { get: ClrPropertyViewModel::group },
        Priority: String { get: ClrPropertyViewModel::priority },
        Value: Option<BoxedValue> { get: ClrPropertyViewModel::value, set: ClrPropertyViewModel::set_value },
        AssignedType: ValueType { get: ClrPropertyViewModel::assigned_type },
        PropertyType: ValueType { get: ClrPropertyViewModel::property_type },
        Type: String { get: |this: &Rc<ClrPropertyViewModel>| this.to_property_view_model().type_() },
        IsReadonly: bool { get: ClrPropertyViewModel::is_readonly },
        IsAttached: Option<bool> { get: ClrPropertyViewModel::is_attached },
        IsPinned: bool {
            get: |this: &Rc<ClrPropertyViewModel>| this.is_pinned(),
            set: |this: &Rc<ClrPropertyViewModel>, value: bool| this.to_property_view_model().set_is_pinned(value)
        },
        FullName: String { get: |this: &Rc<ClrPropertyViewModel>| this.to_property_view_model().full_name() },
    ],
    methods: [
        fn Update() => ClrPropertyViewModel::update,
    ],
    notify_property_changed: ClrPropertyViewModel,
});
