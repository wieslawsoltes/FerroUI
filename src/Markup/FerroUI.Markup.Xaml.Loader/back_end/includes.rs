//! Discover literal include sources from the same pre-transform syntax as the loader.
use crate::compiler_extensions::FerroXamlIlCompiler;
use std::rc::Rc;
use xamlx::{ast::XamlDocument, exceptions::XamlResult};

/// Most documents have no includes. XML names cannot contain character references.
pub(crate) fn may_include(xml: &str) -> bool {
    xml.contains("ResourceInclude") || xml.contains("StyleInclude")
}

/// Include discovery is advisory. A malformed document is diagnosed during its normal compilation.
pub fn include_sources(xml: &str) -> Vec<String> {
    if !may_include(xml) {
        return Vec::new();
    }
    FerroXamlIlCompiler::parse_source_syntax(xml)
        .and_then(|doc| syntax_includes(&doc))
        .unwrap_or_default()
}

pub(crate) fn syntax_includes(document: &XamlDocument) -> XamlResult<Vec<String>> {
    use xamlx::ast::{
        visit_node, IXamlAstNode, IXamlAstVisitor, XamlAstNamePropertyReference,
        XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlAstXamlPropertyValueNode,
        XamlAstXmlTypeReference,
    };
    fn literal_source(object: &XamlAstObjectNode) -> Option<String> {
        let ty = object.type_.borrow().cast::<XamlAstXmlTypeReference>()?;
        if !matches!(
            ty.name().as_str(),
            "MergeResourceInclude" | "ResourceInclude" | "StyleInclude"
        ) {
            return None;
        }
        object.children.borrow().iter().find_map(|child| {
            let property = child.cast::<XamlAstXamlPropertyValueNode>()?;
            let member = property.property().cast::<XamlAstNamePropertyReference>()?;
            if member.name() != "Source"
                || !member
                    .declaring_type
                    .borrow()
                    .equals(&**object.type_.borrow())
            {
                return None;
            }
            let values = property.values.borrow();
            let [value] = values.as_slice() else {
                return None;
            };
            let source = value.cast::<XamlAstTextNode>()?.text();
            (!source.is_empty()).then_some(source)
        })
    }

    struct Includes(Vec<String>);
    impl IXamlAstVisitor for Includes {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
            if let Some(source) = node
                .cast::<XamlAstObjectNode>()
                .and_then(|object| literal_source(&object))
            {
                self.0.push(source);
            }
            Ok(node)
        }
        fn push(&mut self, _: Rc<dyn IXamlAstNode>) {}
        fn pop(&mut self) {}
    }
    let mut includes = Includes(Vec::new());
    visit_node(&document.root()?, &mut includes)?;
    Ok(includes.0)
}

#[cfg(test)]
mod tests {
    use super::include_sources;

    #[test]
    fn include_sources_are_found_in_document_order() {
        let xaml = r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="x" xmlns:local="urn:local">
  <!-- <StyleInclude Source="/Commented.xaml" /> -->
  <Styles.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source="/Accents/Base.xaml" />
        <ResourceInclude x:Key="k" Source='Controls/Button.xaml'/>
        <ResourceInclude Source="{Binding X}" />
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Styles.Resources>
  <StyleInclude
      Source = "ferres://Other/Styles.xaml" />
  <ResourceInclude Source="/A&amp;B.xaml" />
  <StyleInclude><StyleInclude.Source>Child.xaml</StyleInclude.Source></StyleInclude>
  <Border Tag="Source=x" DataSource="y" />
  <local:StyleInclude Source="/Local.xaml" />
</Styles>"#;
        assert_eq!(
            include_sources(xaml),
            [
                "/Accents/Base.xaml",
                "Controls/Button.xaml",
                "ferres://Other/Styles.xaml",
                "/A&B.xaml",
                "Child.xaml",
                "/Local.xaml"
            ]
        );
    }
}
