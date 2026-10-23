//! Port of `Diagnostics/ViewLocator.cs`: the data template of the DevTools window that builds
//! the view of a view model.

use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::MarkupType;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, TypeInfo};
use ferroui_controls::templates::{IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam};
use ferroui_controls::{Control, TextBlock};
use std::rc::{Rc, Weak};

/// The namespace of the view models of DevTools.
const VIEW_MODELS_NAMESPACE: &str = "FerroUI.Diagnostics.ViewModels";

/// `ViewModelBase` and the classes of the view models that derive from it upstream, directly or
/// through `PropertyViewModel`, `SetterViewModel`, `TreeNode` and `EventTreeNodeBase`.
const VIEW_MODEL_BASE_CLASSES: &[&str] = &[
    "ViewModelBase",
    "BindingSetterViewModel",
    "ClrPropertyViewModel",
    "ControlDetailsViewModel",
    "ControlLayoutViewModel",
    "EventOwnerTreeNode",
    "EventTreeNode",
    "EventTreeNodeBase",
    "EventsPageViewModel",
    "FerroPropertyViewModel",
    "FilterViewModel",
    "FiredEvent",
    "HotKeyPageViewModel",
    "LogicalTreeNode",
    "MainViewModel",
    "PropertyViewModel",
    "PseudoClassViewModel",
    "ResourceSetterViewModel",
    "SetterViewModel",
    "TreeNode",
    "TreePageViewModel",
    "ValueFrameViewModel",
    "VisualTreeNode",
];

/// Builds the view of a view model: the class whose full name is the full name of the class of
/// the view model with `ViewModel` replaced by `View`.
pub struct ViewLocator {
    this: Weak<ViewLocator>,
}

impl PartialEq for ViewLocator {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ViewLocator {
    pub fn new() -> Rc<ViewLocator> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The locator as the data template contract.
    pub fn as_data_template(&self) -> Rc<dyn IDataTemplate> {
        self.this.upgrade().expect("the locator is alive while it is used")
    }

    /// The markup metadata of the type of a value (`data.GetType()`).
    fn markup_type_of(data: &BoxedValue) -> Option<&'static MarkupType> {
        MarkupType::find_by_handle(ValueType::of_value(&**data).id())
    }

    /// `data.GetType().FullName`: the full name of the markup type of the value (the name of
    /// its Rust type when markup does not know it).
    fn full_name_of(data: &BoxedValue) -> String {
        match Self::markup_type_of(data) {
            Some(markup_type) => markup_type.full_name(),
            None => ValueType::of_value(&**data).name().to_string(),
        }
    }

    /// `Type.GetType(name)`: the class of the full name, among the classes markup knows.
    fn find_type(name: &str) -> Option<&'static TypeInfo> {
        let (namespace, name) = name.rsplit_once('.').unwrap_or(("", name));
        TypeInfo::find(namespace, name)
    }

    /// `data is ViewModelBase`: the markup type of the value, or one of its base types (`base:` of
    /// its declaration, walked as reflection walks base classes), is `ViewModelBase` or one of
    /// the view models that derive from it.
    fn is_view_model_base(data: &BoxedValue) -> bool {
        // Deviation (DEVIATIONS.md, Developer tools): the view models of the port embed
        // `ViewModelBase` (a plain struct) and their markup metadata does not state it as their
        // base type, so the derivation is the list of the classes that derive from it upstream.
        let mut current = Self::markup_type_of(data);
        while let Some(markup_type) = current {
            if markup_type.namespace() == VIEW_MODELS_NAMESPACE && VIEW_MODEL_BASE_CLASSES.contains(&markup_type.name) {
                return true;
            }
            current = markup_type.base_type();
        }
        false
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ViewLocator {
    fn build(&self, data: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(data.as_ref(), None)
    }
}

/// Deviation (DEVIATIONS.md, Developer tools): the locator is a recycling data template. The view
/// it built last for a content is reused for the next view model of the same view, which gets it as
/// its data context, where upstream builds a new view for every view model: the details of a
/// control are a new view model at every selection of a node, and building their view again (the
/// property grid with its columns, rows and editors, the layout, the value frames) made the
/// properties appear a moment after the selection. The views follow their data context.
impl IRecyclingDataTemplate for ViewLocator {
    fn build_with_existing(&self, data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        let data = data?;

        let name = Self::full_name_of(data).replace("ViewModel", "View");
        let type_ = Self::find_type(&name);

        if let (Some(existing), Some(type_)) = (existing, type_) {
            if std::ptr::eq(existing.get_type(), type_) {
                return Some(existing);
            }
        }

        if let Some(type_) = type_ {
            // `(Control)Activator.CreateInstance(type)!`.
            let instance = type_
                .create_instance()
                .unwrap_or_else(|| panic!("No parameterless constructor defined for type '{name}'."));
            return Some(
                instance
                    .cast::<Control>()
                    .unwrap_or_else(|| panic!("Unable to cast an object of type '{name}' to type 'Control'.")),
            );
        }

        let text_block = TextBlock::new();
        text_block.set_text(Some(name.as_str()));
        Some(text_block.upcast())
    }
}

impl IDataTemplate for ViewLocator {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        data.is_some_and(Self::is_view_model_base)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}

ferro_markup_type!(class ViewLocator {
    this: Rc<ViewLocator>,
    handles: [ViewLocator, Rc<ViewLocator>, Option<Rc<ViewLocator>>],
    interfaces: [Rc<dyn IDataTemplate>],
    constructors: [() => ViewLocator::new],
});
