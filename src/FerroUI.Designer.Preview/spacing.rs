//! Limit native spacing plans to the authored RSX property that produced each handle.
use crate::document::DesignDocument;
use crate::edit_selection::{authored_range, rsx_control_ranges};
use crate::edits::editor_edits;
use crate::protocol::EditResult;
use crate::renderer::Stage;
use ferroui_designer::mcp::SPACING_PROPERTIES;
use ferroui_designer_core::{Edit, NodeId};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(crate) struct RsxOwners(BTreeMap<(NodeId, &'static str), Result<(), &'static str>>);

impl RsxOwners {
    pub(crate) fn capture(design: &DesignDocument<'_>) -> Option<Self> {
        let rsx = design.rsx.as_ref()?;
        let mut properties = BTreeMap::new();
        for node in design.xaml.ast().elements() {
            let Some((index, mapped)) = design.rsx_node(node.id) else {
                continue;
            };
            for &property in SPACING_PROPERTIES {
                if mapped.supports_literal(property) {
                    properties.insert(
                        (node.id, property),
                        rsx.spacing_property_support(index, property),
                    );
                }
            }
        }
        Some(Self(properties))
    }

    fn reason(&self, handle: &Value) -> Option<&'static str> {
        let owner = handle["owner"].as_u64()?;
        let property = handle["property"].as_str()?;
        match self.0.get(&(owner, property)) {
            Some(result) => result.err(),
            None => Some("This property has no mapped RSX literal."),
        }
    }

    pub(crate) fn check_query(&self, name: &str, response: &mut Value) -> Result<(), String> {
        if name == "designer_spacing_helpers" {
            if let Some(handles) = response["handles"].as_array_mut() {
                for handle in handles {
                    if let Some(reason) = self
                        .reason(handle)
                        .filter(|_| handle["disabledReason"].is_null())
                    {
                        handle["disabledReason"] = json!(reason);
                    }
                }
            }
        } else if name == "designer_spacing_draft" {
            if let Some(reason) = self.reason(&response["handle"]) {
                return Err(reason.into());
            }
        }
        Ok(())
    }
}

pub(crate) fn source_edit(
    stage: &Stage,
    design: &DesignDocument<'_>,
    args: &Value,
) -> Result<EditResult, String> {
    let (handle, edits) = stage
        .sources
        .run(|| stage.studio.spacing_source_plan(args))?;
    if let Some(reason) = stage
        .rsx_spacing
        .as_ref()
        .and_then(|owners| owners.reason(&handle))
    {
        return Err(reason.into());
    }
    if edits.is_empty() {
        return editor_edits(design, Vec::new());
    }
    let session = stage.session.borrow();
    let (patches, retained) = if let Some(rsx) = &design.rsx {
        let [Edit::SetAttribute { node, name, value }] = edits.as_slice() else {
            return Err("This spacing handle does not map to a single RSX property".into());
        };
        if handle["owner"] != *node || handle["property"] != name.as_str() {
            return Err("The spacing plan targets a different RSX property".into());
        }
        let (index, _) = design
            .rsx_node(*node)
            .ok_or("The spacing owner has no RSX mapping")?;
        let patches = rsx.edit_spacing_property(index, name, value)?;
        let retained = rsx_control_ranges(design, &patches, session.selection())?;
        (patches, retained)
    } else {
        let mut candidate = session.document().snapshot_without_history();
        let patches = candidate
            .apply("Adjust spacing", edits)
            .map_err(|error| error.to_string())?
            .patches;
        let retained = session
            .selection()
            .iter()
            .filter_map(|&node| authored_range(design, &candidate, &patches, node))
            .collect();
        (patches, retained)
    };
    let mut result = editor_edits(design, patches)?;
    result.retained_selection = retained;
    Ok(result)
}
