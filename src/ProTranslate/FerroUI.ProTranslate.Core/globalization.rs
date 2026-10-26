use crate::event::Event;
use crate::*;
use std::{cell::RefCell, rc::Rc};
struct GlobalizationInner {
    cultures: CultureService,
    translations: TranslationService,
    region_provider: Rc<dyn RegionProfileProvider>,
    resolver: Rc<dyn MeasurementSystemResolver>,
    converter: Rc<dyn UnitConversionService>,
    formatter: Rc<dyn LocalizedUnitFormatter>,
    region_override: RefCell<Option<RegionProfile>>,
    measurement_override: RefCell<Option<MeasurementSystem>>,
    changed: Event<CultureChangedEventArgs>,
    _subscription: Subscription,
}
#[derive(Clone)]
pub struct GlobalizationService(Rc<GlobalizationInner>);
impl GlobalizationService {
    /// Uses the translation service's culture source, so lookup and display state
    /// cannot accidentally diverge through two unrelated culture services.
    pub fn new(translations: TranslationService) -> Self {
        Self::with_services(
            translations,
            Rc::new(DefaultRegionProfileProvider),
            Rc::new(DefaultMeasurementSystemResolver),
            Rc::new(DefaultUnitConversionService),
            Rc::new(DefaultLocalizedUnitFormatter),
        )
    }
    pub fn with_services(
        translations: TranslationService,
        region_provider: Rc<dyn RegionProfileProvider>,
        resolver: Rc<dyn MeasurementSystemResolver>,
        converter: Rc<dyn UnitConversionService>,
        formatter: Rc<dyn LocalizedUnitFormatter>,
    ) -> Self {
        let cultures = translations.cultures();
        Self(Rc::new_cyclic(
            |weak: &std::rc::Weak<GlobalizationInner>| {
                let weak = weak.clone();
                let subscription = cultures.subscribe(move |event| {
                    if let Some(inner) = weak.upgrade() {
                        inner.changed.emit(event.clone());
                    }
                });
                GlobalizationInner {
                    cultures,
                    translations,
                    region_provider,
                    resolver,
                    converter,
                    formatter,
                    region_override: RefCell::new(None),
                    measurement_override: RefCell::new(None),
                    changed: Event::default(),
                    _subscription: subscription,
                }
            },
        ))
    }
    pub fn cultures(&self) -> CultureService {
        self.0.cultures.clone()
    }
    pub fn translations(&self) -> TranslationService {
        self.0.translations.clone()
    }
    pub fn unit_converter(&self) -> Rc<dyn UnitConversionService> {
        self.0.converter.clone()
    }
    pub fn unit_formatter(&self) -> Rc<dyn LocalizedUnitFormatter> {
        self.0.formatter.clone()
    }
    pub fn current_culture(&self) -> Culture {
        self.0.cultures.current_culture()
    }
    pub fn flow_direction(&self) -> TextFlowDirection {
        self.0.cultures.flow_direction()
    }
    pub fn set_culture(&self, culture: Culture) -> bool {
        self.0.cultures.set_culture(culture)
    }
    pub fn get_string(&self, key: &str) -> Result<LocalizedString, TranslationError> {
        self.0.translations.get_string(key)
    }
    pub fn region_profile(&self) -> Result<RegionProfile, TranslationError> {
        let culture = self.current_culture();
        let region_override = self.0.region_override.borrow().clone();
        self.region_profile_for(&culture, region_override.as_ref())
    }
    fn region_profile_for(
        &self,
        culture: &Culture,
        region_override: Option<&RegionProfile>,
    ) -> Result<RegionProfile, TranslationError> {
        let result = self.0.region_provider.get_profile(culture, region_override);
        if let Err(error) = &result {
            self.0.translations.report(ProTranslateDiagnostic {
                kind: ProTranslateDiagnosticKind::RegionProfileFailure,
                severity: ProTranslateDiagnosticSeverity::Error,
                message: error.to_string(),
                key: None,
                culture: Some(culture.clone()),
                provider_name: None,
                error: Some(error.clone()),
            });
        }
        result
    }
    pub fn measurement_system_profile(&self) -> Result<MeasurementSystemProfile, TranslationError> {
        self.measurement_system_profile_for(&self.current_culture())
    }
    fn measurement_system_profile_for(
        &self,
        culture: &Culture,
    ) -> Result<MeasurementSystemProfile, TranslationError> {
        // Capture overrides before entering host callbacks: a provider may
        // change them or the culture while resolving this operation's region.
        let region_override = self.0.region_override.borrow().clone();
        let system = *self.0.measurement_override.borrow();
        let region = self.region_profile_for(culture, region_override.as_ref())?;
        Ok(self.0.resolver.resolve(&region, system))
    }
    pub fn measurement_system(&self) -> Result<MeasurementSystem, TranslationError> {
        Ok(self.measurement_system_profile()?.system)
    }
    pub fn is_metric(&self) -> Result<bool, TranslationError> {
        Ok(self.measurement_system()? == MeasurementSystem::Metric)
    }
    pub fn set_region_override(&self, region: RegionProfile) -> bool {
        if self.0.region_override.borrow().as_ref() == Some(&region) {
            return false;
        }
        *self.0.region_override.borrow_mut() = Some(region);
        self.notify();
        true
    }
    pub fn clear_region_override(&self) -> bool {
        let changed = self.0.region_override.borrow_mut().take().is_some();
        if changed {
            self.notify();
        }
        changed
    }
    pub fn set_measurement_system_override(&self, system: MeasurementSystem) -> bool {
        if *self.0.measurement_override.borrow() == Some(system) {
            return false;
        }
        *self.0.measurement_override.borrow_mut() = Some(system);
        self.notify();
        true
    }
    pub fn clear_measurement_system_override(&self) -> bool {
        let changed = self.0.measurement_override.borrow_mut().take().is_some();
        if changed {
            self.notify();
        }
        changed
    }
    pub fn convert_measurement(
        &self,
        value: f64,
        source: MeasurementUnit,
    ) -> Result<MeasurementValue, TranslationError> {
        self.0
            .converter
            .convert_to_profile(value, source, &self.measurement_system_profile()?)
    }
    pub fn format_measurement(
        &self,
        value: f64,
        unit: MeasurementUnit,
        format: Option<&str>,
    ) -> Result<String, TranslationError> {
        self.0.formatter.format(
            MeasurementValue { value, unit },
            &self.current_culture(),
            format,
        )
    }
    pub fn format_measurement_for_profile(
        &self,
        value: f64,
        source: MeasurementUnit,
        format: Option<&str>,
    ) -> Result<String, TranslationError> {
        let culture = self.current_culture();
        let profile = self.measurement_system_profile_for(&culture)?;
        let value = self
            .0
            .converter
            .convert_to_profile(value, source, &profile)?;
        self.0.formatter.format(value, &culture, format)
    }
    pub fn subscribe(&self, callback: impl Fn(&CultureChangedEventArgs) + 'static) -> Subscription {
        self.0.changed.subscribe(callback)
    }
    fn notify(&self) {
        let snapshot = self.0.cultures.snapshot();
        self.0.changed.emit(CultureChangedEventArgs {
            old: snapshot.clone(),
            new: snapshot,
        });
    }
}
