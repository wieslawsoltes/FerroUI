use crate::protocol::{Counterpart, Language, Source, MAX_SOURCE_BYTES};
use crate::projects::Project;
use ferroui_designer_core::{Document, NodeId, SourceSpan};
use ferroui_designer_language::{LineIndex, Range, Schema};
use ferroui_designer_project::RustModuleSources;
use ferroui_designer_rsx::{RsxAnimation, RsxCatalog, RsxNode, RsxProjection, RsxStateGroup};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

pub(crate) struct DesignDocument<'a> {
    pub(crate) xaml: Document,
    pub(crate) source: &'a str,
    pub(crate) rsx: Option<RsxProjection<'a>>,
    pub(crate) index: LineIndex,
    pub(crate) project: Option<&'a Project>,
    pub(crate) rsx_animations: BTreeMap<NodeId, RsxAnimation>,
    pub(crate) rsx_states: BTreeMap<(NodeId, String), RsxStateGroup>,
    rsx_nodes: BTreeMap<NodeId, usize>,
}

impl<'a> DesignDocument<'a> {
    pub(crate) fn parse(input: &'a Source, project: Option<&'a Project>) -> Result<Self, String> {
        Self::parse_with_rust_modules(input, project, None)
    }

    pub(crate) fn parse_with_rust_modules(
        input: &'a Source,
        project: Option<&'a Project>,
        modules: Option<&'a RustModuleSources>,
    ) -> Result<Self, String> {
        if input.source.len() > MAX_SOURCE_BYTES {
            return Err("Designer source exceeds 1 MiB".into());
        }
        if !input.uri.starts_with("file:") {
            return Err("The designer uses a local file URI for relative resources".into());
        }
        let index = LineIndex::new(&input.source);
        let rsx = match input.language {
            Language::Xaml => None,
            Language::Rust => Some(rsx_catalog(project).project_with_modules(
                &input.source,
                index.offset(input.caret).map_err(|e| e.to_string())?,
                modules,
            )?),
        };
        let mut markup = rsx.as_ref().map(RsxProjection::preview_markup);
        let xaml = match &mut markup {
            Some(markup) => Document::new(std::mem::take(&mut markup.source)),
            None => Document::new(input.source.as_str()),
        }.map_err(|e| e.to_string())?;
        let mut rsx_nodes = BTreeMap::new();
        let mut rsx_animations = BTreeMap::new();
        let mut rsx_states = BTreeMap::new();
        for (i, node) in xaml
            .ast()
            .nodes_in_source_order()
            .filter(|node| node.is_element())
            .enumerate()
        {
            if i >= 2000 {
                return Err("The preview supports up to 2000 source elements".into());
            }
            if let Some(markup) = &markup {
                let offset = node.span.start;
                if let Ok(index) = markup.controls.binary_search(&offset) {
                    rsx_nodes.insert(node.id, index);
                }
                if let Ok(index) = markup.animations.binary_search_by_key(&offset, |(start, _)| *start) {
                    rsx_animations.insert(node.id, markup.animations[index].1.clone());
                }
                let first = markup.state_groups.partition_point(|(start, _)| *start < offset);
                for (_, group) in markup.state_groups[first..].iter().take_while(|(start, _)| *start == offset) {
                    rsx_states.insert((node.id, group.name().to_owned()), (*group).clone());
                }
            }
        }
        Ok(Self {
            xaml,
            source: &input.source,
            rsx,
            index,
            project,
            rsx_nodes,
            rsx_animations,
            rsx_states,
        })
    }

    pub(crate) fn range(&self, span: SourceSpan) -> Result<Range, String> {
        self.index.range(span).map_err(|e| e.to_string())
    }

    pub(crate) fn node_name_range(&self, id: NodeId) -> Result<Range, String> {
        let span = if self.rsx.is_some() {
            self.rsx_node(id).ok_or("This native control has no Rust source mapping")?.1.name
        } else {
            self.xaml.ast().node(id).ok_or("The source selection changed")?.name_span
        };
        self.range(span)
    }

    pub(crate) fn schema(&self) -> &Schema {
        match self.project { Some(project) => &project.schema, None => schema() }
    }

    pub(crate) fn rsx_catalog(&self) -> &RsxCatalog { rsx_catalog(self.project) }

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
            let animated = rsx.has_projected_animations();
            let styled = rsx.has_projected_styles();
            let states = rsx.has_projected_states();
            let columns = rsx.has_projected_columns();
            let complete = rsx.omitted_count == 0;
            let reason = if columns {
                Some("Edit authored Rust column collections in Columns")
            } else if states {
                Some("Edit mapped Rust state declarations in States")
            } else if animated {
                Some("Edit mapped animation keys in Timeline")
            } else if styled {
                Some("Edit mapped Rust styles in Styles & Themes")
            } else if !complete {
                Some("Edit mapped literals in Properties or change the Rust source")
            } else if !rsx.can_replace_structure() {
                Some("Edit mapped properties here. Use the canvas or Outline to arrange controls.")
            } else {
                None
            };
            return Counterpart {
                source: Some(self.xaml.source().to_owned()),
                editable: complete && !styled && !states && !columns,
                reason: reason.map(str::to_owned),
            };
        }
        match self.rsx_catalog().xaml_to_rsx(&self.xaml) {
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

fn rsx_catalog(project: Option<&Project>) -> &RsxCatalog {
    match project { Some(project) => &project.rsx, None => RsxCatalog::built_in() }
}

pub(crate) fn schema() -> &'static Schema {
    shared_schema()
}

pub(crate) fn shared_schema() -> &'static Arc<Schema> {
    static SCHEMA: OnceLock<Arc<Schema>> = OnceLock::new();
    SCHEMA.get_or_init(|| Arc::new(Schema::ferroui()))
}
