//! Native notifying models for flow, hierarchy, and geographic charts.
// Source models: LiveChartsCore/Defaults/{SankeyNode,SankeyLink,TreemapNode}.cs
// and SkiaSharp/Drawing/Geometries/HeatLand.cs. MIT; see the crate NOTICE.

use ferroui_base::collections::FerroList;
use ferroui_base::media::Color;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::WeakEventSender;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_type, ferro_properties, instantiate, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, ObjectType, Ref, StyledProperty,
};
use ferroui_livecharts::geo_map_chart::{
    GeoMarker as GeoMarkerSnapshot, HeatLand as HeatLandSnapshot,
};
use ferroui_livecharts::sankey_chart_engine::{
    SankeyLink as SankeyLinkSnapshot, SankeyNode as SankeyNodeSnapshot,
};
use ferroui_livecharts::treemap_chart_engine::TreemapNode as TreemapNodeSnapshot;
use ferroui_livecharts::{geo::GeoCoordinate, ChartError};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    ops::Deref,
    rc::Rc,
};

ferroui_base::ferro_thread_local! { static NEXT_NODE_ID:Cell<u64>=Cell::new(1); }
fn next_node_id() -> u64 {
    NEXT_NODE_ID.with(|next| {
        let id = next.get();
        next.set(id.wrapping_add(1).max(1));
        id
    })
}

/// Native collection handles are reference-like values in markup. Registering
/// their optional forms preserves null (a tree leaf) separately from an empty
/// collection (an internal node), including for bindings and runtime setters.
pub(crate) fn register_value_types() {
    fn register() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<SankeyNodes>();
        ValueTypes::register_nullable::<SankeyLinks>();
        ValueTypes::register_nullable::<TreemapNodes>();
        ValueTypes::register_nullable::<HeatLands>();
        ValueTypes::register_nullable::<GeoMarkers>();
    }
    ferroui_base::data::core::ValueTypes::register_global(register);
}

#[repr(C)]
pub struct SankeyNode {
    base: FerroObject,
}
ferro_class!(SankeyNode:FerroObject);
ferro_class_info!(SankeyNode {
    new: SankeyNode::new
});
impl FerroObjectImpl for SankeyNode {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_id(next_node_id());
    }
}
impl SankeyNode {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_name(name: impl Into<String>) -> Ref<Self> {
        let node = Self::new();
        node.set_name(name.into());
        node
    }
    pub fn from_snapshot(value: &SankeyNodeSnapshot) -> Ref<Self> {
        let node = Self::new();
        node.set_id(value.id);
        node.set_name(value.name.clone());
        node.set_color(value.color);
        node
    }
    pub fn snapshot(&self) -> SankeyNodeSnapshot {
        SankeyNodeSnapshot {
            id: self.id(),
            name: self.name(),
            color: self.color(),
        }
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_model(self.to_ref(), handler)
    }
    pub fn id(&self) -> u64 {
        self.get_value(Self::id_property())
    }
    pub fn set_id(&self, value: u64) {
        self.set_value(Self::id_property(), value);
    }
    pub fn name(&self) -> String {
        self.get_value(Self::name_property())
    }
    pub fn set_name(&self, value: String) {
        self.set_value(Self::name_property(), value);
    }
    pub fn color(&self) -> Option<Color> {
        self.get_value(Self::color_property())
    }
    pub fn set_color(&self, value: Option<Color>) {
        self.set_value(Self::color_property(), value);
    }
}
ferro_properties! {impl SankeyNode {
    pub fn id_property()->StyledProperty<u64> {FerroProperty::register::<SankeyNode,_>("Id",0)}
    pub fn name_property()->StyledProperty<String> {FerroProperty::register::<SankeyNode,_>("Name",String::new())}
    pub fn color_property()->StyledProperty<Option<Color>> {FerroProperty::register::<SankeyNode,_>("Color",None)}
}}

#[repr(C)]
pub struct SankeyLink {
    base: FerroObject,
}
ferro_class!(SankeyLink:FerroObject);
ferro_class_info!(SankeyLink {
    new: SankeyLink::new
});
impl FerroObjectImpl for SankeyLink {}
impl SankeyLink {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_nodes(source: Ref<SankeyNode>, target: Ref<SankeyNode>, weight: f64) -> Ref<Self> {
        let link = Self::new();
        link.set_source(Some(source));
        link.set_target(Some(target));
        link.set_weight(weight);
        link
    }
    pub fn from_snapshot(value: &SankeyLinkSnapshot) -> Ref<Self> {
        let link = Self::new();
        link.set_source_id(value.source);
        link.set_target_id(value.target);
        link.set_weight(value.weight);
        link.set_color(value.color);
        link
    }
    pub fn snapshot(&self) -> SankeyLinkSnapshot {
        SankeyLinkSnapshot {
            source: self
                .source()
                .map_or_else(|| self.source_id(), |node| node.id()),
            target: self
                .target()
                .map_or_else(|| self.target_id(), |node| node.id()),
            weight: self.weight(),
            color: self.color(),
        }
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_model(self.to_ref(), handler)
    }
    pub fn source(&self) -> Option<Ref<SankeyNode>> {
        self.get_value(Self::source_property())
    }
    pub fn set_source(&self, value: Option<Ref<SankeyNode>>) {
        self.set_value(Self::source_property(), value);
    }
    pub fn target(&self) -> Option<Ref<SankeyNode>> {
        self.get_value(Self::target_property())
    }
    pub fn set_target(&self, value: Option<Ref<SankeyNode>>) {
        self.set_value(Self::target_property(), value);
    }
    pub fn source_id(&self) -> u64 {
        self.get_value(Self::source_id_property())
    }
    pub fn set_source_id(&self, value: u64) {
        self.set_value(Self::source_id_property(), value);
    }
    pub fn target_id(&self) -> u64 {
        self.get_value(Self::target_id_property())
    }
    pub fn set_target_id(&self, value: u64) {
        self.set_value(Self::target_id_property(), value);
    }
    pub fn weight(&self) -> f64 {
        self.get_value(Self::weight_property())
    }
    pub fn set_weight(&self, value: f64) {
        self.set_value(Self::weight_property(), value);
    }
    pub fn color(&self) -> Option<Color> {
        self.get_value(Self::color_property())
    }
    pub fn set_color(&self, value: Option<Color>) {
        self.set_value(Self::color_property(), value);
    }
}
ferro_properties! {impl SankeyLink {
    pub fn source_property()->StyledProperty<Option<Ref<SankeyNode>>> {FerroProperty::register::<SankeyLink,_>("Source",None)}
    pub fn target_property()->StyledProperty<Option<Ref<SankeyNode>>> {FerroProperty::register::<SankeyLink,_>("Target",None)}
    pub fn source_id_property()->StyledProperty<u64> {FerroProperty::register::<SankeyLink,_>("SourceId",0)}
    pub fn target_id_property()->StyledProperty<u64> {FerroProperty::register::<SankeyLink,_>("TargetId",0)}
    pub fn weight_property()->StyledProperty<f64> {FerroProperty::register::<SankeyLink,_>("Weight",0.0)}
    pub fn color_property()->StyledProperty<Option<Color>> {FerroProperty::register::<SankeyLink,_>("Color",None)}
}}

#[repr(C)]
pub struct TreemapNode {
    base: FerroObject,
}
ferro_class!(TreemapNode:FerroObject);
ferro_class_info!(TreemapNode {new:TreemapNode::new,markup:{content:Children}});
impl FerroObjectImpl for TreemapNode {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_id(next_node_id());
    }
}
impl TreemapNode {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_value(value: f64, name: impl Into<String>) -> Ref<Self> {
        let node = Self::new();
        node.set_value(value);
        node.set_name(name.into());
        node
    }
    pub fn with_children(name: impl Into<String>, children: TreemapNodes) -> Ref<Self> {
        let node = Self::new();
        node.set_name(name.into());
        node.set_children(Some(children));
        node
    }

    /// Returns an owned acyclic engine tree. Shared subtrees are copied; reference cycles are errors.
    pub fn snapshot(&self) -> Result<TreemapNodeSnapshot, ChartError> {
        snapshot_tree(self.to_ref())
    }

    pub fn from_snapshot(value: &TreemapNodeSnapshot) -> Ref<Self> {
        enum Frame<'a> {
            Enter(&'a TreemapNodeSnapshot),
            Exit(&'a TreemapNodeSnapshot, usize),
        }
        let mut pending = vec![Frame::Enter(value)];
        let mut output = Vec::new();
        while let Some(frame) = pending.pop() {
            match frame {
                Frame::Enter(value) => {
                    let count = value.children.as_ref().map_or(0, Vec::len);
                    pending.push(Frame::Exit(value, count));
                    if let Some(children) = &value.children {
                        pending.extend(children.iter().rev().map(Frame::Enter));
                    }
                }
                Frame::Exit(value, count) => {
                    let children = output.split_off(output.len() - count);
                    let node = Self::new();
                    node.set_id(value.id);
                    node.set_name(value.name.clone());
                    node.set_value(value.value);
                    node.set_color(value.color);
                    if value.children.is_some() {
                        node.set_children(Some(TreemapNodes::from_items(children)));
                    }
                    output.push(node);
                }
            }
        }
        // Exactly one root was entered; every exit pushes its result once.
        output.pop().expect("tree conversion always emits its root")
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_model(self.to_ref(), handler)
    }
    pub fn id(&self) -> u64 {
        self.get_value(Self::id_property())
    }
    pub fn set_id(&self, value: u64) {
        self.base.set_value(Self::id_property(), value);
    }
    pub fn name(&self) -> String {
        self.get_value(Self::name_property())
    }
    pub fn set_name(&self, value: String) {
        self.base.set_value(Self::name_property(), value);
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_value(&self, value: f64) {
        self.base.set_value(Self::value_property(), value);
    }
    pub fn children(&self) -> Option<TreemapNodes> {
        self.get_value(Self::children_property())
    }
    pub fn set_children(&self, value: Option<TreemapNodes>) {
        self.base.set_value(Self::children_property(), value);
    }
    pub fn color(&self) -> Option<Color> {
        self.get_value(Self::color_property())
    }
    pub fn set_color(&self, value: Option<Color>) {
        self.base.set_value(Self::color_property(), value);
    }
}
ferro_properties! {impl TreemapNode {
    pub fn id_property()->StyledProperty<u64> {FerroProperty::register::<TreemapNode,_>("Id",0)}
    pub fn name_property()->StyledProperty<String> {FerroProperty::register::<TreemapNode,_>("Name",String::new())}
    pub fn value_property()->StyledProperty<f64> {FerroProperty::register::<TreemapNode,_>("Value",0.0)}
    pub fn children_property()->StyledProperty<Option<TreemapNodes>> {FerroProperty::register::<TreemapNode,_>("Children",None)}
    pub fn color_property()->StyledProperty<Option<Color>> {FerroProperty::register::<TreemapNode,_>("Color",None)}
}}

#[repr(C)]
pub struct HeatLand {
    base: FerroObject,
}
ferro_class!(HeatLand:FerroObject);
ferro_class_info!(HeatLand { new: HeatLand::new });
impl FerroObjectImpl for HeatLand {}
impl HeatLand {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn with_value(name: impl Into<String>, value: f64) -> Ref<Self> {
        let land = Self::new();
        land.set_name(name.into());
        land.set_value(value);
        land
    }
    pub fn from_snapshot(value: &HeatLandSnapshot) -> Ref<Self> {
        let land = Self::with_value(value.name.clone(), value.value);
        land.set_color(value.color);
        land
    }
    pub fn snapshot(&self) -> HeatLandSnapshot {
        HeatLandSnapshot {
            name: self.name(),
            value: self.value(),
            color: self.color(),
        }
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_model(self.to_ref(), handler)
    }
    pub fn name(&self) -> String {
        self.get_value(Self::name_property())
    }
    pub fn set_name(&self, value: String) {
        self.base.set_value(Self::name_property(), value);
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_value(&self, value: f64) {
        self.base.set_value(Self::value_property(), value);
    }
    pub fn color(&self) -> Option<Color> {
        self.get_value(Self::color_property())
    }
    pub fn set_color(&self, value: Option<Color>) {
        self.base.set_value(Self::color_property(), value);
    }
}
ferro_properties! {impl HeatLand {
    pub fn name_property()->StyledProperty<String> {FerroProperty::register::<HeatLand,_>("Name",String::new())}
    pub fn value_property()->StyledProperty<f64> {FerroProperty::register::<HeatLand,_>("Value",0.0)}
    pub fn color_property()->StyledProperty<Option<Color>> {FerroProperty::register::<HeatLand,_>("Color",None)}
}}

#[repr(C)]
pub struct GeoMarker {
    base: FerroObject,
}
ferro_class!(GeoMarker:FerroObject);
ferro_class_info!(GeoMarker {
    new: GeoMarker::new
});
impl FerroObjectImpl for GeoMarker {}
impl GeoMarker {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn at(longitude: f64, latitude: f64, label: impl Into<String>) -> Ref<Self> {
        let marker = Self::new();
        marker.set_longitude(longitude);
        marker.set_latitude(latitude);
        marker.set_label(label.into());
        marker
    }
    pub fn from_snapshot(value: &GeoMarkerSnapshot) -> Ref<Self> {
        let marker = Self::at(
            value.coordinate.longitude,
            value.coordinate.latitude,
            value.label.clone(),
        );
        marker.set_radius(value.radius);
        marker.set_value(value.value);
        marker.set_color(value.color);
        marker.set_show_label(value.show_label);
        marker
    }
    pub fn snapshot(&self) -> GeoMarkerSnapshot {
        GeoMarkerSnapshot {
            coordinate: GeoCoordinate::new(self.longitude(), self.latitude()),
            radius: self.radius(),
            label: self.label(),
            value: self.value(),
            color: self.color(),
            show_label: self.show_label(),
        }
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_model(self.to_ref(), handler)
    }
    pub fn longitude(&self) -> f64 {
        self.get_value(Self::longitude_property())
    }
    pub fn set_longitude(&self, value: f64) {
        self.base.set_value(Self::longitude_property(), value);
    }
    pub fn latitude(&self) -> f64 {
        self.get_value(Self::latitude_property())
    }
    pub fn set_latitude(&self, value: f64) {
        self.base.set_value(Self::latitude_property(), value);
    }
    pub fn radius(&self) -> f64 {
        self.get_value(Self::radius_property())
    }
    pub fn set_radius(&self, value: f64) {
        self.base.set_value(Self::radius_property(), value);
    }
    pub fn label(&self) -> String {
        self.get_value(Self::label_property())
    }
    pub fn set_label(&self, value: String) {
        self.base.set_value(Self::label_property(), value);
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_value(&self, value: f64) {
        self.base.set_value(Self::value_property(), value);
    }
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }
    pub fn set_color(&self, value: Color) {
        self.base.set_value(Self::color_property(), value);
    }
    pub fn show_label(&self) -> bool {
        self.get_value(Self::show_label_property())
    }
    pub fn set_show_label(&self, value: bool) {
        self.base.set_value(Self::show_label_property(), value);
    }
}
ferro_properties! {impl GeoMarker {
    pub fn longitude_property()->StyledProperty<f64> {FerroProperty::register::<GeoMarker,_>("Longitude",0.0)}
    pub fn latitude_property()->StyledProperty<f64> {FerroProperty::register::<GeoMarker,_>("Latitude",0.0)}
    pub fn radius_property()->StyledProperty<f64> {FerroProperty::register::<GeoMarker,_>("Radius",5.0)}
    pub fn label_property()->StyledProperty<String> {FerroProperty::register::<GeoMarker,_>("Label",String::new())}
    pub fn value_property()->StyledProperty<f64> {FerroProperty::register::<GeoMarker,_>("Value",0.0)}
    pub fn color_property()->StyledProperty<Color> {FerroProperty::register::<GeoMarker,_>("Color",Color::from_rgb(66,99,235))}
    pub fn show_label_property()->StyledProperty<bool> {FerroProperty::register::<GeoMarker,_>("ShowLabel",false)}
}}

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNodes(FerroList<Ref<SankeyNode>>);
impl SankeyNodes {
    pub fn new() -> Self {
        Self(FerroList::new())
    }
    pub fn from_items(items: impl IntoIterator<Item = Ref<SankeyNode>>) -> Self {
        Self(FerroList::from_items(items))
    }
    pub fn snapshots(&self) -> Vec<SankeyNodeSnapshot> {
        self.to_vec().iter().map(|node| node.snapshot()).collect()
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_list(&self.0, handler)
    }
}
impl Default for SankeyNodes {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for SankeyNodes {
    type Target = FerroList<Ref<SankeyNode>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<FerroList<Ref<SankeyNode>>> for SankeyNodes {
    fn from(items: FerroList<Ref<SankeyNode>>) -> Self {
        Self(items)
    }
}
ferro_markup_type!(class SankeyNodes {
    namespace:"FerroUI.LiveCharts",handles:[SankeyNodes,Option<SankeyNodes>],constructors:[()=>SankeyNodes::new],
    properties:[Count:i32{get:|items:&SankeyNodes|items.count() as i32}],
    methods:[fn Add(Ref<SankeyNode>)=>|items:&SankeyNodes,item:Ref<SankeyNode>|items.add(item)],
});

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyLinks(FerroList<Ref<SankeyLink>>);
impl SankeyLinks {
    pub fn new() -> Self {
        Self(FerroList::new())
    }
    pub fn from_items(items: impl IntoIterator<Item = Ref<SankeyLink>>) -> Self {
        Self(FerroList::from_items(items))
    }
    pub fn snapshots(&self) -> Vec<SankeyLinkSnapshot> {
        self.to_vec().iter().map(|link| link.snapshot()).collect()
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_list(&self.0, handler)
    }
}
impl Default for SankeyLinks {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for SankeyLinks {
    type Target = FerroList<Ref<SankeyLink>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<FerroList<Ref<SankeyLink>>> for SankeyLinks {
    fn from(items: FerroList<Ref<SankeyLink>>) -> Self {
        Self(items)
    }
}
ferro_markup_type!(class SankeyLinks {
    namespace:"FerroUI.LiveCharts",handles:[SankeyLinks,Option<SankeyLinks>],constructors:[()=>SankeyLinks::new],
    properties:[Count:i32{get:|items:&SankeyLinks|items.count() as i32}],
    methods:[fn Add(Ref<SankeyLink>)=>|items:&SankeyLinks,item:Ref<SankeyLink>|items.add(item)],
});

#[derive(Clone, Debug, PartialEq)]
pub struct TreemapNodes(FerroList<Ref<TreemapNode>>);
impl TreemapNodes {
    pub fn new() -> Self {
        Self(FerroList::new())
    }
    pub fn from_items(items: impl IntoIterator<Item = Ref<TreemapNode>>) -> Self {
        Self(FerroList::from_items(items))
    }
    pub fn snapshots(&self) -> Result<Vec<TreemapNodeSnapshot>, ChartError> {
        self.to_vec().iter().map(|node| node.snapshot()).collect()
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_list(&self.0, handler)
    }
}
impl Default for TreemapNodes {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for TreemapNodes {
    type Target = FerroList<Ref<TreemapNode>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<FerroList<Ref<TreemapNode>>> for TreemapNodes {
    fn from(items: FerroList<Ref<TreemapNode>>) -> Self {
        Self(items)
    }
}
ferro_markup_type!(class TreemapNodes {
    namespace:"FerroUI.LiveCharts",handles:[TreemapNodes,Option<TreemapNodes>],constructors:[()=>TreemapNodes::new],
    properties:[Count:i32{get:|items:&TreemapNodes|items.count() as i32}],
    methods:[fn Add(Ref<TreemapNode>)=>|items:&TreemapNodes,item:Ref<TreemapNode>|items.add(item)],
});

#[derive(Clone, Debug, PartialEq)]
pub struct HeatLands(FerroList<Ref<HeatLand>>);
impl HeatLands {
    pub fn new() -> Self {
        Self(FerroList::new())
    }
    pub fn from_items(items: impl IntoIterator<Item = Ref<HeatLand>>) -> Self {
        Self(FerroList::from_items(items))
    }
    pub fn snapshots(&self) -> Vec<HeatLandSnapshot> {
        self.to_vec().iter().map(|land| land.snapshot()).collect()
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_list(&self.0, handler)
    }
}
impl Default for HeatLands {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for HeatLands {
    type Target = FerroList<Ref<HeatLand>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<FerroList<Ref<HeatLand>>> for HeatLands {
    fn from(items: FerroList<Ref<HeatLand>>) -> Self {
        Self(items)
    }
}
ferro_markup_type!(class HeatLands {
    namespace:"FerroUI.LiveCharts",handles:[HeatLands,Option<HeatLands>],constructors:[()=>HeatLands::new],
    properties:[Count:i32{get:|items:&HeatLands|items.count() as i32}],
    methods:[fn Add(Ref<HeatLand>)=>|items:&HeatLands,item:Ref<HeatLand>|items.add(item)],
});

#[derive(Clone, Debug, PartialEq)]
pub struct GeoMarkers(FerroList<Ref<GeoMarker>>);
impl GeoMarkers {
    pub fn new() -> Self {
        Self(FerroList::new())
    }
    pub fn from_items(items: impl IntoIterator<Item = Ref<GeoMarker>>) -> Self {
        Self(FerroList::from_items(items))
    }
    pub fn snapshots(&self) -> Vec<GeoMarkerSnapshot> {
        self.to_vec()
            .iter()
            .map(|marker| marker.snapshot())
            .collect()
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        observe_list(&self.0, handler)
    }
}
impl Default for GeoMarkers {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for GeoMarkers {
    type Target = FerroList<Ref<GeoMarker>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<FerroList<Ref<GeoMarker>>> for GeoMarkers {
    fn from(items: FerroList<Ref<GeoMarker>>) -> Self {
        Self(items)
    }
}
ferro_markup_type!(class GeoMarkers {
    namespace:"FerroUI.LiveCharts",handles:[GeoMarkers,Option<GeoMarkers>],constructors:[()=>GeoMarkers::new],
    properties:[Count:i32{get:|items:&GeoMarkers|items.count() as i32}],
    methods:[fn Add(Ref<GeoMarker>)=>|items:&GeoMarkers,item:Ref<GeoMarker>|items.add(item)],
});

fn snapshot_tree(root: Ref<TreemapNode>) -> Result<TreemapNodeSnapshot, ChartError> {
    enum Frame {
        Enter(Ref<TreemapNode>),
        Exit(Ref<TreemapNode>, Option<usize>),
    }
    let mut pending = vec![Frame::Enter(root)];
    let mut active = HashSet::new();
    let mut output = Vec::new();
    while let Some(frame) = pending.pop() {
        match frame {
            Frame::Enter(node) => {
                if !active.insert(node.clone()) {
                    return Err(ChartError::InvalidData(
                        "Treemap children contain a reference cycle".into(),
                    ));
                }
                let children = node.children().map(|children| children.to_vec());
                pending.push(Frame::Exit(node, children.as_ref().map(Vec::len)));
                if let Some(children) = children {
                    pending.extend(children.into_iter().rev().map(Frame::Enter));
                }
            }
            Frame::Exit(node, count) => {
                let children = count.map(|count| output.split_off(output.len() - count));
                let value = TreemapNodeSnapshot {
                    id: node.id(),
                    name: node.name(),
                    value: node.value(),
                    children,
                    color: node.color(),
                };
                active.remove(&node);
                output.push(value);
            }
        }
    }
    output
        .pop()
        .ok_or_else(|| ChartError::InvalidData("Treemap snapshot did not contain a root".into()))
}

trait ObservableModel: ObjectType {
    fn collect(&self, collector: &mut WatchCollector);
}
impl ObservableModel for SankeyNode {
    fn collect(&self, collector: &mut WatchCollector) {
        collector.object(self.to_ref().upcast());
    }
}
impl ObservableModel for HeatLand {
    fn collect(&self, collector: &mut WatchCollector) {
        collector.object(self.to_ref().upcast());
    }
}
impl ObservableModel for GeoMarker {
    fn collect(&self, collector: &mut WatchCollector) {
        collector.object(self.to_ref().upcast());
    }
}
impl ObservableModel for SankeyLink {
    fn collect(&self, collector: &mut WatchCollector) {
        if !collector.object(self.to_ref().upcast()) {
            return;
        }
        if let Some(source) = self.source() {
            source.collect(collector);
        }
        if let Some(target) = self.target() {
            target.collect(collector);
        }
    }
}
impl ObservableModel for TreemapNode {
    fn collect(&self, collector: &mut WatchCollector) {
        let mut pending = vec![self.to_ref()];
        while let Some(node) = pending.pop() {
            if !collector.object(node.clone().upcast()) {
                continue;
            }
            if let Some(children) = node.children() {
                collector.list(&children.0);
                pending.extend(children.to_vec());
            }
        }
    }
}

type ChangeCallback = Rc<dyn Fn()>;
struct WatchCollector {
    callback: ChangeCallback,
    subscriptions: Vec<Rc<dyn IDisposable>>,
    objects: HashSet<Ref<FerroObject>>,
    lists: HashSet<usize>,
}
impl WatchCollector {
    fn object(&mut self, object: Ref<FerroObject>) -> bool {
        if !self.objects.insert(object.clone()) {
            return false;
        }
        let callback = self.callback.clone();
        self.subscriptions
            .push(object.property_changed(move |_| callback()));
        true
    }
    fn list<T: Clone + 'static>(&mut self, list: &FerroList<T>) {
        if !self.lists.insert(list.sender_address()) {
            return;
        }
        let callback = self.callback.clone();
        let token = list.add_collection_changed(Rc::new(move |_| callback()));
        let weak = list.downgrade_sender();
        self.subscriptions.push(Disposable::create(move || {
            if let Some(list) = FerroList::<T>::upgrade_sender(&weak) {
                list.remove_collection_changed(token);
            }
        }));
    }
}

struct DeepObservation {
    collect: Box<dyn Fn(&mut WatchCollector)>,
    callback: ChangeCallback,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}
impl DeepObservation {
    fn new(
        collect: impl Fn(&mut WatchCollector) + 'static,
        handler: impl Fn() + 'static,
    ) -> Rc<dyn IDisposable> {
        let state = Rc::new(Self {
            collect: Box::new(collect),
            callback: Rc::new(handler),
            subscriptions: RefCell::new(Vec::new()),
            disposed: Cell::new(false),
        });
        Self::rebind(&state);
        state
    }
    fn rebind(state: &Rc<Self>) {
        if state.disposed.get() {
            return;
        }
        let weak = Rc::downgrade(state);
        let callback: ChangeCallback = Rc::new(move || {
            if let Some(state) = weak.upgrade() {
                if state.disposed.get() {
                    return;
                }
                Self::rebind(&state);
                // No source-list borrow is held while invoking application code.
                (state.callback)();
            }
        });
        let mut collector = WatchCollector {
            callback,
            subscriptions: Vec::new(),
            objects: HashSet::new(),
            lists: HashSet::new(),
        };
        (state.collect)(&mut collector);
        let previous = state.subscriptions.replace(collector.subscriptions);
        for subscription in previous {
            subscription.dispose();
        }
    }
}
impl IDisposable for DeepObservation {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let previous = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in previous {
            subscription.dispose();
        }
    }
}
impl Drop for DeepObservation {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
fn observe_model<T: ObservableModel>(
    model: Ref<T>,
    handler: impl Fn() + 'static,
) -> Rc<dyn IDisposable> {
    let weak = model.downgrade();
    DeepObservation::new(
        move |collector| {
            if let Some(model) = weak.upgrade() {
                model.collect(collector);
            }
        },
        handler,
    )
}
fn observe_list<T: ObservableModel>(
    list: &FerroList<Ref<T>>,
    handler: impl Fn() + 'static,
) -> Rc<dyn IDisposable> {
    let weak = list.downgrade_sender();
    DeepObservation::new(
        move |collector| {
            if let Some(list) = FerroList::<Ref<T>>::upgrade_sender(&weak) {
                collector.list(&list);
                for model in list.to_vec() {
                    model.collect(collector);
                }
            }
        },
        handler,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;

    #[test]
    fn markup_collection_conversion_preserves_null_and_empty_children() {
        use ferroui_base::metadata::{from_markup_value, into_markup_value};
        let _scope = Dispatcher::unit_test_scope();
        register_value_types();
        let children = TreemapNodes::new();
        let value = into_markup_value(children.clone());
        let converted = from_markup_value::<Option<TreemapNodes>>(&value).unwrap();
        assert_eq!(converted, Some(children.clone()));
        assert_eq!(from_markup_value::<Option<TreemapNodes>>(&None), Some(None));
        let parent = TreemapNode::new();
        parent.set_children(converted);
        children.add(TreemapNode::with_value(12., "native child"));
        assert_eq!(parent.snapshot().unwrap().children.unwrap()[0].value, 12.);
    }

    #[test]
    fn native_sankey_nodes_have_distinct_ids_and_reference_links_follow_id_changes() {
        let _scope = Dispatcher::unit_test_scope();
        let a = SankeyNode::with_name("A");
        let b = SankeyNode::with_name("B");
        assert_ne!(a.id(), b.id());
        let link = SankeyLink::with_nodes(a.clone(), b.clone(), 7.0);
        let calls = Rc::new(Cell::new(0));
        let recorded = calls.clone();
        let subscription = link.changed(move || recorded.set(recorded.get() + 1));
        a.set_id(42);
        assert_eq!(calls.get(), 1);
        assert_eq!(link.snapshot().source, 42);
        link.set_target_id(99);
        link.set_target(None);
        assert_eq!(link.snapshot().target, 99);
        let before = calls.get();
        b.set_name("Detached".into());
        assert_eq!(calls.get(), before);
        subscription.dispose();
        a.set_name("Ignored".into());
        assert_eq!(calls.get(), before);
    }
    #[test]
    fn collection_observes_add_remove_replace_and_item_properties() {
        let _scope = Dispatcher::unit_test_scope();
        let items = SankeyNodes::new();
        let a = SankeyNode::new();
        let b = SankeyNode::new();
        let calls = Rc::new(Cell::new(0));
        let recorded = calls.clone();
        let subscription = items.changed(move || recorded.set(recorded.get() + 1));
        items.add(a.clone());
        a.set_name("A".into());
        assert_eq!(calls.get(), 2);
        items.set(0, b.clone());
        assert_eq!(calls.get(), 3);
        a.set_name("removed".into());
        assert_eq!(calls.get(), 3);
        b.set_name("B".into());
        assert_eq!(calls.get(), 4);
        items.clear();
        assert_eq!(calls.get(), 5);
        b.set_name("after clear".into());
        assert_eq!(calls.get(), 5);
        subscription.dispose();
        assert!(!items.has_collection_changed_subscribers());
    }
    #[test]
    fn tree_deep_observation_rebinds_children_and_deduplicates_shared_nodes() {
        let _scope = Dispatcher::unit_test_scope();
        let leaf = TreemapNode::with_value(10.0, "leaf");
        let children = TreemapNodes::from_items([leaf.clone(), leaf.clone()]);
        let root = TreemapNode::with_children("root", children.clone());
        let roots = TreemapNodes::from_items([root.clone()]);
        let calls = Rc::new(Cell::new(0));
        let recorded = calls.clone();
        let subscription = roots.changed(move || recorded.set(recorded.get() + 1));
        leaf.set_value(20.0);
        assert_eq!(calls.get(), 1);
        let next = TreemapNode::with_value(30.0, "next");
        children.add(next.clone());
        next.set_value(40.0);
        assert_eq!(calls.get(), 3);
        root.set_children(Some(TreemapNodes::new()));
        assert_eq!(calls.get(), 4);
        leaf.set_value(50.0);
        next.set_value(60.0);
        assert_eq!(calls.get(), 4);
        assert!(!children.has_collection_changed_subscribers());
        drop(subscription);
        assert!(!roots.has_collection_changed_subscribers());
    }
    #[test]
    fn empty_internal_nodes_and_roundtrip_snapshots_preserve_hierarchy() {
        let _scope = Dispatcher::unit_test_scope();
        let leaf = TreemapNode::with_value(10.0, "leaf");
        assert!(leaf.snapshot().unwrap().children.is_none());
        leaf.set_children(Some(TreemapNodes::new()));
        let snapshot = leaf.snapshot().unwrap();
        assert_eq!(snapshot.children, Some(vec![]));
        assert_eq!(
            TreemapNode::from_snapshot(&snapshot).snapshot().unwrap(),
            snapshot
        );
        let root = TreemapNode::with_children("parent", TreemapNodes::from_items([leaf]));
        root.set_value(100.0);
        let snapshot = root.snapshot().unwrap();
        assert_eq!(
            TreemapNode::from_snapshot(&snapshot).snapshot().unwrap(),
            snapshot
        );
    }
    #[test]
    fn reference_cycles_are_reported_and_observation_does_not_recurse() {
        let _scope = Dispatcher::unit_test_scope();
        let root = TreemapNode::with_value(10.0, "cyclic");
        root.set_children(Some(TreemapNodes::from_items([root.clone()])));
        let subscription = root.changed(|| {});
        assert!(root.snapshot().unwrap_err().to_string().contains("cycle"));
        root.set_name("still observable".into());
        root.set_children(None);
        subscription.dispose();
    }
    #[test]
    fn change_handler_can_modify_collection_and_disposal_releases_sources() {
        let _scope = Dispatcher::unit_test_scope();
        let item = HeatLand::with_value("bra", 10.0);
        let items = HeatLands::from_items([item.clone()]);
        let changed_items = items.clone();
        let clear_once = Rc::new(Cell::new(false));
        let guard = clear_once.clone();
        let subscription = items.changed(move || {
            if !guard.replace(true) {
                changed_items.clear();
            }
        });
        item.set_value(20.0);
        assert!(items.is_empty());
        assert_eq!(item.property_changed_subscriber_count(), 0);
        subscription.dispose();
        assert!(!items.has_collection_changed_subscribers());
        let marker = GeoMarker::at(-74.0, 40.7, "NYC");
        let weak = marker.downgrade();
        let subscription = marker.changed(|| {});
        drop(marker);
        assert!(weak.upgrade().is_none());
        drop(subscription);
    }
}
