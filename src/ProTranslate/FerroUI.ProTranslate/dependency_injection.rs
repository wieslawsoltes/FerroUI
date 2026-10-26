use crate::TranslationBindingSource;
use ferroui_dependency_injection::{
    Dependency, ServiceCollection, ServiceDescriptor, ServiceLifetime, ServiceProvider,
    ServiceResult,
};
use ferroui_protranslate_core::{
    Culture, CultureService, DefaultLocalizedUnitFormatter, DefaultMeasurementSystemResolver,
    DefaultRegionProfileProvider, DefaultUnitConversionService, GlobalizationService,
    InMemoryTranslationProvider, LocalizedUnitFormatter, MeasurementSystemResolver,
    RegionProfileProvider, TranslationCacheOptions, TranslationFallbackOptions,
    TranslationProvider, TranslationService, UnitConversionService,
};
use std::rc::Rc;

/// Registers ProTranslate services with FerroUI's native dependency injection container.
pub trait ProTranslateServiceCollectionExtensions {
    /// Adds one shared culture source, provider, translation service and globalization
    /// service. Registration does not mutate the ambient XAML source; bootstrap the
    /// built provider explicitly with [`use_protranslate`](ProTranslateServiceProviderExtensions::use_protranslate).
    fn add_protranslate(
        &mut self,
        provider: Rc<dyn TranslationProvider>,
        cultures: CultureService,
    ) -> &mut Self;

    /// Registers defaults using the framework's current UI culture. Existing
    /// registrations take precedence, including custom provider/service factories.
    fn add_protranslate_defaults(&mut self) -> &mut Self;

    fn add_protranslate_with_options(
        &mut self,
        provider: Rc<dyn TranslationProvider>,
        cultures: CultureService,
        fallback: TranslationFallbackOptions,
        cache: TranslationCacheOptions,
    ) -> &mut Self;

    /// Adds the native binding source for an already-registered core service.
    fn add_protranslate_ui(&mut self) -> &mut Self;
}

impl ProTranslateServiceCollectionExtensions for ServiceCollection {
    fn add_protranslate(
        &mut self,
        provider: Rc<dyn TranslationProvider>,
        cultures: CultureService,
    ) -> &mut Self {
        self.add_protranslate_with_options(
            provider,
            cultures,
            Default::default(),
            Default::default(),
        )
    }

    fn add_protranslate_defaults(&mut self) -> &mut Self {
        let culture =
            Culture::parse(ferroui_base::utilities::CultureInfo::current_ui_culture().name())
                .unwrap_or_else(|_| Culture::invariant());
        self.add_protranslate(
            Rc::new(InMemoryTranslationProvider::new("Default")),
            CultureService::new(culture),
        )
    }

    fn add_protranslate_with_options(
        &mut self,
        provider: Rc<dyn TranslationProvider>,
        cultures: CultureService,
        fallback: TranslationFallbackOptions,
        cache: TranslationCacheOptions,
    ) -> &mut Self {
        self.try_add(ServiceDescriptor::instance::<dyn TranslationProvider>(
            provider,
        ))
        .try_add(ServiceDescriptor::instance(Rc::new(cultures)))
        .try_add(ServiceDescriptor::instance(Rc::new(fallback)))
        .try_add(ServiceDescriptor::instance(Rc::new(cache)))
        .try_add(ServiceDescriptor::instance::<dyn RegionProfileProvider>(
            Rc::new(DefaultRegionProfileProvider),
        ))
        .try_add(
            ServiceDescriptor::instance::<dyn MeasurementSystemResolver>(Rc::new(
                DefaultMeasurementSystemResolver,
            )),
        )
        .try_add(ServiceDescriptor::instance::<dyn UnitConversionService>(
            Rc::new(DefaultUnitConversionService),
        ))
        .try_add(ServiceDescriptor::instance::<dyn LocalizedUnitFormatter>(
            Rc::new(DefaultLocalizedUnitFormatter),
        ))
        .try_add(
            ServiceDescriptor::new(ServiceLifetime::Singleton, |provider| {
                Ok(Rc::new(TranslationService::with_options(
                    provider.get_required_service::<dyn TranslationProvider>()?,
                    (*provider.get_required_service::<CultureService>()?).clone(),
                    (*provider.get_required_service::<TranslationFallbackOptions>()?).clone(),
                    (*provider.get_required_service::<TranslationCacheOptions>()?).clone(),
                )))
            })
            .with_dependencies(vec![
                Dependency::required::<dyn TranslationProvider>(),
                Dependency::required::<CultureService>(),
                Dependency::required::<TranslationFallbackOptions>(),
                Dependency::required::<TranslationCacheOptions>(),
            ]),
        )
        .try_add(
            ServiceDescriptor::new(ServiceLifetime::Singleton, |provider| {
                Ok(Rc::new(GlobalizationService::with_services(
                    (*provider.get_required_service::<TranslationService>()?).clone(),
                    provider.get_required_service::<dyn RegionProfileProvider>()?,
                    provider.get_required_service::<dyn MeasurementSystemResolver>()?,
                    provider.get_required_service::<dyn UnitConversionService>()?,
                    provider.get_required_service::<dyn LocalizedUnitFormatter>()?,
                )))
            })
            .with_dependencies(vec![
                Dependency::required::<TranslationService>(),
                Dependency::required::<dyn RegionProfileProvider>(),
                Dependency::required::<dyn MeasurementSystemResolver>(),
                Dependency::required::<dyn UnitConversionService>(),
                Dependency::required::<dyn LocalizedUnitFormatter>(),
            ]),
        );
        self.add_protranslate_ui()
    }

    fn add_protranslate_ui(&mut self) -> &mut Self {
        self.try_add(
            ServiceDescriptor::new(ServiceLifetime::Singleton, |provider| {
                Ok(Rc::new(TranslationBindingSource::new(
                    (*provider.get_required_service::<TranslationService>()?).clone(),
                )))
            })
            .with_dependencies(vec![Dependency::required::<TranslationService>()])
            .with_disposal::<TranslationBindingSource>(),
        )
    }
}

/// Explicit bootstrap from a built container to the UI-thread-local XAML source.
pub trait ProTranslateServiceProviderExtensions {
    fn use_protranslate(&self) -> ServiceResult<TranslationBindingSource>;
}

impl ProTranslateServiceProviderExtensions for ServiceProvider {
    fn use_protranslate(&self) -> ServiceResult<TranslationBindingSource> {
        let source = match self.get_service::<TranslationBindingSource>()? {
            Some(source) => (*source).clone(),
            None => TranslationBindingSource::new(
                (*self.get_required_service::<TranslationService>()?).clone(),
            ),
        };
        TranslationBindingSource::use_default_source(source.clone());
        Ok(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_protranslate_core::{Culture, InMemoryTranslationProvider};

    #[test]
    fn container_shares_culture_state_and_bootstraps_explicitly() {
        let mut services = ServiceCollection::new();
        let provider = Rc::new(InMemoryTranslationProvider::new("DI"));
        provider.insert(Culture::parse("pl-PL").unwrap(), "Title", "Witaj");
        services.add_protranslate(
            provider,
            CultureService::new(Culture::parse("en-US").unwrap()),
        );
        let container = services.build_service_provider(Default::default()).unwrap();
        let source = container.use_protranslate().unwrap();
        container
            .get_required_service::<GlobalizationService>()
            .unwrap()
            .set_culture(Culture::parse("pl-PL").unwrap());
        assert_eq!(source.service().get_string("Title").unwrap().value, "Witaj");
        assert_eq!(
            container
                .get_required_service::<CultureService>()
                .unwrap()
                .current_culture()
                .name(),
            "pl-PL"
        );
    }

    #[test]
    fn registration_is_lazy_idempotent_and_preserves_existing_options_and_factories() {
        use std::cell::Cell;
        let mut services = ServiceCollection::new();
        let calls = Rc::new(Cell::new(0));
        let calls_for_factory = calls.clone();
        let provider = Rc::new(InMemoryTranslationProvider::new("Custom"));
        provider.insert(
            Culture::parse("pl-PL").unwrap(),
            "Title",
            "Existing registration",
        );
        services.add_singleton::<dyn TranslationProvider, _>(move |_| {
            calls_for_factory.set(calls_for_factory.get() + 1);
            Ok(provider.clone())
        });
        let cultures = CultureService::new(Culture::parse("pl-PL").unwrap());
        services.add_singleton_instance(Rc::new(cultures.clone()));
        let options = TranslationFallbackOptions {
            format_failure_behavior:
                ferroui_protranslate_core::TranslationFormatFailureBehavior::ReportAndReturnError,
            ..Default::default()
        };
        services.add_singleton_instance(Rc::new(options.clone()));
        services.add_protranslate_defaults();
        let count = services.len();
        services.add_protranslate_defaults();
        assert_eq!(services.len(), count);
        assert_eq!(calls.get(), 0);
        assert_eq!(cultures.subscriber_count(), 0);
        let container = services.build_service_provider(Default::default()).unwrap();
        let service = container
            .get_required_service::<TranslationService>()
            .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(service.options(), options);
        assert_eq!(
            service.get_string("Title").unwrap().value,
            "Existing registration"
        );
        let source = container
            .get_required_service::<TranslationBindingSource>()
            .unwrap();
        let previous = TranslationBindingSource::default_source();
        let ambient = container.use_protranslate().unwrap();
        assert!(ambient == *source);
        container.dispose().unwrap();
        assert!(source.is_disposed());
        TranslationBindingSource::use_default_source(previous);
    }

    #[test]
    fn globalization_resolves_each_application_supplied_contract() {
        use ferroui_protranslate_core::{
            MeasurementSystem, MeasurementSystemProfile, MeasurementUnit, MeasurementValue,
            RegionProfile, TranslationError,
        };
        struct Regions;
        impl RegionProfileProvider for Regions {
            fn get_profile(
                &self,
                culture: &Culture,
                _: Option<&RegionProfile>,
            ) -> Result<RegionProfile, TranslationError> {
                RegionProfile::for_region("PL", culture)
            }
        }
        struct Resolver;
        impl MeasurementSystemResolver for Resolver {
            fn resolve(
                &self,
                _: &RegionProfile,
                _: Option<MeasurementSystem>,
            ) -> MeasurementSystemProfile {
                MeasurementSystemProfile::for_system(MeasurementSystem::USCustomary)
            }
        }
        struct Converter;
        impl UnitConversionService for Converter {
            fn convert(
                &self,
                _: f64,
                _: MeasurementUnit,
                _: MeasurementUnit,
            ) -> Result<f64, TranslationError> {
                Ok(42.0)
            }
        }
        struct Formatter;
        impl LocalizedUnitFormatter for Formatter {
            fn format(
                &self,
                value: MeasurementValue,
                culture: &Culture,
                _: Option<&str>,
            ) -> Result<String, TranslationError> {
                Ok(format!(
                    "custom:{}:{:?}:{}",
                    value.value,
                    value.unit,
                    culture.name()
                ))
            }
        }
        let mut services = ServiceCollection::new();
        services
            .add_singleton_instance::<dyn RegionProfileProvider>(Rc::new(Regions))
            .add_singleton_instance::<dyn MeasurementSystemResolver>(Rc::new(Resolver))
            .add_singleton_instance::<dyn UnitConversionService>(Rc::new(Converter))
            .add_singleton_instance::<dyn LocalizedUnitFormatter>(Rc::new(Formatter));
        services.add_protranslate(
            Rc::new(InMemoryTranslationProvider::new("Test")),
            CultureService::new(Culture::parse("en-US").unwrap()),
        );
        let container = services.build_service_provider(Default::default()).unwrap();
        let globalization = container
            .get_required_service::<GlobalizationService>()
            .unwrap();
        assert_eq!(
            globalization
                .region_profile()
                .unwrap()
                .two_letter_iso_region_name,
            "PL"
        );
        assert_eq!(
            globalization
                .format_measurement_for_profile(10.0, MeasurementUnit::Celsius, None)
                .unwrap(),
            "custom:42:Fahrenheit:en-US"
        );
    }

    #[test]
    fn bootstrap_supports_a_preexisting_core_service_without_ui_registration() {
        let provider = Rc::new(InMemoryTranslationProvider::new("Existing"));
        let service = TranslationService::new(provider, CultureService::new(Culture::invariant()));
        let mut services = ServiceCollection::new();
        services.add_singleton_instance(Rc::new(service.clone()));
        let container = services.build_service_provider(Default::default()).unwrap();
        let previous = TranslationBindingSource::default_source();
        let source = container.use_protranslate().unwrap();
        service
            .cultures()
            .set_culture(Culture::parse("pl-PL").unwrap());
        assert_eq!(source.culture().name(), "pl-PL");
        TranslationBindingSource::use_default_source(previous);
    }
}
