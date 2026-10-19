//! `ObjChain1` (`TestModels/ObjChain1.cs`).

use super::ObjChain2;
use std::rc::Rc;

notifying_model! {
    /// The first link of a chain of objects (`ObjChain1`).
    pub struct ObjChain1 (changing: false, check_equality: true) {
        chain2 / set_chain2: Option<Rc<ObjChain2>> = None,
    }
}
