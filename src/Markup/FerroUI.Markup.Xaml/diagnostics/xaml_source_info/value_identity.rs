//! Weak source stamps for shared XAML values outside the class model.
use super::XamlSourceInfo;
use crate::{
    templates::{ControlTemplate, DataTemplate, TreeDataTemplate},
    FromXamlObject,
};
use ferroui_base::{AnyValue, BoxedValue};
use ferroui_controls::templates::{IControlTemplate, IDataTemplate};
use std::{
    any::TypeId,
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::{LazyLock, RwLock},
};

enum Owner {
    Boxed(Weak<dyn AnyValue>),
    ControlTemplate(Weak<ControlTemplate>),
    DataTemplate(Weak<dyn IDataTemplate>),
}

impl Owner {
    fn alive(&self) -> bool {
        match self {
            Self::Boxed(object) => object.strong_count() != 0,
            Self::ControlTemplate(template) => template.strong_count() != 0,
            Self::DataTemplate(template) => template.strong_count() != 0,
        }
    }
}

ferroui_base::ferro_thread_local! {
    static SOURCES: RefCell<HashMap<usize, (Owner, XamlSourceInfo)>> =
        RefCell::new(HashMap::new());
}

type UnboxShared = fn(&BoxedValue) -> Option<BoxedValue>;
static SHARED_TYPES: LazyLock<RwLock<HashMap<TypeId, UnboxShared>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub(super) fn register_shared_value<T: PartialEq + 'static>() {
    let mut types = SHARED_TYPES.write().unwrap();
    types.insert(TypeId::of::<Rc<T>>(), |value| {
        value.downcast_ref::<Rc<T>>().map(|shared| shared.clone() as BoxedValue)
    });
    types.insert(TypeId::of::<Option<Rc<T>>>(), |value| {
        let shared = value.downcast_ref::<Option<Rc<T>>>()?.as_ref()?;
        Some(shared.clone() as BoxedValue)
    });
}

fn shared_value(value: &BoxedValue) -> Option<BoxedValue> {
    let unbox = SHARED_TYPES.read().unwrap().get(&value.value_type_id()).copied()?;
    unbox(value)
}

pub(super) fn set(value: &BoxedValue, info: Option<XamlSourceInfo>) {
    let (address, owner) = if let Some(template) = control_template(value) {
        (address_of(&template), Owner::ControlTemplate(Rc::downgrade(&template)))
    } else if let Some(template) = data_template(value) {
        (address_of(&template), Owner::DataTemplate(Rc::downgrade(&template)))
    } else {
        let shared = shared_value(value);
        let value = shared.as_ref().unwrap_or(value);
        (address_of(value), Owner::Boxed(Rc::downgrade(value)))
    };
    SOURCES.with(|sources| {
        let mut sources = sources.borrow_mut();
        sources.retain(|_, (owner, _)| owner.alive());
        if let Some(info) = info {
            sources.insert(address, (owner, info));
        } else {
            sources.remove(&address);
        }
    });
}

pub(super) fn get(value: &BoxedValue) -> Option<XamlSourceInfo> {
    let address = control_template(value).map(|template| address_of(&template))
        .or_else(|| data_template(value).map(|template| address_of(&template)))
        .or_else(|| shared_value(value).map(|shared| address_of(&shared)))
        .unwrap_or_else(|| address_of(value));
    lookup(address)
}

pub(super) fn for_control_template(value: &Rc<dyn IControlTemplate>) -> Option<XamlSourceInfo> {
    let template = ControlTemplate::from_control_template(value)?;
    lookup(address_of(&template))
}

pub(super) fn for_data_template(value: &Rc<dyn IDataTemplate>) -> Option<XamlSourceInfo> {
    lookup(address_of(value))
}

fn lookup(address: usize) -> Option<XamlSourceInfo> {
    SOURCES.with(|sources| {
        let sources = sources.borrow();
        let (owner, info) = sources.get(&address)?;
        owner.alive().then(|| info.clone())
    })
}

fn address_of<T: ?Sized>(value: &Rc<T>) -> usize {
    Rc::as_ptr(value) as *const () as usize
}

fn data_template(value: &BoxedValue) -> Option<Rc<dyn IDataTemplate>> {
    if let Some(template) = Rc::<DataTemplate>::from_xaml_object(value) {
        return Some(template);
    }
    if let Some(template) = Rc::<TreeDataTemplate>::from_xaml_object(value) {
        return Some(template);
    }
    value.downcast_ref::<Rc<dyn IDataTemplate>>()
        .or_else(|| value.downcast_ref::<Option<Rc<dyn IDataTemplate>>>().and_then(Option::as_ref))
        .cloned()
}

fn control_template(value: &BoxedValue) -> Option<Rc<ControlTemplate>> {
    Rc::<ControlTemplate>::from_xaml_object(value).or_else(|| {
        value
            .downcast_ref::<Rc<dyn IControlTemplate>>()
            .or_else(|| {
                value
                    .downcast_ref::<Option<Rc<dyn IControlTemplate>>>()
                    .and_then(Option::as_ref)
            })
            .and_then(ControlTemplate::from_control_template)
    })
}
