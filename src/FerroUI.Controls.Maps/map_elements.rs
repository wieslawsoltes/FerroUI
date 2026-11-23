//! What is drawn over the tiles.

use crate::projection::{project, world_size, BasicGeoposition, MapView};
use ferroui_base::media::Color;
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The radius of an icon, in pixels.
pub(crate) const ICON_RADIUS: f64 = 7.0;
/// How far from an element, in pixels, a click still counts as on it.
const HIT_TOLERANCE: f64 = 4.0;

/// A marker at a place, with an optional title.
#[derive(Clone, Debug, PartialEq)]
pub struct MapIcon {
    pub location: BasicGeoposition,
    pub title: Option<String>,
    pub color: Color,
}

impl MapIcon {
    pub fn new(location: BasicGeoposition) -> Self {
        Self {
            location,
            title: None,
            color: Color::from_rgb(0xD1, 0x34, 0x38),
        }
    }
}

/// A line through places.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPolyline {
    pub path: Vec<BasicGeoposition>,
    /// What assistive technology calls the line.
    pub title: Option<String>,
    pub stroke: Color,
    pub stroke_thickness: f64,
}

impl MapPolyline {
    pub fn new(path: Vec<BasicGeoposition>) -> Self {
        Self {
            path,
            title: None,
            stroke: Color::from_rgb(0x00, 0x78, 0xD4),
            stroke_thickness: 3.0,
        }
    }
}

/// An area bounded by places.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPolygon {
    pub path: Vec<BasicGeoposition>,
    /// What assistive technology calls the area.
    pub title: Option<String>,
    pub fill: Color,
    pub stroke: Color,
    pub stroke_thickness: f64,
}

impl MapPolygon {
    pub fn new(path: Vec<BasicGeoposition>) -> Self {
        Self {
            path,
            title: None,
            fill: Color::from_argb(0x55, 0x00, 0x78, 0xD4),
            stroke: Color::from_rgb(0x00, 0x78, 0xD4),
            stroke_thickness: 2.0,
        }
    }
}

/// One thing on a map.
#[derive(Clone, Debug, PartialEq)]
pub enum MapElement {
    Icon(MapIcon),
    Polyline(MapPolyline),
    Polygon(MapPolygon),
}

impl MapElement {
    fn title(&self) -> Option<&str> {
        match self {
            Self::Icon(icon) => icon.title.as_deref(),
            Self::Polyline(line) => line.title.as_deref(),
            Self::Polygon(polygon) => polygon.title.as_deref(),
        }
    }

    fn first_place(&self) -> Option<BasicGeoposition> {
        match self {
            Self::Icon(icon) => Some(icon.location),
            Self::Polyline(line) => line.path.first().copied(),
            Self::Polygon(polygon) => polygon.path.first().copied(),
        }
    }

    /// Whether it has anything to focus: a line or an area may have an empty path.
    pub(crate) fn has_place(&self) -> bool {
        self.first_place().is_some()
    }

    /// The name of the kind of element, for automation.
    pub(crate) fn class_name(&self) -> &'static str {
        match self {
            Self::Icon(_) => "MapIcon",
            Self::Polyline(_) => "MapPolyline",
            Self::Polygon(_) => "MapPolygon",
        }
    }

    /// Its title, or what it is and where.
    pub(crate) fn accessible_name(&self) -> String {
        if let Some(title) = self.title().filter(|title| !title.trim().is_empty()) {
            return title.to_owned();
        }
        let kind = match self {
            Self::Icon(_) => "Map icon",
            Self::Polyline(_) => "Map line",
            Self::Polygon(_) => "Map area",
        };
        match self.first_place() {
            Some(place) => format!("{kind} at {:.3}, {:.3}", place.latitude, place.longitude),
            None => kind.to_owned(),
        }
    }

    /// The point of the view that stands for the element: an icon, the middle point of a line,
    /// the average of the corners of an area.
    pub(crate) fn view_anchor(&self, view: &MapView) -> Option<(f64, f64)> {
        match self {
            Self::Icon(icon) => Some(view_path(&[icon.location], view)[0]),
            Self::Polyline(line) => {
                let points = view_path(&line.path, view);
                points.get(points.len() / 2).copied()
            }
            Self::Polygon(polygon) => {
                let points = view_path(&polygon.path, view);
                if points.is_empty() {
                    return None;
                }
                let count = points.len() as f64;
                let sum = points
                    .iter()
                    .fold((0.0, 0.0), |sum, point| (sum.0 + point.0, sum.1 + point.1));
                Some((sum.0 / count, sum.1 / count))
            }
        }
    }

    /// Left, top, right and bottom of what is drawn, in pixels of the view.
    pub(crate) fn view_bounds(&self, view: &MapView) -> Option<(f64, f64, f64, f64)> {
        let (points, reach) = match self {
            Self::Icon(icon) => (view_path(&[icon.location], view), ICON_RADIUS),
            Self::Polyline(line) => (view_path(&line.path, view), line.stroke_thickness / 2.0),
            Self::Polygon(polygon) => (
                view_path(&polygon.path, view),
                polygon.stroke_thickness / 2.0,
            ),
        };
        let first = *points.first()?;
        let (left, top, right, bottom) =
            points
                .iter()
                .fold((first.0, first.1, first.0, first.1), |bounds, point| {
                    (
                        bounds.0.min(point.0),
                        bounds.1.min(point.1),
                        bounds.2.max(point.0),
                        bounds.3.max(point.1),
                    )
                });
        Some((left - reach, top - reach, right + reach, bottom + reach))
    }
}

/// What a click on a map element reports.
#[derive(Clone, Debug, PartialEq)]
pub struct MapElementClickEventArgs {
    /// The element, the topmost at the point.
    pub element: MapElement,
    /// The place that was clicked.
    pub location: BasicGeoposition,
}

/// A group of elements, drawn in order. A change redraws the maps that show the layer.
pub struct MapElementsLayer {
    elements: RefCell<Vec<MapElement>>,
    is_hidden: Cell<bool>,
    on_changed: RefCell<Vec<Rc<dyn Fn()>>>,
    map_element_click: Event<MapElementClickEventArgs>,
}

impl MapElementsLayer {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            elements: RefCell::new(Vec::new()),
            is_hidden: Cell::new(false),
            on_changed: RefCell::new(Vec::new()),
            map_element_click: Event::new(),
        })
    }

    pub fn elements(&self) -> Vec<MapElement> {
        self.elements.borrow().clone()
    }

    /// How many elements the layer has.
    pub fn len(&self) -> usize {
        self.elements.borrow().len()
    }

    /// Whether the layer has no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.borrow().is_empty()
    }

    /// The element at `index`.
    pub fn element(&self, index: usize) -> Option<MapElement> {
        self.elements.borrow().get(index).cloned()
    }

    /// Whether the element at `index` can have the keyboard focus. Reads it in place.
    pub(crate) fn can_focus(&self, index: usize) -> bool {
        self.is_visible()
            && self
                .elements
                .borrow()
                .get(index)
                .is_some_and(MapElement::has_place)
    }

    pub fn add(&self, element: MapElement) {
        self.elements.borrow_mut().push(element);
        self.changed();
    }

    pub fn clear(&self) {
        self.elements.borrow_mut().clear();
        self.changed();
    }

    pub fn is_visible(&self) -> bool {
        !self.is_hidden.get()
    }

    pub fn set_is_visible(&self, value: bool) {
        if self.is_hidden.replace(!value) == value {
            self.changed();
        }
    }

    /// Raised when the user clicks or taps an element of this layer.
    pub fn map_element_click(&self) -> &Event<MapElementClickEventArgs> {
        &self.map_element_click
    }

    pub(crate) fn raise_click(&self, args: &MapElementClickEventArgs) {
        self.map_element_click.raise(args);
    }

    pub(crate) fn add_listener(&self, action: Rc<dyn Fn()>) {
        self.on_changed.borrow_mut().push(action);
    }

    pub(crate) fn remove_listener(&self, action: &Rc<dyn Fn()>) {
        self.on_changed
            .borrow_mut()
            .retain(|listener| !Rc::ptr_eq(listener, action));
    }

    fn changed(&self) {
        let listeners = self.on_changed.borrow().clone();
        for listener in listeners {
            listener();
        }
    }
}

/// The next element that can have the focus, as positions of layer and element, in the order
/// they are drawn. Going back from nothing, or past either end, gives `None`.
pub(crate) fn neighbour(
    layers: &[Rc<MapElementsLayer>],
    from: Option<(usize, usize)>,
    forward: bool,
) -> Option<(usize, usize)> {
    if forward {
        let (mut layer, mut index) = from.map_or((0, 0), |(layer, index)| (layer, index + 1));
        while layer < layers.len() {
            while index < layers[layer].len() {
                if layers[layer].can_focus(index) {
                    return Some((layer, index));
                }
                index += 1;
            }
            layer += 1;
            index = 0;
        }
        None
    } else {
        let (mut layer, mut end) = from?;
        loop {
            while end > 0 {
                end -= 1;
                if layers[layer].can_focus(end) {
                    return Some((layer, end));
                }
            }
            layer = layer.checked_sub(1)?;
            end = layers[layer].len();
        }
    }
}

/// Places as pixels of a view, one path: each point takes the copy of the repeating world nearest
/// the point before it, so a line across the date line stays one short line.
pub(crate) fn view_path(path: &[BasicGeoposition], view: &MapView) -> Vec<(f64, f64)> {
    let world = world_size(view.zoom);
    let (center_x, center_y) = project(view.center, view.zoom);
    let mut points = Vec::with_capacity(path.len());
    let mut previous: Option<f64> = None;
    for place in path {
        let (x, y) = project(*place, view.zoom);
        let near = previous.unwrap_or(center_x);
        let x = x - world * ((x - near) / world).round();
        previous = Some(x);
        points.push(view.frame_to_view(
            x - center_x + view.width / 2.0,
            y - center_y + view.height / 2.0,
        ));
    }
    points
}

fn distance_to_segment(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let length = dx * dx + dy * dy;
    let along = if length == 0.0 {
        0.0
    } else {
        (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length).clamp(0.0, 1.0)
    };
    (point.0 - start.0 - along * dx).hypot(point.1 - start.1 - along * dy)
}

/// Whether `point` is inside a closed path, by the even-odd rule.
fn is_inside(point: (f64, f64), path: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut previous = match path.last() {
        Some(last) => *last,
        None => return false,
    };
    for &current in path {
        if (current.1 > point.1) != (previous.1 > point.1) {
            let crossing = current.0
                + (point.1 - current.1) / (previous.1 - current.1) * (previous.0 - current.0);
            if point.0 < crossing {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

/// Whether an element, as it is drawn on `view`, is at `point`.
pub(crate) fn hits(element: &MapElement, view: &MapView, point: (f64, f64)) -> bool {
    let near_line = |path: &[(f64, f64)], thickness: f64, is_closed: bool| {
        let reach = thickness / 2.0 + HIT_TOLERANCE;
        let open = path
            .windows(2)
            .any(|pair| distance_to_segment(point, pair[0], pair[1]) <= reach);
        let closing = is_closed
            && path.len() > 2
            && distance_to_segment(point, path[path.len() - 1], path[0]) <= reach;
        open || closing
    };
    match element {
        MapElement::Icon(icon) => {
            let at = view_path(&[icon.location], view)[0];
            (at.0 - point.0).hypot(at.1 - point.1) <= ICON_RADIUS + HIT_TOLERANCE
        }
        MapElement::Polyline(line) => {
            near_line(&view_path(&line.path, view), line.stroke_thickness, false)
        }
        MapElement::Polygon(polygon) => {
            let path = view_path(&polygon.path, view);
            is_inside(point, &path) || near_line(&path, polygon.stroke_thickness, true)
        }
    }
}

/// The topmost element at `point`, with its layer: later layers and later elements are on top.
pub(crate) fn hit_test(
    layers: &[Rc<MapElementsLayer>],
    view: &MapView,
    point: (f64, f64),
) -> Option<(Rc<MapElementsLayer>, MapElement)> {
    for layer in layers.iter().rev().filter(|layer| layer.is_visible()) {
        let elements = layer.elements();
        if let Some(element) = elements
            .iter()
            .rev()
            .find(|element| hits(element, view, point))
        {
            return Some((layer.clone(), element.clone()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(latitude: f64, longitude: f64) -> BasicGeoposition {
        BasicGeoposition {
            latitude,
            longitude,
        }
    }

    fn view(center: BasicGeoposition, zoom: f64, heading: f64) -> MapView {
        MapView::new(center, zoom, heading, 400.0, 300.0)
    }

    #[test]
    fn the_center_is_the_middle_of_the_view() {
        let center = place(10.0, 20.0);
        for heading in [0.0, 77.0] {
            let points = view_path(&[center], &view(center, 5.0, heading));
            assert!((points[0].0 - 200.0).abs() < 1e-6);
            assert!((points[0].1 - 150.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_line_across_the_date_line_stays_short() {
        let path = [place(0.0, 179.0), place(0.0, -179.0)];
        let points = view_path(&path, &view(place(0.0, 179.5), 3.0, 0.0));
        let degrees = (points[1].0 - points[0].0) / world_size(3.0) * 360.0;
        assert!((degrees - 2.0).abs() < 1e-6, "{degrees}");
        let turned = view_path(&path, &view(place(0.0, 179.5), 3.0, 90.0));
        let length = (turned[1].0 - turned[0].0).hypot(turned[1].1 - turned[0].1);
        assert!(
            (length / world_size(3.0) * 360.0 - 2.0).abs() < 1e-6,
            "{length}"
        );
    }

    #[test]
    fn a_place_takes_the_copy_of_the_world_near_the_view() {
        let points = view_path(&[place(0.0, -179.0)], &view(place(0.0, 179.0), 2.0, 0.0));
        let expected = 200.0 + world_size(2.0) * 2.0 / 360.0;
        assert!((points[0].0 - expected).abs() < 1e-6);
    }

    #[test]
    fn an_icon_is_hit_where_it_is_drawn_at_any_heading() {
        let center = place(0.0, 0.0);
        let icon = MapElement::Icon(MapIcon::new(place(10.0, 15.0)));
        for heading in [0.0, 40.0, 90.0, 222.0] {
            let view = view(center, 4.0, heading);
            let at = view.place_to_view(place(10.0, 15.0));
            assert!(hits(&icon, &view, at));
            assert!(hits(
                &icon,
                &view,
                (at.0 + ICON_RADIUS + HIT_TOLERANCE - 0.5, at.1)
            ));
            assert!(!hits(
                &icon,
                &view,
                (at.0 + ICON_RADIUS + HIT_TOLERANCE + 0.5, at.1)
            ));
        }
    }

    #[test]
    fn a_line_is_hit_along_its_length_only() {
        let line =
            MapElement::Polyline(MapPolyline::new(vec![place(0.0, -10.0), place(0.0, 10.0)]));
        let view = view(place(0.0, 0.0), 4.0, 60.0);
        assert!(hits(&line, &view, view.place_to_view(place(0.0, 5.0))));
        assert!(!hits(&line, &view, view.place_to_view(place(8.0, 5.0))));
        assert!(!hits(&line, &view, view.place_to_view(place(0.0, 14.0))));
    }

    #[test]
    fn a_polygon_is_hit_inside_and_on_its_closing_edge() {
        let square = vec![
            place(-10.0, -10.0),
            place(-10.0, 10.0),
            place(10.0, 10.0),
            place(10.0, -10.0),
        ];
        let polygon = MapElement::Polygon(MapPolygon::new(square));
        for heading in [0.0, 33.0, 180.0] {
            let view = view(place(0.0, 0.0), 4.0, heading);
            assert!(hits(&polygon, &view, view.place_to_view(place(2.0, 3.0))));
            assert!(
                hits(&polygon, &view, view.place_to_view(place(0.0, -10.0))),
                "the closing edge"
            );
            assert!(!hits(&polygon, &view, view.place_to_view(place(0.0, 16.0))));
        }
    }

    #[test]
    fn the_topmost_element_is_found_and_a_hidden_layer_is_not() {
        let under = MapElementsLayer::new();
        let over = MapElementsLayer::new();
        let first = MapIcon::new(place(1.0, 1.0));
        let mut second = first.clone();
        second.title = Some("on top".to_owned());
        under.add(MapElement::Icon(first.clone()));
        over.add(MapElement::Icon(first));
        over.add(MapElement::Icon(second.clone()));
        let layers = [under.clone(), over.clone()];
        let view = view(place(0.0, 0.0), 4.0, 25.0);
        let at = view.place_to_view(place(1.0, 1.0));
        let (layer, element) = hit_test(&layers, &view, at).unwrap();
        assert!(Rc::ptr_eq(&layer, &over));
        assert_eq!(element, MapElement::Icon(second));
        over.set_is_visible(false);
        assert!(Rc::ptr_eq(&hit_test(&layers, &view, at).unwrap().0, &under));
        under.set_is_visible(false);
        assert!(hit_test(&layers, &view, at).is_none());
    }

    #[test]
    fn focus_moves_through_the_elements_in_drawing_order() {
        let first = MapElementsLayer::new();
        first.add(MapElement::Icon(MapIcon::new(place(0.0, 0.0))));
        first.add(MapElement::Polyline(MapPolyline::new(Vec::new())));
        first.add(MapElement::Icon(MapIcon::new(place(1.0, 1.0))));
        let hidden = MapElementsLayer::new();
        hidden.add(MapElement::Icon(MapIcon::new(place(2.0, 2.0))));
        hidden.set_is_visible(false);
        let empty = MapElementsLayer::new();
        let last = MapElementsLayer::new();
        last.add(MapElement::Icon(MapIcon::new(place(3.0, 3.0))));
        let layers = [first, hidden, empty, last];

        let mut seen = Vec::new();
        let mut at = None;
        while let Some(next) = neighbour(&layers, at, true) {
            seen.push(next);
            at = Some(next);
        }
        assert_eq!(
            seen,
            [(0, 0), (0, 2), (3, 0)],
            "an empty path and a hidden layer are passed"
        );

        let mut back = Vec::new();
        while let Some(previous) = neighbour(&layers, at, false) {
            back.push(previous);
            at = Some(previous);
        }
        assert_eq!(back, [(0, 2), (0, 0)]);
        assert_eq!(neighbour(&layers, None, false), None);
    }

    #[test]
    fn an_element_is_named_by_its_title_or_by_what_and_where_it_is() {
        let mut icon = MapIcon::new(place(1.5, -2.25));
        assert_eq!(
            MapElement::Icon(icon.clone()).accessible_name(),
            "Map icon at 1.500, -2.250"
        );
        icon.title = Some("   ".to_owned());
        assert_eq!(
            MapElement::Icon(icon.clone()).accessible_name(),
            "Map icon at 1.500, -2.250"
        );
        icon.title = Some("Harbour".to_owned());
        assert_eq!(MapElement::Icon(icon).accessible_name(), "Harbour");
        let mut line = MapPolyline::new(vec![place(0.0, 1.0)]);
        assert_eq!(
            MapElement::Polyline(line.clone()).accessible_name(),
            "Map line at 0.000, 1.000"
        );
        line.title = Some("Ferry".to_owned());
        assert_eq!(MapElement::Polyline(line).accessible_name(), "Ferry");
        assert_eq!(
            MapElement::Polygon(MapPolygon::new(Vec::new())).accessible_name(),
            "Map area"
        );
    }

    #[test]
    fn the_bounds_of_an_element_include_what_is_drawn() {
        let view = view(place(0.0, 0.0), 4.0, 0.0);
        let icon = MapElement::Icon(MapIcon::new(place(0.0, 0.0)));
        assert_eq!(icon.view_bounds(&view), Some((193.0, 143.0, 207.0, 157.0)));
        let line =
            MapElement::Polyline(MapPolyline::new(vec![place(0.0, -10.0), place(0.0, 10.0)]));
        let (left, top, right, bottom) = line.view_bounds(&view).unwrap();
        assert!(
            left < 200.0
                && right > 200.0
                && (top - 148.5).abs() < 1e-6
                && (bottom - 151.5).abs() < 1e-6
        );
        assert_eq!(
            MapElement::Polygon(MapPolygon::new(Vec::new())).view_bounds(&view),
            None
        );
    }

    #[test]
    fn a_layer_tells_its_listeners_of_a_change() {
        let layer = MapElementsLayer::new();
        let count = Rc::new(Cell::new(0));
        let seen = count.clone();
        let listener: Rc<dyn Fn()> = Rc::new(move || seen.set(seen.get() + 1));
        layer.add_listener(listener.clone());
        layer.add(MapElement::Icon(MapIcon::new(place(1.0, 2.0))));
        layer.set_is_visible(false);
        layer.set_is_visible(false);
        layer.clear();
        assert_eq!(count.get(), 3);
        layer.remove_listener(&listener);
        layer.set_is_visible(true);
        assert_eq!(count.get(), 3);
    }
}
