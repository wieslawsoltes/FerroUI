use ferroui_base::FerroObject;
use ferroui_designer_core::{Document, NodeId};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use std::collections::{BTreeMap, BTreeSet};

/// Matches live objects to declarations in one accepted XAML document.
pub struct NativeSourceIndex {
    sources: BTreeSet<String>,
    lines: BTreeMap<usize, Vec<(usize, NodeId)>>,
}

impl NativeSourceIndex {
    pub fn new(document: &Document, root: &FerroObject, document_name: &str) -> Self {
        let sources = source_identities(root, document_name);
        let mut lines = BTreeMap::<usize, Vec<(usize, NodeId)>>::new();
        let mut nodes = document.ast().elements().collect::<Vec<_>>();
        nodes.sort_unstable_by_key(|node| node.span.start);
        let mut previous = 0;
        let (mut line, mut column) = (1, 1);
        for node in nodes {
            for character in document.valid_source()[previous..node.span.start].chars() {
                if character == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
            }
            previous = node.span.start;
            lines.entry(line).or_default().push((column, node.id));
        }
        Self { sources, lines }
    }

    pub fn resolve_object(&self, object: &FerroObject) -> Option<NodeId> {
        self.resolve(&XamlSourceInfo::for_object(object)?, |_| true)
    }

    pub(crate) fn resolve(
        &self,
        info: &XamlSourceInfo,
        eligible: impl Fn(NodeId) -> bool,
    ) -> Option<NodeId> {
        if !self.contains(info) {
            return None;
        }
        let column = usize::try_from(info.line_position()).ok()?;
        let entries = self.lines.get(&usize::try_from(info.line_number()).ok()?)?;
        // The loader can point to '<' or the first character of the name. Reject ties.
        let mut best = None;
        let mut distance = 3;
        for &(expected, id) in entries {
            if !eligible(id) {
                continue;
            }
            let delta = expected.abs_diff(column);
            if delta < distance {
                best = Some(id);
                distance = delta;
            } else if delta == distance {
                best = None;
            }
        }
        best
    }

    fn contains(&self, info: &XamlSourceInfo) -> bool {
        info.source_uri()
            .is_some_and(|uri| self.sources.contains(uri.as_str()))
    }
}

fn source_identities(root: &FerroObject, name: &str) -> BTreeSet<String> {
    if let Some(uri) = XamlSourceInfo::for_object(root).and_then(|info| info.source_uri().cloned())
    {
        return BTreeSet::from([uri.to_string()]);
    }
    // The reload and release loaders can retain different authority spelling.
    let canonical = ferroui_base::utilities::UriRef::parse_absolute(name)
        .map(String::from)
        .unwrap_or_else(|_| name.to_owned());
    [name, canonical.as_str()]
        .into_iter()
        .filter_map(|name| {
            XamlSourceInfo::new(1, 1, Some(name))
                .source_uri()
                .map(ToString::to_string)
        })
        .collect()
}
