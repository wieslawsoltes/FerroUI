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
}
