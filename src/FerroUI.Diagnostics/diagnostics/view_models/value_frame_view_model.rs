//! Port of `Diagnostics/ViewModels/ValueFrameViewModel.cs`.

use super::binding_setter_view_model::{binding_kind, binding_of};
use super::{BindingSetterViewModel, ResourceSetterViewModel, SetterViewModel, SetterViewModelRef, ViewModelBase};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::diagnostics::{IValueFrameDiagnostic, IValueFrameDiagnosticFrameType};
use ferroui_base::input::platform::IClipboard;
use ferroui_base::styling::{ControlTheme, Style, StyleBase};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, StyledElement, TypeInfo};
use ferroui_controls::ItemsSource;
use ferroui_markup_xaml::markup_extensions::{DynamicResourceExtension, StaticResourceExtension};
use std::cell::RefCell;
use std::rc::Rc;

/// A frame of the values of a control (its local values, a style or control theme that
/// applies to it, the values its template gives it), with the setters of the frame.
pub struct ValueFrameViewModel {
    base: ViewModelBase,
    value_frame: Rc<dyn IValueFrameDiagnostic>,
    is_active: RefCell<bool>,
    is_visible: RefCell<bool>,
    description: Option<String>,
    setters: Vec<SetterViewModelRef>,
}

impl PartialEq for ValueFrameViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ValueFrameViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ValueFrameViewModel {
    pub fn new(
        styled_element: &StyledElement,
        value_frame: Rc<dyn IValueFrameDiagnostic>,
        clipboard: Option<Rc<dyn IClipboard>>,
    ) -> Rc<Self> {
        let source = Self::source_to_string(value_frame.source());
        let description = match (value_frame.type_(), source) {
            // `"Local Values " + source`: a null source concatenates as nothing.
            (IValueFrameDiagnosticFrameType::Local, source) => format!("Local Values {}", source.unwrap_or_default()),
            (IValueFrameDiagnosticFrameType::Template, source) => format!("Template {}", source.unwrap_or_default()),
            (IValueFrameDiagnosticFrameType::Theme, source) => format!("Theme {}", source.unwrap_or_default()),
            (_, Some(source)) if !source.is_empty() => source,
            // `_valueFrame.Priority.ToString()`: the name of the priority.
            _ => format!("{:?}", value_frame.priority()),
        };

        let mut setters = Vec::new();

        for entry in value_frame.values() {
            let setter_property = entry.property;
            // The value of a property of type `object` is held as `Option<BoxedValue>`: its
            // contents, a null for an empty one, as the managed original sees the object.
            let setter_value = entry.value.and_then(|value| match value.downcast_ref::<Option<BoxedValue>>() {
                Some(untyped) => untyped.clone(),
                None => Some(value),
            });

            let resource_info = Self::get_resource_info(setter_value.as_ref());

            let setter_vm = if let Some((resource_key, is_dynamic)) = resource_info {
                let resource_value = styled_element.find_resource(&resource_key_of(&resource_key));

                SetterViewModelRef::Resource(ResourceSetterViewModel::new(
                    setter_property,
                    resource_key,
                    resource_value,
                    is_dynamic,
                    clipboard.clone(),
                ))
            } else {
                let is_binding = Self::is_binding(setter_value.as_ref());

                if is_binding {
                    SetterViewModelRef::Binding(BindingSetterViewModel::new(
                        setter_property,
                        setter_value,
                        clipboard.clone(),
                    ))
                } else {
                    SetterViewModelRef::Setter(SetterViewModel::new(setter_property, setter_value, clipboard.clone()))
                }
            };
            setters.push(setter_vm);
        }

        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            value_frame,
            is_active: RefCell::new(false),
            is_visible: RefCell::new(false),
            description: Some(description),
            setters,
        });

        this.set_is_visible(true);

        this.update();

        this
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

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn setters(&self) -> &[SetterViewModelRef] {
        &self.setters
    }

    /// The setters as a source of items, each as a value of its class (for the data templates
    /// of the view, which choose by the class of the setter).
    pub fn setters_source(&self) -> ItemsSource {
        ItemsSource::from_items(self.setters.iter().map(|setter| Some(setter.boxed())))
    }

    pub fn update(&self) {
        self.set_is_active(self.value_frame.is_active());
    }

    /// The key of the resource a setter value refers to and whether the resource is dynamic:
    /// `None` for a value that is neither a static nor a dynamic resource with a key.
    fn get_resource_info(value: Option<&BoxedValue>) -> Option<(BoxedValue, bool)> {
        let value = value?;

        if let Some(static_resource) = value.downcast_ref::<Rc<StaticResourceExtension>>() {
            if let Some(resource_key) = static_resource.resource_key() {
                return Some((resource_key, false));
            }
        }

        // A dynamic resource is a binding of the setter: a setter of a style holds it as its
        // binding description.
        if let Some(dynamic_resource) = value.downcast_ref::<Rc<DynamicResourceExtension>>() {
            if let Some(resource_key) = dynamic_resource.resource_key() {
                return Some((resource_key, true));
            }
        }
        if let Some(binding) = binding_of(value) {
            if let Some(dynamic_resource) = binding.as_any().and_then(|any| any.downcast_ref::<DynamicResourceExtension>()) {
                if let Some(resource_key) = dynamic_resource.resource_key() {
                    return Some((resource_key, true));
                }
            }
        }

        None
    }

    /// Whether a setter value is a binding of one of the kinds the view shows as a binding.
    fn is_binding(value: Option<&BoxedValue>) -> bool {
        value.is_some_and(|value| binding_kind(value).is_some())
    }

    fn source_to_string(source: Option<BoxedValue>) -> Option<String> {
        let source = source?;
        let object = ValueTypes::as_object(&*source)?;

        if let Some(style) = object.cast::<Style>() {
            let mut current_style: Option<Ref<StyleBase>> = Some(style.upcast());
            let mut selectors: Vec<String> = Vec::new();

            while let Some(current) = current_style {
                if let Some(selector) = current.cast::<Style>().and_then(|style| style.selector()) {
                    selectors.push(selector.to_string());
                }
                if let Some(theme) = current.cast::<ControlTheme>() {
                    selectors.push(format!("Theme {}", theme.target_type().map(TypeInfo::name).unwrap_or_default()));
                }

                current_style = current.parent();
            }

            // `string.Concat(selectors)` enumerates the stack from the last pushed selector: the
            // outermost style first.
            selectors.reverse();

            Some(selectors.concat().replace('^', ""))
        } else if let Some(control_theme) = object.cast::<ControlTheme>() {
            control_theme.target_type().map(|target_type| target_type.name().to_string())
        } else if let Some(styled_element) = object.cast::<StyledElement>() {
            Some(styled_element.style_key().name().to_string())
        } else {
            None
        }
    }
}

/// The key of a resource as the resource lookup takes it: text, a class, or a key itself.
///
/// Deviation (DEVIATIONS.md, Developer tools): the markup crate's conversion of an untyped key
/// (`object_casts::resource_key_of`) is internal to it; this repeats its cases for text, classes
/// and keys, and looks any other key up by its text.
fn resource_key_of(value: &BoxedValue) -> ResourceKey {
    if let Some(key) = value.downcast_ref::<ResourceKey>() {
        return key.clone();
    }
    if let Some(text) = value.downcast_ref::<String>() {
        return ResourceKey::from(text.as_str());
    }
    if let Some(text) = value.downcast_ref::<&'static str>() {
        return ResourceKey::from(*text);
    }
    if let Some(type_) = value.downcast_ref::<&'static TypeInfo>() {
        return ResourceKey::Type(type_);
    }
    if let Some(type_) = value.downcast_ref::<ValueType>() {
        if let Some((class, _)) = TypeInfo::find_by_handle(type_.id()) {
            return ResourceKey::Type(class);
        }
    }
    ResourceKey::from(ValueTypes::to_display_string(Some(value)))
}

ferro_markup_type!(class ValueFrameViewModel {
    this: Rc<ValueFrameViewModel>,
    handles: [ValueFrameViewModel, Rc<ValueFrameViewModel>, Option<Rc<ValueFrameViewModel>>],
    properties: [
        IsActive: bool { get: ValueFrameViewModel::is_active, set: ValueFrameViewModel::set_is_active },
        IsVisible: bool { get: ValueFrameViewModel::is_visible, set: ValueFrameViewModel::set_is_visible },
        Description: Option<String> { get: ValueFrameViewModel::description },
        Setters: ItemsSource { get: ValueFrameViewModel::setters_source },
    ],
    methods: [
        fn Update() => ValueFrameViewModel::update,
    ],
    notify_property_changed: ValueFrameViewModel,
});
