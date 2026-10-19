//! `AppBuilderExtensionsRegistrationTests` (`AppBuilderExtensionsRegistrationTests.cs`): the
//! registration of views by the extensions of the application builder and the view registrar.
//!
//! The reference scans its test assembly; here the views of the tests are declared as one
//! [`ViewAssembly`] ([`test_assembly`]), in the order of the classes of the reference's file.

use super::view_support::{same_builder, start};
use crate::{view_control, AppBuilderExtensions, ControlViewFor, IControlViewFor, ViewAssembly, ViewRegistrar};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, ObjectType, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{AppBuilder, Application, ContentControlImpl, ControlImpl, UserControl};
use ferroui_reactiveui::{reactive_object, IActivatableView, IViewForOf, ReactiveObject};
use ferroui_splat::{AppLocator, MutableDependencyResolverExt, ReadonlyDependencyResolverExt};
use std::cell::RefCell;
use std::rc::Rc;

/// A view model of the tests (`TestVm : ReactiveObject, ITestVm`); never created, the tests
/// resolve views of its type.
#[allow(dead_code)]
pub struct TestVm {
    reactive: ReactiveObject,
}
reactive_object!(TestVm);

impl PartialEq for TestVm {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The contract interface of the test view model (`ITestVm`), a type of the assembly that is not a
/// view.
#[allow(dead_code)]
trait ITestVm {
    /// A marker value for the test contract (`TestContract`).
    fn test_contract(&self) -> Option<()>;
}

impl ITestVm for TestVm {
    fn test_contract(&self) -> Option<()> {
        None
    }
}

/// A view model for a view whose contract attribute has no contract (`NoContractVm`).
#[allow(dead_code)]
pub struct NoContractVm {
    reactive: ReactiveObject,
}
reactive_object!(NoContractVm);

impl PartialEq for NoContractVm {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A distinct view model used to validate the handling of duplicate assemblies
/// (`DistinctRegistrationVm`).
#[allow(dead_code)]
pub struct DistinctRegistrationVm {
    reactive: ReactiveObject,
}
reactive_object!(DistinctRegistrationVm);

impl PartialEq for DistinctRegistrationVm {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A user control that is a view of `$vm` with a plain `ViewModel` property (`$name :
/// UserControl, IViewFor<$vm>`).
macro_rules! test_view {
    ($(#[$meta:meta])* $name:ident, $vm:ty) => {
        $(#[$meta])*
        #[repr(C)]
        pub(crate) struct $name {
            base: UserControl,
            view_model: RefCell<Option<Rc<$vm>>>,
        }

        ferro_class!($name: UserControl);
        ferro_impl_classes!(
            $name: FerroObjectImpl,
            StyledElementImpl,
            VisualImpl,
            LayoutableImpl,
            InteractiveImpl,
            InputElementImpl,
            ControlImpl,
            TemplatedControlImpl,
            ContentControlImpl
        );

        impl $name {
            fn construct() -> Self {
                Self { base: UserControl::construct(), view_model: RefCell::new(None) }
            }

            fn new() -> Ref<Self> {
                instantiate(Self::construct())
            }

            /// A new view of the class, as ReactiveUI sees it (the constructor of the registrar).
            fn view() -> Rc<ControlViewFor<$name>> {
                ControlViewFor::new(Self::new())
            }
        }

        impl IControlViewFor for $name {
            type ViewModel = $vm;

            fn view_model_of(&self) -> Option<Rc<$vm>> {
                self.view_model.borrow().clone()
            }

            fn set_view_model_of(&self, view_model: Option<Rc<$vm>>) {
                *self.view_model.borrow_mut() = view_model;
            }
        }
    };
}

test_view!(
    /// A test view for `TestVm` without a contract (`TestView`).
    TestView, TestVm
);
test_view!(
    /// A test view for `TestVm` with the contract `"C1"` (`ContractedTestView`).
    ContractedTestView, TestVm
);
test_view!(
    /// A view for `TestVm` that the service locator provides (`DiBackedView`).
    DiBackedView, TestVm
);
test_view!(
    /// A view whose contract attribute has no contract (`NoContractView`): the default contract.
    NoContractView, NoContractVm
);
test_view!(
    /// A distinct view used to validate the handling of duplicate assemblies
    /// (`DistinctRegistrationView`).
    DistinctRegistrationView, DistinctRegistrationVm
);

/// A view used to validate the fallback to the constructor (`FallbackView : UserControl`): not a
/// view of ReactiveUI, created by the registrar's `CreateView`.
struct FallbackView(Ref<UserControl>);

fn fallback_view() -> Rc<FallbackView> {
    Rc::new(FallbackView(UserControl::new()))
}

ferroui_base::ferro_thread_local! {
    /// The assembly of the tests, one per thread so that it is the same assembly each time.
    static TEST_ASSEMBLY: Rc<ViewAssembly> = Rc::new(
        ViewAssembly::new("FerroUI.ReactiveUI.Controls.Tests")
            .other_type::<dyn ITestVm>(true)
            .other_type::<TestVm>(false)
            .view::<TestVm, ControlViewFor<TestView>>(None, TestView::view)
            .view::<TestVm, ControlViewFor<ContractedTestView>>(Some("C1"), ContractedTestView::view)
            .view::<TestVm, ControlViewFor<DiBackedView>>(None, DiBackedView::view)
            .other_type::<NoContractVm>(false)
            .view::<NoContractVm, ControlViewFor<NoContractView>>(None, NoContractView::view)
            .other_type::<FallbackView>(false)
            .other_type::<DistinctRegistrationVm>(false)
            .view::<DistinctRegistrationVm, ControlViewFor<DistinctRegistrationView>>(None, DistinctRegistrationView::view),
    );
}

/// The assembly of the tests (`typeof(AppBuilderExtensionsRegistrationTests).Assembly`).
fn test_assembly() -> Rc<ViewAssembly> {
    TEST_ASSEMBLY.with(Rc::clone)
}

/// The control behind a resolved view, as the class `T` (`IsTypeOf<T>()`).
fn control_of<VM: 'static, T: ObjectType>(view: &Rc<dyn IViewForOf<VM>>) -> Option<Ref<T>> {
    let view: &dyn IActivatableView = &**view;
    view_control(view).and_then(|control| control.cast::<T>())
}

/// The views registered for the view model type `VM` (`CountServices`).
fn count_services<VM: 'static>() -> usize {
    AppLocator::get_services::<dyn IViewForOf<VM>>().len()
}

#[test]
fn register_views_internal_registers_view_for_view_model_using_activator() {
    let _app = start();
    let resolver = AppLocator::current_mutable();
    ViewRegistrar::register_views_internal(&*resolver, &[test_assembly()]);

    let resolved = AppLocator::get_service::<dyn IViewForOf<TestVm>>();

    assert!(resolved.is_some());
}

#[test]
fn register_views_internal_honors_view_contract_attribute() {
    let _app = start();
    let resolver = AppLocator::current_mutable();
    ViewRegistrar::register_views_internal(&*resolver, &[test_assembly()]);

    let resolved_default = AppLocator::get_service::<dyn IViewForOf<TestVm>>();
    let resolved_c1 = AppLocator::current().get_service_with_contract::<dyn IViewForOf<TestVm>>(Some("C1"));

    assert!(resolved_default.is_some());
    let resolved_c1 = resolved_c1.expect("a view is registered under C1");
    assert!(control_of::<TestVm, ContractedTestView>(&resolved_c1).is_some());
}

#[test]
fn register_views_internal_view_contract_attribute_without_contract_uses_default_contract() {
    let _app = start();
    let resolver = AppLocator::current_mutable();
    ViewRegistrar::register_views_internal(&*resolver, &[test_assembly()]);

    let resolved = AppLocator::get_service::<dyn IViewForOf<NoContractVm>>().expect("a view is registered");

    assert!(control_of::<NoContractVm, NoContractView>(&resolved).is_some());
}

#[test]
fn register_views_internal_prefers_di_resolution_over_activator() {
    let _app = start();
    let resolver = AppLocator::current_mutable();
    let resolver_backed_view = DiBackedView::view();
    resolver.register_constant::<ControlViewFor<DiBackedView>>(resolver_backed_view.clone());

    ViewRegistrar::register_views_internal(&*resolver, &[test_assembly()]);

    let resolved = AppLocator::get_service::<dyn IViewForOf<TestVm>>().expect("a view is registered");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&resolved), Rc::as_ptr(&resolver_backed_view)));
}

#[test]
fn create_view_when_resolver_throws_falls_back_to_activator() {
    let _app = start();
    AppLocator::current_mutable().register::<FallbackView>(|| panic!("expected"));

    let resolved = ViewRegistrar::create_view::<FallbackView>(fallback_view);

    assert!(resolved.0.is::<UserControl>());
}

// Not ported: CreateView_WhenActivatorReturnsNull_ThrowsInvalidOperationException - reflection
// (`Activator.CreateInstance` gives null for `Nullable<int>`; a constructor always gives a view).
// Not ported: RegisterReactiveUIViews_Throws_On_Null_Builder - argument validation.
// Not ported: RegisterReactiveUIViewsFromAssemblyOf_Returns_Same_Builder - reflection (the
// assembly of a type).
// Not ported: RegisterReactiveUIViewsFromEntryAssembly_Does_Not_Throw - reflection (the entry
// assembly of the process).

#[test]
fn register_reactive_ui_views_from_entry_assembly_with_null_entry_returns_same_builder() {
    let _app = start();
    let builder = AppBuilder::configure::<Application>();

    let result = builder.register_reactive_ui_views_from_entry_assembly(None);

    assert!(same_builder(&result, &builder));
}

#[test]
fn register_reactive_ui_views_from_entry_assembly_with_entry_assembly_registers_views() {
    let _app = start();
    let builder = AppBuilder::configure::<Application>();

    let result = builder.register_reactive_ui_views_from_entry_assembly(Some(test_assembly()));
    let resolver = AppLocator::current_mutable();
    ViewRegistrar::register_views(Some(&*resolver), Some(&[test_assembly()]));

    let resolved = AppLocator::get_service::<dyn IViewForOf<DistinctRegistrationVm>>().expect("a view is registered");

    assert!(same_builder(&result, &builder));
    assert!(control_of::<DistinctRegistrationVm, DistinctRegistrationView>(&resolved).is_some());
}

#[test]
fn register_reactive_ui_views_after_platform_callback_with_empty_assemblies_returns() {
    let _app = start();
    let builder = AppBuilder::configure::<Application>().register_reactive_ui_views(Vec::new());

    let resolver = AppLocator::current_mutable();
    ViewRegistrar::register_views(Some(&*resolver), Some(&[]));

    assert!(same_builder(&builder, &builder));
    assert_eq!(count_services::<TestVm>(), 0);
}

#[test]
fn register_reactive_ui_views_with_unavailable_inputs_returns() {
    let _app = start();
    let resolver = AppLocator::current_mutable();
    let assembly = test_assembly();

    ViewRegistrar::register_views(None, Some(&[assembly]));
    ViewRegistrar::register_views(Some(&*resolver), None);
    ViewRegistrar::register_views(Some(&*resolver), Some(&[]));

    assert!(std::ptr::addr_eq(Rc::as_ptr(&AppLocator::current_mutable()), Rc::as_ptr(&resolver)));
    assert_eq!(count_services::<TestVm>(), 0);
}

#[test]
fn register_reactive_ui_views_after_platform_callback_registers_views() {
    let _app = start();
    let _ = AppBuilder::configure::<Application>().register_reactive_ui_views(vec![test_assembly()]);

    let resolver = AppLocator::current_mutable();
    ViewRegistrar::register_views(Some(&*resolver), Some(&[test_assembly()]));

    let resolved = AppLocator::get_service::<dyn IViewForOf<DistinctRegistrationVm>>().expect("a view is registered");
    assert!(control_of::<DistinctRegistrationVm, DistinctRegistrationView>(&resolved).is_some());
}

#[test]
fn register_views_internal_ignores_duplicate_assemblies() {
    let _app = start();
    let resolver = AppLocator::current_mutable();
    let assembly = test_assembly();
    let before = count_services::<DistinctRegistrationVm>();

    ViewRegistrar::register_views_internal(&*resolver, &[assembly.clone(), assembly]);

    let after = count_services::<DistinctRegistrationVm>();
    let resolved = AppLocator::get_service::<dyn IViewForOf<DistinctRegistrationVm>>().expect("a view is registered");

    assert_eq!(after, before + 1);
    assert!(control_of::<DistinctRegistrationVm, DistinctRegistrationView>(&resolved).is_some());
}
