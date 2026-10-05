//! Test support shared by the shape and image tests: installs the base
//! crate's mock render interface (geometries that behave like their bounding
//! rectangles) for the lifetime of a guard value.

use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::testing::MockPlatformRenderInterface;
use std::rc::Rc;

/// Keeps a [`MockPlatformRenderInterface`] registered until dropped.
pub struct MockRenderInterfaceScope {
    scope: Rc<dyn IDisposable>,
    render_interface: Rc<MockPlatformRenderInterface>,
}

impl MockRenderInterfaceScope {
    /// Registers a mock render interface in a fresh locator scope, which
    /// lasts until the returned value is dropped.
    pub fn install() -> MockRenderInterfaceScope {
        let (scope, render_interface) = MockPlatformRenderInterface::install();
        MockRenderInterfaceScope { scope, render_interface }
    }

    /// The installed render interface.
    #[allow(dead_code)]
    pub fn render_interface(&self) -> &Rc<MockPlatformRenderInterface> {
        &self.render_interface
    }
}

impl Drop for MockRenderInterfaceScope {
    fn drop(&mut self) {
        self.scope.dispose();
    }
}
