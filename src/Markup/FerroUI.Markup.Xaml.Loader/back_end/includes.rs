//! The includes of a document: what the run-time loader and a host of the emitter collect the
//! group of a document with, before the document is compiled.

use xamlx::parsers::XDocumentXamlParser;

/// The element names of the includes the group transformers link.
const INCLUDE_ELEMENTS: [&str; 3] = ["MergeResourceInclude", "ResourceInclude", "StyleInclude"];

/// The `Source` values of the include elements of a document, in document order (#850).
///
/// The document is read by the XML parser of the compiler, with its rules
/// (`XDocumentXamlParser::xml_text`: a document type declaration is ignored): an include is an
/// element whose local name is one of [`INCLUDE_ELEMENTS`], whatever its prefix, and its source
/// is the value of its `Source` attribute without a namespace, with the entities decoded. Text,
/// CDATA and comments are not elements. A source given by a markup extension (`{..}`) is left to
/// the run time. The build (`rust_emitter::class_document_group`) and the run-time loader collect
/// a group with this function, so both find the same documents.
///
/// A document that is not well formed has no includes: it fails when it is compiled or loaded,
/// with the diagnostic of the parser, which a guess at its includes would not change.
pub fn include_sources(xaml: &str) -> Vec<String> {
    let Ok(text) = XDocumentXamlParser::xml_text(xaml) else { return Vec::new() };
    let Ok(document) = roxmltree::Document::parse(&text) else { return Vec::new() };
    document
        .descendants()
        .filter(|node| node.is_element() && INCLUDE_ELEMENTS.contains(&node.tag_name().name()))
        .filter_map(|node| node.attribute("Source"))
        .filter(|source| !source.is_empty() && !source.starts_with('{'))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::include_sources;

    #[test]
    fn include_sources_are_found_in_document_order() {
        let xaml = r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="x" xmlns:local="using:Local">
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
  <Border Tag="Source=x" DataSource="y" />
  <local:StyleInclude Source="/Local.xaml" />
</Styles>"#;
        assert_eq!(
            include_sources(xaml),
            ["/Accents/Base.xaml", "Controls/Button.xaml", "ferres://Other/Styles.xaml", "/Local.xaml"]
        );
    }

    /// Not from upstream (#850): a `Source` is the value of the attribute, with its entities
    /// decoded, as the parser of the document reads it.
    #[test]
    fn an_entity_in_a_source_is_decoded() {
        let xaml = "<Styles xmlns='https://github.com/ferroui'>\
                    <StyleInclude Source='/A&amp;B.xaml'/><StyleInclude Source='/C&#x44;.xaml'/></Styles>";
        assert_eq!(include_sources(xaml), ["/A&B.xaml", "/CD.xaml"]);
    }

    /// Not from upstream (#850): a `>` in the value of an attribute does not end the tag of an
    /// include.
    #[test]
    fn a_greater_than_sign_in_an_attribute_does_not_end_the_tag() {
        let xaml = "<ResourceDictionary xmlns='https://github.com/ferroui' \
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\
                    <ResourceInclude x:Key='a>b' Source='/Keyed.xaml'/></ResourceDictionary>";
        assert_eq!(include_sources(xaml), ["/Keyed.xaml"]);
    }

    /// Not from upstream (#850): what CDATA and comments hold is text, not an include.
    #[test]
    fn an_include_in_cdata_or_a_comment_is_not_an_include() {
        let xaml = "<Styles xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\
                    <Styles.Resources><x:String x:Key='Text'><![CDATA[<StyleInclude Source='/Text.xaml'/>]]></x:String>\
                    </Styles.Resources><!-- <StyleInclude Source='/Comment.xaml'/> -->\
                    <StyleInclude Source='/Real.xaml'/></Styles>";
        assert_eq!(include_sources(xaml), ["/Real.xaml"]);
    }

    /// Not from upstream (#850): a document that is not well formed has no includes; it fails when
    /// it is loaded, with the diagnostic of its parser.
    #[test]
    fn a_document_that_is_not_well_formed_has_no_includes() {
        let xaml = "<Styles xmlns='https://github.com/ferroui'><StyleInclude Source='/A.xaml'/><Border></Styles>";
        assert_eq!(include_sources(xaml), Vec::<String>::new());
    }

    /// Not from upstream (#850): a document type declaration is ignored, as the parser of the
    /// document ignores it.
    #[test]
    fn a_document_type_declaration_is_ignored() {
        let xaml = "<!DOCTYPE Styles>\
                    <Styles xmlns='https://github.com/ferroui'><StyleInclude Source='/A.xaml'/></Styles>";
        assert_eq!(include_sources(xaml), ["/A.xaml"]);
    }
}
