//! Port of `CompilerExtensions/Visitors/NameScopeRegistrationVisitor.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstVisitor, IXamlLineInfo, XamlAstNodeExtensions, XamlAstTextNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::type_system::{IXamlType, XamlPseudoType};

use crate::compiler_extensions::transformers::{
    FerroNameScopeRegistrationXamlIlNode, NestedScopeMetadataNode,
};

/// Collects the names registered in one name scope level of a document:
/// name -> (type of the named element, line info of the name).
///
/// Upstream derives from `Dictionary<string, (IXamlType type, IXamlLineInfo line)>`; the entries
/// are kept in registration order here and exposed through the dictionary-like accessors.
pub struct NameScopeRegistrationVisitor {
    entries: Vec<(String, (Rc<dyn IXamlType>, Rc<dyn IXamlLineInfo>))>,
    target_metadata_scope_level: i32,
    parents: Vec<Rc<dyn IXamlAstNode>>,
    metadata_scope_level: i32,
}

impl Default for NameScopeRegistrationVisitor {
    /// `new NameScopeRegistrationVisitor()`: initial level 0, target level 1.
    fn default() -> Self {
        Self::new(0, 1)
    }
}

impl NameScopeRegistrationVisitor {
    pub fn new(initial_metadata_scope_level: i32, target_metadata_scope_level: i32) -> Self {
        Self {
            entries: Vec::new(),
            target_metadata_scope_level,
            parents: Vec::new(),
            metadata_scope_level: initial_metadata_scope_level,
        }
    }

    /// `this[name] = value`.
    pub fn set(&mut self, name: &str, value: (Rc<dyn IXamlType>, Rc<dyn IXamlLineInfo>)) {
        match self.entries.iter_mut().find(|(key, _)| key == name) {
            Some(entry) => entry.1 = value,
            None => self.entries.push((name.to_string(), value)),
        }
    }

    /// `TryGetValue(name, out value)`.
    pub fn get(&self, name: &str) -> Option<&(Rc<dyn IXamlType>, Rc<dyn IXamlLineInfo>)> {
        self.entries
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    pub fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// `Count`.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The registrations, in registration order.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&str, &(Rc<dyn IXamlType>, Rc<dyn IXamlLineInfo>))> {
        self.entries.iter().map(|(key, value)| (key.as_str(), value))
    }
}

impl IXamlAstVisitor for NameScopeRegistrationVisitor {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if self.metadata_scope_level == self.target_metadata_scope_level {
            if let Some(name_scope_registration) =
                node.cast::<FerroNameScopeRegistrationXamlIlNode>()
            {
                if let Some(text_node) = name_scope_registration.name().cast::<XamlAstTextNode>() {
                    let type_ = name_scope_registration
                        .target_type
                        .clone()
                        .unwrap_or_else(XamlPseudoType::unknown);
                    let text = text_node.text();
                    self.set(&text, (type_, text_node));
                }
            }
        }

        Ok(node)
    }

    fn push(&mut self, node: Rc<dyn IXamlAstNode>) {
        if node.is::<NestedScopeMetadataNode>() {
            self.metadata_scope_level += 1;
        }
        self.parents.push(node);
    }

    fn pop(&mut self) {
        if let Some(old_parent) = self.parents.pop() {
            if old_parent.is::<NestedScopeMetadataNode>() {
                self.metadata_scope_level -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xamlx::ast::{
        visit_node, IXamlAstValueNode, XamlAstClrTypeReference, XamlAstObjectNode, XamlLineInfo,
    };

    use crate::testing::{create_test_framework, TestFramework};

    fn text(fw: &TestFramework, line: i32, value: &str) -> Rc<dyn IXamlAstValueNode> {
        XamlAstTextNode::with_type(
            &XamlLineInfo::new(line, 1),
            value,
            true,
            Some(fw.t("System.String")),
        )
    }

    fn object(fw: &TestFramework, children: Vec<Rc<dyn IXamlAstNode>>) -> Rc<XamlAstObjectNode> {
        let line_info = XamlLineInfo::new(1, 1);
        let node = XamlAstObjectNode::new(
            &line_info,
            XamlAstClrTypeReference::new(&line_info, fw.types.control.clone(), false),
        );
        *node.children.borrow_mut() = children;
        node
    }

    /// root
    ///   register "outer" (Border)
    ///   nested scope
    ///     register "inner" (no type)
    ///     register <non-text name>
    ///     nested scope
    ///       register "deep" (TextBlock)
    ///     register "inner" (TextBlock), again
    fn tree(fw: &TestFramework) -> Rc<dyn IXamlAstNode> {
        let border = fw.t("FerroUI.Controls.Border");
        let text_block = fw.t("FerroUI.Controls.TextBlock");
        let deep_scope = NestedScopeMetadataNode::new(object(
            fw,
            vec![FerroNameScopeRegistrationXamlIlNode::new(
                text(fw, 30, "deep"),
                Some(text_block.clone()),
            )],
        ));
        let non_text_name: Rc<dyn IXamlAstValueNode> = object(fw, vec![]);
        let inner_scope = NestedScopeMetadataNode::new(object(
            fw,
            vec![
                FerroNameScopeRegistrationXamlIlNode::new(text(fw, 20, "inner"), None),
                FerroNameScopeRegistrationXamlIlNode::new(non_text_name, Some(border.clone())),
                object(fw, vec![deep_scope]),
                FerroNameScopeRegistrationXamlIlNode::new(text(fw, 40, "inner"), Some(text_block)),
            ],
        ));
        object(
            fw,
            vec![
                FerroNameScopeRegistrationXamlIlNode::new(text(fw, 10, "outer"), Some(border)),
                object(fw, vec![inner_scope]),
            ],
        )
    }

    fn names(visitor: &NameScopeRegistrationVisitor) -> Vec<(String, String, i32)> {
        visitor
            .iter()
            .map(|(name, (type_, line))| (name.to_string(), type_.name(), line.line()))
            .collect()
    }

    #[test]
    fn collects_the_registrations_of_the_target_scope_level() {
        let fw = create_test_framework();
        let root = tree(&fw);

        // Default: start at level 0, collect level 1 (the first nested scope).
        let mut visitor = NameScopeRegistrationVisitor::default();
        visit_node(&root, &mut visitor).expect("visited");
        // "inner" is registered twice: the last registration wins, the position is kept.
        assert_eq!(names(&visitor), [("inner".to_string(), "TextBlock".to_string(), 40)]);
        assert_eq!(visitor.len(), 1);
        assert!(visitor.contains_key("inner"));
        assert!(!visitor.contains_key("outer"));
        assert!(visitor.get("deep").is_none());

        // The root scope.
        let mut visitor = NameScopeRegistrationVisitor::new(0, 0);
        visit_node(&root, &mut visitor).expect("visited");
        assert_eq!(names(&visitor), [("outer".to_string(), "Border".to_string(), 10)]);

        // The innermost scope.
        let mut visitor = NameScopeRegistrationVisitor::new(0, 2);
        visit_node(&root, &mut visitor).expect("visited");
        assert_eq!(names(&visitor), [("deep".to_string(), "TextBlock".to_string(), 30)]);

        // Starting inside a scope shifts the levels.
        let mut visitor = NameScopeRegistrationVisitor::new(1, 1);
        visit_node(&root, &mut visitor).expect("visited");
        assert_eq!(names(&visitor), [("outer".to_string(), "Border".to_string(), 10)]);

        // Nothing at that level.
        let mut visitor = NameScopeRegistrationVisitor::new(0, 5);
        visit_node(&root, &mut visitor).expect("visited");
        assert!(visitor.is_empty());
    }

    #[test]
    fn a_registration_without_target_type_is_of_unknown_type() {
        let fw = create_test_framework();
        let root: Rc<dyn IXamlAstNode> = object(
            &fw,
            vec![FerroNameScopeRegistrationXamlIlNode::new(text(&fw, 5, "x"), None)],
        );
        let mut visitor = NameScopeRegistrationVisitor::new(0, 0);
        visit_node(&root, &mut visitor).expect("visited");
        let (type_, line) = visitor.get("x").expect("registered");
        assert!(XamlPseudoType::is_unknown(&**type_));
        assert_eq!((line.line(), line.position()), (5, 1));
    }
}
