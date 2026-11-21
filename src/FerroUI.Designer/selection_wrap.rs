//! Native layout measurement and one-step placement for Studio and editor hosts.
use crate::workspace_tools::{is_custom_panel, measure, parent, Measurement};
use crate::DesignerSession;
use ferroui_designer_core::{Edit, NodeId, SourceSpan, TextPatch};
use ferroui_designer_rsx::{RsxProjection, RsxWrap};
use ferroui_designer_tools::{plan_selection_wrap, SelectionWrap, WrapKind};

pub(crate) struct WrapContext<'a> {
    session: &'a DesignerSession,
    parent: NodeId,
    measurement: Measurement,
    rsx: Option<RsxProjection<'a>>,
}

pub(crate) struct PreparedWrap {
    pub(crate) layout: SelectionWrap,
    pub(crate) patches: Vec<TextPatch>,
    pub(crate) wrapper: SourceSpan,
}

impl<'a> WrapContext<'a> {
    pub(crate) fn new(
        session: &'a DesignerSession,
        projection: Option<RsxProjection<'a>>,
    ) -> Result<Self, String> {
        if !session.can_manipulate() {
            return Err(
                "Wait for the current editable native preview before wrapping controls".into(),
            );
        }
        let selection = session.selection();
        let &first = selection
            .first()
            .ok_or("Select one or more authored controls to wrap")?;
        let owner = parent(session, first)?;
        session.ensure_layer_manipulable(owner)?;
        for &node in selection {
            session.ensure_layer_manipulable(node)?;
            if parent(session, node)? != owner || session.preview.visual(node).is_none() {
                return Err("Select consecutive authored siblings in one native layout".into());
            }
        }
        if is_custom_panel(session, owner)
            || session
                .runtime_layout_capabilities(owner)
                .is_none_or(|capabilities| capabilities.generated_items || !capabilities.drop_items)
        {
            return Err("This native container has no editable authored child slot".into());
        }
        let rsx = match projection {
            Some(projection) => Some(projection),
            None => session
                .rust
                .as_ref()
                .map(|source| source.projection())
                .transpose()?,
        };
        Ok(Self {
            session,
            parent: owner,
            measurement: measure(session, owner, None)?,
            rsx,
        })
    }

    pub(crate) fn layout(&self, kind: WrapKind) -> Result<SelectionWrap, String> {
        plan_selection_wrap(
            &self.measurement.context(self.session.document()),
            self.parent,
            self.session.selection(),
            kind,
        )
        .map_err(|error| error.to_string())
    }

    pub(crate) fn prepare(&self, kind: WrapKind) -> Result<PreparedWrap, String> {
        let document = self.session.document();
        let layout = self.layout(kind)?;
        let (patches, wrapper) = if let Some(rsx) = &self.rsx {
            let mapping = rsx.preview_markup();
            let ordinal = |node| {
                let start = document
                    .ast()
                    .node(node)
                    .ok_or("The wrapped source element changed")?
                    .span
                    .start;
                mapping
                    .controls
                    .binary_search(&start)
                    .map_err(|_| "A selected control has no RSX source mapping")
            };
            let children = layout
                .children
                .iter()
                .map(|&node| ordinal(node))
                .collect::<Result<Vec<_>, _>>()?;
            let attributes = layout
                .child_edits()
                .iter()
                .map(|edit| match edit {
                    Edit::SetAttribute { node, name, value } => {
                        Ok((ordinal(*node)?, name.as_str(), Some(value.as_str())))
                    }
                    Edit::RemoveAttribute { node, name } => {
                        Ok((ordinal(*node)?, name.as_str(), None))
                    }
                    _ => Err("The layout plan has an unsupported Rust edit"),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let planned = rsx.wrap_children(RsxWrap {
                parent: ordinal(layout.parent)?,
                children: &children,
                fragment: &layout.wrapper_fragment(),
                guarded_properties: &layout.guarded_properties,
                attributes: &attributes,
            })?;
            (planned.patches, planned.wrapper)
        } else {
            let patches = layout
                .xaml_patches(document)
                .map_err(|error| error.to_string())?;
            let mut candidate = document.snapshot_without_history();
            candidate
                .apply_text_patches(document.revision(), kind.label(), patches.clone())
                .map_err(|error| error.to_string())?;
            let ast = candidate.ast();
            let wrapper = ast
                .node(layout.children[0])
                .and_then(|child| child.parent)
                .and_then(|parent| ast.node(parent))
                .filter(|node| node.parent == Some(layout.container))
                .ok_or("The selected controls could not retain their source identity")?;
            if ast.element_children(wrapper.id) != layout.children {
                return Err(
                    "The generated wrapper did not retain the selected controls in source order"
                        .into(),
                );
            }
            (patches, wrapper.name_span)
        };
        Ok(PreparedWrap {
            layout,
            patches,
            wrapper,
        })
    }
}

impl DesignerSession {
    /// Wrap the current authored selection and select the new wrapper after one source edit.
    pub fn wrap_selection(&mut self, kind: WrapKind) -> Result<NodeId, String> {
        let plan = WrapContext::new(self, None)?.prepare(kind)?;
        self.commit_wrap(plan)
    }

    pub(crate) fn commit_wrap(&mut self, plan: PreparedWrap) -> Result<NodeId, String> {
        let label = format!("Wrap in {}", plan.layout.kind.label());
        if self.uses_rust_source() {
            self.commit_rust_patches(&label, plan.patches)
        } else {
            self.commit_xaml_patches(label, plan.patches)
        }
        .map_err(|error| error.to_string())?;
        self.changed(true);
        let selected = self
            .document()
            .ast()
            .elements()
            .find(|node| {
                if let Some(rust) = &self.rust {
                    rust.map()
                        .and_then(|map| map.control(node.span.start))
                        .is_some_and(|(_, name)| name == plan.wrapper)
                } else {
                    node.name_span == plan.wrapper
                }
            })
            .map(|node| node.id)
            .ok_or("The wrapped source was applied but its selection could not be restored")?;
        self.select([selected]);
        Ok(selected)
    }
}
