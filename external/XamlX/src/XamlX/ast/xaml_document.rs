//! Port of `Ast/XamlDocument.cs`.

use std::collections::HashMap;
use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};

use super::IXamlAstNode;

#[derive(Default)]
pub struct XamlDocument {
    root: Option<Rc<dyn IXamlAstNode>>,
    pub document: Option<String>,
    pub namespace_aliases: HashMap<String, String>,
}

impl XamlDocument {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn root(&self) -> XamlResult<Rc<dyn IXamlAstNode>> {
        self.root
            .clone()
            .ok_or_else(|| XamlError::invalid_operation("Root hasn't been set"))
    }

    pub fn set_root(&mut self, value: Rc<dyn IXamlAstNode>) {
        self.root = Some(value);
    }
}
