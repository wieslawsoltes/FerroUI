//! Port of `CompilerExtensions/Transformers/FerroBindingExtensionTransformer.cs`.

use std::cell::Cell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlAstVisitor, IXamlLineInfo,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlAstXmlDirective,
    XamlAstXmlTypeReference, XamlValueWithSideEffectNodeBase,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl, XamlNamespaces};

use super::FERRO_XML_NAMESPACE;

/// `XamlBindingsTransformException : XamlTransformException`.
///
/// Errors are values of the single [`XamlError`] type; the derived class is represented by a
/// `XamlError::Transform` tagged with the class name, which is what
/// `FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro` (code `FRN2100`) and
/// [`Self::is`] look at.
pub struct XamlBindingsTransformException;

impl XamlBindingsTransformException {
    pub const TYPE_NAME: &'static str = "XamlBindingsTransformException";

    /// `new XamlBindingsTransformException(message, lineInfo, innerException)`.
    pub fn new(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner_exception: Option<XamlError>,
    ) -> XamlError {
        let error = match inner_exception {
            Some(inner) => {
                XamlError::transform_exception_with_inner(message, Some(line_info), inner)
            }
            None => XamlError::transform_exception(message, Some(line_info)),
        };
        error.with_derived_type_name(Self::TYPE_NAME)
    }

    /// `e is XamlBindingsTransformException`.
    pub fn is(error: &XamlError) -> bool {
        error.is_derived_type(Self::TYPE_NAME)
    }
}

/// Handles the `x:CompileBindings` directive and turns the `Binding` type reference into
/// `CompiledBinding` or `ReflectionBinding`.
#[derive(Default)]
pub struct FerroBindingExtensionTransformer {
    /// `CompileBindingsByDefault { get; set; }`.
    pub compile_bindings_by_default: Cell<bool>,
}

impl FerroBindingExtensionTransformer {
    pub fn new() -> Self {
        Self::default()
    }
}

/// `bool.TryParse`: surrounding white space (and NUL characters) is ignored, the comparison
/// with `True`/`False` is case-insensitive.
fn try_parse_boolean(text: &str) -> Option<bool> {
    let trimmed = text.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    if trimmed.eq_ignore_ascii_case("true") {
        Some(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

impl IXamlAstTransformer for FerroBindingExtensionTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if context
            .first_parent_node()
            .is_some_and(|p| p.is::<FerroXamlIlCompileBindingsNode>())
        {
            return Ok(node);
        }

        if let Some(obj) = node.cast::<XamlAstObjectNode>() {
            let children = obj.children.borrow().clone();
            for item in children {
                let Some(directive) = item.cast::<XamlAstXmlDirective>() else {
                    continue;
                };
                let values = directive.values.borrow().clone();
                if directive.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                    && *directive.name.borrow() == "CompileBindings"
                    && values.len() == 1
                {
                    let compile_bindings = values[0]
                        .cast::<XamlAstTextNode>()
                        .and_then(|text| try_parse_boolean(&text.text()));
                    let Some(compile_bindings) = compile_bindings else {
                        return Err(XamlBindingsTransformException::new(
                            "The value of x:CompileBindings must be a literal boolean value.",
                            &*values[0],
                            None,
                        ));
                    };

                    {
                        let mut children = obj.children.borrow_mut();
                        if let Some(index) = children.iter().position(|c| c.same_node(&directive)) {
                            children.remove(index);
                        }
                    }

                    return Ok(FerroXamlIlCompileBindingsNode::new(obj, compile_bindings));
                }
            }
        }

        // Convert the <Binding> tag to either a CompiledBinding or ReflectionBinding tag.

        if let Some(tref) = node.cast::<XamlAstXmlTypeReference>() {
            if tref.name() == "Binding"
                && tref.xml_namespace().as_deref() == Some(FERRO_XML_NAMESPACE)
            {
                tref.is_markup_extension.set(true);

                let compile_bindings = context
                    .parent_nodes()
                    .into_iter()
                    .find_map(|n| n.cast::<FerroXamlIlCompileBindingsNode>())
                    .map(|n| n.compile_bindings)
                    .unwrap_or_else(|| self.compile_bindings_by_default.get());

                *tref.name.borrow_mut() = if compile_bindings {
                    "CompiledBinding".to_string()
                } else {
                    "ReflectionBinding".to_string()
                };
            }
        }

        Ok(node)
    }
}

/// Scope marker: bindings below [`FerroXamlIlCompileBindingsNode::value`] are compiled
/// (`x:CompileBindings="True"`) or reflection bindings (`"False"`). The node is removed again
/// by `FerroXamlIlCompiledBindingsMetadataRemover`; it never reaches a back end.
pub struct FerroXamlIlCompileBindingsNode {
    pub base: XamlValueWithSideEffectNodeBase,
    pub compile_bindings: bool,
}

impl FerroXamlIlCompileBindingsNode {
    pub fn new(value: Rc<dyn IXamlAstValueNode>, compile_bindings: bool) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(&*value.clone(), value),
            compile_bindings,
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.base.value()
    }
}

xaml_line_info_impl!(FerroXamlIlCompileBindingsNode, base);

impl IXamlAstNode for FerroXamlIlCompileBindingsNode {
    xaml_ast_node_members!("FerroXamlIlCompileBindingsNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for FerroXamlIlCompileBindingsNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}
