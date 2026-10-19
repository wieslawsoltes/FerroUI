//! `ObjChain3` (`TestModels/ObjChain3.cs`).

use super::HostTestFixture;
use std::rc::Rc;

notifying_model! {
    /// The third link of a chain of objects (`ObjChain3`).
    pub struct ObjChain3 (changing: false, check_equality: true) {
        host / set_host: Option<Rc<HostTestFixture>> = None,
    }
}
