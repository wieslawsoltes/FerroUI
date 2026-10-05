//! Not a port of an upstream file: the classes the corpus of the differential
//! harness of the Rust emitter (`crate::emitter`) needs beyond the framework.
//! Generated code names them by the public Rust paths registered here.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::DuplicateSetterError;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, InitializationError, Ref,
    StyledElementImpl, StyledElementImplExt, TypeInfo, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};

use super::TypeModule;

/// A control whose `EndInit` fails, after ending the initialisation, with the
/// error a style with two setters for its `Tag` gives: the failure of a member
/// generated code calls.
#[repr(C)]
pub struct FailingEndInit {
    base: Control,
}

ferro_class!(FailingEndInit: Control);
ferro_impl_classes!(FailingEndInit: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(FailingEndInit {
    new: FailingEndInit::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests" },
});

impl FerroObjectImpl for FailingEndInit {}

impl StyledElementImpl for FailingEndInit {
    fn try_end_init(this: &Self) -> Result<(), InitializationError> {
        Self::parent_try_end_init(this)?;
        Err(InitializationError::DuplicateSetter(DuplicateSetterError {
            property: "Tag".to_string(),
            style: "FailingEndInit".to_string(),
        }))
    }
}

impl FailingEndInit {
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

pub(crate) const MODULE: TypeModule = TypeModule { types: &[FailingEndInit::TYPE], ..TypeModule::EMPTY };

/// The public Rust paths of the classes of this module, for generated code.
pub(crate) const RUST_PATHS: &[(&TypeInfo, &str)] =
    &[(FailingEndInit::TYPE, "ferroui_markup_xaml_tests::support::emitter::FailingEndInit")];
