//! Scanner-only fixture for the typed descriptors consumed by batch registration.
use ferroui_base::data::core::{NullableRegistration, ValueTypes};
use std::rc::Rc;

pub trait IServiceProvider {}
pub struct Model;

// Named tables carry the same type information as inline const arrays. The
// source scanner describes potential registrations, not runtime reachability.
const NAMED: &[NullableRegistration] = &[NullableRegistration::of::<i64>()];

fn register() {
    use crate::Model as LocalModel;
    ValueTypes::register_nullable_types(NAMED);
    ValueTypes::register_nullable_types(&const { [
        NullableRegistration::of::<Rc<dyn IServiceProvider>>(),
        ferroui_base::data::core::NullableRegistration::of::<Rc<LocalModel>>(),
    ] });
    ValueTypes::register_nullable::<String>();

    macro_rules! descriptors {
        ($($type_:ty),* $(,)?) => {
            ValueTypes::register_nullable_types(&const { [
                $(NullableRegistration::of::<$type_>()),*
            ] });
        };
    }
    descriptors![u8, u16,];

    // Unknown types and unsupported macro rules stay visible in the coverage
    // report, so consumers cannot mistake an incomplete scan for a full model.
    ValueTypes::register_nullable_types(&[NullableRegistration::of::<_>()]);
    macro_rules! guarded {
        ($type_:ty, $guard:block) => {
            if $guard {
                ValueTypes::register_nullable_types(&[NullableRegistration::of::<$type_>()]);
            }
        };
    }
    guarded!(u32, { true });
}

#[cfg(test)]
mod tests {
    fn local_only() {
        super::ValueTypes::register_nullable_types(&[super::NullableRegistration::of::<f64>()]);
    }
}
