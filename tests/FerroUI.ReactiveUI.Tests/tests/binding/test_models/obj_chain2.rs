//! `ObjChain2` (`TestModels/ObjChain2.cs`).

use super::ObjChain3;
use std::rc::Rc;

notifying_model! {
    /// The second link of a chain of objects (`ObjChain2`).
    pub struct ObjChain2 (changing: false, check_equality: true) {
        chain3 / set_chain3: Option<Rc<ObjChain3>> = None,
    }
}
