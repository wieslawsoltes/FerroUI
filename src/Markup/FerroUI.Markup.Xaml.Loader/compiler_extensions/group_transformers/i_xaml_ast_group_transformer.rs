//! Port of `CompilerExtensions/GroupTransformers/IXamlAstGroupTransformer.cs`.

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use xamlx::ast::{visit_node, IXamlAstNode};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{
    AstTransformationContext, ContextXamlAstVisitor, IContextXamlAstVisitorCore,
    TransformerConfiguration,
};

use crate::compiler_extensions::IXamlDocumentResource;

/// `AstGroupTransformationContext : AstTransformationContext`.
///
/// The base class members are reachable through `Deref`. The overridden `Document` property is
/// [`AstGroupTransformationContext::document`]; the base context is kept in sync through its
/// document override, so diagnostics reported through either type name the current document.
pub struct AstGroupTransformationContext {
    base: AstTransformationContext,
    current_document: RefCell<Option<Rc<dyn IXamlDocumentResource>>>,
    documents: Vec<Rc<dyn IXamlDocumentResource>>,
}

impl Deref for AstGroupTransformationContext {
    type Target = AstTransformationContext;

    fn deref(&self) -> &AstTransformationContext {
        &self.base
    }
}

impl AstGroupTransformationContext {
    pub fn new(
        documents: Vec<Rc<dyn IXamlDocumentResource>>,
        configuration: Rc<TransformerConfiguration>,
    ) -> Self {
        let rv = Self {
            base: AstTransformationContext::new(configuration, None),
            current_document: RefCell::new(None),
            documents,
        };
        rv.base.set_document_override(Some(Some(rv.document())));
        rv
    }

    /// The base class instance, for APIs that take an `AstTransformationContext`.
    pub fn as_transformation_context(&self) -> &AstTransformationContext {
        &self.base
    }

    /// `Document`: the file path of the current document.
    pub fn document(&self) -> String {
        self.current_document
            .borrow()
            .as_ref()
            .and_then(|d| d.file_source())
            .map(|f| f.file_path())
            .unwrap_or_else(|| "{unknown document}".to_string())
    }

    pub fn current_document(&self) -> Option<Rc<dyn IXamlDocumentResource>> {
        self.current_document.borrow().clone()
    }

    pub fn set_current_document(&self, value: Option<Rc<dyn IXamlDocumentResource>>) {
        *self.current_document.borrow_mut() = value;
        self.base.set_document_override(Some(Some(self.document())));
    }

    pub fn documents(&self) -> &[Rc<dyn IXamlDocumentResource>] {
        &self.documents
    }

    /// `Visit(root, transformer)`. Named `visit_group` because the base class `visit` (taking an
    /// `IXamlAstTransformer`) stays reachable through `Deref`.
    pub fn visit_group(
        &self,
        root: &Rc<dyn IXamlAstNode>,
        transformer: &dyn IXamlAstGroupTransformer,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let mut visitor = ContextXamlAstVisitor::new(
            &self.base,
            Visitor {
                context: self,
                transformer,
            },
        );
        visit_node(root, &mut visitor)
    }

    /// `VisitChildren(root, transformer)`.
    pub fn visit_children_group(
        &self,
        root: &dyn IXamlAstNode,
        transformer: &dyn IXamlAstGroupTransformer,
    ) -> XamlResult<()> {
        let mut visitor = ContextXamlAstVisitor::new(
            &self.base,
            Visitor {
                context: self,
                transformer,
            },
        );
        root.visit_children(&mut visitor)
    }
}

struct Visitor<'a> {
    context: &'a AstGroupTransformationContext,
    transformer: &'a dyn IXamlAstGroupTransformer,
}

impl IContextXamlAstVisitorCore for Visitor<'_> {
    fn get_transformer_info(&self) -> String {
        self.transformer.transformer_name()
    }

    fn visit_core(
        &mut self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        // Upstream casts the context it is handed back to the group context it was created with.
        self.transformer.transform(self.context, node)
    }
}

pub trait IXamlAstGroupTransformer {
    fn transform(
        &self,
        context: &AstGroupTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>>;

    /// `GetType().Name`, used in "internal compiler error" messages.
    fn transformer_name(&self) -> String {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use xamlx::ast::{XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlDocument};
    use xamlx::exceptions::XamlError;
    use xamlx::type_system::{IFileSource, IXamlMethod};

    use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypesExtensions;
    use crate::compiler_extensions::{
        IXamlDocumentTypeBuilderProvider, XamlDocumentResource, XamlDocumentUsage,
    };
    use crate::testing::create_test_framework;

    struct TestFile(&'static str);

    impl IFileSource for TestFile {
        fn file_path(&self) -> String {
            self.0.to_string()
        }
        fn file_contents(&self) -> Vec<u8> {
            Vec::new()
        }
    }

    struct TestProvider {
        populate: Rc<dyn IXamlMethod>,
        build: Option<Rc<dyn IXamlMethod>>,
    }

    impl IXamlDocumentTypeBuilderProvider for TestProvider {
        fn populate_method(&self) -> Rc<dyn IXamlMethod> {
            self.populate.clone()
        }
        fn build_method(&self) -> Option<Rc<dyn IXamlMethod>> {
            self.build.clone()
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn document(
        fw: &crate::testing::TestFramework,
        path: Option<&'static str>,
        created: Rc<RefCell<i32>>,
    ) -> Rc<XamlDocumentResource> {
        let populate = fw.types.i_name_scope_complete.clone();
        let build = fw.types.get_class_property.clone();
        XamlDocumentResource::new(
            Rc::new(RefCell::new(XamlDocument::new())),
            path.map(|p| format!("resm:{p}")),
            path.map(|p| Rc::new(TestFile(p)) as Rc<dyn IFileSource>),
            Some(fw.types.control.clone()),
            true,
            Box::new(move || {
                *created.borrow_mut() += 1;
                Ok(Rc::new(TestProvider {
                    populate: populate.clone(),
                    build: Some(build.clone()),
                }))
            }),
        )
    }

    #[test]
    fn document_resource_creates_its_type_builder_provider_once_and_becomes_used() {
        let fw = create_test_framework();
        let created = Rc::new(RefCell::new(0));
        let resource = document(&fw, Some("/app/Main.xaml"), created.clone());
        assert_eq!(resource.usage(), XamlDocumentUsage::Unknown);
        assert!(resource.is_public());
        assert_eq!(resource.uri().as_deref(), Some("resm:/app/Main.xaml"));
        assert_eq!(resource.class_type().map(|t| t.name()).as_deref(), Some("Control"));
        assert_eq!(*created.borrow(), 0);

        resource.set_usage(XamlDocumentUsage::Merged);
        assert_eq!(resource.usage(), XamlDocumentUsage::Merged);

        assert_eq!(resource.populate_method().expect("populate").name(), "Complete");
        assert_eq!(resource.usage(), XamlDocumentUsage::Used);
        assert_eq!(
            resource.build_method().expect("build").map(|m| m.name()).as_deref(),
            Some("GetClassProperty")
        );
        assert_eq!(*created.borrow(), 1);

        // A failing factory fails the accessors and leaves the usage alone.
        let failing = XamlDocumentResource::new(
            Rc::new(RefCell::new(XamlDocument::new())),
            None,
            None,
            None,
            false,
            Box::new(|| Err(XamlError::invalid_operation("no output type"))),
        );
        assert!(failing.populate_method().is_err());
        assert_eq!(failing.usage(), XamlDocumentUsage::Unknown);
    }

    #[test]
    fn document_is_the_file_of_the_current_document() {
        let fw = create_test_framework();
        let created = Rc::new(RefCell::new(0));
        let main: Rc<dyn IXamlDocumentResource> = document(&fw, Some("/app/Main.xaml"), created.clone());
        let anonymous: Rc<dyn IXamlDocumentResource> = document(&fw, None, created);
        let context = AstGroupTransformationContext::new(
            vec![main.clone(), anonymous.clone()],
            fw.configuration.clone(),
        );
        assert_eq!(context.documents().len(), 2);
        assert!(context.current_document().is_none());
        assert_eq!(context.document(), "{unknown document}");
        assert_eq!(context.as_transformation_context().current_document(), Some("{unknown document}".to_string()));

        context.set_current_document(Some(main));
        assert_eq!(context.document(), "/app/Main.xaml");
        assert_eq!(context.as_transformation_context().current_document(), Some("/app/Main.xaml".to_string()));

        context.set_current_document(Some(anonymous));
        assert_eq!(context.document(), "{unknown document}");

        // The well-known types are reachable from the group context.
        assert!(Rc::ptr_eq(&context.get_ferro_types(), &fw.types));
    }

    struct Renamer;

    impl IXamlAstGroupTransformer for Renamer {
        fn transform(
            &self,
            context: &AstGroupTransformationContext,
            node: Rc<dyn IXamlAstNode>,
        ) -> XamlResult<Rc<dyn IXamlAstNode>> {
            if let Some(text) = node.cast::<XamlAstTextNode>() {
                if text.text() == "fail" {
                    return Err(XamlError::invalid_operation("boom"));
                }
                *text.text.borrow_mut() = format!("{}@{}", text.text(), context.document());
            }
            Ok(node)
        }
    }

    #[test]
    fn visits_with_a_group_transformer_and_reports_failures_against_the_current_document() {
        let fw = create_test_framework();
        let created = Rc::new(RefCell::new(0));
        let main: Rc<dyn IXamlDocumentResource> = document(&fw, Some("/app/Main.xaml"), created);
        let context = AstGroupTransformationContext::new(vec![main.clone()], fw.configuration.clone());
        context.set_current_document(Some(main));

        let line_info = xamlx::ast::XamlLineInfo::new(7, 3);
        let string = fw.t("System.String");
        let root = XamlAstObjectNode::new(
            &line_info,
            xamlx::ast::XamlAstClrTypeReference::new(&line_info, fw.types.control.clone(), false),
        );
        let ok = XamlAstTextNode::with_type(&line_info, "ok", true, Some(string.clone()));
        let failing = XamlAstTextNode::with_type(&line_info, "fail", true, Some(string));
        root.arguments.borrow_mut().push(ok.clone());
        root.arguments.borrow_mut().push(failing);

        let root_node: Rc<dyn IXamlAstNode> = root.clone();
        let visited = context.visit_group(&root_node, &Renamer).expect("non-fatal errors");
        assert!(visited.same_node(&root_node));
        assert_eq!(ok.text(), "ok@/app/Main.xaml");
        // The failed node was replaced by a skip node and the failure reported.
        assert!(root.arguments.borrow()[1].is_skipped());
        let diagnostics = fw.reported_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].document.as_deref(), Some("/app/Main.xaml"));
        assert_eq!(diagnostics[0].title, "Internal compiler error: boom (Renamer) Line 7, position 3.");
        assert_eq!(diagnostics[0].code, "FRN2000");

        // visit_children_group does not transform the root itself.
        let text_root: Rc<dyn IXamlAstNode> = ok;
        context
            .visit_children_group(&*text_root, &Renamer)
            .expect("visited");
        assert_eq!(
            text_root.cast::<XamlAstTextNode>().expect("text").text(),
            "ok@/app/Main.xaml"
        );
    }
}
