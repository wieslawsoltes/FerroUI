//! Port of `Diagnostics/ViewModels/FerroPropertyViewModel.cs` (the class is named after the
//! framework's property type).

use super::property_view_model::runtime_type_of;
use super::{PropertyViewModel, PropertyViewModelKey, PropertyViewModelRef};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::BindingPriority;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, FerroProperty, Ref};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// The view model of a registered property of an object: its value, the priority the value has
/// and the group the property is shown in.
pub struct FerroPropertyViewModel {
    base: PropertyViewModel,
    target: Ref<FerroObject>,
    assigned_type: RefCell<ValueType>,
    value: RefCell<Option<BoxedValue>>,
    priority: RefCell<String>,
    group: RefCell<String>,
    property_type: ValueType,
    property: &'static FerroProperty,
    name: String,
    declaring_type: Option<ValueType>,
}

impl PartialEq for FerroPropertyViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Deref for FerroPropertyViewModel {
    type Target = PropertyViewModel;

    fn deref(&self) -> &PropertyViewModel {
        &self.base
    }
}

impl INotifyPropertyChanged for FerroPropertyViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl FerroPropertyViewModel {
    pub fn new(o: &FerroObject, property: &'static FerroProperty) -> Rc<Self> {
        let name = if property.is_attached() {
            format!("[{}.{}]", property.owner_type().name(), property.name())
        } else {
            property.name().to_string()
        };
        let owner_type = property.owner_type();
        let declaring_type = owner_type.handle().map(|handle| ValueType::new(handle, owner_type.name()));
        let property_type = ValueType::new(property.property_type(), property.property_type_name());

        let this = Rc::new(Self {
            base: PropertyViewModel::construct(),
            target: o.to_ref(),
            assigned_type: RefCell::new(property_type),
            value: RefCell::new(None),
            priority: RefCell::new(String::new()),
            group: RefCell::new(String::new()),
            property_type,
            property,
            name,
            declaring_type,
        });

        this.update();

        this
    }

    /// The handle of the view model typed with the abstract class.
    pub fn to_property_view_model(self: &Rc<Self>) -> PropertyViewModelRef {
        PropertyViewModelRef::Ferro(self.clone())
    }

    pub fn property(&self) -> &'static FerroProperty {
        self.property
    }

    pub fn key(&self) -> PropertyViewModelKey {
        PropertyViewModelKey::Property(self.property)
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn is_attached(&self) -> Option<bool> {
        Some(self.property.is_attached())
    }

    pub fn priority(&self) -> String {
        self.priority.borrow().clone()
    }

    pub fn assigned_type(&self) -> ValueType {
        *self.assigned_type.borrow()
    }

    pub fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().clone()
    }

    /// Sets the property of the object to `value` (a local value) and reads it back. A value the
    /// property does not take is ignored (the managed original swallows the exception).
    pub fn set_value(&self, value: Option<BoxedValue>) {
        if self.property.is_read_only() {
            return;
        }
        let property_type = self.property_type;
        let Some(Some(converted)) = ValueTypes::try_convert(value.as_ref(), property_type) else {
            return;
        };
        if !self.property.is_valid_value(converted.as_any()) {
            return;
        }

        self.target.set_value_untyped(self.property, converted.as_any(), BindingPriority::LocalValue);
        self.update();
    }

    pub fn group(&self) -> String {
        if self.is_pinned() {
            "Pinned".to_string()
        } else {
            self.group.borrow().clone()
        }
    }

    pub fn declaring_type(&self) -> Option<ValueType> {
        self.declaring_type
    }

    pub fn property_type(&self) -> ValueType {
        self.property_type
    }

    pub fn is_readonly(&self) -> bool {
        self.property.is_read_only()
    }

    pub fn update(&self) {
        let base = self.base.view_model_base();

        if self.property.is_direct() {
            // The getter of a direct property cannot fail here (the managed original reports
            // the exception of a getter as the value).
            let value = ValueTypes::normalize(self.target.get_value_untyped(self.property));
            let value_type = runtime_type_of(value.as_ref());

            base.raise_and_set_if_changed(&self.value, value, "Value");
            base.raise_and_set_if_changed(&self.assigned_type, value_type.unwrap_or(self.property_type), "AssignedType");
            base.raise_and_set_if_changed(&self.priority, "Direct".to_string(), "Priority");

            *self.group.borrow_mut() = "Properties".to_string();
        } else {
            let diag = self.target.get_diagnostic(self.property);

            let value = ValueTypes::normalize(diag.value().clone());
            let value_type = runtime_type_of(value.as_ref());
            let priority: Option<BindingPriority> = Some(diag.priority());

            base.raise_and_set_if_changed(&self.value, value, "Value");
            base.raise_and_set_if_changed(&self.assigned_type, value_type.unwrap_or(self.property_type), "AssignedType");

            if let Some(priority) = priority {
                // `priority.ToString()`: the name of the priority.
                base.raise_and_set_if_changed(&self.priority, format!("{priority:?}"), "Priority");
                let group = if self.is_attached() == Some(true) { "Attached Properties" } else { "Properties" };
                base.raise_and_set_if_changed(&self.group, group.to_string(), "Group");
            } else {
                base.raise_and_set_if_changed(&self.priority, "Unset".to_string(), "Priority");
                base.raise_and_set_if_changed(&self.group, "Unset".to_string(), "Group");
            }
        }
        base.raise_property_changed("Type");
    }

    /// The override of `OnPropertyChanged`: pinning the property moves it to another group.
    pub(crate) fn on_property_changed(&self, property_name: &str) {
        if property_name == "IsPinned" {
            self.base.raise_property_changed("Group");
        }
    }
}

ferro_markup_type!(class FerroPropertyViewModel {
    this: Rc<FerroPropertyViewModel>,
    handles: [FerroPropertyViewModel, Rc<FerroPropertyViewModel>, Option<Rc<FerroPropertyViewModel>>],
    properties: [
        Property: &'static FerroProperty { get: FerroPropertyViewModel::property },
        Name: String { get: FerroPropertyViewModel::name },
        Group: String { get: FerroPropertyViewModel::group },
        Priority: String { get: FerroPropertyViewModel::priority },
        Value: Option<BoxedValue> { get: FerroPropertyViewModel::value, set: FerroPropertyViewModel::set_value },
        AssignedType: ValueType { get: FerroPropertyViewModel::assigned_type },
        PropertyType: ValueType { get: FerroPropertyViewModel::property_type },
        Type: String { get: |this: &Rc<FerroPropertyViewModel>| this.to_property_view_model().type_() },
        IsReadonly: bool { get: FerroPropertyViewModel::is_readonly },
        IsAttached: Option<bool> { get: FerroPropertyViewModel::is_attached },
        IsPinned: bool {
            get: |this: &Rc<FerroPropertyViewModel>| this.is_pinned(),
            set: |this: &Rc<FerroPropertyViewModel>, value: bool| this.to_property_view_model().set_is_pinned(value)
        },
        FullName: String { get: |this: &Rc<FerroPropertyViewModel>| this.to_property_view_model().full_name() },
    ],
    methods: [
        fn Update() => FerroPropertyViewModel::update,
    ],
    notify_property_changed: FerroPropertyViewModel,
});
