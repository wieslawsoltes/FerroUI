//! `ResolvesUsingBaseClassBeforeInterfaceFallback`: the legacy fallback takes the view of the base
//! class of a view model before the view of its trait.
//!
//! A base class is a class of the object model; the view model is a `Ref` of it.

use crate::prelude::*;

pub mod view_models {
    use crate::prelude::*;

    #[repr(C)]
    pub struct BaseViewModel {
        base: FerroObject,
    }

    ferro_class!(BaseViewModel: FerroObject);
    ferro_impl_classes!(BaseViewModel: FerroObjectImpl);

    impl BaseViewModel {
        pub fn construct() -> Self {
            Self { base: FerroObject::construct() }
        }
    }

    pub trait IAlternateViewModel {}

    #[repr(C)]
    pub struct ConcreteViewModel {
        base: BaseViewModel,
    }

    ferro_class!(ConcreteViewModel: BaseViewModel);
    ferro_impl_classes!(ConcreteViewModel: FerroObjectImpl);

    impl ConcreteViewModel {
        pub fn new() -> Ref<Self> {
            instantiate(Self { base: BaseViewModel::construct() })
        }
    }

    impl IAlternateViewModel for ConcreteViewModel {}
}

pub mod views {
    use crate::prelude::*;

    test_view!(BaseView);
    test_view!(AlternateView);
}

#[static_view_locator]
#[derive(Default)]
pub struct InheritanceViewLocator;

#[cfg(test)]
mod tests {
    use super::view_models::ConcreteViewModel;
    use super::InheritanceViewLocator;
    use crate::support::{class_name, start};
    use ferroui_base::{BoxedValue, FerroObject, Ref};
    use std::rc::Rc;

    #[test]
    fn resolves_using_base_class_before_interface_fallback() {
        let _app = start();
        let view_model: BoxedValue = Rc::new(ConcreteViewModel::new().upcast::<FerroObject>());
        let _: &Ref<FerroObject> = view_model.downcast_ref().expect("an object");

        let control = InheritanceViewLocator.build(Some(&view_model)).expect("a view");
        assert_eq!(class_name(&control), "BaseView");
    }
}
