//! Projection of immutable UI snapshots into inert, namespaced SVG metadata.

use crate::{
    SvgSemanticExportMode, SvgSemanticExportOptions, SvgSemanticNode, SvgSemanticTree,
    SvgSemanticValue, SVG_SEMANTIC_NAMESPACE,
};
use std::fmt::{Display, Write};

fn attribute(output: &mut String, name: &str, value: &str) {
    write!(output, " {name}=\"").unwrap();
    crate::xml::escape(output, value, true);
    output.push('"');
}

fn scalar(output: &mut String, name: &str, value: impl Display) {
    write!(output, " {name}=\"{value}\"").unwrap();
}

fn optional(output: &mut String, name: &str, value: Option<impl Display>) {
    if let Some(value) = value {
        scalar(output, name, value);
    }
}

fn text_element(output: &mut String, name: &str, value: &str) {
    write!(output, "<ui:{name}>").unwrap();
    crate::xml::escape(output, value, false);
    write!(output, "</ui:{name}>").unwrap();
}

fn optional_text(output: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        text_element(output, name, value);
    }
}

// Layer IDs cannot start with internal-. Length-prefixing the layer component also prevents
// collisions between ("a-b", "c") and ("a", "b-c"), without changing caller semantic keys.
pub(crate) fn node_id(layer_id: &str, key: &str) -> String {
    format!("ferro-internal-ui-{}-{layer_id}-{key}", layer_id.len())
}

pub(crate) fn write_layer_attributes(
    output: &mut String,
    layer_id: &str,
    tree: &SvgSemanticTree,
    options: &SvgSemanticExportOptions,
) {
    if options.mode != SvgSemanticExportMode::MetadataAndAttributes {
        return;
    }
    attribute(output, "data-ui-schema", SVG_SEMANTIC_NAMESPACE);
    scalar(output, "data-ui-node-count", tree.len());
    // A forest has no single representative control. Avoid inventing one for that case.
    let mut roots = tree.nodes().iter().filter(|node| node.parent.is_none());
    if let Some(root) = roots.next().filter(|_| roots.next().is_none()) {
        attribute(output, "data-ui-root", &node_id(layer_id, &root.id));
        attribute(output, "data-ui-type", &root.type_name);
        if options.include_names {
            if let Some(name) = &root.name {
                attribute(output, "data-ui-name", name);
            }
        }
        if options.include_accessibility {
            if let Some(accessibility) = &root.accessibility {
                if let Some(id) = &accessibility.automation_id {
                    attribute(output, "data-ui-automation-id", id);
                }
                if let Some(role) = &accessibility.role {
                    attribute(output, "data-ui-role", role);
                }
            }
        }
    }
}

pub(crate) fn write_layer_metadata(
    output: &mut String,
    layer_id: &str,
    tree: &SvgSemanticTree,
    options: &SvgSemanticExportOptions,
) {
    if options.mode == SvgSemanticExportMode::None {
        return;
    }
    output.push_str("<metadata><ui:tree");
    attribute(output, "xmlns:ui", SVG_SEMANTIC_NAMESPACE);
    output.push_str(" schema-version=\"1\" hierarchy=\"visual\" coordinate-space=\"layer-local\" association=\"layer\"");
    scalar(output, "node-count", tree.len());
    output.push_str("><ui:rendering");
    attribute(output, "href", &format!("#ferro-{layer_id}"));
    output.push_str("/>");
    for node in tree.nodes() {
        write_node(output, layer_id, node, options);
    }
    output.push_str("</ui:tree></metadata>");
}

fn relationship(output: &mut String, layer_id: &str, kind: &str, target: Option<&str>) {
    if let Some(target) = target {
        write!(output, "<ui:{kind}").unwrap();
        attribute(output, "href", &format!("#{}", node_id(layer_id, target)));
        output.push_str("/>");
    }
}

fn write_node(
    output: &mut String,
    layer_id: &str,
    node: &SvgSemanticNode,
    options: &SvgSemanticExportOptions,
) {
    output.push_str("<ui:node");
    attribute(output, "id", &node_id(layer_id, &node.id));
    attribute(output, "key", &node.id);
    attribute(output, "type", &node.type_name);
    output.push('>');
    relationship(output, layer_id, "parent", node.parent.as_deref());
    if options.include_relations {
        relationship(
            output,
            layer_id,
            "logical-parent",
            node.logical_parent.as_deref(),
        );
        relationship(
            output,
            layer_id,
            "templated-parent",
            node.templated_parent.as_deref(),
        );
    }
    if options.include_names {
        optional_text(output, "name", node.name.as_deref());
    }
    if options.include_classes && !node.classes.is_empty() {
        output.push_str("<ui:classes>");
        for class in &node.classes {
            text_element(output, "class", class);
        }
        output.push_str("</ui:classes>");
    }
    let r = node.layout_bounds;
    let m = node.local_to_layer;
    write!(output, "<ui:geometry layout-space=\"parent-layout\" layout-bounds=\"{} {} {} {}\" local-bounds=\"0 0 {} {}\" matrix-order=\"m11 m12 m13 m21 m22 m23 m31 m32 m33\" local-to-layer=\"{} {} {} {} {} {} {} {} {}\"/>",
        r.x, r.y, r.width, r.height, r.width, r.height,
        m.m11, m.m12, m.m13, m.m21, m.m22, m.m23, m.m31, m.m32, m.m33).unwrap();
    let state = &node.state;
    output.push_str("<ui:state");
    scalar(output, "visible", state.visible);
    scalar(output, "effectively-visible", state.effectively_visible);
    scalar(output, "render-suppressed", state.render_suppressed);
    scalar(output, "opacity", state.opacity);
    scalar(output, "z-index", state.z_index);
    scalar(output, "clip-to-bounds", state.clip_to_bounds);
    scalar(output, "has-clip", state.has_clip);
    scalar(output, "has-opacity-mask", state.has_opacity_mask);
    scalar(output, "has-effect", state.has_effect);
    output.push_str("/>");
    if options.include_input_state {
        output.push_str("<ui:input");
        optional(output, "enabled", state.enabled);
        optional(output, "effectively-enabled", state.effectively_enabled);
        optional(output, "focusable", state.focusable);
        optional(output, "focused", state.focused);
        optional(output, "hit-test-visible", state.hit_test_visible);
        optional(output, "tab-stop", state.is_tab_stop);
        optional(output, "tab-index", state.tab_index);
        output.push_str("/>");
    }
    if options.include_layout {
        if let Some(layout) = &node.layout {
            output.push_str("<ui:layout");
            write!(
                output,
                " desired-size=\"{} {}\" margin=\"{} {} {} {}\"",
                layout.desired_size.width,
                layout.desired_size.height,
                layout.margin.left,
                layout.margin.top,
                layout.margin.right,
                layout.margin.bottom
            )
            .unwrap();
            for (name, value, missing) in [
                ("width", layout.width, "auto"),
                ("height", layout.height, "auto"),
                ("max-width", layout.max_width, "unbounded"),
                ("max-height", layout.max_height, "unbounded"),
            ] {
                if let Some(value) = value {
                    scalar(output, name, value);
                } else {
                    attribute(output, name, missing);
                }
            }
            scalar(output, "min-width", layout.min_width);
            scalar(output, "min-height", layout.min_height);
            attribute(output, "horizontal-alignment", &layout.horizontal_alignment);
            attribute(output, "vertical-alignment", &layout.vertical_alignment);
            scalar(output, "measure-valid", layout.measure_valid);
            scalar(output, "arrange-valid", layout.arrange_valid);
            if let Some(grid) = &layout.grid {
                output.push_str("><ui:grid");
                scalar(output, "row", grid.row);
                scalar(output, "column", grid.column);
                scalar(output, "row-span", grid.row_span);
                scalar(output, "column-span", grid.column_span);
                output.push_str("/></ui:layout>");
            } else {
                output.push_str("/>");
            }
        }
    }
    if options.include_accessibility {
        if let Some(accessibility) = &node.accessibility {
            output.push_str("<ui:accessibility");
            optional(output, "required", accessibility.required);
            optional(output, "heading-level", accessibility.heading_level);
            optional(output, "position-in-set", accessibility.position_in_set);
            optional(output, "size-of-set", accessibility.size_of_set);
            output.push('>');
            optional_text(
                output,
                "automation-id",
                accessibility.automation_id.as_deref(),
            );
            optional_text(output, "name", accessibility.name.as_deref());
            optional_text(output, "help-text", accessibility.help_text.as_deref());
            optional_text(output, "role", accessibility.role.as_deref());
            optional_text(
                output,
                "landmark-type",
                accessibility.landmark_type.as_deref(),
            );
            optional_text(
                output,
                "live-setting",
                accessibility.live_setting.as_deref(),
            );
            output.push_str("</ui:accessibility>");
        }
    }
    if options.include_text {
        optional_text(output, "text", node.text.as_deref());
    }
    if options.include_source {
        if let Some(source) = &node.source {
            output.push_str("<ui:source");
            attribute(output, "uri", &source.uri);
            optional(output, "line", source.line);
            optional(output, "column", source.column);
            output.push_str("/>");
        }
    }
    if options.include_properties && !node.properties.is_empty() {
        output.push_str("<ui:properties>");
        for (name, value) in &node.properties {
            output.push_str("<ui:property");
            attribute(output, "name", name);
            let kind = match value {
                SvgSemanticValue::String(_) => "string",
                SvgSemanticValue::Number(_) => "number",
                SvgSemanticValue::Bool(_) => "boolean",
                SvgSemanticValue::StringList(_) => "string-list",
            };
            attribute(output, "type", kind);
            output.push('>');
            match value {
                SvgSemanticValue::String(value) => crate::xml::escape(output, value, false),
                SvgSemanticValue::Number(value) => write!(output, "{value}").unwrap(),
                SvgSemanticValue::Bool(value) => write!(output, "{value}").unwrap(),
                SvgSemanticValue::StringList(values) => {
                    for value in values {
                        text_element(output, "item", value);
                    }
                }
            }
            output.push_str("</ui:property>");
        }
        output.push_str("</ui:properties>");
    }
    output.push_str("</ui:node>");
}
