//! `DefaultServiceProviderFactory.cs` and `IServiceProviderFactory.cs`.
use crate::*;

pub trait IServiceProviderFactory<TContainerBuilder> {
    fn create_builder(&self, services: ServiceCollection) -> TContainerBuilder;
    fn create_service_provider(&self, builder: TContainerBuilder)
        -> ServiceResult<ServiceProvider>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultServiceProviderFactory {
    options: ServiceProviderOptions,
}
impl DefaultServiceProviderFactory {
    pub fn new(options: ServiceProviderOptions) -> Self {
        Self { options }
    }
}
impl IServiceProviderFactory<ServiceCollection> for DefaultServiceProviderFactory {
    fn create_builder(&self, services: ServiceCollection) -> ServiceCollection {
        services
    }
    fn create_service_provider(
        &self,
        builder: ServiceCollection,
    ) -> ServiceResult<ServiceProvider> {
        builder.build_service_provider(self.options)
    }
}
