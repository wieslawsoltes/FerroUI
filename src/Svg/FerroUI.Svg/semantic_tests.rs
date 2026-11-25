use crate::*;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Colors};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Matrix, Rect, Size, Thickness, Vector};
use std::collections::BTreeSet;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

fn add_layer(document: &mut SvgDocument, id: &str, opacity_group: bool) -> SvgLayerId {
    document
        .record_layer(
            SvgLayerOptions::new(id, format!("Drawing {id}")).with_depth(2.0),
            |context| {
                if opacity_group {
                    context.push_opacity(0.5, None);
                }
                context.draw_rectangle(
                    Some(&ImmutableSolidColorBrush::new(Colors::RED)),
                    None,
                    Rect::new(4.125, 7.25, 30.5, 18.75).into(),
                    &BoxShadows::default(),
                );
                if opacity_group {
                    context.pop_opacity();
                }
            },
        )
        .unwrap()
}

fn scene() -> SvgDocument {
    let mut document = SvgDocument::new(Size::new(96.0, 64.0)).unwrap();
    add_layer(&mut document, "ui", false);
    document
}

fn layout() -> SvgSemanticLayout {
    SvgSemanticLayout {
        desired_size: Size::new(30.0, 18.0),
        margin: Thickness::new(-1.0, 2.0, 3.0, 4.0),
        width: None,
        height: Some(18.0),
        min_width: 0.0,
        min_height: 0.0,
        max_width: None,
        max_height: Some(64.0),
        horizontal_alignment: "Stretch".into(),
        vertical_alignment: "Center".into(),
        measure_valid: true,
        arrange_valid: true,
        grid: None,
    }
}

fn nodes() -> Vec<SvgSemanticNode> {
    let mut root = SvgSemanticNode::new("root", "FerroUI.Controls.Panel");
    root.name = Some("SemanticRoot".into());
    root.classes = vec!["component".into(), ":enabled".into()];
    root.layout_bounds = Rect::new(9.0, 8.0, 96.0, 64.0);
    root.local_to_layer = Matrix::create_translation(2.0, 3.0);
    root.layout = Some(layout());
    root.layout.as_mut().unwrap().grid = Some(SvgSemanticGridPlacement {
        row: 1,
        column: 2,
        row_span: 2,
        column_span: 1,
    });
    root.state.enabled = Some(true);
    root.state.is_tab_stop = Some(false);
    root.state.tab_index = Some(4);
    root.properties.insert(
        "design-token".into(),
        SvgSemanticValue::String("AccentPrimary".into()),
    );

    let mut child = SvgSemanticNode::new("action", "FerroUI.Controls.Button");
    child.parent = Some("root".into());
    child.logical_parent = Some("root".into());
    child.templated_parent = Some("root".into());
    child.name = Some("SemanticAction".into());
    child.layout_bounds = Rect::new(4.125, 7.25, 30.5, 18.75);
    child.local_to_layer = Matrix::create_translation(6.125, 10.25);
    child.state.focusable = Some(true);
    child.text = Some("ConfidentialDisplayText".into());
    child.accessibility = Some(SvgSemanticAccessibility {
        automation_id: Some("ConfidentialAutomationId".into()),
        name: Some("ConfidentialAccessibleName".into()),
        help_text: Some("ConfidentialHelpText".into()),
        role: Some("Button".into()),
        required: Some(true),
        ..Default::default()
    });
    child.source = Some(SvgSemanticSource {
        uri: "source:///ConfidentialView.xaml".into(),
        line: Some(12),
        column: Some(7),
    });
    vec![root, child]
}

fn tree() -> SvgSemanticTree {
    SvgSemanticTree::new(nodes()).unwrap()
}

fn semantic_options() -> SvgExportOptions {
    SvgExportOptions {
        semantics: SvgSemanticExportOptions::all(),
        pretty: false,
        ..Default::default()
    }
}

fn semantic_node_count(xml: &roxmltree::Document<'_>) -> usize {
    xml.descendants()
        .filter(|node| node.has_tag_name((SVG_SEMANTIC_NAMESPACE, "node")))
        .count()
}

fn assert_local_references_resolve(xml: &roxmltree::Document<'_>) {
    let mut ids = BTreeSet::new();
    for node in xml.descendants().filter(|node| node.is_element()) {
        if let Some(id) = node.attribute("id") {
            assert!(ids.insert(id), "duplicate id {id}");
        }
    }
    for node in xml.descendants().filter(|node| node.is_element()) {
        for attribute in node
            .attributes()
            .filter(|attribute| attribute.name() == "href")
        {
            if let Some(target) = attribute.value().strip_prefix('#') {
                assert!(ids.contains(target), "dangling reference #{target}");
            }
        }
    }
}

fn contains_semantic_value(xml: &roxmltree::Document<'_>, expected: &str) -> bool {
    xml.descendants().any(|node| {
        if node.tag_name().namespace() != Some(SVG_SEMANTIC_NAMESPACE) {
            return false;
        }
        node.attributes()
            .any(|attribute| attribute.value() == expected)
            || node
                .children()
                .any(|child| child.is_text() && child.text() == Some(expected))
    })
}

fn painted_structure(xml: &roxmltree::Document<'_>) -> Vec<(String, Vec<(String, String)>)> {
    xml.descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().namespace() == Some(SVG_NS)
                && !node
                    .ancestors()
                    .any(|parent| parent.has_tag_name((SVG_NS, "metadata")))
        })
        .map(|node| {
            (
                node.tag_name().name().into(),
                node.attributes()
                    .filter(|attribute| !attribute.name().starts_with("data-ui-"))
                    .map(|attribute| (attribute.name().into(), attribute.value().into()))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn semantic_graph_rejects_aliases_dangling_links_and_visual_cycles() {
    assert!(SvgSemanticTree::new(Vec::new()).unwrap().is_empty());
    for id in ["", "contains space", "xml:prefix", "a/b", "Żółw"] {
        assert!(SvgSemanticTree::new(vec![SvgSemanticNode::new(id, "Control")]).is_err());
    }
    let mut values = nodes();
    values[1].id = values[0].id.clone();
    assert!(SvgSemanticTree::new(values).is_err());
    let mut values = nodes();
    values[0].parent = Some("action".into());
    assert!(SvgSemanticTree::new(values).is_err());
    let mut values = nodes();
    values[1].parent = Some("action".into());
    assert!(SvgSemanticTree::new(values).is_err());
    let mut values = nodes();
    values[1].logical_parent = Some("outside".into());
    assert!(SvgSemanticTree::new(values).is_err());
    let mut values = nodes();
    values[1].templated_parent = Some("outside".into());
    assert!(SvgSemanticTree::new(values).is_err());
}

#[test]
fn semantic_ancestry_relationships_reject_self_links_and_cycles() {
    let mut values = nodes();
    values[0].logical_parent = Some("action".into());
    assert!(SvgSemanticTree::new(values).is_err());
    let mut values = nodes();
    values[0].templated_parent = Some("action".into());
    assert!(SvgSemanticTree::new(values).is_err());
    let mut values = nodes();
    values[0].logical_parent = Some("root".into());
    assert!(SvgSemanticTree::new(values).is_err());
    let mut values = nodes();
    values[0].templated_parent = Some("root".into());
    assert!(SvgSemanticTree::new(values).is_err());

    // Logical/template order need not equal visual preorder: a later node may
    // own a relationship without turning it into visual ancestry.
    let mut values = nodes();
    values[1].logical_parent = None;
    values[0].logical_parent = Some("action".into());
    assert!(SvgSemanticTree::new(values).is_ok());
}

#[test]
fn semantic_model_rejects_nonfinite_geometry_values_and_invalid_xml_characters() {
    let mut value = SvgSemanticNode::new("node", "Control");
    value.layout_bounds = Rect::new(f64::NAN, 0.0, 1.0, 1.0);
    assert!(SvgSemanticTree::new(vec![value]).is_err());
    let mut value = SvgSemanticNode::new("node", "Control");
    value.local_to_layer.m13 = f64::INFINITY;
    assert!(SvgSemanticTree::new(vec![value]).is_err());
    let mut value = SvgSemanticNode::new("node", "Control");
    value.state.opacity = -0.01;
    assert!(SvgSemanticTree::new(vec![value]).is_err());
    let mut value = SvgSemanticNode::new("node", "Control");
    value
        .properties
        .insert("value".into(), SvgSemanticValue::Number(f64::NAN));
    assert!(SvgSemanticTree::new(vec![value]).is_err());
    let mut value = SvgSemanticNode::new("node", "Control");
    value.name = Some("invalid\0name".into());
    assert!(SvgSemanticTree::new(vec![value]).is_err());
    let mut value = SvgSemanticNode::new("node", "Control");
    value.source = Some(SvgSemanticSource {
        uri: "source:///View.xaml".into(),
        line: Some(0),
        column: None,
    });
    assert!(SvgSemanticTree::new(vec![value]).is_err());
}

#[test]
fn semantic_layout_auto_and_mapping_preserve_source_coordinates() {
    let mut node = SvgSemanticNode::new("node", "Control");
    node.layout = Some(layout());
    node.layout_bounds = Rect::new(10.0, 20.0, 0.0, 0.0);
    node.local_to_layer = Matrix::create_translation(3.0, 5.0);
    let original = SvgSemanticTree::new(vec![node]).unwrap();
    let mapped = original
        .clone()
        .transformed(Matrix::create_scale(2.0, 3.0))
        .unwrap();
    assert_eq!(
        mapped.nodes()[0].layout_bounds,
        original.nodes()[0].layout_bounds
    );
    assert_eq!(mapped.nodes()[0].local_to_layer.m31, 6.0);
    assert_eq!(mapped.nodes()[0].local_to_layer.m32, 15.0);
    assert_eq!(mapped.nodes()[0].layout.as_ref().unwrap().width, None);
    assert_eq!(mapped.nodes()[0].layout.as_ref().unwrap().max_width, None);

    let mut invalid = Matrix::IDENTITY;
    invalid.m31 = f64::INFINITY;
    assert!(original.transformed(invalid).is_err());
}

#[test]
fn semantic_text_budget_counts_utf8_bytes() {
    let mut node = SvgSemanticNode::new("node", "Control");
    node.text = Some("é".repeat(SvgSemanticTree::MAX_TEXT_BYTES / 2 + 1));
    assert!(SvgSemanticTree::new(vec![node]).is_err());
}

#[test]
fn semantic_value_budget_bounds_empty_collection_expansion() {
    let mut node = SvgSemanticNode::new("node", "Control");
    node.classes = vec![String::new(); SvgSemanticTree::MAX_VALUES + 1];
    assert!(SvgSemanticTree::new(vec![node]).is_err());
    let mut node = SvgSemanticNode::new("node", "Control");
    node.properties.insert(
        "empty-values".into(),
        SvgSemanticValue::StringList(vec![String::new(); SvgSemanticTree::MAX_VALUES + 1]),
    );
    assert!(SvgSemanticTree::new(vec![node]).is_err());
}

#[test]
fn disabled_semantics_are_byte_identical_and_do_not_evict_prepared_fragments() {
    let mut document = scene();
    let options = SvgExportOptions::compact();
    let original = document.export(&options).unwrap();
    document.set_layer_semantics("ui", tree()).unwrap();
    let attached = document.export(&options).unwrap();
    assert_eq!(attached.svg, original.svg);
    assert_eq!(attached.report.semantic_layers, 0);
    assert_eq!(attached.report.semantic_nodes, 0);
    assert_eq!(attached.report.prepared_vector_cache_hits, 1);

    let enriched = document.export(&semantic_options()).unwrap();
    assert_eq!(enriched.report.semantic_layers, 1);
    assert_eq!(enriched.report.semantic_nodes, 2);
    assert_eq!(enriched.report.prepared_vector_cache_hits, 1);
    let baseline_xml = roxmltree::Document::parse(&original.svg).unwrap();
    let enriched_xml = roxmltree::Document::parse(&enriched.svg).unwrap();
    assert_eq!(
        painted_structure(&baseline_xml),
        painted_structure(&enriched_xml)
    );
    assert_local_references_resolve(&enriched_xml);
    assert_eq!(document.export(&options).unwrap().svg, original.svg);
}

#[test]
fn semantic_projection_redacts_content_without_recapturing_or_replaying() {
    let mut document = scene();
    document.set_layer_semantics("ui", tree()).unwrap();
    let full = document.export(&semantic_options()).unwrap();
    for value in [
        "ConfidentialDisplayText",
        "ConfidentialAutomationId",
        "ConfidentialAccessibleName",
        "ConfidentialHelpText",
        "ConfidentialView.xaml",
    ] {
        assert!(full.svg.contains(value), "missing opted-in field {value}");
    }
    let structural = document
        .export(&SvgExportOptions {
            semantics: SvgSemanticExportOptions::metadata(),
            ..SvgExportOptions::compact()
        })
        .unwrap();
    assert!(!structural.svg.contains("Confidential"));
    assert!(structural.svg.contains("SemanticAction"));
    assert!(structural.svg.contains("AccentPrimary"));
    assert!(!structural.svg.contains("data-ui-"));
    assert_eq!(structural.report.prepared_vector_cache_hits, 1);

    let minimal = document
        .export(&SvgExportOptions {
            semantics: SvgSemanticExportOptions {
                mode: SvgSemanticExportMode::MetadataAndAttributes,
                include_names: false,
                include_classes: false,
                include_layout: false,
                include_input_state: false,
                include_relations: false,
                include_properties: false,
                ..SvgSemanticExportOptions::metadata()
            },
            ..SvgExportOptions::compact()
        })
        .unwrap();
    let xml = roxmltree::Document::parse(&minimal.svg).unwrap();
    assert_eq!(semantic_node_count(&xml), 2);
    assert!(!minimal.svg.contains("SemanticAction"));
    assert!(!minimal.svg.contains("SemanticRoot"));
    assert!(!minimal.svg.contains("AccentPrimary"));
    assert!(!minimal.svg.contains(":enabled"));
    assert!(!minimal.svg.contains("data-ui-name"));
    assert!(!minimal.svg.contains("data-ui-automation-id"));
    for tag in [
        "layout",
        "grid",
        "input",
        "logical-parent",
        "templated-parent",
        "properties",
    ] {
        assert!(
            !xml.descendants()
                .any(|n| n.has_tag_name((SVG_SEMANTIC_NAMESPACE, tag))),
            "unexpected {tag}"
        );
    }
}

#[test]
fn semantic_strings_preserve_unicode_whitespace_and_cannot_create_active_markup() {
    let text = "Żółw 🦀\n\t\r<script onload=\"bad()\">& ' </metadata>";
    let mut node = SvgSemanticNode::new("stable", "FerroUI.Controls.Button");
    node.name = Some(text.into());
    node.text = Some(text.into());
    node.accessibility = Some(SvgSemanticAccessibility {
        help_text: Some(text.into()),
        ..Default::default()
    });
    node.properties
        .insert("onclick".into(), SvgSemanticValue::String(text.into()));
    node.properties.insert(
        "sequence".into(),
        SvgSemanticValue::StringList(vec!["".into(), text.into(), "tail".into()]),
    );
    node.source = Some(SvgSemanticSource {
        uri: text.into(),
        line: Some(1),
        column: Some(1),
    });
    let mut document = scene();
    document
        .set_layer_semantics("ui", SvgSemanticTree::new(vec![node]).unwrap())
        .unwrap();
    for pretty in [false, true] {
        let output = document
            .export(&SvgExportOptions {
                pretty,
                ..semantic_options()
            })
            .unwrap();
        let xml = roxmltree::Document::parse(&output.svg).unwrap();
        assert!(contains_semantic_value(&xml, text));
        assert!(contains_semantic_value(&xml, "onclick"));
        assert!(!xml
            .descendants()
            .any(|node| node.tag_name().name() == "script"));
        assert!(!xml
            .descendants()
            .filter(|node| node.is_element())
            .any(|node| node
                .attributes()
                .any(|attribute| attribute.name().starts_with("on"))));
        assert_local_references_resolve(&xml);
    }
}

#[test]
fn semantic_numbers_are_not_changed_by_svg_geometry_quantization() {
    let precise = 12.345678912345;
    let mut node = SvgSemanticNode::new("precise", "Control");
    node.layout_bounds = Rect::new(precise, 1.0, 12.0, 14.0);
    node.local_to_layer = Matrix::create_translation(precise, 1.0);
    node.properties
        .insert("measured-value".into(), SvgSemanticValue::Number(precise));
    let mut document = scene();
    document
        .set_layer_semantics("ui", SvgSemanticTree::new(vec![node]).unwrap())
        .unwrap();
    let output = document
        .export(&SvgExportOptions {
            precision: Some(0),
            ..semantic_options()
        })
        .unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let needle = precise.to_string();
    let occurrences = xml
        .descendants()
        .filter(|node| node.tag_name().namespace() == Some(SVG_SEMANTIC_NAMESPACE))
        .flat_map(|node| node.attributes())
        .filter(|attribute| attribute.value().contains(&needle))
        .count();
    assert!(
        occurrences >= 2,
        "bounds and transform must retain exact values: {}",
        output.svg
    );
    assert!(
        contains_semantic_value(&xml, &needle),
        "numeric property must retain its exact value"
    );
}

#[test]
fn semantic_layer_namespaces_filters_and_removal_keep_references_consistent() {
    let mut document = scene();
    let first = document.layers()[0].options().id.clone();
    let second = add_layer(&mut document, "ui-child", false);
    document.set_layer_semantics(&first, tree()).unwrap();
    document.set_layer_semantics("ui-child", tree()).unwrap();
    let output = document.export(&semantic_options()).unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    assert_eq!(semantic_node_count(&xml), 4);
    assert_local_references_resolve(&xml);
    let filtered = document
        .export(&SvgExportOptions {
            view: SvgView {
                include: SvgLayerFilter::id("ui-child"),
                ..Default::default()
            },
            ..semantic_options()
        })
        .unwrap();
    let filtered_xml = roxmltree::Document::parse(&filtered.svg).unwrap();
    assert_eq!(semantic_node_count(&filtered_xml), 2);
    assert_eq!(filtered.report.semantic_layers, 1);
    assert_eq!(filtered.report.semantic_nodes, 2);
    assert!(!filtered_xml
        .descendants()
        .any(|node| node.attribute("id") == Some("ferro-ui")));
    assert_local_references_resolve(&filtered_xml);

    assert!(document.remove_layer(&second));
    assert!(document.layer_semantics("ui-child").is_none());
    add_layer(&mut document, "ui-child", false);
    assert!(
        document.layer_semantics("ui-child").is_none(),
        "reusing a layer key must not revive old UI metadata"
    );
    let output = document.export(&semantic_options()).unwrap();
    assert_eq!(output.report.semantic_layers, 1);
    assert_local_references_resolve(&roxmltree::Document::parse(&output.svg).unwrap());
}

#[test]
fn replacing_and_clearing_semantics_do_not_mutate_or_evict_recorded_artwork() {
    let mut document = scene();
    assert!(document.set_layer_semantics("absent", tree()).is_err());
    document.set_layer_semantics("ui", tree()).unwrap();
    document.export(&semantic_options()).unwrap();
    let mut replacement = SvgSemanticNode::new("replacement", "ReplacementControl");
    replacement.name = Some("CurrentSemanticName".into());
    document
        .set_layer_semantics("ui", SvgSemanticTree::new(vec![replacement]).unwrap())
        .unwrap();
    let output = document.export(&semantic_options()).unwrap();
    assert!(output.svg.contains("CurrentSemanticName"));
    assert!(!output.svg.contains("SemanticAction"));
    assert_eq!(output.report.prepared_vector_cache_hits, 1);
    assert_eq!(output.report.semantic_nodes, 1);
    assert!(document.clear_layer_semantics("ui"));
    assert!(!document.clear_layer_semantics("ui"));
    let output = document.export(&semantic_options()).unwrap();
    assert_eq!(output.report.semantic_nodes, 0);
    assert_eq!(output.report.prepared_vector_cache_hits, 1);
    assert!(!output.svg.contains(SVG_SEMANTIC_NAMESPACE));
}

#[test]
fn raster_fallback_and_inspection_views_preserve_semantic_provenance() {
    let mut document = SvgDocument::new(Size::new(96.0, 64.0)).unwrap();
    add_layer(&mut document, "raster", true);
    document.set_layer_semantics("raster", tree()).unwrap();
    for inspection in [
        SvgInspection::Normal,
        SvgInspection::x_ray(),
        SvgInspection::wireframe(),
    ] {
        let output = document
            .export(&SvgExportOptions {
                raster_scale: 1.0,
                view: SvgView {
                    inspection,
                    exploded: Some(SvgExplodedView {
                        step: Vector::new(4.0, -3.0),
                        show_connectors: false,
                        show_labels: false,
                    }),
                    ..Default::default()
                },
                ..semantic_options()
            })
            .unwrap();
        assert_eq!(output.report.raster_layers, 1);
        assert_eq!(output.report.semantic_layers, 1);
        assert_eq!(output.report.semantic_nodes, 2);
        let xml = roxmltree::Document::parse(&output.svg).unwrap();
        assert_local_references_resolve(&xml);
        assert_eq!(semantic_node_count(&xml), 2);
        assert!(!xml
            .descendants()
            .filter(|node| node.is_element())
            .any(|node| node.attribute("role") == Some("button")
                || node.attribute("tabindex").is_some()));
    }
    assert!(document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::VectorOnly,
            ..semantic_options()
        })
        .is_err());
    assert!(document
        .export(&SvgExportOptions {
            max_raster_pixels: 1,
            ..semantic_options()
        })
        .is_err());
}
