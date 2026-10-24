// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::element_factory::{
    ElementFactory, ElementFactoryError, ElementFactoryGetArgs, ElementFactoryImpl,
    ElementFactoryRecycleArgs, IElementFactory,
};
use super::recycle_pool::RecyclePool;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::Control;
use ferroui_base::collections::FerroDictionary;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, Ref, StyledProperty, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Event arguments used to select a keyed template.
#[derive(Clone)]
pub struct SelectTemplateEventArgs(Rc<SelectTemplateEventData>);
struct SelectTemplateEventData {
    template_key: RefCell<String>,
    data: Option<BoxedValue>,
    owner: Option<WeakRef<Control>>,
}
impl SelectTemplateEventArgs {
    pub fn new(data: Option<BoxedValue>, owner: Option<Ref<Control>>) -> Self {
        Self(Rc::new(SelectTemplateEventData {
            template_key: RefCell::new(String::new()),
            data,
            owner: owner.map(|owner| owner.downgrade()),
        }))
    }
    pub fn template_key(&self) -> String {
        self.0.template_key.borrow().clone()
    }
    pub fn set_template_key(&self, key: impl Into<String>) {
        *self.0.template_key.borrow_mut() = key.into();
    }
    pub fn data_context(&self) -> Option<BoxedValue> {
        self.0.data.clone()
    }
    pub fn owner(&self) -> Option<Ref<Control>> {
        self.0.owner.as_ref().and_then(WeakRef::upgrade)
    }
}
impl PartialEq for SelectTemplateEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

pub type ElementFactoryTemplates = FerroDictionary<String, Rc<dyn IDataTemplate>>;

/// A factory that selects named templates and shares elements through a pool.
#[repr(C)]
pub struct RecyclingElementFactory {
    base: ElementFactory,
    select_template_key: HandlerList<dyn Fn(&SelectTemplateEventArgs)>,
}
ferro_class! {
    RecyclingElementFactory: ElementFactory, virtuals RecyclingElementFactoryImpl: ElementFactoryImpl {
        fn on_select_template_key_core(this, data_context: Option<BoxedValue>, owner: Option<Ref<Control>>) -> Result<String, ElementFactoryError>;
    }
}
ferro_class_info!(RecyclingElementFactory {
    new: RecyclingElementFactory::new,
    interfaces: [Rc<dyn IElementFactory> => recycling_element_factory_handle],
    markup: {
        content: Templates,
        events: [
            SelectTemplateKey(Option<Ref<RecyclingElementFactory>>, SelectTemplateEventArgs) =>
                |factory: &Ref<RecyclingElementFactory>, handler: ferroui_base::metadata::MarkupDelegate| {
                    let sender = factory.downgrade();
                    factory.select_template_key(move |args| {
                        handler.invoke(&[
                            ferroui_base::metadata::into_markup_value(sender.upgrade()),
                            ferroui_base::metadata::into_markup_value(args.clone()),
                        ]);
                    })
                },
        ],
    },
});
impl FerroObjectImpl for RecyclingElementFactory {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_templates(Some(ElementFactoryTemplates::new()));
    }
}

ferro_properties! {
    impl RecyclingElementFactory {
        pub fn recycle_pool_property() -> StyledProperty<Option<Ref<RecyclePool>>> {
            FerroProperty::register::<RecyclingElementFactory, _>("RecyclePool", None)
        }
        pub fn templates_property() -> StyledProperty<Option<ElementFactoryTemplates>> {
            FerroProperty::register::<RecyclingElementFactory, _>("Templates", None)
        }
    }
}

impl RecyclingElementFactory {
    pub fn construct() -> Self {
        Self {
            base: ElementFactory::construct(),
            select_template_key: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn recycle_pool(&self) -> Option<Ref<RecyclePool>> {
        self.get_value(Self::recycle_pool_property())
    }
    pub fn set_recycle_pool(&self, pool: Option<Ref<RecyclePool>>) {
        self.set_value(Self::recycle_pool_property(), pool);
    }
    pub fn templates(&self) -> ElementFactoryTemplates {
        self.get_value(Self::templates_property())
            .expect("Templates property is null.")
    }
    pub fn set_templates(&self, templates: Option<ElementFactoryTemplates>) {
        self.set_value(Self::templates_property(), templates);
    }
    pub fn select_template_key(
        &self,
        handler: impl Fn(&SelectTemplateEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.select_template_key.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.select_template_key.remove(token);
            }
        })
    }
}
impl RecyclingElementFactoryImpl for RecyclingElementFactory {
    fn on_select_template_key_core(
        this: &Self,
        data: Option<BoxedValue>,
        owner: Option<Ref<Control>>,
    ) -> Result<String, ElementFactoryError> {
        // A fresh shared event-args handle permits a selection handler to
        // request another item without corrupting the outer selection.
        let args = SelectTemplateEventArgs::new(data, owner);
        for (_, handler) in this.select_template_key.snapshot().iter() {
            handler(&args);
        }
        let key = args.template_key();
        if key.is_empty() {
            Err(ElementFactoryError::EmptyTemplateKey)
        } else {
            Ok(key)
        }
    }
}
impl ElementFactoryImpl for RecyclingElementFactory {
    fn get_element_core(
        this: &Self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        let templates = this
            .get_value(Self::templates_property())
            .filter(|templates| !templates.is_empty())
            .ok_or(ElementFactoryError::MissingTemplates)?;
        let key = if templates.count() == 1 {
            templates.keys().remove(0)
        } else {
            this.on_select_template_key_core(args.data(), args.parent())?
        };
        if key.is_empty() {
            return Err(ElementFactoryError::EmptyTemplateKey);
        }
        let pool = this
            .recycle_pool()
            .ok_or(ElementFactoryError::MissingRecyclePool)?;
        if let Some(element) = pool.try_get_element_with_owner(&key, args.parent())? {
            return Ok(element);
        }
        let template = templates
            .try_get_value(&key)
            .ok_or_else(|| ElementFactoryError::UnknownTemplateKey(key.clone()))?;
        let element = template
            .build(&args.data())
            .ok_or(ElementFactoryError::MissingTemplate)?;
        RecyclePool::set_reuse_key(&element, key);
        Ok(element)
    }
    fn recycle_element_core(
        this: &Self,
        args: &ElementFactoryRecycleArgs,
    ) -> Result<(), ElementFactoryError> {
        let element = args.element();
        let key = RecyclePool::get_reuse_key(&element);
        this.recycle_pool()
            .ok_or(ElementFactoryError::MissingRecyclePool)?
            .put_element_with_owner(element, &key, args.parent())
    }
}
fn recycling_element_factory_handle(
    value: Ref<RecyclingElementFactory>,
) -> Rc<dyn IElementFactory> {
    value.as_element_factory()
}
