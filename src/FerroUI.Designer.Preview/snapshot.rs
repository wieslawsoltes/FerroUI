use crate::document::DesignDocument;
use crate::edits::{addable_properties, can_remove, editable_attribute};
use crate::protocol::{Language, OutlineNode, Property, Render, Snapshot};
use crate::projects::Project;
use crate::renderer::Renderer;
use ferroui_designer_project::local_preview_sources_with_resources;
use ferroui_markup_xaml::source_overlay::SourceSnapshot;

pub(crate) fn render(renderer: &mut Renderer, input: &Render, project: Option<&Project>) -> Result<Snapshot, String> {
    renderer.retire_frame(input);
    let rust = match (input.document.language, &input.workspace_root, &input.rust_project) {
        (Language::Rust, Some(root), Some(rust)) => rust.capture(root, &input.document.uri, &input.document.source, &input.local_sources),
        _ => None,
    };
    if rust.is_none() && input.rust_project.is_some()
        && renderer.accepted_rust_module_document.as_deref() == Some(input.document.uri.as_str())
    {
        return Err("Repair the Rust module sources to update Design".into());
    }
    let design = DesignDocument::parse_with_rust_modules(&input.document, project, rust.as_ref())?;
    let counterpart = design.counterpart();
    let mut notices = Vec::new();
    let (blocks, active_block) = if let Some(rsx) = &design.rsx {
        if rsx.omitted_count != 0 {
            notices.push(format!(
                "Static preview · {} omitted Rust items ({})",
                rsx.omitted_count,
                rsx.omissions.join(", ")
            ));
        }
        (
            rsx.blocks
                .iter()
                .map(|span| design.range(*span))
                .collect::<Result<_, _>>()?,
            Some(design.range(rsx.invocation)?),
        )
    } else {
        (Vec::new(), None)
    };
    let local = local_preview_sources_with_resources(input.workspace_root.as_deref(), &input.document.uri, &design.xaml, &input.local_sources, project.map(|project| &project.resources));
    let mut dependencies = local.dependencies;
    if let Some(rust) = &rust {
        dependencies.extend(rust.dependencies().map(str::to_owned));
    }
    let snapshot = SourceSnapshot::new(local.accepted, local.unavailable);
    let layers = input.layer_view.as_ref().filter(|layers| layers.block.as_ref() == active_block.as_ref().map(|range| &range.start))
        .and_then(|input| serde_json::from_value(input.state.clone()).ok());
    let (image, bounds, render_error, timeline_pose) = match snapshot.run(|| {
        renderer.render(input, design.xaml.source(), snapshot.clone(), local.includes, layers.as_ref())
    }) {
        Ok(frame) => {
            notices.extend(frame.notices);
            (Some(frame.image), frame.bounds, None, frame.timeline_pose)
        }
        Err(error) => (None, Default::default(), Some(error), None),
    };
    let mut assets = project.map_or_else(Vec::new, |project| project.assets.clone());
    let mut project_assets = project.map_or_else(Vec::new, |project| project.toolbox.clone());
    if let Some(root) = &input.workspace_root {
        let snippets = ferroui_designer_project::file_path(&input.document.uri)
            .map_err(|error| error.to_string())
            .and_then(|file| ferroui_designer::project_designer_assets(root, &file));
        match snippets {
            Ok(snippets) => {
                project_assets.extend(snippets.iter().map(|asset| crate::toolbox::describe(asset, design.rsx_catalog())));
                assets.extend(snippets);
            }
            Err(error) => notices.push(format!("Project Toolbox: {error}")),
        }
    }
    if let Some(stage) = &mut renderer.stage {
        stage.placement_policy = crate::rsx_placement::PlacementPolicy::of(&design);
        stage.authored_rsx = crate::rsx_animation::AuthoredRsxFrame::capture(&design, &input.document.uri);
        stage.rsx_spacing = crate::spacing::RsxOwners::capture(&design);
        stage.project_assets = assets;
    }
    let nodes = design
        .xaml
        .ast()
        .nodes_in_source_order()
        .filter(|node| node.is_element() && (design.rsx.is_none() || design.rsx_node(node.id).is_some()))
        .map(|node| {
            let (span, name_span) = match design.rsx_node(node.id) {
                Some((_, rsx)) => (rsx.span, rsx.name),
                None => (node.span, node.name_span),
            };
            let properties = node
                .attributes
                .iter()
                .filter(|attr| !attr.is_namespace())
                .map(|attr| Property {
                    name: attr.name.clone(),
                    value: attr.value.clone(),
                    editable: editable_attribute(&design, node, &attr.name),
                })
                .collect();
            Ok(OutlineNode {
                id: node.id,
                parent: element_parent(&design, node.parent),
                label: node.name.clone(),
                name: node.xaml_name().map(str::to_owned),
                range: design.range(span)?,
                name_range: design.range(name_span)?,
                layer_start: node.span.start,
                bounds: bounds.get(&node.id).copied(),
                properties,
                addable_properties: addable_properties(&design, node),
                can_insert: design.editable_children(node.id) && renderer.stage.as_ref().is_some_and(|stage| stage.session.borrow().can_insert_outline_child(node.id)),
                can_remove: can_remove(&design, node),
                can_move: node.parent.is_some_and(|parent| design.editable_children(parent)) && renderer.stage.as_ref().is_some_and(|stage| stage.session.borrow().can_move_outline_node(node.id)),
                can_set_image_source: crate::asset_source::can_set_image_source(&design, node),
                selectable: renderer.stage.as_ref().is_some_and(|stage| stage.session.borrow().can_canvas_select(node.id)),
            })
        })
        .collect::<Result<_, String>>()?;
    let rust_animation_owners = design.rsx.as_ref().filter(|_| image.is_some()).map_or_else(Vec::new, |rsx| {
        design.xaml.ast().elements().filter_map(|node| {
            let (ordinal, _) = design.rsx_node(node.id)?;
            rsx.can_create_animation(ordinal).then_some(node.id)
        }).collect()
    });
    let (definition_scope, definition_selection) = renderer.stage.as_ref().map_or((None, Vec::new()), |stage| {
        let session = stage.session.borrow();
        let scope = session.definition_scope().map(|scope| scope.node);
        let selection = if scope.is_some() { session.selection().to_vec() } else { Vec::new() };
        (scope, selection)
    });
    let layer_view = renderer.stage.as_ref()
        .filter(|_| layers.is_some() && image.is_some())
        .map(|stage| stage.session.borrow().persist_layer_view());
    let snapshot = Snapshot {
        nodes,
        blocks,
        active_block,
        counterpart,
        image,
        render_error,
        notices,
        dependencies: dependencies.into_iter().collect(),
        project_assets,
        mapped_rust_animations: !design.rsx_animations.is_empty(),
        rust_animation_owners,
        timeline_pose,
        definition_scope,
        definition_selection,
        layer_view,
    };
    renderer.accepted_rust_module_document = rust.as_ref().map(|_| input.document.uri.clone());
    if snapshot.image.is_none() {
        renderer.retain_source_frame(input, rust);
    }
    Ok(snapshot)
}

fn element_parent(design: &DesignDocument<'_>, mut parent: Option<u64>) -> Option<u64> {
    while let Some(node) = parent.and_then(|id| design.xaml.ast().node(id)) {
        if node.is_element() { return Some(node.id); }
        parent = node.parent;
    }
    None
}
