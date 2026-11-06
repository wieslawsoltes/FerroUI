//! The models of the geographic map, adapted from the pinned `HeatLand` (SkiaSharp
//! `Drawing/Geometries/HeatLand.cs`, `IWeigthedMapLand`) and, in place of a `GeoVisualElement`
//! around a circle visual, the native `GeoMarker`; and what the chart reads of them: the engine
//! snapshots of the heat land series and the observation of their lands and of the markers.

use crate::{ChartSeries, GeoMarkers, HeatLandSeries, HeatLands};
use ferroui_base::collections::FerroList;
use ferroui_base::media::Color;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use ferroui_livecharts::{ChartError, Series};
use std::rc::Rc;

/// A weighted land of a heat land series (`HeatLand`): the key of a land of the map, its short
/// name (the ISO 3166-1 alpha-3 code in lower case on the world map), and its value.
#[repr(C)]
pub struct HeatLand {
    base: FerroObject,
    entity_id: u64,
}
ferro_class!(HeatLand: FerroObject);
ferro_class_info!(HeatLand {
    new: HeatLand::new
});
impl FerroObjectImpl for HeatLand {}
impl HeatLand {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            entity_id: crate::identity::next_entity_id(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// A land of the given key and value (`HeatLand(name, value)`).
    pub fn with_value(name: impl Into<String>, value: f64) -> Ref<Self> {
        let land = Self::new();
        land.set_name(name.into());
        land.set_numeric_value(value);
        land
    }
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }
    pub fn snapshot(&self) -> ferroui_livecharts::HeatLand {
        ferroui_livecharts::HeatLand::new(self.name(), self.value())
    }
    pub fn name(&self) -> String {
        self.base.get_value(Self::name_property())
    }
    pub fn set_name(&self, value: String) {
        self.base.set_value(Self::name_property(), value);
    }
    /// The weight of the land (`Value`).
    pub fn value(&self) -> f64 {
        self.base.get_value(Self::value_property())
    }
    /// Sets `Value`; named apart from the property system's `set_value`.
    pub fn set_numeric_value(&self, value: f64) {
        self.base.set_value(Self::value_property(), value);
    }
}
ferro_properties! { impl HeatLand {
    pub fn name_property() -> StyledProperty<String> {
        FerroProperty::register::<HeatLand, _>("Name", String::new())
    }
    pub fn value_property() -> StyledProperty<f64> {
        FerroProperty::register::<HeatLand, _>("Value", 0.0)
    }
} }

/// A circle at a geographic coordinate over the map, the native counterpart of a
/// `GeoVisualElement` around a `GeometryVisual<CircleGeometry>`: it is centred on its
/// coordinate, follows pan, zoom and the rotation of a globe at a constant size, and is hidden on
/// the far side of the globe. Its defaults are the city markers of the source's `MapsTests`: 14
/// pixels of orange red with a white outline of 2.
#[repr(C)]
pub struct GeoMarker {
    base: FerroObject,
    entity_id: u64,
}
ferro_class!(GeoMarker: FerroObject);
ferro_class_info!(GeoMarker {
    new: GeoMarker::new
});
impl FerroObjectImpl for GeoMarker {}
impl GeoMarker {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            entity_id: crate::identity::next_entity_id(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// A marker at the given longitude and latitude, in degrees.
    pub fn at(longitude: f64, latitude: f64) -> Ref<Self> {
        let marker = Self::new();
        marker.set_longitude(longitude);
        marker.set_latitude(latitude);
        marker
    }
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }
    pub fn snapshot(&self) -> ferroui_livecharts::GeoMarker {
        ferroui_livecharts::GeoMarker {
            longitude: self.longitude(),
            latitude: self.latitude(),
            size: self.size(),
            fill: self.fill(),
            stroke: self.stroke(),
            stroke_width: self.stroke_thickness(),
        }
    }
    pub fn longitude(&self) -> f64 {
        self.base.get_value(Self::longitude_property())
    }
    pub fn set_longitude(&self, value: f64) {
        self.base.set_value(Self::longitude_property(), value);
    }
    pub fn latitude(&self) -> f64 {
        self.base.get_value(Self::latitude_property())
    }
    pub fn set_latitude(&self, value: f64) {
        self.base.set_value(Self::latitude_property(), value);
    }
    /// The diameter.
    pub fn size(&self) -> f64 {
        self.base.get_value(Self::size_property())
    }
    pub fn set_size(&self, value: f64) {
        self.base.set_value(Self::size_property(), value);
    }
    pub fn fill(&self) -> Option<Color> {
        self.base.get_value(Self::fill_property())
    }
    pub fn set_fill(&self, value: Option<Color>) {
        self.base.set_value(Self::fill_property(), value);
    }
    pub fn stroke(&self) -> Option<Color> {
        self.base.get_value(Self::stroke_property())
    }
    pub fn set_stroke(&self, value: Option<Color>) {
        self.base.set_value(Self::stroke_property(), value);
    }
    pub fn stroke_thickness(&self) -> f64 {
        self.base.get_value(Self::stroke_thickness_property())
    }
    pub fn set_stroke_thickness(&self, value: f64) {
        self.base.set_value(Self::stroke_thickness_property(), value);
    }
}
ferro_properties! { impl GeoMarker {
    pub fn longitude_property() -> StyledProperty<f64> {
        FerroProperty::register::<GeoMarker, _>("Longitude", 0.0)
    }
    pub fn latitude_property() -> StyledProperty<f64> {
        FerroProperty::register::<GeoMarker, _>("Latitude", 0.0)
    }
    pub fn size_property() -> StyledProperty<f64> {
        FerroProperty::register::<GeoMarker, _>("Size", 14.0)
    }
    pub fn fill_property() -> StyledProperty<Option<Color>> {
        FerroProperty::register::<GeoMarker, _>("Fill", Some(Color::from_rgb(255, 87, 51)))
    }
    pub fn stroke_property() -> StyledProperty<Option<Color>> {
        FerroProperty::register::<GeoMarker, _>("Stroke", Some(Color::from_rgb(255, 255, 255)))
    }
    pub fn stroke_thickness_property() -> StyledProperty<f64> {
        FerroProperty::register::<GeoMarker, _>("StrokeThickness", 2.0)
    }
} }

/// The engine series of a series of a geographic map, from the snapshot of its shared
/// properties. The source map casts its series to `IGeoSeries`; another kind of series is an
/// error.
pub(crate) fn heat_land_series(
    model: &Ref<ChartSeries>,
    series: Series,
) -> Result<ferroui_livecharts::HeatLandSeries, ChartError> {
    let Some(heat) = model.cast::<HeatLandSeries>() else {
        return Err(ChartError::InvalidData(
            "a geographic map takes heat land series only".into(),
        ));
    };
    Ok(ferroui_livecharts::HeatLandSeries {
        series,
        lands: heat.lands().to_vec().iter().map(|l| l.snapshot()).collect(),
        color_stops: heat.color_stops(),
        min_value: heat.min_value(),
        max_value: heat.max_value(),
    })
}

/// Observes the lands of a heat land series (`CollectionDeepObserver` of `CoreHeatLandSeries`):
/// `changed` when a land changes, `restructured` when the collection does (the subscriptions are
/// then made again). Other series have none.
pub(crate) fn observe_lands(
    model: &Ref<ChartSeries>,
    changed: Rc<dyn Fn()>,
    restructured: Rc<dyn Fn()>,
) -> Vec<Subscription> {
    let mut subscriptions = Vec::new();
    if let Some(heat) = model.cast::<HeatLandSeries>() {
        let lands: HeatLands = heat.lands();
        subscriptions.push(observe_list(&lands, restructured));
        for land in lands.to_vec() {
            let changed = changed.clone();
            subscriptions.push(Subscription::new(land.property_changed(move |_| changed())));
        }
    }
    subscriptions
}

/// Observes the markers of a chart, as the source observes its visual elements.
pub(crate) fn observe_markers(
    markers: &GeoMarkers,
    changed: Rc<dyn Fn()>,
    restructured: Rc<dyn Fn()>,
) -> Vec<Subscription> {
    let mut subscriptions = vec![observe_list(markers, restructured)];
    for marker in markers.to_vec() {
        let changed = changed.clone();
        subscriptions.push(Subscription::new(marker.property_changed(move |_| changed())));
    }
    subscriptions
}

fn observe_list<T: Clone + 'static>(list: &FerroList<T>, handler: Rc<dyn Fn()>) -> Subscription {
    let token = list.add_collection_changed(Rc::new(move |_| handler()));
    let list = list.clone();
    Subscription::new(Disposable::create(move || {
        list.remove_collection_changed(token);
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use std::cell::Cell;

    #[test]
    fn lands_and_markers_snapshot_their_properties() {
        let _scope = Dispatcher::unit_test_scope();
        let land = HeatLand::with_value("bra", 13.0);
        assert_eq!(land.snapshot(), ferroui_livecharts::HeatLand::new("bra", 13.0));
        let marker = GeoMarker::at(-74.0, 40.71);
        assert_eq!(
            marker.snapshot(),
            ferroui_livecharts::GeoMarker::new(-74.0, 40.71)
        );
        marker.set_size(20.0);
        assert_eq!(marker.snapshot().size, 20.0);
    }

    #[test]
    fn observation_reports_changes_and_restructuring_and_ends_on_drop() {
        let _scope = Dispatcher::unit_test_scope();
        let series = HeatLandSeries::new();
        let land = HeatLand::with_value("fra", 10.0);
        series.lands().add(land.clone());
        let changes = Rc::new(Cell::new(0));
        let rebinds = Rc::new(Cell::new(0));
        let (c, r) = (changes.clone(), rebinds.clone());
        let subscriptions = observe_lands(
            series.upcast_ref::<ChartSeries>(),
            Rc::new(move || c.set(c.get() + 1)),
            Rc::new(move || r.set(r.get() + 1)),
        );
        land.set_numeric_value(20.0);
        assert_eq!((changes.get(), rebinds.get()), (1, 0));
        series.lands().add(HeatLand::with_value("deu", 5.0));
        assert_eq!((changes.get(), rebinds.get()), (1, 1));
        drop(subscriptions);
        land.set_numeric_value(30.0);
        series.lands().clear();
        assert_eq!((changes.get(), rebinds.get()), (1, 1));
    }
}
