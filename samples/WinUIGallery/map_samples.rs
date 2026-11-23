//! The code behind of the pinned MapControl page in Rust: the center, the zoom level and the layer
//! with the pin. The source shows Bing map data through its service token; FerroUI has no built-in
//! provider, so the sample uses the offline grid of the crate.

use ferroui_base::Ref;
use ferroui_controls::Control;
use ferroui_controls_maps::{
    grid_tile_source, BasicGeoposition, GridTileLoader, MapControl, MapElement, MapElementsLayer, MapIcon,
};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id != "MapControl" {
        return;
    }
    let map = root.get_control::<MapControl>("map1");
    map.set_tile_source(Some(Rc::new(grid_tile_source())), Some(Rc::new(GridTileLoader)));
    map.set_center(BasicGeoposition { latitude: 0.0, longitude: 0.0 });
    map.set_zoom_level(1.0);
    let layer = MapElementsLayer::new();
    layer.add(MapElement::Icon(MapIcon::new(BasicGeoposition { latitude: -30.034647, longitude: -51.217659 })));
    map.add_layer(layer);
}
