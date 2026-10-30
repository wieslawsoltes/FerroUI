//! Typed, notifying string properties used by the compiled XAML view.

use crate::generated::ProTranslateStrings;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_protranslate::TranslationBindingSource;
use ferroui_protranslate_core::{
    GlobalizationService, MeasurementUnit, Subscription, TranslationError,
};
use std::rc::Rc;

pub struct MainWindowViewModel {
    strings: ProTranslateStrings,
    globalization: GlobalizationService,
    changed: Event<str>,
    strings_token: u64,
    _globalization_subscription: Subscription,
}

impl MainWindowViewModel {
    pub fn new(source: TranslationBindingSource) -> Rc<Self> {
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let strings = ProTranslateStrings::new(source.service());
            let globalization = GlobalizationService::new(source.service());
            let strings_weak = weak.clone();
            let strings_token = strings.property_changed().add(Rc::new(move |_| {
                if let Some(this) = strings_weak.upgrade() {
                    this.changed.raise("");
                }
            }));
            let weak = weak.clone();
            let subscription = globalization.subscribe(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.changed.raise("");
                }
            });
            Self {
                strings,
                globalization,
                changed: Event::new(),
                strings_token,
                _globalization_subscription: subscription,
            }
        })
    }

    pub fn title(&self) -> Result<String, TranslationError> {
        self.strings.value_title()
    }
    pub fn subtitle(&self) -> Result<String, TranslationError> {
        self.strings.value_subtitle()
    }
    pub fn globalization(&self) -> GlobalizationService {
        self.globalization.clone()
    }

    pub fn region_details(&self) -> Result<String, TranslationError> {
        let culture = self.globalization.current_culture();
        let region = self.globalization.region_profile()?;
        self.strings.format_regiondetails([
            culture.name().into(),
            region.two_letter_iso_region_name.into(),
        ])
    }

    pub fn measurements(&self) -> Result<String, TranslationError> {
        let distance = self.globalization.format_measurement_for_profile(
            5.0,
            MeasurementUnit::Kilometer,
            Some("N1"),
        )?;
        let temperature = self.globalization.format_measurement_for_profile(
            21.0,
            MeasurementUnit::Celsius,
            Some("N1"),
        )?;
        self.strings
            .format_measurements([distance.into(), temperature.into()])
    }
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for MainWindowViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}
impl Drop for MainWindowViewModel {
    fn drop(&mut self) {
        self.strings.property_changed().remove(self.strings_token);
    }
}

ferroui_base::ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    properties: [
        Title: String { try_get: MainWindowViewModel::title },
        Subtitle: String { try_get: MainWindowViewModel::subtitle },
        RegionDetails: String { try_get: MainWindowViewModel::region_details },
        Measurements: String { try_get: MainWindowViewModel::measurements },
    ],
    notify_property_changed: MainWindowViewModel,
});
