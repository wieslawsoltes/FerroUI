//! Not from upstream: the page of the `MapControl`, on the offline grid of its crate, with an
//! icon, a line and an area.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, Control, Grid, TextBlock};
use ferroui_controls_maps::{
    grid_tile_source, BasicGeoposition, GridTileLoader, MapControl, MapElement, MapElementClickEventArgs,
    MapElementsLayer, MapIcon, MapPolygon, MapPolyline,
};
use std::rc::Rc;

fn place(latitude: f64, longitude: f64) -> BasicGeoposition {
    BasicGeoposition { latitude, longitude }
}

#[repr(C)]
pub struct MapControlPage {
    base: ContentPage,
}

content_page_class!(MapControlPage);
ferro_class_info!(MapControlPage { new: MapControlPage::new });
xaml_class!(MapControlPage, "/Pages/MapControlPage.xaml");

impl MapControlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let map = MapControl::new();
        Grid::set_row(&map, 2);
        map.set_tile_source(Some(Rc::new(grid_tile_source())), Some(Rc::new(GridTileLoader)));
        map.set_zoom_level(2.0);

        let layer = MapElementsLayer::new();
        layer.add(MapElement::Polygon(MapPolygon::new(vec![
            place(40.0, -20.0),
            place(40.0, 30.0),
            place(10.0, 30.0),
            place(10.0, -20.0),
        ])));
        layer.add(MapElement::Polyline(MapPolyline::new(vec![
            place(-20.0, 150.0),
            place(-10.0, 175.0),
            place(5.0, -170.0),
        ])));
        let mut icon = MapIcon::new(place(25.0, 5.0));
        icon.title = Some("An icon".to_owned());
        layer.add(MapElement::Icon(icon));
        let clicked = this.get_control::<TextBlock>("Clicked").downgrade();
        layer.map_element_click().add(Rc::new(move |args: &MapElementClickEventArgs| {
            let Some(clicked) = clicked.upgrade() else { return };
            let kind = match args.element {
                MapElement::Icon(_) => "the icon",
                MapElement::Polyline(_) => "the line",
                MapElement::Polygon(_) => "the area",
            };
            let text = format!(
                "Clicked {kind} at {:.3}, {:.3}.",
                args.location.latitude, args.location.longitude
            );
            clicked.set_text(Some(text.as_str()));
        }));
        map.add_layer(layer);

        this.get_control::<Grid>("LayoutRoot").children().add(map.upcast::<Control>());
        this
    }
}
