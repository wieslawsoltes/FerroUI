use super::{IDataTemplate, ITemplateWithParam};
use crate::Control;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref};
use std::fmt;
use std::rc::Rc;

/// Why a template selector could not select a template. The hosts that report errors (the
/// repeater) pass it on; the others treat it as no template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateSelectionError {
    /// The item or the container is not one the selector accepts.
    InvalidArgument(String),
    /// The selector failed.
    Failed(String),
}

impl fmt::Display for TemplateSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(message) | Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for TemplateSelectionError {}

/// Selects the data template of an item, and optionally of the container that shows it: WinUI's
/// `DataTemplateSelector` contract. [`DataTemplateSelector`] is the class markup and derived
/// classes use; a Rust type can implement the trait directly.
///
/// The `ItemTemplateSelector`, `ContentTemplateSelector` and `MenuItemTemplateSelector` properties
/// of the controls, and the item template of the repeater, take a selector. It is consulted only
/// when the control has no template of its own.
pub trait IDataTemplateSelector {
    /// `SelectTemplate(item)`: the template of an item.
    fn select_template(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>>;

    /// `SelectTemplate(item, container)`: the template of an item in a container.
    fn select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>>;

    /// [`select_template`](Self::select_template) for a selector that can fail. A host that
    /// reports errors calls this form.
    fn try_select_template(
        &self,
        item: Option<&BoxedValue>,
    ) -> Result<Option<Rc<dyn IDataTemplate>>, TemplateSelectionError> {
        Ok(self.select_template(item))
    }

    /// [`select_template_with_container`](Self::select_template_with_container) for a selector
    /// that can fail.
    fn try_select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Result<Option<Rc<dyn IDataTemplate>>, TemplateSelectionError> {
        Ok(self.select_template_with_container(item, container))
    }

    /// The object behind the selector, for a [`DataTemplateSelector`]: two handles of the same
    /// object are the same selector.
    fn as_object(&self) -> Option<Ref<FerroObject>> {
        None
    }
}

impl PartialEq for dyn IDataTemplateSelector {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(left), Some(right)) => left == right,
            _ => std::ptr::addr_eq(self as *const Self, other as *const Self),
        }
    }
}

/// The template a control chooses for an item it shows in a container: the selector's choice for
/// the item in the container, else for the item alone, so that a selector may implement either
/// form (WinUI's `ItemsControl` and `ContentControl` call the first; a selector written for a
/// virtualizing panel implements the second).
pub fn select_data_template(
    selector: &dyn IDataTemplateSelector,
    item: Option<&BoxedValue>,
    container: Option<&Ref<Control>>,
) -> Option<Rc<dyn IDataTemplate>> {
    selector.select_template_with_container(item, container).or_else(|| selector.select_template(item))
}

/// WinUI's `DataTemplateSelector`: the base of the selectors that markup creates and that derived
/// classes write. `SelectTemplate` calls the matching `SelectTemplateCore`, which a derived class
/// overrides ([`DataTemplateSelectorImpl`]); the base selects no template.
#[repr(C)]
pub struct DataTemplateSelector {
    base: FerroObject,
}

ferro_class! {
    DataTemplateSelector: FerroObject, virtuals DataTemplateSelectorImpl: FerroObjectImpl {
        /// `SelectTemplateCore(item)`: the template of an item. `None` in the base.
        fn select_template_core(this, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>>;
        /// `SelectTemplateCore(item, container)`: the template of an item in a container. `None`
        /// in the base.
        fn select_template_core_with_container(
            this,
            item: Option<&BoxedValue>,
            container: Option<&Ref<Control>>
        ) -> Option<Rc<dyn IDataTemplate>>;
    }
}
ferro_class_info!(DataTemplateSelector {
    new: DataTemplateSelector::new,
    interfaces: [Rc<dyn IDataTemplateSelector> => data_template_selector_handle],
    markup: {
        methods: [
            fn SelectTemplate(Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> =>
                |selector: &Ref<DataTemplateSelector>, item: Option<BoxedValue>| selector.select_template(item.as_ref()),
            fn SelectTemplate(Option<BoxedValue>, Option<Ref<Control>>) -> Option<Rc<dyn IDataTemplate>> =>
                |selector: &Ref<DataTemplateSelector>, item: Option<BoxedValue>, container: Option<Ref<Control>>| {
                    selector.select_template_with_container(item.as_ref(), container.as_ref())
                },
        ],
    },
});

impl FerroObjectImpl for DataTemplateSelector {}

impl DataTemplateSelectorImpl for DataTemplateSelector {
    fn select_template_core(_this: &Self, _item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        None
    }

    fn select_template_core_with_container(
        _this: &Self,
        _item: Option<&BoxedValue>,
        _container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        None
    }
}

impl DataTemplateSelector {
    fn static_constructor() {
        ValueTypes::register_interface_object::<Rc<dyn IDataTemplateSelector>>(|selector| selector.as_object());
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `SelectTemplate(item)`.
    pub fn select_template(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.select_template_core(item)
    }

    /// `SelectTemplate(item, container)`.
    pub fn select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        self.select_template_core_with_container(item, container)
    }

    /// The selector as the contract the controls take.
    pub fn as_data_template_selector(&self) -> Rc<dyn IDataTemplateSelector> {
        Rc::new(DataTemplateSelectorHandle(self.to_ref()))
    }
}

struct DataTemplateSelectorHandle(Ref<DataTemplateSelector>);

impl IDataTemplateSelector for DataTemplateSelectorHandle {
    fn select_template(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.0.select_template(item)
    }

    fn select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        self.0.select_template_with_container(item, container)
    }

    fn as_object(&self) -> Option<Ref<FerroObject>> {
        Some(self.0.clone().upcast())
    }
}

fn data_template_selector_handle(selector: Ref<DataTemplateSelector>) -> Rc<dyn IDataTemplateSelector> {
    selector.as_data_template_selector()
}

/// A data template that builds what a selector chooses for each item: the template for the item,
/// else for the item without a container, as WinUI's items factory asks (a NavigationView's menu
/// items). It matches every item.
pub(crate) fn selector_template(selector: Rc<dyn IDataTemplateSelector>) -> Rc<dyn IDataTemplate> {
    Rc::new(SelectorTemplate(selector))
}

struct SelectorTemplate(Rc<dyn IDataTemplateSelector>);

impl SelectorTemplate {
    fn selected(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.0.select_template(item).or_else(|| self.0.select_template_with_container(item, None))
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for SelectorTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.selected(param.as_ref())?.build(param)
    }
}

impl IDataTemplate for SelectorTemplate {
    fn match_(&self, _data: Option<&BoxedValue>) -> bool {
        true
    }
}
