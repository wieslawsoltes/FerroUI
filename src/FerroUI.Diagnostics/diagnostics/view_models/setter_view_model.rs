//! Port of `Diagnostics/ViewModels/SetterViewModel.cs`.
//!
//! The class is the base of [`BindingSetterViewModel`] and [`ResourceSetterViewModel`], which
//! embed it and dereference to it. A list of setters of any of the three classes (the
//! `List<SetterViewModel>` of a value frame) holds [`SetterViewModelRef`]s: the shared handle
//! of an instance of one of the classes, through which the virtual `CopyValue` dispatches.

use super::{BindingSetterViewModel, ResourceSetterViewModel, ViewModelBase};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::input::platform::{ClipboardExtensions, IClipboard};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// A setter of a value frame: the property, the value, and whether the setter is the one that
/// is in effect (`IsActive`) and whether the filter of the setters shows it (`IsVisible`).
pub struct SetterViewModel {
    base: ViewModelBase,
    is_active: RefCell<bool>,
    is_visible: RefCell<bool>,
    property: &'static FerroProperty,
    name: String,
    value: Option<BoxedValue>,
    clipboard: Option<Rc<dyn IClipboard>>,
}

impl PartialEq for SetterViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for SetterViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl SetterViewModel {
    pub fn new(
        property: &'static FerroProperty,
        value: Option<BoxedValue>,
        clipboard: Option<Rc<dyn IClipboard>>,
    ) -> Rc<Self> {
        Rc::new(Self::construct(property, value, clipboard))
    }

    /// The state of the class, for the classes that derive from it (the constructor body runs
    /// on it before it is shared).
    pub fn construct(
        property: &'static FerroProperty,
        value: Option<BoxedValue>,
        clipboard: Option<Rc<dyn IClipboard>>,
    ) -> Self {
        let this = Self {
            base: ViewModelBase::new(),
            is_active: RefCell::new(false),
            is_visible: RefCell::new(false),
            property,
            name: property.name().to_string(),
            value,
            clipboard,
        };

        this.set_is_active(true);
        this.set_is_visible(true);

        this
    }

    pub fn property(&self) -> &'static FerroProperty {
        self.property
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn value(&self) -> Option<BoxedValue> {
        self.value.clone()
    }

    pub fn is_active(&self) -> bool {
        *self.is_active.borrow()
    }

    pub fn set_is_active(&self, value: bool) {
        self.base.raise_and_set_if_changed(&self.is_active, value, "IsActive");
    }

    pub fn is_visible(&self) -> bool {
        *self.is_visible.borrow()
    }

    pub fn set_is_visible(&self, value: bool) {
        self.base.raise_and_set_if_changed(&self.is_visible, value, "IsVisible");
    }

    /// `CopyValue` of this class (the virtual member dispatches through [`SetterViewModelRef`]):
    /// copies the text of the value.
    pub fn copy_value(&self) {
        // `Value?.ToString()`: nothing to copy for a null value.
        let Some(value) = self.value.as_ref() else {
            return;
        };
        let text_to_copy = ValueTypes::to_display_string(Some(value));

        self.copy_to_clipboard(&text_to_copy);
    }

    pub fn copy_property_name(&self) {
        self.copy_to_clipboard(self.property.name());
    }

    /// `CopyToClipboard`: starts setting the text of the clipboard, without waiting for it (the
    /// managed original discards the task).
    pub fn copy_to_clipboard(&self, value: &str) {
        let Some(clipboard) = self.clipboard.clone() else {
            return;
        };
        let text = value.to_string();
        let _ = Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let _ = clipboard.set_text_async(Some(&text)).await;
        });
    }
}

/// The shared handle of a setter view model of any of the classes `SetterViewModel`,
/// `BindingSetterViewModel` and `ResourceSetterViewModel`: a reference typed with the base
/// class. It dereferences to the base class and dispatches the virtual member.
#[derive(Clone)]
pub enum SetterViewModelRef {
    Setter(Rc<SetterViewModel>),
    Binding(Rc<BindingSetterViewModel>),
    Resource(Rc<ResourceSetterViewModel>),
}

impl PartialEq for SetterViewModelRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq::<SetterViewModel>(&**self, &**other)
    }
}

impl Deref for SetterViewModelRef {
    type Target = SetterViewModel;

    fn deref(&self) -> &SetterViewModel {
        match self {
            Self::Setter(setter) => setter,
            Self::Binding(setter) => setter,
            Self::Resource(setter) => setter,
        }
    }
}

impl SetterViewModelRef {
    /// The virtual `CopyValue`.
    pub fn copy_value(&self) {
        match self {
            Self::Setter(setter) => setter.copy_value(),
            Self::Binding(setter) => setter.copy_value(),
            Self::Resource(setter) => setter.copy_value(),
        }
    }

    /// The setter as an untyped value of its class (what a list of the setters holds for the
    /// data templates that choose by the class).
    pub fn boxed(&self) -> BoxedValue {
        match self {
            Self::Setter(setter) => setter.clone(),
            Self::Binding(setter) => setter.clone(),
            Self::Resource(setter) => setter.clone(),
        }
    }

    /// The setter of an untyped value of one of the classes (`DataContext as SetterViewModel`).
    pub fn from_boxed(value: &BoxedValue) -> Option<Self> {
        if let Some(setter) = value.downcast_ref::<Rc<BindingSetterViewModel>>() {
            return Some(Self::Binding(setter.clone()));
        }
        if let Some(setter) = value.downcast_ref::<Rc<ResourceSetterViewModel>>() {
            return Some(Self::Resource(setter.clone()));
        }
        if let Some(setter) = value.downcast_ref::<Rc<SetterViewModel>>() {
            return Some(Self::Setter(setter.clone()));
        }
        // The box that is the shared object itself.
        let any: Rc<dyn std::any::Any> = value.clone();
        let any = match any.downcast::<BindingSetterViewModel>() {
            Ok(setter) => return Some(Self::Binding(setter)),
            Err(any) => any,
        };
        let any = match any.downcast::<ResourceSetterViewModel>() {
            Ok(setter) => return Some(Self::Resource(setter)),
            Err(any) => any,
        };
        any.downcast::<SetterViewModel>().ok().map(Self::Setter)
    }
}

ferro_markup_type!(class SetterViewModel {
    this: Rc<SetterViewModel>,
    handles: [SetterViewModel, Rc<SetterViewModel>, Option<Rc<SetterViewModel>>],
    properties: [
        Property: &'static FerroProperty { get: SetterViewModel::property },
        Name: String { get: SetterViewModel::name },
        Value: Option<BoxedValue> { get: SetterViewModel::value },
        IsActive: bool { get: SetterViewModel::is_active, set: SetterViewModel::set_is_active },
        IsVisible: bool { get: SetterViewModel::is_visible, set: SetterViewModel::set_is_visible },
    ],
    methods: [
        fn CopyValue() => SetterViewModel::copy_value,
        fn CopyPropertyName() => SetterViewModel::copy_property_name,
    ],
    notify_property_changed: SetterViewModel,
});
