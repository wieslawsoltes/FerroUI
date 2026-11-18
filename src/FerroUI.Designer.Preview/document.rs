use crate::protocol::{Counterpart, Language, Source, MAX_SOURCE_BYTES};
use ferroui_designer_core::{Document, NodeId, SourceSpan};
use ferroui_designer_language::{LineIndex, Range, Schema};
use ferroui_designer_rsx::{project_rust, xaml_to_rsx, RsxNode, RsxProjection};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub(crate) struct DesignDocument<'a> {
    pub(crate) xaml: Document,
    pub(crate) source: &'a str,
    pub(crate) rsx: Option<RsxProjection<'a>>,
    pub(crate) index: LineIndex,
    rsx_nodes: BTreeMap<NodeId, usize>,
}

impl<'a> DesignDocument<'a> {
    pub(crate) fn parse(input: &'a Source) -> Result<Self, String> {
        if input.source.len() > MAX_SOURCE_BYTES {
            return Err("Designer source exceeds 1 MiB".into());
        }
        if !input.uri.starts_with("file:") {
            return Err("The designer uses a local file URI for relative resources".into());
        }
        let index = LineIndex::new(&input.source);
        let rsx = match input.language {
            Language::Xaml => None,
            Language::Rust => Some(project_rust(
                &input.source,
                index.offset(input.caret).map_err(|e| e.to_string())?,
            )?),
        };
        let xaml = match &rsx {
            Some(projection) => Document::new(projection.xaml()),
            None => Document::new(input.source.as_str()),
        }
        .map_err(|e| e.to_string())?;
        let mut rsx_nodes = BTreeMap::new();
        for (i, node) in xaml
            .ast()
            .nodes_in_source_order()
            .filter(|node| node.is_element())
            .enumerate()
        {
            if i >= 2000 {
                return Err("The preview supports up to 2000 source elements".into());
            }
            if rsx.is_some() {
                rsx_nodes.insert(node.id, i);
            }
        }
        Ok(Self {
            xaml,
            source: &input.source,
            rsx,
            index,
            rsx_nodes,
        })
    }

    pub(crate) fn range(&self, span: SourceSpan) -> Result<Range, String> {
        self.index.range(span).map_err(|e| e.to_string())
    }

    pub(crate) fn rsx_node(&self, id: NodeId) -> Option<(usize, &RsxNode)> {
        let index = *self.rsx_nodes.get(&id)?;
        Some((index, &self.rsx.as_ref()?.nodes[index]))
    }

    pub(crate) fn editable_structure(&self) -> bool {
        self.rsx
            .as_ref()
            .is_none_or(RsxProjection::can_replace_structure)
    }

    pub(crate) fn editable_children(&self, id: NodeId) -> bool {
        self.rsx.is_none() || self.rsx_node(id).is_some_and(|(_, node)| node.editable_children())
    }

    pub(crate) fn counterpart(&self) -> Counterpart {
        if let Some(rsx) = &self.rsx {
            let complete = rsx.omitted_count == 0;
            return Counterpart {
                source: Some(self.xaml.source().to_owned()),
                editable: complete,
                reason: (!complete).then(|| {
                    "Edit mapped literals in the inspector or change the Rust source".into()
                }),
            };
        }
        match xaml_to_rsx(&self.xaml) {
            Ok(source) => Counterpart {
                source: Some(source),
                editable: true,
                reason: None,
            },
            Err(reason) => Counterpart {
                source: None,
                editable: false,
                reason: Some(reason),
            },
        }
    }
}

pub(crate) fn schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(Schema::ferroui)
}
