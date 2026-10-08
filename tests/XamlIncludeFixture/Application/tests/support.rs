//! Not a port of an upstream file: the scope the tests run in and the helpers
//! that take the place of `FerroRuntimeXamlLoader.LoadGroup` for compiled
//! documents.

use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{BoxedValue, FerroLocator, FerroObject, ObjectType, Ref};
use ferroui_controls::Control;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::XamlLoadException;

/// The scope of a test of a class that derives from upstream's XAML test base: its
/// own dispatcher, loaded queue and service locator scope. The run-time loader is
/// not registered: every document of a test is compiled, and an include that was
/// not linked at build time would fail to load.
pub struct XamlTestScope {
    locator: Option<Rc<dyn IDisposable>>,
    _dispatcher: UnitTestDispatcherScope,
}

/// Starts the scope of a test (the constructor of the test base class).
pub fn xaml_test_base() -> XamlTestScope {
    crate::register_types();
    let dispatcher = Dispatcher::unit_test_scope();
    Control::reset_loaded_queue_for_unit_tests();
    let locator = FerroLocator::enter_scope();
    XamlTestScope { locator: Some(locator), _dispatcher: dispatcher }
}

impl Drop for XamlTestScope {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            Dispatcher::reset_for_unit_tests();
        }
        if let Some(locator) = self.locator.take() {
            locator.dispose();
        }
        Control::reset_loaded_queue_for_unit_tests();
    }
}

/// A build function of a compiled document.
pub type Build<T> = fn(Option<Rc<dyn IServiceProvider>>) -> Result<Ref<T>, XamlLoadException>;

/// Builds a compiled document as `LoadGroup` builds a document of the group: with a
/// root service provider (`XamlIlRuntimeHelpers.CreateRootServiceProviderV3(null)`).
pub fn try_build<T: ObjectType>(build: Build<T>) -> Result<Ref<T>, XamlLoadException> {
    crate::register_types();
    build(Some(XamlIlRuntimeHelpers::create_root_service_provider_v3(None)))
}

/// [`try_build`] of a document that loads.
#[track_caller]
pub fn build<T: ObjectType>(build: Build<T>) -> Ref<T> {
    try_build(build).unwrap_or_else(|error| panic!("the document failed to load: {}", describe(&error)))
}

/// A load error as text, with the errors it carries.
pub fn describe(error: &XamlLoadException) -> String {
    match error.inner_exception() {
        Some(inner) => format!("{}\n ---> {inner}", error.message()),
        None => error.message().to_string(),
    }
}

/// `Assert.IsType<T>(object)`: the object is of exactly the class `T`.
#[track_caller]
pub fn assert_is_type<T: ObjectType>(object: &FerroObject) {
    assert!(
        std::ptr::eq(object.get_type(), T::TYPE),
        "expected an object of type '{}', found '{}'",
        T::TYPE.name(),
        object.get_type().name()
    );
}

/// `Assert.IsType<T>(value)` for an untyped value. Returns the instance.
#[track_caller]
pub fn assert_value_is_type<T: ObjectType>(value: &Option<BoxedValue>) -> Ref<T> {
    let Some(object) = value.as_ref().and_then(|value| ValueTypes::as_object(&**value)) else {
        panic!(
            "expected an object of type '{}', found '{}'",
            T::TYPE.name(),
            value.as_ref().map_or("null", |value| value.type_name())
        )
    };
    assert_is_type::<T>(&object);
    object.cast::<T>().expect("an object of exactly the class is an instance of it")
}
