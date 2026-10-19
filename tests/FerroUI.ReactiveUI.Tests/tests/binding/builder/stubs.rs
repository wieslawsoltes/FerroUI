//! The stubs of the builder tests: `StubBindingTypeConverter`, `StubFallbackConverter` and
//! `StubSetMethodBindingConverter` (`TestModels`), the `TestModule` and the `StubCommandBinder` of
//! the tests, kept here so that the tests of the builder do not depend on the models of the
//! converter tests.

use ferroui_reactiveui::*;
use ferroui_splat::{IModule, IMutableDependencyResolver};
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::Rc;

/// The affinity of the stub converters.
const STUB_AFFINITY: i32 = 1;

/// A converter of one pair of types that converts with a closure (`StubBindingTypeConverter`).
pub struct StubBindingTypeConverter {
    from_type: ConverterType,
    to_type: ConverterType,
    convert: Box<dyn Fn(Option<&dyn Any>) -> Option<Rc<dyn Any>>>,
}

impl StubBindingTypeConverter {
    /// The converter from `F` to `T`.
    pub fn new<F: 'static, T: 'static>(convert: impl Fn(Option<&dyn Any>) -> Option<Rc<dyn Any>> + 'static) -> Rc<Self> {
        Rc::new(Self { from_type: ConverterType::of::<F>(), to_type: ConverterType::of::<T>(), convert: Box::new(convert) })
    }
}

impl IBindingTypeConverter for StubBindingTypeConverter {
    fn from_type(&self) -> ConverterType {
        self.from_type
    }

    fn to_type(&self) -> ConverterType {
        self.to_type
    }

    fn get_affinity_for_objects(&self) -> i32 {
        STUB_AFFINITY
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        (self.convert)(from)
    }
}

/// A fallback converter that converts with a closure (`StubFallbackConverter`).
pub struct StubFallbackConverter {
    convert: Box<dyn Fn(&dyn Any) -> Option<Rc<dyn Any>>>,
}

impl StubFallbackConverter {
    /// The converter.
    pub fn new(convert: impl Fn(&dyn Any) -> Option<Rc<dyn Any>> + 'static) -> Rc<Self> {
        Rc::new(Self { convert: Box::new(convert) })
    }
}

impl IBindingFallbackConverter for StubFallbackConverter {
    fn get_affinity_for_objects(&self, _from_type: ConverterType, _to_type: ConverterType) -> i32 {
        STUB_AFFINITY
    }

    fn try_convert(&self, _from_type: ConverterType, from: &dyn Any, _to_type: ConverterType, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        (self.convert)(from)
    }
}

/// A converter of set methods that sets nothing (`StubSetMethodBindingConverter`).
pub struct StubSetMethodBindingConverter;

impl ISetMethodBindingConverter for StubSetMethodBindingConverter {
    fn get_affinity_for_objects(&self, _from_type: Option<ConverterType>, _to_type: Option<ConverterType>) -> i32 {
        STUB_AFFINITY
    }

    fn perform_set(&self, to_target: Option<Rc<dyn Any>>, _new_value: Option<Rc<dyn Any>>, _arguments: Option<&[Option<Rc<dyn Any>>]>) -> Option<Rc<dyn Any>> {
        to_target
    }
}

/// A module that records that it was configured (`TestModule`).
pub struct TestModule {
    configured: Rc<Cell<bool>>,
}

impl TestModule {
    /// The module that sets `configured` when it is configured.
    pub fn new(configured: &Rc<Cell<bool>>) -> Self {
        Self { configured: configured.clone() }
    }
}

impl IModule for TestModule {
    fn configure(&self, _resolver: &dyn IMutableDependencyResolver) {
        self.configured.set(true);
    }
}

/// A command binder with no affinity that binds nothing (`StubCommandBinder`).
pub struct StubCommandBinder;

impl ICreatesCommandBinding for StubCommandBinder {
    fn get_affinity_for_object(&self, _target_type: TypeId, _has_event_target: bool) -> i32 {
        0
    }

    fn bind_command_to_object(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }

    fn bind_command_to_object_event(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _event_name: &str,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }

    fn bind_command_to_object_with_handlers(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _subscription: CommandEventSubscription,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }
}

/// Whether `items` holds `item` (`Contains`, by reference).
pub fn contains<T: ?Sized>(items: &[Rc<T>], item: &Rc<T>) -> bool {
    items.iter().any(|candidate| Rc::ptr_eq(candidate, item))
}
