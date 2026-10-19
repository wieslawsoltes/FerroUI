//! The objects of the tests of ReactiveUI.Binding (`ReactiveUI.Binding.Tests/TestModels`): plain
//! objects that raise `PropertyChanged` (and `PropertyChanging`), with no dependency on
//! `ReactiveObject`, as upstream's are.

/// Declares an object whose properties notify their changes, as the models of upstream's tests
/// write them by hand: a `RefCell` per property, a getter, and a setter that raises
/// `PropertyChanging` (when `changing` is `true`), stores the value and raises `PropertyChanged`
/// with the PascalCase name of the property. With `check_equality: true`, a setter given the
/// value the property holds does nothing. A model compares by identity (`ReferenceEquals`).
macro_rules! notifying_model {
    (@same true, $current:expr, $value:expr) => {
        $current == $value
    };
    (@same false, $current:expr, $value:expr) => {
        false
    };
    (
        $(#[$meta:meta])*
        pub struct $name:ident (changing: $changing:tt, check_equality: $check:tt) {
            $($field:ident / $setter:ident : $ty:ty = $default:expr),* $(,)?
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            $($field: ::std::cell::RefCell<$ty>,)*
            property_changed: ::ferroui_system::events::Event<str>,
            property_changing: ::ferroui_system::events::Event<str>,
        }

        #[allow(dead_code)]
        impl $name {
            /// The object with the default values of its properties.
            pub fn new() -> ::std::rc::Rc<Self> {
                ::std::rc::Rc::new(Self {
                    $($field: ::std::cell::RefCell::new($default),)*
                    property_changed: ::ferroui_system::events::Event::new(),
                    property_changing: ::ferroui_system::events::Event::new(),
                })
            }

            $(
                pub fn $field(&self) -> $ty {
                    self.$field.borrow().clone()
                }

                #[allow(clippy::partialeq_to_none)]
                pub fn $setter(&self, value: $ty) {
                    if notifying_model!(@same $check, *self.$field.borrow(), value) {
                        return;
                    }
                    let name = ::ferroui_system::component_model::property_name_of(stringify!($field));
                    if $changing {
                        self.property_changing.raise(&name);
                    }
                    *self.$field.borrow_mut() = value;
                    self.property_changed.raise(&name);
                }
            )*

            /// Raises `PropertyChanged` for `name` without changing anything.
            pub fn raise_property_changed(&self, name: &str) {
                self.property_changed.raise(name);
            }

            /// Raises `PropertyChanging` for `name` without changing anything.
            pub fn raise_property_changing(&self, name: &str) {
                self.property_changing.raise(name);
            }

            /// The number of handlers of `PropertyChanged` and of `PropertyChanging`.
            pub fn handler_counts(&self) -> (usize, usize) {
                (self.property_changed.handler_count(), self.property_changing.handler_count())
            }
        }

        impl ::ferroui_system::component_model::INotifyPropertyChanged for $name {
            fn property_changed(&self) -> &::ferroui_system::events::Event<str> {
                &self.property_changed
            }

            fn as_notify_property_changing(&self) -> Option<&dyn ::ferroui_system::component_model::INotifyPropertyChanging> {
                if $changing {
                    Some(self)
                } else {
                    None
                }
            }
        }

        impl ::ferroui_system::component_model::INotifyPropertyChanging for $name {
            fn property_changing(&self) -> &::ferroui_system::events::Event<str> {
                &self.property_changing
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::eq(self, other)
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}@{:p}", stringify!($name), self)
            }
        }
    };
}

mod host_test_fixture;
mod manual_observable;
mod non_observable_test_fixture;
mod non_reactive_i_notify_property_changed_object;
mod obj_chain1;
mod obj_chain2;
mod obj_chain3;
mod observed_value_fixture;
mod owner_class;
mod poco_chain_root;
mod poco_model;
mod recording_observer;
mod test_address;
mod test_fixture;
mod test_view_model;
mod test_when_any_obs_view_model;
mod when_any_test_fixture;
mod wide_arity_fixture;
mod wide_arity_observable_fixture;

pub use host_test_fixture::HostTestFixture;
pub use manual_observable::ManualObservable;
pub use non_observable_test_fixture::NonObservableTestFixture;
pub use non_reactive_i_notify_property_changed_object::NonReactiveINotifyPropertyChangedObject;
pub use obj_chain1::ObjChain1;
pub use obj_chain2::ObjChain2;
pub use obj_chain3::ObjChain3;
pub use observed_value_fixture::ObservedValueFixture;
pub use poco_chain_root::PocoChainRoot;
pub use poco_model::PocoModel;
pub use recording_observer::RecordingObserver;
pub use test_address::TestAddress;
pub use test_fixture::TestFixture;
pub use test_view_model::TestViewModel;
pub use test_when_any_obs_view_model::TestWhenAnyObsViewModel;
pub use when_any_test_fixture::WhenAnyTestFixture;
pub use wide_arity_fixture::WideArityFixture;
pub use wide_arity_observable_fixture::WideArityObservableFixture;
mod amount_view;
mod amount_view_model;
mod dispatch_stub_view;
mod dispatch_stub_view_model;
mod manual_sequencer;
mod recording_synchronization_context;
mod stub_view_thread_invoker;
mod stub_binding_type_converter;
mod stub_fallback_converter;
mod stub_set_method_binding_converter;
pub use amount_view::AmountView;
pub use amount_view_model::AmountViewModel;
pub use dispatch_stub_view::DispatchStubView;
pub use dispatch_stub_view_model::DispatchStubViewModel;
pub use manual_sequencer::ManualSequencer;
pub use recording_synchronization_context::RecordingSynchronizationContext;
pub use stub_view_thread_invoker::StubViewThreadInvoker;
pub use stub_binding_type_converter::StubBindingTypeConverter;
pub use stub_fallback_converter::StubFallbackConverter;
mod click_command_binder;
mod dispatch_stub_control;
pub use click_command_binder::ClickCommandBinder;
pub use dispatch_stub_control::DispatchStubControl;
