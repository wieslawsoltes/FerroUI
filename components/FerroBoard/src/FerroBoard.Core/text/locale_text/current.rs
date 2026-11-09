//! Existing composition-root locale-name resolver; Core remains independent of controls.
//! A plain synchronous function resolves the CALLER's ambient context; no UI dependency,
//! thread-affine resource registry, dispatcher or additional asynchronous operation is introduced.
use super::CollationError;
use std::sync::OnceLock;
type Provider = fn() -> Result<String, CollationError>;
static PROVIDER: OnceLock<Provider> = OnceLock::new();
static DEFAULT: OnceLock<Result<String, CollationError>> = OnceLock::new();
/// Install once at composition-root initialization. Reinstalling the same function is idempotent.
/// A different host provider is rejected explicitly rather than silently changing global ordering.
pub fn install_current_culture_provider(provider: Provider) -> Result<(), &'static str> {
    let installed = PROVIDER.get_or_init(|| provider);
    if std::ptr::fn_addr_eq(*installed, provider) {
        Ok(())
    } else {
        Err("current-culture provider already installed")
    }
}
/// SDK registration installs a resolver for Base CurrentCulture on the calling thread. Pure Core
/// uses the OS-preferred locale until a host resolver is installed. Platform culture
/// validation is delegated to the native locale service.
pub fn current_culture_name() -> Result<String, CollationError> {
    if let Some(provider) = PROVIDER.get() {
        provider()
    } else {
        DEFAULT
            .get_or_init(crate::text::locale_text::system_locale_name)
            .clone()
    }
}
