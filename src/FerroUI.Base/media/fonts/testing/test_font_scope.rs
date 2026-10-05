use std::rc::Rc;

use crate::platform::{IAssetLoader, IFontManagerImpl};
use crate::reactive::IDisposable;
use crate::FerroLocator;

use super::{TestAssetLoader, TestFontManagerImpl};

/// A locator scope with a font backend and an asset loader bound. The scope
/// ends (and the previous services are restored) when the value is dropped.
///
/// The font manager is created on first use
/// (`FontManager::current()`) and lives in the scope, so every test gets a
/// fresh one. Additional services (font manager options, a text shaper) can
/// be bound with `FerroLocator::current_mutable()` after the scope started
/// and before the font manager is first used.
pub struct TestFontScope {
    scope: Rc<dyn IDisposable>,
    font_manager_impl: Rc<TestFontManagerImpl>,
    asset_loader: Rc<TestAssetLoader>,
}

#[allow(dead_code)] // the full harness API is kept for the text tests built on top
impl TestFontScope {
    /// Starts a scope with the default font set ([`TestFontManagerImpl::headless`]).
    pub fn start() -> TestFontScope {
        Self::with_font_manager(TestFontManagerImpl::headless())
    }

    /// Starts a scope with the given font backend and an empty asset loader.
    pub fn with_font_manager(font_manager_impl: Rc<TestFontManagerImpl>) -> TestFontScope {
        Self::with_services(font_manager_impl, Rc::new(TestAssetLoader::new()))
    }

    /// Starts a scope with the given font backend and asset loader.
    pub fn with_services(
        font_manager_impl: Rc<TestFontManagerImpl>,
        asset_loader: Rc<TestAssetLoader>,
    ) -> TestFontScope {
        let scope = FerroLocator::enter_scope();

        let locator = FerroLocator::current_mutable();

        locator.bind::<dyn IFontManagerImpl>().to_constant(font_manager_impl.clone());
        locator.bind::<dyn IAssetLoader>().to_constant(asset_loader.clone());

        TestFontScope { scope, font_manager_impl, asset_loader }
    }

    /// Starts a scope with the default font set and the embedded test fonts
    /// ([`test_fonts::asset_loader`](super::test_fonts::asset_loader)).
    pub fn with_assets() -> TestFontScope {
        Self::with_services(TestFontManagerImpl::headless(), super::test_fonts::asset_loader())
    }

    /// The font backend of the scope.
    pub fn font_manager_impl(&self) -> &Rc<TestFontManagerImpl> {
        &self.font_manager_impl
    }

    /// The asset loader of the scope.
    pub fn asset_loader(&self) -> &Rc<TestAssetLoader> {
        &self.asset_loader
    }
}

impl Drop for TestFontScope {
    fn drop(&mut self) {
        self.scope.dispose();
    }
}
