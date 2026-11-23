//! What is drawn over the tiles.

use crate::projection::{project, world_size, BasicGeoposition};
use ferroui_base::media::Color;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
    pub stroke: Color,
    pub stroke_thickness: f64,
}

impl MapPolyline {
    pub fn new(path: Vec<BasicGeoposition>) -> Self {
        Self {
            path,
            stroke: Color::from_rgb(0x00, 0x78, 0xD4),
            stroke_thickness: 3.0,
        }
    }
}

/// An area bounded by places.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPolygon {
    pub path: Vec<BasicGeoposition>,
    pub fill: Color,
    pub stroke: Color,
    pub stroke_thickness: f64,
}

impl MapPolygon {
    pub fn new(path: Vec<BasicGeoposition>) -> Self {
        Self {
            path,
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

/// A group of elements, drawn in order. A change redraws the maps that show the layer.
#[derive(Default)]
pub struct MapElementsLayer {
    elements: RefCell<Vec<MapElement>>,
    is_hidden: Cell<bool>,
    on_changed: RefCell<Vec<Rc<dyn Fn()>>>,
}

impl MapElementsLayer {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn elements(&self) -> Vec<MapElement> {
        self.elements.borrow().clone()
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

/// Places as pixels of a view, one path: each point takes the copy of the repeating world nearest
/// the point before it, so a line across the date line stays one short line.
pub(crate) fn view_path(
    path: &[BasicGeoposition],
    center: BasicGeoposition,
    zoom: f64,
    width: f64,
    height: f64,
) -> Vec<(f64, f64)> {
    let world = world_size(zoom);
    let (center_x, center_y) = project(center, zoom);
    let mut points = Vec::with_capacity(path.len());
    let mut previous: Option<f64> = None;
    for place in path {
        let (x, y) = project(*place, zoom);
        let near = previous.unwrap_or(center_x);
        let x = x - world * ((x - near) / world).round();
        previous = Some(x);
        points.push((x - center_x + width / 2.0, y - center_y + height / 2.0));
    }
    points
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

    #[test]
    fn the_center_is_the_middle_of_the_view() {
        let center = place(10.0, 20.0);
        let points = view_path(&[center], center, 5.0, 400.0, 300.0);
        assert!((points[0].0 - 200.0).abs() < 1e-6);
        assert!((points[0].1 - 150.0).abs() < 1e-6);
    }

    #[test]
    fn a_line_across_the_date_line_stays_short() {
        let path = [place(0.0, 179.0), place(0.0, -179.0)];
        let points = view_path(&path, place(0.0, 179.5), 3.0, 400.0, 300.0);
        let degrees = (points[1].0 - points[0].0) / world_size(3.0) * 360.0;
        assert!((degrees - 2.0).abs() < 1e-6, "{degrees}");
    }

    #[test]
    fn a_place_takes_the_copy_of_the_world_near_the_view() {
        let points = view_path(&[place(0.0, -179.0)], place(0.0, 179.0), 2.0, 400.0, 300.0);
        let expected = 200.0 + world_size(2.0) * 2.0 / 360.0;
        assert!((points[0].0 - expected).abs() < 1e-6);
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
