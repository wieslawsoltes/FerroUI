//! The application builder: Splat.Core's `IAppBuilder`, `IAppInstance` and `IModule`, and
//! Splat.Builder's `AppBuilder` and `SplatBuilderExtensions`.

mod app_builder;

pub use app_builder::{AppBuilder, IAppBuilder, IAppInstance, IModule, ModuleExt, SplatBuilderExt};
