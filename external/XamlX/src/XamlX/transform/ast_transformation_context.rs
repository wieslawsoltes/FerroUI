//! Port of `Transform/AstTransformationContext.cs`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use crate::ast::IXamlAstTypeReference;
use crate::ast::{
    visit_node, IXamlAstNode, IXamlAstValueNode, IXamlAstVisitor,
    SkipXamlValueWithManipulationNode, XamlAstClrTypeReference, XamlAstNodeExtensions,
    XamlDocument,
};
use crate::diagnostics::{exception_to_diagnostic, XamlDiagnostic, XamlDiagnosticSeverity};
use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::XamlPseudoType;

use super::{IXamlAstTransformer, TransformerConfiguration, XamlContextBase};

pub struct AstTransformationContext {
    base: XamlContextBase,
    root_object: RefCell<Option<Rc<dyn IXamlAstValueNode>>>,
    document: Option<String>,
    /// See [`AstTransformationContext::set_document_override`].
    document_override: RefCell<Option<Option<String>>>,
    pub namespace_aliases: RefCell<HashMap<String, String>>,
    configuration: Rc<TransformerConfiguration>,
}

/// Gives access to the `XamlContextBase` members (the C# base class).
impl Deref for AstTransformationContext {
    type Target = XamlContextBase;

    fn deref(&self) -> &XamlContextBase {
        &self.base
    }
}

impl AstTransformationContext {
    pub fn new(
        configuration: Rc<TransformerConfiguration>,
        xaml_document: Option<&XamlDocument>,
    ) -> Self {
        Self {
            base: XamlContextBase::new(),
            root_object: RefCell::new(None),
            document: xaml_document.and_then(|d| d.document.clone()),
            document_override: RefCell::new(None),
            namespace_aliases: RefCell::new(
                xaml_document
                    .map(|d| d.namespace_aliases.clone())
                    .unwrap_or_default(),
            ),
            configuration,
        }
    }

    pub fn document(&self) -> Option<&str> {
        self.document.as_deref()
    }

    /// The value of the (virtual upstream) `Document` property: the override installed with
    /// [`AstTransformationContext::set_document_override`] if there is one, the document the
    /// context was created for otherwise. Diagnostics and errors are attributed to it.
    pub fn current_document(&self) -> Option<String> {
        match &*self.document_override.borrow() {
            Some(document) => document.clone(),
            None => self.document.clone(),
        }
    }

    /// Stands in for a derived context overriding the `Document` property: `Some(value)` makes
    /// [`AstTransformationContext::current_document`] return `value`, `None` removes the override.
    pub fn set_document_override(&self, document: Option<Option<String>>) {
        *self.document_override.borrow_mut() = document;
    }

    pub fn configuration(&self) -> &Rc<TransformerConfiguration> {
        &self.configuration
    }

    pub fn root_object(&self) -> XamlResult<Rc<dyn IXamlAstValueNode>> {
        self.root_object
            .borrow()
            .clone()
            .ok_or_else(|| XamlError::invalid_operation("RootObject hasn't been set"))
    }

    pub fn set_root_object(&self, value: Rc<dyn IXamlAstValueNode>) {
        *self.root_object.borrow_mut() = Some(value);
    }

    /// `NamespaceAliases.TryGetValue`.
    pub fn try_get_namespace_alias(&self, alias: &str) -> Option<String> {
        self.namespace_aliases.borrow().get(alias).cloned()
    }

    /// Reports a diagnostic to the configured handler and returns the resulting severity.
    /// With `throw_on_fatal`, a fatal diagnostic is turned into an error.
    pub fn report_diagnostic(
        &self,
        mut diagnostic: XamlDiagnostic,
        throw_on_fatal: bool,
    ) -> XamlResult<XamlDiagnosticSeverity> {
        if diagnostic
            .document
            .as_deref()
            .is_none_or(|d| d.trim().is_empty())
        {
            diagnostic.document = self.current_document();
        }

        let severity = self
            .configuration
            .diagnostics_handler
            .report_diagnostic(&diagnostic);
        if throw_on_fatal && severity >= XamlDiagnosticSeverity::Fatal {
            return Err(diagnostic.to_exception());
        }

        Ok(severity)
    }

    pub fn visit(
        &self,
        root: &Rc<dyn IXamlAstNode>,
        transformer: &dyn IXamlAstTransformer,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let mut visitor = ContextXamlAstVisitor::new(self, TransformerVisitorCore { transformer });
        visit_node(root, &mut visitor)
    }

    pub fn visit_children(
        &self,
        root: &dyn IXamlAstNode,
        transformer: &dyn IXamlAstTransformer,
    ) -> XamlResult<()> {
        let mut visitor = ContextXamlAstVisitor::new(self, TransformerVisitorCore { transformer });
        root.visit_children(&mut visitor)
    }

    /// Returns true when the error was reported as non-fatal and the transformation may go on.
    pub fn on_unhandled_transform_error(&self, exception: &XamlError) -> bool {
        match self.report_diagnostic(exception_to_diagnostic(exception, self), false) {
            Ok(severity) => severity < XamlDiagnosticSeverity::Fatal,
            Err(_) => false,
        }
    }
}

/// The overridable part of the upstream abstract `ContextXamlAstVisitor`.
pub trait IContextXamlAstVisitorCore {
    fn get_transformer_info(&self) -> String;
    fn visit_core(
        &mut self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>>;
}

/// A visitor bound to a transformation context: it maintains the parent stack and converts
/// transformer failures into diagnostics, replacing the failed node with a "skip" node when the
/// diagnostics handler does not treat the error as fatal.
pub struct ContextXamlAstVisitor<'a, C: IContextXamlAstVisitorCore> {
    context: &'a AstTransformationContext,
    core: C,
}

impl<'a, C: IContextXamlAstVisitorCore> ContextXamlAstVisitor<'a, C> {
    pub fn new(context: &'a AstTransformationContext, core: C) -> Self {
        Self { context, core }
    }

    fn exception_handler(
        &self,
        e: XamlError,
        node: &Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let report_exception = if e.is_xml_exception() {
            e
        } else {
            XamlError::transform_exception_with_inner(
                format!(
                    "Internal compiler error: {} ({})",
                    e.message(),
                    self.core.get_transformer_info()
                ),
                Some(&**node),
                e,
            )
            .with_document(self.context.current_document())
        };

        if self.context.on_unhandled_transform_error(&report_exception) {
            Ok(if node.is::<dyn IXamlAstTypeReference>() {
                XamlAstClrTypeReference::new(&**node, XamlPseudoType::unknown(), false)
            } else {
                SkipXamlValueWithManipulationNode::new(&**node)
            })
        } else {
            Err(report_exception)
        }
    }
}

impl<C: IContextXamlAstVisitorCore> IXamlAstVisitor for ContextXamlAstVisitor<'_, C> {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        match self.core.visit_core(self.context, node.clone()) {
            Ok(output_node) => Ok(output_node),
            Err(e) => self.exception_handler(e, &node),
        }
    }

    fn push(&mut self, node: Rc<dyn IXamlAstNode>) {
        self.context.push_parent(node);
    }

    fn pop(&mut self) {
        self.context.pop_parent();
    }
}

struct TransformerVisitorCore<'a> {
    transformer: &'a dyn IXamlAstTransformer,
}

impl IContextXamlAstVisitorCore for TransformerVisitorCore<'_> {
    fn get_transformer_info(&self) -> String {
        self.transformer.transformer_name()
    }

    fn visit_core(
        &mut self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        self.transformer.transform(context, node)
    }
}
