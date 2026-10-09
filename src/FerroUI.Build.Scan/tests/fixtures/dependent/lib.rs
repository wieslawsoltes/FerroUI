//! The second fixture crate of the tests of the source scanner (`ferroui-build`,
//! `scanner`), built on the first one (`fixture`): what a scan resolves only with the
//! model of a crate it is built on. It is read as files and never compiled.

use ferroui_base::{ferro_class, ferro_properties, StyledProperty};
use fixture::*;
use std::rc::Rc;

pub mod panel;

/// A border of the other crate, named through its glob import.
#[repr(C)]
pub struct Card {
    base: Border,
}

ferro_class!(Card: Border);

ferro_properties! {
    impl Card {
        /// An owner added to a property the other crate declares.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<Card>()
        }

        pub fn dock_property() -> StyledProperty<Dock> {
            FerroProperty::register::<Card, _>("Dock", Dock::Left)
        }

        fn fill_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Self::background_property()
        }
    }
}
