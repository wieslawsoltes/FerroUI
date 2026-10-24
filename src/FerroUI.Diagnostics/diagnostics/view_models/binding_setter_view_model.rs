//! Port of `Diagnostics/ViewModels/BindingSetterViewModel.cs`.

use super::SetterViewModel;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingBase, CompiledBinding, ReflectionBinding, TemplateBinding};
use ferroui_base::input::platform::IClipboard;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};
use ferroui_markup::data::Binding;
use ferroui_markup_xaml::markup_extensions::CompiledBindingExtension;
use std::ops::Deref;
use std::rc::Rc;

/// A setter whose value is a binding: the path of the binding, with a tint and a tooltip that
/// tell the kind of binding.
pub struct BindingSetterViewModel {
    base: SetterViewModel,
    tint: Rc<dyn IBrush>,
    value_type_tooltip: String,
    path: String,
}

impl PartialEq for BindingSetterViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Deref for BindingSetterViewModel {
    type Target = SetterViewModel;

    fn deref(&self) -> &SetterViewModel {
        &self.base
    }
}

impl INotifyPropertyChanged for BindingSetterViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

/// The binding a setter value holds: the binding description of a setter of a style
/// (`Rc<dyn BindingBase>`), or a binding of one of the classes held as itself.
pub(crate) fn binding_of(value: &BoxedValue) -> Option<Rc<dyn BindingBase>> {
    if let Some(binding) = value.downcast_ref::<Rc<dyn BindingBase>>() {
        return Some(binding.clone());
    }
    if let Some(binding) = value.downcast_ref::<Rc<Binding>>() {
        return Some(binding.clone());
    }
    if let Some(binding) = value.downcast_ref::<Rc<ReflectionBinding>>() {
        return Some(binding.clone());
    }
    if let Some(binding) = value.downcast_ref::<Rc<CompiledBindingExtension>>() {
        return Some(binding.clone());
    }
    if let Some(binding) = value.downcast_ref::<Rc<CompiledBinding>>() {
        return Some(binding.clone());
    }
    if let Some(binding) = value.downcast_ref::<Rc<TemplateBinding>>() {
        return Some(binding.clone());
    }
    None
}

/// The kinds of binding the developer tools tell apart (the cases of the type switches of the
/// managed original).
pub(crate) enum BindingKind {
    /// `Binding`: a binding with a string path, resolved at run time.
    Reflection { path: String },
    /// `CompiledBindingExtension`: a compiled binding.
    Compiled { path: String },
    /// `TemplateBinding`, with the property of the templated parent it binds to.
    Template { property: Option<&'static FerroProperty> },
}

/// The kind of the binding a setter value holds; `None` for a value that is not one of the
/// three kinds of binding.
pub(crate) fn binding_kind(value: &BoxedValue) -> Option<BindingKind> {
    let binding = binding_of(value)?;
    let any = binding.as_any()?;

    if let Some(binding) = any.downcast_ref::<Binding>() {
        return Some(BindingKind::Reflection { path: binding.path() });
    }
    // The markup `Binding` derives from it, and the run-time loader creates it for a binding
    // that is not compiled.
    if let Some(binding) = any.downcast_ref::<ReflectionBinding>() {
        return Some(BindingKind::Reflection { path: binding.path() });
    }
    if let Some(binding) = any.downcast_ref::<CompiledBindingExtension>() {
        return Some(BindingKind::Compiled { path: binding.path().map(|path| path.to_string()).unwrap_or_default() });
    }
    // The binding the compiled markup of a document creates for `{CompiledBinding}`: what the
    // extension provides.
    if let Some(binding) = any.downcast_ref::<CompiledBinding>() {
        return Some(BindingKind::Compiled { path: binding.path().map(|path| path.to_string()).unwrap_or_default() });
    }
    if let Some(binding) = any.downcast_ref::<TemplateBinding>() {
        return Some(BindingKind::Template { property: binding.property() });
    }

    None
}

impl BindingSetterViewModel {
    /// # Panics
    /// When `value` is not a binding of one of the three kinds (`ArgumentException`: "Invalid
    /// binding type").
    pub fn new(
        property: &'static FerroProperty,
        value: Option<BoxedValue>,
        clipboard: Option<Rc<dyn IClipboard>>,
    ) -> Rc<Self> {
        let kind = value.as_ref().and_then(binding_kind);

        let (path, tint, value_type_tooltip): (String, Rc<dyn IBrush>, &str) = match kind {
            Some(BindingKind::Reflection { path }) => (path, Brushes::cornflower_blue(), "Reflection Binding"),
            Some(BindingKind::Compiled { path }) => (path, Brushes::dark_green(), "Compiled Binding"),
            Some(BindingKind::Template { property }) => {
                let path = match property {
                    Some(template_property) => {
                        format!("{}.{}", template_property.owner_type().name(), template_property.name())
                    }
                    None => "Unassigned".to_string(),
                };

                (path, Brushes::orange_red(), "Template Binding")
            }
            None => panic!("Invalid binding type (Parameter 'value')"),
        };

        Rc::new(Self {
            base: SetterViewModel::construct(property, value, clipboard),
            tint,
            value_type_tooltip: value_type_tooltip.to_string(),
            path,
        })
    }

    pub fn tint(&self) -> Rc<dyn IBrush> {
        self.tint.clone()
    }

    pub fn value_type_tooltip(&self) -> String {
        self.value_type_tooltip.clone()
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// The override of `CopyValue`: copies the path of the binding.
    pub fn copy_value(&self) {
        self.base.copy_to_clipboard(&self.path);
    }
}

ferro_markup_type!(class BindingSetterViewModel {
    this: Rc<BindingSetterViewModel>,
    handles: [BindingSetterViewModel, Rc<BindingSetterViewModel>, Option<Rc<BindingSetterViewModel>>],
    base: SetterViewModel,
    properties: [
        Property: &'static FerroProperty { get: |this: &Rc<BindingSetterViewModel>| this.property() },
        Name: String { get: |this: &Rc<BindingSetterViewModel>| this.name() },
        Value: Option<BoxedValue> { get: |this: &Rc<BindingSetterViewModel>| this.value() },
        IsActive: bool {
            get: |this: &Rc<BindingSetterViewModel>| this.is_active(),
            set: |this: &Rc<BindingSetterViewModel>, value: bool| this.set_is_active(value)
        },
        IsVisible: bool {
            get: |this: &Rc<BindingSetterViewModel>| this.is_visible(),
            set: |this: &Rc<BindingSetterViewModel>, value: bool| this.set_is_visible(value)
        },
        Tint: Rc<dyn IBrush> { get: BindingSetterViewModel::tint },
        ValueTypeTooltip: String { get: BindingSetterViewModel::value_type_tooltip },
        Path: String { get: BindingSetterViewModel::path },
    ],
    methods: [
        fn CopyValue() => BindingSetterViewModel::copy_value,
        fn CopyPropertyName() => |this: &Rc<BindingSetterViewModel>| this.copy_property_name(),
    ],
    notify_property_changed: BindingSetterViewModel,
});
