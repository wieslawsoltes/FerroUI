use crate::NativeMenu;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroObject, ObjectType, Ref, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

/// Exports a menu as the native menu of the platform.
pub trait INativeMenuExporter {
    /// Sets the menu to export; `None` removes the exported menu.
    fn set_native_menu(&self, menu: Option<Ref<NativeMenu>>);
}

/// Exports the native menu of a top-level.
pub trait ITopLevelNativeMenuExporter: INativeMenuExporter {
    /// Whether the menu is exported to the platform.
    fn is_native_menu_exported(&self) -> bool;

    /// Raised when [`is_native_menu_exported`](Self::is_native_menu_exported)
    /// changes.
    ///
    /// Disposing the returned value removes the handler.
    fn on_is_native_menu_exported_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

/// Something that can provide a native menu exporter.
pub trait INativeMenuExporterProvider {
    fn native_menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>>;
}

type Adapter = Rc<dyn Fn(&FerroObject) -> Option<Rc<dyn INativeMenuExporterProvider>>>;

thread_local! {
    static PROVIDER_TYPES: RefCell<Vec<(&'static TypeInfo, Adapter)>> = const { RefCell::new(Vec::new()) };
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`INativeMenuExporterProvider`] through the handle created by `adapter`.
///
/// A class cannot implement the trait itself (its methods would shadow the
/// inherent ones of the class): it provides a small adapter handle that
/// does, and makes it known here, normally from its class initialization.
pub fn register_native_menu_exporter_provider<T: ObjectType>(
    adapter: fn(Ref<T>) -> Rc<dyn INativeMenuExporterProvider>,
) {
    PROVIDER_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            let adapter: Adapter =
                Rc::new(move |object| object.downcast_ref::<T>().map(|target| adapter(FerroObject::ref_of(target))));
            types.push((T::TYPE, adapter));
        }
    });
}

/// The object viewed as a provider of a native menu exporter, if its class
/// implements [`INativeMenuExporterProvider`].
pub fn as_native_menu_exporter_provider(object: &FerroObject) -> Option<Rc<dyn INativeMenuExporterProvider>> {
    let adapter = PROVIDER_TYPES.with(|types| {
        let types = types.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, adapter)) = types.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(adapter.clone());
            }
            current = type_.base_type();
        }
        None
    });
    adapter.and_then(|adapter| adapter(object))
}
