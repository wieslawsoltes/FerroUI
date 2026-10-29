//! Original settings test application and Headless builder fixture.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::{AppBuilder, Application, ApplicationImpl, NewApplication};
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
#[repr(C)]
pub(crate) struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_class_info!(App {});
ferro_impl_classes!(App:FerroObjectImpl,ApplicationImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
pub(crate) struct TestAppBuilder;
impl TestAppBuilder {
    pub(crate) fn build_ferro_app() -> AppBuilder {
        AppBuilder::configure::<App>().use_headless(FerroHeadlessPlatformOptions::default())
    }
}
