//! Port of `Diagnostics/ViewModels/ResourceSetterViewModel.cs`.

use super::SetterViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::input::platform::IClipboard;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};
use std::ops::Deref;
use std::rc::Rc;

/// A setter whose value is a resource: the key of the resource and the value it has for the
/// control, with a tint and a tooltip that tell a dynamic resource from a static one.
pub struct ResourceSetterViewModel {
    base: SetterViewModel,
    key: BoxedValue,
    tint: Rc<dyn IBrush>,
    value_type_tooltip: String,
}

impl PartialEq for ResourceSetterViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Deref for ResourceSetterViewModel {
    type Target = SetterViewModel;

    fn deref(&self) -> &SetterViewModel {
        &self.base
    }
}

impl INotifyPropertyChanged for ResourceSetterViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ResourceSetterViewModel {
    pub fn new(
        property: &'static FerroProperty,
        resource_key: BoxedValue,
        resource_value: Option<BoxedValue>,
        is_dynamic: bool,
        clipboard: Option<Rc<dyn IClipboard>>,
    ) -> Rc<Self> {
        let tint: Rc<dyn IBrush> = if is_dynamic { Brushes::orange() } else { Brushes::brown() };

        Rc::new(Self {
            base: SetterViewModel::construct(property, resource_value, clipboard),
            key: resource_key,
            tint,
            value_type_tooltip: if is_dynamic { "Dynamic Resource" } else { "Static Resource" }.to_string(),
        })
    }

    pub fn key(&self) -> BoxedValue {
        self.key.clone()
    }

    pub fn tint(&self) -> Rc<dyn IBrush> {
        self.tint.clone()
    }

    pub fn value_type_tooltip(&self) -> String {
        self.value_type_tooltip.clone()
    }

    /// The `CopyValue` of the base class (this class does not override it).
    pub fn copy_value(&self) {
        self.base.copy_value();
    }

    pub fn copy_resource_key(&self) {
        // `Key?.ToString()`: the key is never null.
        let text_to_copy = ValueTypes::to_display_string(Some(&self.key));

        self.base.copy_to_clipboard(&text_to_copy);
    }
}

ferro_markup_type!(class ResourceSetterViewModel {
    this: Rc<ResourceSetterViewModel>,
    handles: [ResourceSetterViewModel, Rc<ResourceSetterViewModel>, Option<Rc<ResourceSetterViewModel>>],
    base: SetterViewModel,
    properties: [
        Property: &'static FerroProperty { get: |this: &Rc<ResourceSetterViewModel>| this.property() },
        Name: String { get: |this: &Rc<ResourceSetterViewModel>| this.name() },
        Value: Option<BoxedValue> { get: |this: &Rc<ResourceSetterViewModel>| this.value() },
        IsActive: bool {
            get: |this: &Rc<ResourceSetterViewModel>| this.is_active(),
            set: |this: &Rc<ResourceSetterViewModel>, value: bool| this.set_is_active(value)
        },
        IsVisible: bool {
            get: |this: &Rc<ResourceSetterViewModel>| this.is_visible(),
            set: |this: &Rc<ResourceSetterViewModel>, value: bool| this.set_is_visible(value)
        },
        Key: Option<BoxedValue> { get: |this: &Rc<ResourceSetterViewModel>| Some(this.key()) },
        Tint: Rc<dyn IBrush> { get: ResourceSetterViewModel::tint },
        ValueTypeTooltip: String { get: ResourceSetterViewModel::value_type_tooltip },
    ],
    methods: [
        fn CopyValue() => ResourceSetterViewModel::copy_value,
        fn CopyPropertyName() => |this: &Rc<ResourceSetterViewModel>| this.copy_property_name(),
        fn CopyResourceKey() => ResourceSetterViewModel::copy_resource_key,
    ],
    notify_property_changed: ResourceSetterViewModel,
});
