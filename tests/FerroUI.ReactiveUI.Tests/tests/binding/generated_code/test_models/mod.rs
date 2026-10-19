//! The objects of the tests of the generated code (`ReactiveUI.Binding.GeneratedCode.TestModels`):
//! plain objects that raise `PropertyChanged` (and `PropertyChanging`), as upstream's are, the
//! view models backed by `to_property`, and the scenarios ([`scenarios`]) that observe them, which
//! upstream compiles in the models' project so that the generator sees the calls.

/// Declares an object whose properties notify their changes, as the models of upstream's
/// generated-code tests write them by hand: a `RefCell` per property, a getter, and a setter that
/// does nothing when given the value the property holds, and otherwise raises `PropertyChanging`
/// (when `changing` is `true`), stores the value and raises `PropertyChanged` with the PascalCase
/// name of the property. A model compares by identity (`ReferenceEquals`).
macro_rules! generated_code_model {
    (
        $(#[$meta:meta])*
        pub struct $name:ident (changing: $changing:tt) {
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

                pub fn $setter(&self, value: $ty) {
                    if *self.$field.borrow() == value {
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

mod address;
mod big_view_model;
mod host_test_fixture;
mod observable_property_view_model;
mod summary_view_model;
mod test_view_model;
mod when_any_test_fixture;

pub mod scenarios;

pub use address::Address;
pub use big_view_model::BigViewModel;
pub use host_test_fixture::HostTestFixture;
pub use observable_property_view_model::ObservablePropertyViewModel;
pub use summary_view_model::SummaryViewModel;
pub use test_view_model::TestViewModel;
pub use when_any_test_fixture::WhenAnyTestFixture;

// Not ported: BigView, TestView, TodoView, TodoViewBase, TodoViewModel - the views of the binding
// tests (`Bind*`, `BindCommand`), which belong to another work package.
