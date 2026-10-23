use super::*;

fn square(network: &mut VectorNetwork, x: f64, y: f64, width: f64, height: f64) -> VectorRegionId {
    let vertices: Vec<_> = [Point::new(x, y), Point::new(x + width, y), Point::new(x + width, y + height), Point::new(x, y + height)]
        .map(|point| network.add_vertex(point).unwrap()).into_iter().collect();
    let edges = (0..4).map(|index| NetworkEdgeRef::forward(network.connect(vertices[index], vertices[(index + 1) % 4]).unwrap())).collect();
    network.add_region(vec![edges]).unwrap()
}

fn fixture() -> (Document, NodeId, VectorRegionId, VectorRegionId) {
    let mut network = VectorNetwork::default();
    square(&mut network, 10.0, 10.0, 100.0, 80.0);
    let painted = square(&mut network, 30.0, 20.0, 100.0, 50.0);
    let clear = square(&mut network, 60.0, 30.0, 30.0, 30.0);
    network.regions.get_mut(&painted).unwrap().fills = Some(vec![Fill::solid(Color::rgba(255, 0, 0, 128))]);
    network.regions.get_mut(&clear).unwrap().fills = Some(Vec::new());
    let mut node = Node::new(NodeKind::Vector, "Regions", 0.0, 0.0, 180.0, 100.0);
    node.vector_network = Some(network);
    node.fills = vec![Fill::solid(Color::rgba(20, 70, 210, 128))];
    let mut document = Document::new("Region SVG");
    let id = document.insert_node(1, node).unwrap();
    (document, id, painted, clear)
}

fn xml_id<'a, 'input>(xml: &'a roxmltree::Document<'input>, id: &str) -> roxmltree::Node<'a, 'input> {
    xml.descendants().find(|node| node.attribute("id") == Some(id)).unwrap_or_else(|| panic!("missing XML ID {id}"))
}

#[test]
fn region_svg_keeps_explicit_empty_cutters_and_imports_portable_masks() {
    let (document, id, painted, _) = fixture();
    let exported = document.export_svg_report(1, &[]);
    assert!(exported.warnings.is_empty());
    let xml = roxmltree::Document::parse(&exported.source).unwrap();
    let inherited = xml_id(&xml, &format!("network-inherited-cut-{id}"));
    assert_eq!(inherited.attribute("mask-type"), Some("luminance"));
    assert_eq!(inherited.attribute("maskUnits"), Some("userSpaceOnUse"));
    assert_eq!(inherited.attribute("maskContentUnits"), Some("userSpaceOnUse"));
    let explicit = xml_id(&xml, &format!("network-region-cut-{id}-{painted}"));
    assert_eq!(explicit.children().filter(|node| node.has_tag_name("use")).count(), 1);
    let imported = Document::from_svg(&exported.source, "Imported regions").unwrap();
    assert!(imported.nodes.values().any(|node| node.is_mask && node.mask_type == MaskType::Luminance));
    imported.validate().unwrap();
    assert_eq!(Document::from_json(&imported.to_json().unwrap()).unwrap().to_json().unwrap(), imported.to_json().unwrap());
}

#[test]
fn region_svg_paint_stacks_keep_first_entry_topmost() {
    let (mut document, id, painted, _) = fixture();
    document.nodes.get_mut(&id).unwrap().vector_network.as_mut().unwrap().regions.get_mut(&painted).unwrap().fills = Some(vec![
        Fill::solid(Color::rgba(255, 0, 0, 128)), Fill::solid(Color::rgb(0, 255, 0)),
    ]);
    let source = document.export_svg(1, &[]);
    let green = source.rfind("fill=\"#00FF00\"").unwrap();
    let red = source.rfind("fill=\"#FF0000\"").unwrap();
    assert!(green < red, "bottom paint must precede the translucent top paint");
    let imported = Document::from_svg(&source, "Stack").unwrap();
    assert!(imported.nodes.values().any(|node| node.fills.iter().any(|fill| fill.paint == Paint::Solid(Color::rgb(0, 255, 0)))));
}

#[test]
fn region_svg_angular_brushes_use_original_region_and_aggregate_bounds() {
    let (mut document, id, painted, _) = fixture();
    let angular = Fill { paint: Paint::AngularGradient { center: Point::new(0.5, 0.5), angle: 0.0,
        stops: vec![GradientStop { offset: 0.0, color: Color::BLACK }, GradientStop { offset: 1.0, color: Color::WHITE }] }, visible: true, opacity: 1.0 };
    let node = document.nodes.get_mut(&id).unwrap();
    node.fills = vec![angular.clone()];
    node.vector_network.as_mut().unwrap().regions.get_mut(&painted).unwrap().fills = Some(vec![angular]);
    let exported = document.export_svg_report(1, &[]);
    assert_eq!(exported.warnings.len(), 1);
    let xml = roxmltree::Document::parse(&exported.source).unwrap();
    let inherited = xml_id(&xml, &format!("fill-{id}-0"));
    assert_eq!(inherited.attribute("viewBox"), Some("10 10 120 80"));
    let region = xml_id(&xml, &format!("region-{id}-{painted}-0"));
    assert_eq!(region.attribute("viewBox"), Some("30 20 100 50"));
    let sector = region.descendants().find(|node| node.has_tag_name("path")).unwrap();
    assert!(sector.attribute("d").unwrap().starts_with("M80 45 "));
}

#[test]
fn region_svg_image_servers_use_region_origin_and_source_pixel_tile_size() {
    let (mut document, id, painted, _) = fixture();
    let image = Fill { paint: Paint::Image { href: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAADCAYAAAC56t6BAAAA".into(), scale: ImageScale::Stretch, crop: None }, visible: true, opacity: 1.0 };
    document.nodes.get_mut(&id).unwrap().vector_network.as_mut().unwrap().regions.get_mut(&painted).unwrap().fills = Some(vec![image]);
    let source = document.export_svg(1, &[]);
    let xml = roxmltree::Document::parse(&source).unwrap();
    let pattern = xml_id(&xml, &format!("region-{id}-{painted}-0"));
    assert_eq!(pattern.attribute("x"), Some("30"));
    assert_eq!(pattern.attribute("y"), Some("20"));
    assert_eq!(pattern.attribute("width"), Some("100"));
    assert_eq!(pattern.attribute("height"), Some("50"));
    assert_eq!(pattern.attribute("viewBox"), Some("30 20 100 50"));
    let viewport = pattern.children().find(|node| node.has_tag_name("svg")).unwrap();
    assert_eq!(viewport.attribute("x"), Some("30"));
    assert_eq!(viewport.attribute("y"), Some("20"));
    assert_eq!(viewport.attribute("preserveAspectRatio"), Some("none"));
    let node = document.nodes.get_mut(&id).unwrap();
    if let Paint::Image { scale, .. } = &mut node.vector_network.as_mut().unwrap().regions.get_mut(&painted).unwrap().fills.as_mut().unwrap()[0].paint { *scale = ImageScale::Tile; }
    let source = document.export_svg(1, &[]);
    let xml = roxmltree::Document::parse(&source).unwrap();
    let pattern = xml_id(&xml, &format!("region-{id}-{painted}-0"));
    assert_eq!(pattern.attribute("width"), Some("2"));
    assert_eq!(pattern.attribute("height"), Some("3"));
}

#[test]
fn region_svg_vector_masks_ignore_zero_alpha_and_whiten_visible_coverage() {
    let (mut document, id, painted, _) = fixture();
    let node = document.nodes.get_mut(&id).unwrap();
    node.is_mask = true;
    node.mask_type = MaskType::Vector;
    node.fills[0].paint = Paint::Solid(Color::TRANSPARENT);
    let fills = node.vector_network.as_mut().unwrap().regions.get_mut(&painted).unwrap().fills.as_mut().unwrap();
    fills.push(Fill { paint: Paint::Solid(Color::rgb(0, 255, 0)), visible: true, opacity: 0.0 });
    let mut zero_opacity = Stroke::new(Color::BLACK, 10.0);
    zero_opacity.opacity = 0.0;
    node.strokes = vec![Stroke::new(Color::TRANSPARENT, 8.0), zero_opacity];
    document.insert_node(1, Node::new(NodeKind::Rectangle, "Masked", 0.0, 0.0, 180.0, 100.0)).unwrap();
    let source = document.export_svg(1, &[]);
    let xml = roxmltree::Document::parse(&source).unwrap();
    let source = xml_id(&xml, &format!("layer-mask-{id}"));
    let painted_paths: Vec<_> = source.descendants().filter(|node| node.has_tag_name("path") && node.attribute("fill") == Some("#FFFFFF")).collect();
    assert_eq!(painted_paths.len(), 1);
    assert_eq!(painted_paths[0].attribute("fill-opacity"), Some("1"));
    assert!(!source.descendants().any(|node| node.attribute("fill") == Some("#FF0000")));
    assert!(!source.descendants().any(|node| node.attribute("stroke") == Some("#FFFFFF")),
        "zero-alpha and zero-opacity strokes contribute no native vector-mask geometry");
    Document::from_svg(&document.export_svg(1, &[]), "Vector mask").unwrap();
}

#[test]
fn region_svg_cut_tree_stays_linear_in_path_definitions_and_logarithmic_in_reference_depth() {
    let mut network = VectorNetwork::default();
    for index in 0..128 {
        let region = square(&mut network, index as f64, 0.0, 32.0, 32.0);
        network.regions.get_mut(&region).unwrap().fills = Some(vec![Fill::solid(Color::BLACK)]);
    }
    let mut node = Node::new(NodeKind::Vector, "Many regions", 0.0, 0.0, 160.0, 32.0);
    node.vector_network = Some(network);
    let mut document = Document::new("Bounded export");
    document.insert_node(1, node).unwrap();
    let source = document.export_svg(1, &[]);
    let xml = roxmltree::Document::parse(&source).unwrap();
    assert_eq!(xml.descendants().filter(|node| node.has_tag_name("path")).count(), 256);
    assert!(xml.descendants().filter(|node| node.has_tag_name("use")).count() < 128 * 8);
    assert!(source.len() < 160_000);
}
