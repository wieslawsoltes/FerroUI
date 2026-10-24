// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::element_factory::{
    ElementFactory, ElementFactoryError, ElementFactoryGetArgs, ElementFactoryRecycleArgs,
    IElementFactory,
};
use super::recycle_pool::RecyclePool;
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::Control;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{BoxedValue, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// Selects a template based on data and, optionally, the requesting container.
/// The second overload is consulted only when the first returns no template.
pub trait IDataTemplateSelector {
    fn select_template(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>>;
    fn select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>>;
    /// Fallible selectors retain their own error when the item or container
    /// overload fails. Returning no template is the only container fallback.
    fn try_select_template(
        &self,
        item: Option<&BoxedValue>,
    ) -> Result<Option<Rc<dyn IDataTemplate>>, ElementFactoryError> {
        Ok(self.select_template(item))
    }
    fn try_select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Result<Option<Rc<dyn IDataTemplate>>, ElementFactoryError> {
        Ok(self.select_template_with_container(item, container))
    }
}
impl PartialEq for dyn IDataTemplateSelector {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Adapts FerroUI data templates and recycling templates to the element-factory
/// contract while retaining template-specific pools across repeaters.
pub struct ItemTemplateWrapper {
    template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    selector: RefCell<Option<Rc<dyn IDataTemplateSelector>>>,
}
impl PartialEq for ItemTemplateWrapper {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ItemTemplateWrapper {
    pub fn new(template: Rc<dyn IDataTemplate>) -> Rc<Self> {
        Rc::new(Self {
            template: RefCell::new(Some(template)),
            selector: RefCell::new(None),
        })
    }
    pub fn with_selector(selector: Rc<dyn IDataTemplateSelector>) -> Rc<Self> {
        Rc::new(Self {
            template: RefCell::new(None),
            selector: RefCell::new(Some(selector)),
        })
    }
    pub fn template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.template.borrow().clone()
    }
    pub fn set_template(&self, template: Option<Rc<dyn IDataTemplate>>) {
        // Releasing the old implementation can run application destructors
        // that observe or replace this wrapper again.
        let previous = self.template.replace(template);
        drop(previous);
        RecyclePool::prune_template_pools();
    }
    pub fn template_selector(&self) -> Option<Rc<dyn IDataTemplateSelector>> {
        self.selector.borrow().clone()
    }
    pub fn set_template_selector(&self, selector: Option<Rc<dyn IDataTemplateSelector>>) {
        let previous = self.selector.replace(selector);
        drop(previous);
        RecyclePool::prune_template_pools();
    }
}
impl Drop for ItemTemplateWrapper {
    fn drop(&mut self) {
        self.template.get_mut().take();
        self.selector.get_mut().take();
        RecyclePool::prune_template_pools();
    }
}
impl IElementFactory for ItemTemplateWrapper {
    fn is_template_empty(&self, data: Option<&BoxedValue>) -> bool {
        self.template().is_some_and(|template| {
            template.match_(data) && template.build(&data.cloned()).is_none()
        })
    }
    fn get_element(
        &self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        let data = args.data();
        let selected = if let Some(template) = self.template() {
            Some(template)
        } else if let Some(selector) = self.template_selector() {
            match selector.try_select_template(data.as_ref())? {
                Some(template) => Some(template),
                None => {
                    // Source retries with a null container and replaces only
                    // its invalid-argument error with the null-template error.
                    match selector.try_select_template_with_container(data.as_ref(), None) {
                        Err(ElementFactoryError::InvalidArgument(_)) => None,
                        result => result?,
                    }
                }
            }
        } else {
            None
        }
        .ok_or(ElementFactoryError::MissingTemplate)?;
        let existing = if let Some(pool) = RecyclePool::get_pool_instance(&selected) {
            pool.try_get_element_with_owner("", args.parent())?
        } else {
            None
        };
        let element = if let Some(recycling) = selected.as_recycling_data_template() {
            recycling.build_with_existing(data.as_ref(), existing.clone())
        } else {
            existing.clone().or_else(|| selected.build(&data))
        }
        .unwrap_or_else(|| {
            let element = Control::new();
            element.set_width(0.0);
            element.set_height(0.0);
            element
        });
        // A native recycling template may elect to replace its previous root.
        // That root must leave the old panel before it is released.
        if let Some(existing) = existing.filter(|existing| existing != &element) {
            if let Some(parent) = existing
                .visual_parent()
                .and_then(|parent| parent.cast::<ferroui_controls::Panel>())
            {
                parent.children().remove(existing);
            }
        }
        RecyclePool::set_origin_template(&element, Some(selected));
        Ok(element)
    }
    fn recycle_element(&self, args: &ElementFactoryRecycleArgs) -> Result<(), ElementFactoryError> {
        let element = args.element();
        let template = self
            .template()
            .or_else(|| RecyclePool::get_origin_template(&element))
            .ok_or(ElementFactoryError::MissingTemplate)?;
        let pool = RecyclePool::get_pool_instance(&template).unwrap_or_else(|| {
            let pool = RecyclePool::new();
            RecyclePool::set_pool_instance(&template, Some(pool.clone()));
            pool
        });
        pool.put_element_with_owner(element, "", args.parent())
    }
}

/// Converts the untyped ItemTemplate property to its native factory contract.
/// `None` is left for the repeater's data-as-element/default-template path.
pub fn factory_from_template(
    item: Option<&BoxedValue>,
) -> Result<Option<Rc<dyn IElementFactory>>, ElementFactoryError> {
    let Some(item) = item else {
        return Ok(None);
    };
    if let Some(factory) = item.downcast_ref::<Rc<dyn IElementFactory>>() {
        return Ok(Some(factory.clone()));
    }
    if let Some(wrapper) = item.downcast_ref::<Rc<ItemTemplateWrapper>>() {
        return Ok(Some(wrapper.clone()));
    }
    if let Some(factory) = ValueTypes::try_cast(item, ValueType::of::<Rc<dyn IElementFactory>>())
        .and_then(|value| value.downcast_ref::<Rc<dyn IElementFactory>>().cloned())
    {
        return Ok(Some(factory));
    }
    if let Some(object) = ValueTypes::as_object(&**item) {
        if let Some(factory) = object.cast::<ElementFactory>() {
            return Ok(Some(factory.as_element_factory()));
        }
    }
    let converted = ValueTypes::try_cast(item, ValueType::of::<Rc<dyn IDataTemplate>>());
    if let Some(template) = item.downcast_ref::<Rc<dyn IDataTemplate>>().or_else(|| {
        converted
            .as_ref()
            .and_then(|value| value.downcast_ref::<Rc<dyn IDataTemplate>>())
    }) {
        return Ok(Some(ItemTemplateWrapper::new(template.clone())));
    }
    if let Some(template) = item.downcast_ref::<Rc<FuncDataTemplate>>() {
        return Ok(Some(ItemTemplateWrapper::new(template.clone())));
    }
    if let Some(selector) = item.downcast_ref::<Rc<dyn IDataTemplateSelector>>() {
        return Ok(Some(ItemTemplateWrapper::with_selector(selector.clone())));
    }
    Err(ElementFactoryError::InvalidTemplate)
}
