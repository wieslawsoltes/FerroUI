//! `{x:Bind}` across a reload of a preserved root (#525, step X7, docs/porting/x-bind.md).
//!
//! A component or panel reload populates a detached candidate and moves its content into the
//! live root. The bindings of the document root of the candidate's markup belong to the live
//! root: the markup is compiled against the live root's code-behind (`ReloadCodeBehind`), and
//! `XBindExtension` registers the bindings with the live root's `XBindings`. Once the content is
//! in place (and again when a rollback puts the previous content back) the live root's bindings
//! are updated, so the values reflect the markup that is now shown.
use ferroui_base::FerroObject;
use ferroui_markup_xaml::markup_extensions::XBindings;

/// Whether `source` has an `{x:Bind}` (or `{XBind}`) attribute value: a reload of it needs the
/// live root as the code-behind of the candidate.
pub(crate) fn uses_x_bind(source: &str) -> bool {
    const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
    let Ok(document) = roxmltree::Document::parse(source) else {
        return false;
    };
    document.descendants().filter(|node| node.is_element()).any(|node| {
        node.attributes().any(|attribute| {
            let Some(extension) = attribute.value().trim_start().strip_prefix('{') else {
                return false;
            };
            let name = extension
                .split(|c: char| c.is_whitespace() || c == '}' || c == ',')
                .next()
                .unwrap_or_default();
            match name.split_once(':') {
                Some((prefix, "Bind")) => node.lookup_namespace_uri(Some(prefix)) == Some(XAML),
                Some(_) => false,
                None => name == "XBind",
            }
        })
    })
}

/// The live root's bindings after its markup was replaced: read again when initialized,
/// initialized when the root is already live (`XBindings::update_populated`). A binding that
/// cannot be read stays a diagnostic of the bindings, as at the attach of the root.
pub(crate) fn refresh(root: &FerroObject) {
    let _ = XBindings::update_populated(root);
}

#[cfg(test)]
mod tests {
    use super::uses_x_bind;

    #[test]
    fn x_bind_is_found_by_its_namespace_not_its_prefix() {
        let ns = r#"xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml""#;
        assert!(uses_x_bind(&format!(r#"<StackPanel {ns}><TextBlock Text="{{x:Bind Tag}}"/></StackPanel>"#)));
        assert!(uses_x_bind(&format!(r#"<StackPanel {ns}><TextBlock Text="{{x:Bind}}"/></StackPanel>"#)));
        assert!(uses_x_bind(&format!(r#"<StackPanel {ns}><TextBlock Text="{{XBind Tag}}"/></StackPanel>"#)));
        assert!(uses_x_bind(
            r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:w="http://schemas.microsoft.com/winfx/2006/xaml"><TextBlock Text="{w:Bind Tag, Mode=OneWay}"/></StackPanel>"#
        ));
        assert!(!uses_x_bind(&format!(r#"<StackPanel {ns}><TextBlock Text="{{Binding Tag}}"/></StackPanel>"#)));
        assert!(!uses_x_bind(&format!(r#"<StackPanel {ns}><TextBlock Text="x:Bind"/></StackPanel>"#)));
        assert!(!uses_x_bind(
            r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="urn:other"><TextBlock Text="{x:Bind Tag}"/></StackPanel>"#
        ));
    }
}
