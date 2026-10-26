//! The compiled XAML window and its live culture/measurement interaction.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, ControlImpl, TextBlock, TopLevelImpl, Window, WindowBaseImpl,
    WindowImpl,
};
use ferroui_protranslate::TranslationBindingSource;
use ferroui_protranslate_core::{Culture, GlobalizationService, MeasurementUnit};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
    globalization: GlobalizationService,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow {
    new: MainWindow::new
});
xaml_class!(MainWindow, "/MainWindow.xaml");

impl MainWindow {
    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
            globalization: GlobalizationService::new(
                TranslationBindingSource::default_source().service(),
            ),
            subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let source = TranslationBindingSource::default_source();
        for (button, culture) in [
            ("English", "en-US"),
            ("Polish", "pl-PL"),
            ("Arabic", "ar-SA"),
        ] {
            let source = source.clone();
            this.find_control::<Button>(button)
                .expect("language button")
                .click(move |_, _| {
                    source.set_culture(Culture::parse(culture).expect("built-in culture"));
                });
        }
        let weak = this.downgrade();
        this.subscription.replace(Some(source.subscribe(move || {
            if let Some(window) = weak.upgrade() {
                window.refresh_metadata();
            }
        })));
        this.refresh_metadata();
        this
    }

    fn refresh_metadata(&self) {
        let culture = self.globalization.current_culture();
        let region = self
            .globalization
            .region_profile()
            .expect("built-in region");
        let service = self.globalization.translations();
        let region_text = service
            .format(
                "RegionDetails",
                &[
                    culture.name().into(),
                    region.two_letter_iso_region_name.into(),
                ],
            )
            .expect("valid catalog format");
        self.find_control::<TextBlock>("RegionDetails")
            .expect("region display")
            .set_text(Some(&region_text));
        let distance = self
            .globalization
            .format_measurement_for_profile(5.0, MeasurementUnit::Kilometer, Some("N1"))
            .expect("length conversion");
        let temperature = self
            .globalization
            .format_measurement_for_profile(21.0, MeasurementUnit::Celsius, Some("N1"))
            .expect("temperature conversion");
        let measurements = service
            .format("Measurements", &[distance.into(), temperature.into()])
            .expect("valid catalog format");
        self.find_control::<TextBlock>("Measurements")
            .expect("measurement display")
            .set_text(Some(&measurements));
    }
}
