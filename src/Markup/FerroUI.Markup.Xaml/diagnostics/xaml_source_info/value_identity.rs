//! Weak source stamps for shared XAML values outside the class model.
use super::XamlSourceInfo;
use crate::{templates::ControlTemplate, FromXamlObject};
use ferroui_base::{AnyValue, BoxedValue};
use ferroui_controls::templates::IControlTemplate;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

enum Owner {
    Boxed(Weak<dyn AnyValue>),
    Template(Weak<ControlTemplate>),
}

impl Owner {
    fn alive(&self) -> bool {
        match self {
            Self::Boxed(object) => object.strong_count() != 0,
            Self::Template(template) => template.strong_count() != 0,
        }
    }
}

ferroui_base::ferro_thread_local! {
    static SOURCES: RefCell<HashMap<usize, (Owner, XamlSourceInfo)>> =
        RefCell::new(HashMap::new());
}

pub(super) fn set(value: &BoxedValue, info: Option<XamlSourceInfo>) {
    let (address, owner) = match control_template(value) {
        Some(template) => (
            Rc::as_ptr(&template) as usize,
            Owner::Template(Rc::downgrade(&template)),
        ),
        None => (boxed_address(value), Owner::Boxed(Rc::downgrade(value))),
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
    lookup(control_template(value).map_or_else(
        || boxed_address(value),
        |template| Rc::as_ptr(&template) as usize,
    ))
}

pub(super) fn for_template(value: &Rc<dyn IControlTemplate>) -> Option<XamlSourceInfo> {
    let template = ControlTemplate::from_control_template(value)?;
    lookup(Rc::as_ptr(&template) as usize)
}

fn lookup(address: usize) -> Option<XamlSourceInfo> {
    SOURCES.with(|sources| {
        let sources = sources.borrow();
        let (owner, info) = sources.get(&address)?;
        owner.alive().then(|| info.clone())
    })
}

fn boxed_address(value: &BoxedValue) -> usize {
    Rc::as_ptr(value) as *const () as usize
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
