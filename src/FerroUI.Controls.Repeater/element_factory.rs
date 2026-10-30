// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use ferroui_controls::Control;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref,
    WeakRef,
};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Errors reported by element factories and recycle pools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElementFactoryError {
    AbstractMethod(&'static str),
    InvalidOwner,
    MissingChild,
    MissingTemplates,
    MissingRecyclePool,
    EmptyTemplateKey,
    UnknownTemplateKey(String),
    MissingTemplate,
    InvalidTemplate,
    InvalidArgument(String),
    TemplateSelection(String),
}

impl fmt::Display for ElementFactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AbstractMethod(name) => {
                write!(f, "A derived element factory must override {name}.")
            }
            Self::InvalidOwner => f.write_str("The recycle-pool owner must be a Panel or null."),
            Self::MissingChild => f.write_str(
                "ItemsRepeater's child was not found in its owner's Children collection.",
            ),
            Self::MissingTemplates => {
                f.write_str("The Templates property cannot be null or empty.")
            }
            Self::MissingRecyclePool => {
                f.write_str("The RecyclingElementFactory requires a RecyclePool.")
            }
            Self::EmptyTemplateKey => f.write_str(
                "Provide a nonempty template identifier in the SelectTemplateKey event.",
            ),
            Self::UnknownTemplateKey(key) => write!(
                f,
                "No template with key '{key}' was found in the Templates collection."
            ),
            Self::MissingTemplate => {
                f.write_str("Null encountered as data template. That is not a valid value for a data template, and can not be used.")
            }
            Self::InvalidTemplate => f.write_str(
                "ItemTemplate must be a data template, template selector, or element factory.",
            ),
            Self::InvalidArgument(message) | Self::TemplateSelection(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ElementFactoryError {}

/// Arguments passed when the repeater asks a factory to create an element.
#[derive(Clone)]
pub struct ElementFactoryGetArgs(Rc<GetArgsData>);

struct GetArgsData {
    data: RefCell<Option<BoxedValue>>,
    parent: RefCell<Option<WeakRef<Control>>>,
}

impl ElementFactoryGetArgs {
    pub fn new(data: Option<BoxedValue>, parent: Option<Ref<Control>>) -> Self {
        Self(Rc::new(GetArgsData {
            data: RefCell::new(data),
            parent: RefCell::new(parent.map(|p| p.downgrade())),
        }))
    }
    pub fn data(&self) -> Option<BoxedValue> {
        self.0.data.borrow().clone()
    }
    pub fn set_data(&self, data: Option<BoxedValue>) {
        *self.0.data.borrow_mut() = data;
    }
    pub fn parent(&self) -> Option<Ref<Control>> {
        self.0.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn set_parent(&self, parent: Option<Ref<Control>>) {
        *self.0.parent.borrow_mut() = parent.map(|p| p.downgrade());
    }
}

impl Default for ElementFactoryGetArgs {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl PartialEq for ElementFactoryGetArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Arguments passed when an element is returned to its factory.
#[derive(Clone)]
pub struct ElementFactoryRecycleArgs(Rc<RecycleArgsData>);

struct RecycleArgsData {
    element: RefCell<Ref<Control>>,
    parent: RefCell<Option<WeakRef<Control>>>,
}

impl ElementFactoryRecycleArgs {
    pub fn new(element: Ref<Control>, parent: Option<Ref<Control>>) -> Self {
        Self(Rc::new(RecycleArgsData {
            element: RefCell::new(element),
            parent: RefCell::new(parent.map(|p| p.downgrade())),
        }))
    }
    pub fn element(&self) -> Ref<Control> {
        self.0.element.borrow().clone()
    }
    pub fn set_element(&self, element: Ref<Control>) {
        *self.0.element.borrow_mut() = element;
    }
    pub fn parent(&self) -> Option<Ref<Control>> {
        self.0.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn set_parent(&self, parent: Option<Ref<Control>>) {
        *self.0.parent.borrow_mut() = parent.map(|p| p.downgrade());
    }
}

impl PartialEq for ElementFactoryRecycleArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Supplies and recycles the elements of an [`super::ItemsRepeater`].
pub trait IElementFactory {
    fn get_element(
        &self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError>;
    fn recycle_element(&self, args: &ElementFactoryRecycleArgs) -> Result<(), ElementFactoryError>;
    fn as_object(&self) -> Option<Ref<FerroObject>> {
        None
    }
    /// Direct data templates can declare that they produce no visual content.
    /// Factories and selectors determine their content per element instead.
    fn is_template_empty(&self, _data: Option<&BoxedValue>) -> bool {
        false
    }
}

impl PartialEq for dyn IElementFactory {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(left), Some(right)) => left == right,
            _ => std::ptr::addr_eq(self as *const Self, other as *const Self),
        }
    }
}

/// Extensible factory base. Its base virtual methods report the same invalid
/// abstract-method calls as the original; derived factories implement them.
#[repr(C)]
pub struct ElementFactory {
    base: FerroObject,
}

ferro_class! {
    ElementFactory: FerroObject, virtuals ElementFactoryImpl: FerroObjectImpl {
        fn get_element_core(this, args: &ElementFactoryGetArgs) -> Result<Ref<Control>, ElementFactoryError>;
        fn recycle_element_core(this, args: &ElementFactoryRecycleArgs) -> Result<(), ElementFactoryError>;
    }
}
ferro_class_info!(ElementFactory {
    new: ElementFactory::new,
    interfaces: [Rc<dyn IElementFactory> => element_factory_handle],
    markup: {
        methods: [
            try fn GetElement(ElementFactoryGetArgs) -> Ref<Control> =>
                |factory: &Ref<ElementFactory>, args: ElementFactoryGetArgs| factory.get_element(&args),
            try fn RecycleElement(ElementFactoryRecycleArgs) =>
                |factory: &Ref<ElementFactory>, args: ElementFactoryRecycleArgs| factory.recycle_element(&args),
        ],
    },
});
impl FerroObjectImpl for ElementFactory {}
impl ElementFactoryImpl for ElementFactory {
    fn get_element_core(
        _this: &Self,
        _args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        Err(ElementFactoryError::AbstractMethod("GetElementCore"))
    }
    fn recycle_element_core(
        _this: &Self,
        _args: &ElementFactoryRecycleArgs,
    ) -> Result<(), ElementFactoryError> {
        Err(ElementFactoryError::AbstractMethod("RecycleElementCore"))
    }
}
impl ElementFactory {
    fn static_constructor() {
        ValueTypes::register_interface_object::<Rc<dyn IElementFactory>>(|value| value.as_object());
    }
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn get_element(
        &self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        self.get_element_core(args)
    }
    pub fn recycle_element(
        &self,
        args: &ElementFactoryRecycleArgs,
    ) -> Result<(), ElementFactoryError> {
        self.recycle_element_core(args)
    }
    pub fn as_element_factory(&self) -> Rc<dyn IElementFactory> {
        Rc::new(ElementFactoryAdapter(self.to_ref()))
    }
}

struct ElementFactoryAdapter(Ref<ElementFactory>);
impl IElementFactory for ElementFactoryAdapter {
    fn get_element(
        &self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        self.0.get_element(args)
    }
    fn recycle_element(&self, args: &ElementFactoryRecycleArgs) -> Result<(), ElementFactoryError> {
        self.0.recycle_element(args)
    }
    fn as_object(&self) -> Option<Ref<FerroObject>> {
        Some(self.0.clone().upcast())
    }
}
fn element_factory_handle(value: Ref<ElementFactory>) -> Rc<dyn IElementFactory> {
    value.as_element_factory()
}
