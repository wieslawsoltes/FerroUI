//! The includes of a document as its text states them: what the run-time loader and a
//! host of the emitter collect the group of a document with, before anything is parsed.

/// The element names of the includes the group transformers link.
const INCLUDE_ELEMENTS: [&str; 3] = ["MergeResourceInclude", "ResourceInclude", "StyleInclude"];

/// The `Source` values of the include elements of a document, in document order. The
/// text is scanned without parsing it: a document that is not well formed fails when it
/// is loaded, with the position of the error. Sources given by a markup extension are
/// left to the run time.
pub fn include_sources(xaml: &str) -> Vec<String> {
    let mut sources = Vec::new();
    let mut rest = xaml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        if let Some(comment) = rest.strip_prefix("!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            continue;
        }
        let name_end = rest.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(rest.len());
        let name = &rest[..name_end];
        let local_name = name.rsplit(':').next().unwrap_or(name);
        if !INCLUDE_ELEMENTS.contains(&local_name) {
            continue;
        }
        let tag = &rest[name_end..rest[name_end..].find('>').map_or(rest.len(), |end| name_end + end)];
        let mut attributes = tag;
        while let Some(at) = attributes.find("Source") {
            let before_is_boundary = attributes[..at].chars().next_back().is_none_or(char::is_whitespace);
            let after = attributes[at + "Source".len()..].trim_start();
            attributes = &attributes[at + "Source".len()..];
            let Some(value) = after.strip_prefix('=') else { continue };
            if !before_is_boundary {
                continue;
            }
            let value = value.trim_start();
            let Some(quote) = value.chars().next().filter(|c| *c == '"' || *c == '\'') else { continue };
            if let Some(end) = value[1..].find(quote) {
                let source = &value[1..1 + end];
                if !source.is_empty() && !source.starts_with('{') {
                    sources.push(source.to_string());
                }
            }
            break;
        }
    }
    sources
}

#[cfg(test)]
mod tests {
    use super::include_sources;

    #[test]
    fn include_sources_are_found_in_document_order() {
        let xaml = r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="x">
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
