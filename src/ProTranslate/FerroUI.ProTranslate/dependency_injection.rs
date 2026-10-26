use crate::TranslationBindingSource;
use ferroui_dependency_injection::{ServiceCollection, ServiceProvider, ServiceResult};
use ferroui_protranslate_core::{
    CultureService, GlobalizationService, TranslationProvider, TranslationService,
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
}

impl ProTranslateServiceCollectionExtensions for ServiceCollection {
    fn add_protranslate(
        &mut self,
        provider: Rc<dyn TranslationProvider>,
        cultures: CultureService,
    ) -> &mut Self {
        let translations = TranslationService::new(provider.clone(), cultures.clone());
        let globalization = GlobalizationService::new(translations.clone());
        self.add_singleton_instance::<dyn TranslationProvider>(provider)
            .add_singleton_instance(Rc::new(cultures))
            .add_singleton_instance(Rc::new(translations))
            .add_singleton_instance(Rc::new(globalization))
    }
}

/// Explicit bootstrap from a built container to the UI-thread-local XAML source.
pub trait ProTranslateServiceProviderExtensions {
    fn use_protranslate(&self) -> ServiceResult<TranslationBindingSource>;
}

impl ProTranslateServiceProviderExtensions for ServiceProvider {
    fn use_protranslate(&self) -> ServiceResult<TranslationBindingSource> {
        let service = self.get_required_service::<TranslationService>()?;
        let source = TranslationBindingSource::default_source();
        source.set_service((*service).clone());
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
}
