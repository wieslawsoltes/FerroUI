//! Port of `Templates/ItemsPanelTemplate.cs`.

use super::TemplateContent;
use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::templates::ITemplateOf;
use ferroui_controls::Panel;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The template of the panel that hosts the items of an items control.
pub struct ItemsPanelTemplate {
    this: Weak<ItemsPanelTemplate>,
    content: RefCell<Option<BoxedValue>>,
}

impl ItemsPanelTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), content: RefCell::new(None) })
    }

    /// The template content: deferred content built on each
    /// [`build`](Self::build).
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// Builds the panel.
    ///
    /// # Panics
    /// Panics if the content builds a control that is not a panel.
    pub fn build(&self) -> Option<Ref<Panel>> {
        crate::throw(self.try_build())
    }

    /// [`build`](Self::build) without the panic: content that is not a
    /// panel is an error.
    pub fn try_build(&self) -> Result<Option<Ref<Panel>>, crate::XamlLoadException> {
        let Some(result) = TemplateContent::try_load_as::<Ref<ferroui_controls::Control>>(self.content().as_ref())? else {
            return Ok(None);
        };
        match result.result().cast::<Panel>() {
            Some(panel) => Ok(Some(panel)),
            None => Err(crate::XamlLoadException::with_message(format!(
                "Unable to cast object of type '{}' to type 'Panel'.",
                result.result().get_type().name()
            ))),
        }
    }

    /// The template as the typed template contract.
    pub fn as_template(&self) -> Rc<dyn ITemplateOf<Option<Ref<Panel>>>> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl ITemplateOf<Option<Ref<Panel>>> for ItemsPanelTemplate {
    fn build_typed(&self) -> Option<Ref<Panel>> {
        ItemsPanelTemplate::build(self)
    }
}

impl ITemplate for ItemsPanelTemplate {
    fn build(&self) -> BoxedValue {
        Rc::new(ItemsPanelTemplate::build(self))
    }
}

crate::identity_eq!(ItemsPanelTemplate);

ferro_markup_type!(class ItemsPanelTemplate {
    this: Rc<ItemsPanelTemplate>,
    handles: [ItemsPanelTemplate, Rc<ItemsPanelTemplate>, Option<Rc<ItemsPanelTemplate>>],
    interfaces: [Rc<dyn ITemplateOf<Option<Ref<Panel>>>>, Rc<dyn ITemplate>],
    constructors: [() => ItemsPanelTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> { get: ItemsPanelTemplate::content, set: ItemsPanelTemplate::set_content }
            [TemplateContent],
    ],
    methods: [
        try fn Build() -> Option<Ref<Panel>> => |this: &Rc<ItemsPanelTemplate>| this.try_build(),
    ],
    attributes: [ControlTemplateScope],
});
