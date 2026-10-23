//! Bounds pruning and bounded Boolean depth, compared with exhaustive cutters.
use super::*;
use ferrodesign_core::{Fill, GradientStop, NetworkCurve, NetworkEdge, NetworkEdgeRef,
    NetworkRegion, NetworkVertex, VectorNetwork, VectorRegionId};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

// Build the large fixture directly from the public model: interactive connect
// checks duplicate edges, which is separate from renderer preparation cost.
fn contour(network: &mut VectorNetwork, points: &[Point]) -> Vec<NetworkEdgeRef> {
    let start = network.next_id;
    for (offset, &position) in points.iter().enumerate() {
        network.vertices.insert(start + offset as u64, NetworkVertex { position, ..Default::default() });
    }
    network.next_id += points.len() as u64;
    let mut edges = Vec::with_capacity(points.len());
    for offset in 0..points.len() {
        let id = network.next_id;
        network.next_id += 1;
        network.edges.insert(id, NetworkEdge {
            start: start + offset as u64,
            end: start + ((offset + 1) % points.len()) as u64,
            curve: NetworkCurve::Line,
            stroke: true,
        });
        edges.push(NetworkEdgeRef::forward(id));
    }
    edges
}

fn rectangle(network: &mut VectorNetwork, rect: Rect) -> Vec<NetworkEdgeRef> {
    contour(network, &rect.corners())
}

fn region(network: &mut VectorNetwork, contours: Vec<Vec<NetworkEdgeRef>>, fills: Option<Vec<Fill>>) -> VectorRegionId {
    let id = network.next_id;
    network.next_id += 1;
    network.regions.insert(id, NetworkRegion { contours, fills });
    id
}

fn node(network: VectorNetwork, fills: Vec<Fill>) -> Node {
    let mut node = Node::new(NodeKind::Vector, "Region broad phase", 0.0, 0.0, 128.0, 128.0);
    node.vector_network = Some(network);
    node.fills = fills;
    node
}

fn prepared(node: &Node) -> (CachedNetworkFills, Ref<Geometry>, Vec<Rc<dyn IBrush>>) {
    let geometry = shape_geometry(node);
    let mut images = HashMap::new();
    let mut missing = 0;
    let cached = prepare(node, &geometry, &mut images, &mut missing).unwrap();
    let brushes = node.fills.iter().filter(|fill| fill.visible && fill.opacity > 0.0)
        .map(|fill| {
            let (brush, absent) = make_brush(&fill.paint, fill.opacity, &mut images);
            assert!(!absent);
            brush as Rc<dyn IBrush>
        }).collect();
    assert_eq!(missing, 0);
    (cached, geometry, brushes)
}

/// Previous exhaustive suffix semantics, independent of spatial pruning.
fn exhaustive(node: &Node) -> CachedNetworkFills {
    let (mut cached, geometry, _) = prepared(node);
    let mut later = None;
    for region in cached.regions.iter_mut().rev() {
        region.clip = later.as_ref().map(|later: &Ref<Geometry>| {
            combined(GeometryCombineMode::Exclude, region.geometry.clone(), later.clone())
        });
        later = Some(match later {
            Some(later) => combined(GeometryCombineMode::Union, later, region.geometry.clone()),
            None => region.geometry.clone(),
        });
    }
    let inherited = path_geometry(&node.vector_network.as_ref().unwrap().inherited_fill_path());
    cached.inherited_clip = later.map_or(geometry, |later| {
        combined(GeometryCombineMode::Exclude, inherited, later)
    });
    cached
}

fn pixels(node: &Node, cached: &CachedNetworkFills, geometry: &Ref<Geometry>, fills: &[Rc<dyn IBrush>], mask: bool) -> Vec<u8> {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(128, 128));
    {
        let mut context = bitmap.create_drawing_context();
        let transform = context.push_transform(ui_matrix(node.local_transform()));
        if mask {
            context.draw_geometry(Some(&solid_brush(Color::WHITE, 1.0)), None, &cached.vector_mask(node, geometry));
        } else {
            cached.paint(&mut context, geometry, fills);
        }
        context.pop(transform);
    }
    let mut pixels = vec![0; 128 * 128 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 128, 128), &mut pixels, 128 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    pixels
}

fn tree_depth(root: Ref<Geometry>) -> usize {
    let mut stack = vec![(root, 1)];
    let mut maximum = 0;
    while let Some((geometry, depth)) = stack.pop() {
        maximum = maximum.max(depth);
        if let Some(combined) = geometry.cast::<CombinedGeometry>() {
            for child in [combined.geometry1(), combined.geometry2()].into_iter().flatten() {
                stack.push((child, depth + 1));
            }
        }
    }
    maximum
}

#[test]
fn region_scaling_prunes_ten_thousand_disjoint_regions_in_both_axes() {
    const COUNT: usize = 10_000;
    for vertical in [true, false] {
        let bounds: Vec<_> = (0..COUNT).map(|index| Some(if vertical {
            Rect::new(0.0, index as f64 * 4.0, 2.0, 2.0)
        } else {
            Rect::new((index % 100) as f64 * 4.0, (index / 100) as f64 * 4.0, 2.0, 2.0)
        })).collect();
        let index = RegionBoundsIndex::new(&bounds);
        let mut matches = Vec::new();
        for (slot, &rect) in bounds.iter().enumerate() {
            index.later_intersections(rect, slot, &mut matches);
            assert!(matches.is_empty());
        }
        assert!(index.bounds_tests.get() < COUNT * 64,
            "disjoint regions must not require pairwise bounds comparisons: {}", index.bounds_tests.get());
        assert_eq!(index.entries.len(), COUNT * 2 - 1);
    }
}

#[test]
fn region_scaling_ten_thousand_prepared_faces_have_logarithmic_boolean_depth() {
    let scope = backend();
    const COUNT: usize = 10_000;
    let mut network = VectorNetwork::default();
    for index in 0..COUNT {
        let boundary = rectangle(&mut network, Rect::new(2.0, index as f64 * 4.0 + 2.0, 2.0, 2.0));
        region(&mut network, vec![boundary], Some(vec![Fill::solid(Color::BLACK)]));
    }
    network.validate().unwrap();
    let node = node(network, Vec::new());
    let (cached, geometry, _) = prepared(&node);
    assert_eq!(cached.regions.len(), COUNT);
    assert_eq!(cached.cutter_pairs, 0);
    assert!(cached.bounds_tests < COUNT * 64);
    assert!(cached.regions[..COUNT - 1].iter().all(|region| {
        region.clip.as_ref().is_some_and(|clip| clip.ptr_eq(&region.geometry))
    }), "disjoint regions retain native clip coverage without Boolean subtraction");
    let logarithm = COUNT.next_power_of_two().trailing_zeros() as usize;
    assert!(tree_depth(cached.inherited_clip.clone()) <= logarithm + 2);
    assert!(tree_depth(cached.vector_mask(&node, &geometry)) <= logarithm + 1);
    // Actual public platform evaluation of a substantial balanced subtree also
    // checks the lazy Boolean path, without a wall-clock threshold.
    let sample: Vec<_> = cached.regions.iter().take(256).map(|region| region.geometry.clone()).collect();
    let union = balanced_union(&sample).unwrap();
    let bounds = union.bounds();
    assert!((bounds.height - 1022.0).abs() < 1e-6);
    assert!(union.fill_contains(UiPoint::new(3.0, 3.0)));
    assert!(!union.fill_contains(UiPoint::new(3.0, 5.0)));
    scope.dispose();
}

#[test]
fn region_scaling_bounds_include_touches_and_unknowns_but_reject_definite_gaps() {
    let bounds = [Some(Rect::new(0.0, 0.0, 10.0, 10.0)),
        Some(Rect::new(10.0, 0.0, 10.0, 10.0)),
        Some(Rect::new(20.0 + 1e-8, 0.0, 10.0, 10.0)), None];
    let index = RegionBoundsIndex::new(&bounds);
    let mut matches = Vec::new();
    index.later_intersections(bounds[0], 0, &mut matches);
    assert_eq!(matches, vec![1, 3]);
    index.later_intersections(bounds[1], 1, &mut matches);
    assert_eq!(matches, vec![3]);
    index.later_intersections(None, 0, &mut matches);
    assert_eq!(matches, vec![1, 2, 3]);
    assert!(finite_bounds(Rect::new(f64::MAX, 0.0, f64::MAX, 1.0)).is_none());
}

#[test]
fn region_scaling_pruned_curves_and_gradient_bounds_match_exhaustive_pixels() {
    let scope = backend();
    let gradient = Fill { paint: Paint::LinearGradient {
        start: Point::new(0.0, 0.0), end: Point::new(1.0, 1.0),
        stops: vec![GradientStop { offset: 0.0, color: Color::rgba(255, 0, 0, 128) },
            GradientStop { offset: 1.0, color: Color::rgba(0, 0, 255, 180) }],
    }, visible: true, opacity: 0.8 };
    let mut network = VectorNetwork::default();
    for index in 0..3 {
        let x = 8.25 + index as f64 * 34.0;
        let boundary = rectangle(&mut network, Rect::new(x, 30.25, 24.0, 30.0));
        network.edges.get_mut(&boundary[0].edge).unwrap().curve = NetworkCurve::Cubic {
            control1: Point::new(x + 4.0, 6.25), control2: Point::new(x + 20.0, 6.25),
        };
        region(&mut network, vec![boundary], Some(vec![gradient.clone()]));
    }
    let mut node = node(network, Vec::new());
    node.rotation = 5.0;
    node.affine_transform = Some(Transform { a: 1.0, b: 0.0, c: 0.1, d: 1.0, e: 0.0, f: 0.0 });
    let (cached, geometry, fills) = prepared(&node);
    assert_eq!(cached.cutter_pairs, 0);
    let reference = exhaustive(&node);
    assert_eq!(pixels(&node, &cached, &geometry, &fills, false), pixels(&node, &reference, &geometry, &fills, false));
    assert_eq!(pixels(&node, &cached, &geometry, &fills, true), pixels(&node, &reference, &geometry, &fills, true));
    scope.dispose();
}

#[test]
fn region_scaling_overlaps_holes_empty_and_hidden_cutters_match_exhaustive_pixels() {
    let scope = backend();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let mut network = VectorNetwork { fill_rule: rule, ..Default::default() };
        let inherited = rectangle(&mut network, Rect::new(4.0, 4.0, 114.0, 114.0));
        region(&mut network, vec![inherited], None);
        let outer = rectangle(&mut network, Rect::new(10.25, 10.25, 76.0, 70.0));
        let mut points = Rect::new(25.25, 25.25, 20.0, 20.0).corners();
        points.reverse();
        let hole = contour(&mut network, &points);
        region(&mut network, vec![outer, hole], Some(vec![Fill::solid(Color::rgba(255, 0, 0, 128))]));
        let touching = contour(&mut network, &[Point::new(86.25, 10.25), Point::new(110.25, 10.25), Point::new(86.25, 80.25)]);
        region(&mut network, vec![touching], Some(vec![Fill::solid(Color::rgba(0, 255, 0, 128))]));
        let eraser = rectangle(&mut network, Rect::new(50.25, 30.25, 50.0, 20.0));
        region(&mut network, vec![eraser], Some(Vec::new()));
        let hidden = rectangle(&mut network, Rect::new(14.25, 14.25, 10.0, 10.0));
        region(&mut network, vec![hidden], Some(vec![Fill { visible: false, ..Fill::solid(Color::BLACK) }]));
        let node = node(network, vec![Fill::solid(Color::rgba(0, 0, 255, 100))]);
        let (cached, geometry, fills) = prepared(&node);
        let reference = exhaustive(&node);
        assert!(cached.cutter_pairs > 0);
        assert_eq!(pixels(&node, &cached, &geometry, &fills, false), pixels(&node, &reference, &geometry, &fills, false));
        assert_eq!(pixels(&node, &cached, &geometry, &fills, true), pixels(&node, &reference, &geometry, &fills, true));
    }
    scope.dispose();
}
