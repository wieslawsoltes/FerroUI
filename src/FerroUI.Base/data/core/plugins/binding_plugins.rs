use super::{
    ExceptionValidationPlugin, FerroPropertyAccessorPlugin, IDataValidationPlugin, IPropertyAccessorPlugin,
    IStreamPlugin, IndeiValidationPlugin, InpcPropertyAccessorPlugin, MethodAccessorPlugin, ObservableStreamPlugin,
    TaskStreamPlugin,
};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static PROPERTY_ACCESSORS: RefCell<Vec<Rc<dyn IPropertyAccessorPlugin>>> = RefCell::new(vec![
        Rc::new(FerroPropertyAccessorPlugin),
        Rc::new(MethodAccessorPlugin),
        Rc::new(InpcPropertyAccessorPlugin),
    ]);
    static DATA_VALIDATORS: RefCell<Vec<Rc<dyn IDataValidationPlugin>>> = RefCell::new(vec![
        Rc::new(IndeiValidationPlugin),
        Rc::new(ExceptionValidationPlugin),
    ]);
    static STREAM_HANDLERS: RefCell<Vec<Rc<dyn IStreamPlugin>>> = RefCell::new(vec![
        Rc::new(TaskStreamPlugin),
        Rc::new(ObservableStreamPlugin),
    ]);
}

/// Holds the binding plugins registered for the current thread.
pub struct BindingPlugins;

impl BindingPlugins {
    /// A snapshot of the property accessor plugins.
    pub fn property_accessors() -> Vec<Rc<dyn IPropertyAccessorPlugin>> {
        PROPERTY_ACCESSORS.with(|p| p.borrow().clone())
    }

    /// A snapshot of the data validation plugins.
    pub fn data_validators() -> Vec<Rc<dyn IDataValidationPlugin>> {
        DATA_VALIDATORS.with(|p| p.borrow().clone())
    }

    /// A snapshot of the stream plugins.
    pub fn stream_handlers() -> Vec<Rc<dyn IStreamPlugin>> {
        STREAM_HANDLERS.with(|p| p.borrow().clone())
    }

    /// Changes the property accessor plugins.
    pub fn update_property_accessors(f: impl FnOnce(&mut Vec<Rc<dyn IPropertyAccessorPlugin>>)) {
        PROPERTY_ACCESSORS.with(|p| f(&mut p.borrow_mut()))
    }

    /// Changes the data validation plugins.
    pub fn update_data_validators(f: impl FnOnce(&mut Vec<Rc<dyn IDataValidationPlugin>>)) {
        DATA_VALIDATORS.with(|p| f(&mut p.borrow_mut()))
    }

    /// Changes the stream plugins.
    pub fn update_stream_handlers(f: impl FnOnce(&mut Vec<Rc<dyn IStreamPlugin>>)) {
        STREAM_HANDLERS.with(|p| f(&mut p.borrow_mut()))
    }
}
