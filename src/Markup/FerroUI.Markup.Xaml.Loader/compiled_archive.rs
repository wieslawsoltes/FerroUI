//! Incremental linking of portable archives against a precompiled class group.
//! An edit to an inlined dictionary needs a complete group transform. Changes
//! confined to isolated dictionary build methods can use their original callers.
use crate::{
    binary_document::{self, pack},
    runtime::framework::load_exception,
    runtime_document_input::DocumentInput,
    FerroRuntimeXamlLoader, PreparedPackedDocument,
};
use ferroui_base::{
    controls::ResourceDictionary,
    metadata::{from_markup_value, IServiceProvider},
    utilities::Uri,
    BoxedValue, Ref,
};
use ferroui_markup_xaml::{
    archive_includes, CompiledArchiveRoot, RuntimeXamlLoaderConfiguration,
    RuntimeXamlLoaderDocument, XamlLoadError,
};
use std::rc::Rc;
use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstVisitor, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstXmlDirective, XamlAstXmlTypeReference, XamlDocument,
};
use xamlx::exceptions::XamlResult;

fn standalone_dictionary(syntax: &XamlDocument) -> XamlResult<bool> {
    let Some(root) = syntax.root()?.cast::<XamlAstObjectNode>() else {
        return Ok(false);
    };
    let Some(ty) = root.type_.borrow().cast::<XamlAstXmlTypeReference>() else {
        return Ok(false);
    };
    if ty.name() != "ResourceDictionary"
        || ty.xml_namespace.borrow().as_deref() != Some("https://github.com/ferroui")
        || !ty.generic_arguments.borrow().is_empty()
        || ty.is_markup_extension.get()
    {
        return Ok(false);
    }

    struct Boundary(bool);
    impl IXamlAstVisitor for Boundary {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
            // Unknown/dynamic include sources must never escape the closed revision.
            if let Some(ty) = node.cast::<XamlAstXmlTypeReference>() {
                if matches!(
                    ty.name().as_str(),
                    "ResourceInclude" | "StyleInclude" | "MergeResourceInclude"
                ) {
                    self.0 = false;
                }
            }
            if let Some(d) = node.cast::<XamlAstXmlDirective>() {
                if d.namespace.borrow().as_deref()
                    == Some(xamlx::xaml_namespaces::XamlNamespaces::XAML2006)
                    && d.name.borrow().as_str() == "Class"
                {
                    self.0 = false;
                }
            }
            Ok(node)
        }
        fn push(&mut self, _: Rc<dyn IXamlAstNode>) {}
        fn pop(&mut self) {}
    }
    let mut boundary = Boundary(true);
    visit_node(&syntax.root()?, &mut boundary)?;
    Ok(boundary.0)
}

impl FerroRuntimeXamlLoader {
    /// Try the installed program, retaining portable interpretation at changed,
    /// isolated ResourceInclude sites. A miss requires normal whole-group loading.
    pub fn try_load_compiled_archive(
        root: &CompiledArchiveRoot,
        image: &[u8],
        uri: &Uri,
        configuration: &RuntimeXamlLoaderConfiguration,
        parent: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<Option<BoxedValue>, XamlLoadError> {
        if image == root.bytes {
            return root.try_load(image, uri, configuration, parent);
        }
        if !archive_includes::ENABLED
            || root.dictionaries.is_empty()
            || !root.accepts(uri, configuration)
        {
            return Ok(None);
        }
        let Some(changed) = pack::changed_documents(root.bytes, image).map_err(load_exception)?
        else {
            return Ok(None);
        };
        let mut includes = archive_includes::Includes::new();
        for (path, _, bytes) in changed {
            let Some(&(_, uri)) = root.dictionaries.iter().find(|(name, _)| *name == path) else {
                return Ok(None);
            };
            let syntax = binary_document::decode(&bytes).map_err(load_exception)?;
            if !standalone_dictionary(&syntax).map_err(load_exception)? {
                return Ok(None);
            }
            let base =
                Uri::absolute(uri).map_err(|e| XamlLoadError::with_message(e.to_string()))?;
            let mut document = RuntimeXamlLoaderDocument::from_stream_with_base_uri(
                Some(base),
                Box::new(std::io::empty()),
            );
            document.document = Some(path);
            let plan = PreparedPackedDocument::prepare(
                vec![(document, DocumentInput::from_decoded(bytes, syntax))],
                configuration,
            )
            .map_err(load_exception)?;
            includes.insert(
                uri,
                Rc::new(move |provider| {
                    let value = plan.build_at(provider)?;
                    from_markup_value::<Ref<ResourceDictionary>>(&Some(value)).ok_or_else(|| {
                        XamlLoadError::with_message(
                            "Expected a resource dictionary at the compiled include",
                        )
                    })
                }),
            );
        }
        archive_includes::with_revision(root.class, includes, || (root.load)(parent, root.uri))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_closed_untyped_dictionary_roots_are_incrementally_linked() {
        let parse = |xml: &str| {
            crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(xml).unwrap()
        };
        let ns = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
        let accepts = |xml: String| standalone_dictionary(&parse(&xml)).unwrap();
        assert!(accepts(format!(
            "<ResourceDictionary {ns}><x:Double x:Key='A'>9</x:Double></ResourceDictionary>"
        )));
        assert!(!accepts(format!("<Styles {ns}/>")));
        assert!(!accepts(format!(
            "<ResourceDictionary {ns} x:Class='Other'/>"
        )));
        assert!(!accepts("<ResourceDictionary xmlns='urn:user'/>".into()));
        for include in ["ResourceInclude", "MergeResourceInclude", "StyleInclude"] {
            assert!(!accepts(format!(
                "<ResourceDictionary {ns}><{include} Source='Other.xaml'/></ResourceDictionary>"
            )));
            assert!(!accepts(format!("<ResourceDictionary {ns}><{include} Source='{{Binding Path}}'/></ResourceDictionary>")));
        }
    }
}
