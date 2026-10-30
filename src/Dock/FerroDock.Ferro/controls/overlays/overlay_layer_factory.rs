// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IOverlayLayer;
use std::rc::Rc;
/// Creates layers on demand instead of sharing overlay controls between hosts.
pub trait IOverlayLayerFactory {
    /// Creates a layer; null results remain supported at the source interop boundary.
    fn create(&self) -> Option<Rc<dyn IOverlayLayer>>;
}
impl PartialEq for dyn IOverlayLayerFactory {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
/// Source delegate creating an optional overlay layer.
pub type OverlayLayerCreation = dyn Fn() -> Option<Rc<dyn IOverlayLayer>>;

/// Source delegate-based layer factory.
pub struct OverlayLayerFactory {
    factory: Rc<OverlayLayerCreation>,
}
impl OverlayLayerFactory {
    /// Creates a factory; a missing delegate is the source programmer error.
    pub fn new(factory: Option<Rc<OverlayLayerCreation>>) -> Rc<Self> {
        Rc::new(Self {
            factory: factory.expect("factory cannot be null"),
        })
    }
    /// Invokes the original delegate without caching its result.
    pub fn create(&self) -> Option<Rc<dyn IOverlayLayer>> {
        (self.factory)()
    }
}
impl IOverlayLayerFactory for OverlayLayerFactory {
    fn create(&self) -> Option<Rc<dyn IOverlayLayer>> {
        OverlayLayerFactory::create(self)
    }
}
