//! `PocoChainRoot` (`TestModels/PocoChainRoot.cs`).

use super::TestFixture;
use std::rc::Rc;

/// An object without change notification that holds a [`TestFixture`] (`PocoChainRoot`).
pub struct PocoChainRoot {
    leaf: Rc<TestFixture>,
}

#[allow(dead_code)]
impl PocoChainRoot {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { leaf: TestFixture::new() })
    }

    pub fn leaf(&self) -> Rc<TestFixture> {
        self.leaf.clone()
    }
}
