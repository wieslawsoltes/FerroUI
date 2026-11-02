//! The node and link models of the Sankey and treemap series, adapted from the pinned
//! `Defaults/SankeyNode`, `Defaults/SankeyLink` and `Defaults/TreemapNode`, and what the chart
//! reads of the series that hold them: their engine snapshots, the identities of their nodes for
//! the scene transitions, and the observation of every collection, node and link.

use crate::{ChartSeries, SankeyLinks, SankeyNodes, SankeySeries, TreemapNodes, TreemapSeries};
use ferroui_base::collections::FerroList;
use ferroui_base::media::Color;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use ferroui_livecharts::{ChartError, PointId, Series};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// A node of a Sankey series (`SankeyNode`): its label and, in place of the source's
/// `NodeColorMapper`, an optional colour of its own. Links refer to the node object, as the
/// source links by reference; its value is computed by the layout.
#[repr(C)]
pub struct SankeyNode {
    base: FerroObject,
    entity_id: u64,
}
ferro_class!(SankeyNode: FerroObject);
ferro_class_info!(SankeyNode {
    new: SankeyNode::new
});
impl FerroObjectImpl for SankeyNode {}
impl SankeyNode {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            entity_id: crate::identity::next_entity_id(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// A node with the given label (`SankeyNode(name)`).
    pub fn with_name(name: impl Into<String>) -> Ref<Self> {
        let node = Self::new();
        node.set_name(name.into());
        node
    }
    /// The identity of the object, which the links and the engine use.
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }
    pub fn snapshot(&self) -> ferroui_livecharts::SankeyNode {
        ferroui_livecharts::SankeyNode {
            id: self.entity_id,
            name: self.name(),
            color: self.color(),
        }
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
ferro_properties! { impl SankeyNode {
    pub fn name_property() -> StyledProperty<String> {
        FerroProperty::register::<SankeyNode, _>("Name", String::new())
    }
    pub fn color_property() -> StyledProperty<Option<Color>> {
        FerroProperty::register::<SankeyNode, _>("Color", None)
    }
} }

/// A directed weighted edge of a Sankey series (`SankeyLink`) between two node objects and, in
/// place of the source's `LinkColorMapper`, an optional colour of its own. A link without both
/// ends, or with an end that is not a node of the series, is skipped by the layout.
#[repr(C)]
pub struct SankeyLink {
    base: FerroObject,
    entity_id: u64,
}
ferro_class!(SankeyLink: FerroObject);
ferro_class_info!(SankeyLink {
    new: SankeyLink::new
});
impl FerroObjectImpl for SankeyLink {}
impl SankeyLink {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            entity_id: crate::identity::next_entity_id(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// A link of the given weight from `source` to `target` (`SankeyLink(source, target, weight)`).
    pub fn with_nodes(source: &Ref<SankeyNode>, target: &Ref<SankeyNode>, weight: f64) -> Ref<Self> {
        let link = Self::new();
        link.set_source(Some(source.clone()));
        link.set_target(Some(target.clone()));
        link.set_weight(weight);
        link
    }
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }
    /// The engine link. A missing end takes the identity 0, which no node has.
    pub fn snapshot(&self) -> ferroui_livecharts::SankeyLink {
        ferroui_livecharts::SankeyLink {
            source: self.source().map_or(0, |node| node.entity_id()),
            target: self.target().map_or(0, |node| node.entity_id()),
            weight: self.weight(),
            color: self.color(),
        }
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
ferro_properties! { impl SankeyLink {
    pub fn source_property() -> StyledProperty<Option<Ref<SankeyNode>>> {
        FerroProperty::register::<SankeyLink, _>("Source", None)
    }
    pub fn target_property() -> StyledProperty<Option<Ref<SankeyNode>>> {
        FerroProperty::register::<SankeyLink, _>("Target", None)
    }
    pub fn weight_property() -> StyledProperty<f64> {
        FerroProperty::register::<SankeyLink, _>("Weight", 0.0)
    }
    pub fn color_property() -> StyledProperty<Option<Color>> {
        FerroProperty::register::<SankeyLink, _>("Color", None)
    }
} }

/// A node of a treemap series (`TreemapNode`). A leaf carries its weight in `Value`; an internal
/// node has `Children` (an empty list is still an internal node) and either an explicit nonzero
/// value or the sum of its children.
#[repr(C)]
pub struct TreemapNode {
    base: FerroObject,
    entity_id: u64,
}
ferro_class!(TreemapNode: FerroObject);
ferro_class_info!(TreemapNode { new: TreemapNode::new, markup: { content: Children } });
impl FerroObjectImpl for TreemapNode {}
impl TreemapNode {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            entity_id: crate::identity::next_entity_id(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// A leaf of the given weight (`TreemapNode(value, name)`).
    pub fn with_value(value: f64, name: impl Into<String>) -> Ref<Self> {
        let node = Self::new();
        node.set_numeric_value(value);
        node.set_name(name.into());
        node
    }
    /// An internal node whose weight is the sum of its children (`TreemapNode(name, children)`).
    pub fn with_children(name: impl Into<String>, children: TreemapNodes) -> Ref<Self> {
        let node = Self::new();
        node.set_name(name.into());
        node.set_children(Some(children));
        node
    }
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }
    pub fn name(&self) -> String {
        self.base.get_value(Self::name_property())
    }
    pub fn set_name(&self, value: String) {
        self.base.set_value(Self::name_property(), value);
    }
    /// The weight of the node (`Value`).
    pub fn value(&self) -> f64 {
        self.base.get_value(Self::value_property())
    }
    /// Sets `Value`; named apart from the property system's `set_value`.
    pub fn set_numeric_value(&self, value: f64) {
        self.base.set_value(Self::value_property(), value);
    }
    /// The children, or `None` for a leaf.
    pub fn children(&self) -> Option<TreemapNodes> {
        self.base.get_value(Self::children_property())
    }
    pub fn set_children(&self, value: Option<TreemapNodes>) {
        self.base.set_value(Self::children_property(), value);
    }
}
ferro_properties! { impl TreemapNode {
    pub fn name_property() -> StyledProperty<String> {
        FerroProperty::register::<TreemapNode, _>("Name", String::new())
    }
    pub fn value_property() -> StyledProperty<f64> {
        FerroProperty::register::<TreemapNode, _>("Value", 0.0)
    }
    pub fn children_property() -> StyledProperty<Option<TreemapNodes>> {
        FerroProperty::register::<TreemapNode, _>("Children", None)
    }
} }

/// The engine series of a series of a Sankey chart, from the snapshot of its shared properties.
/// The source chart casts its series to `ISankeySeries`; another kind of series is an error.
pub(crate) fn sankey_series(
    model: &Ref<ChartSeries>,
    series: Series,
) -> Result<ferroui_livecharts::SankeySeries, ChartError> {
    let Some(sankey) = model.cast::<SankeySeries>() else {
        return Err(ChartError::InvalidData(
            "a Sankey chart takes Sankey series only".into(),
        ));
    };
    Ok(ferroui_livecharts::SankeySeries {
        series,
        nodes: sankey.nodes().to_vec().iter().map(|n| n.snapshot()).collect(),
        links: sankey.links().to_vec().iter().map(|l| l.snapshot()).collect(),
        node_width: sankey.node_width(),
        node_padding: sankey.node_padding(),
        node_corner_radius: sankey.node_corner_radius(),
        layout_iterations: sankey.layout_iterations(),
        node_label_position: sankey.node_label_position(),
        layout: sankey.layout(),
        arc_span_degrees: sankey.arc_span_degrees(),
        link_fill: sankey.link_fill(),
    })
}

/// The engine series of a series of a treemap chart (`TreemapChartEngine` casts its series to
/// `ITreemapSeries`; another kind of series is an error).
pub(crate) fn treemap_series(
    model: &Ref<ChartSeries>,
    series: Series,
) -> Result<ferroui_livecharts::TreemapSeries, ChartError> {
    let Some(treemap) = model.cast::<TreemapSeries>() else {
        return Err(ChartError::InvalidData(
            "a treemap chart takes treemap series only".into(),
        ));
    };
    Ok(ferroui_livecharts::TreemapSeries {
        series,
        nodes: treemap_snapshot(&treemap.nodes(), &mut Vec::new())?,
    })
}

/// Owned engine trees of the roots, and the identities of their nodes in preorder (the point
/// indexes of the engine). A node shared by two parents is copied; a node that contains itself
/// is an error, where the source would recurse without end.
fn treemap_snapshot(
    roots: &TreemapNodes,
    ids: &mut Vec<u64>,
) -> Result<Vec<ferroui_livecharts::TreemapNode>, ChartError> {
    enum Frame {
        Enter(Ref<TreemapNode>),
        Exit(Ref<TreemapNode>, usize, bool),
    }
    let mut pending: Vec<_> = roots.to_vec().into_iter().rev().map(Frame::Enter).collect();
    let mut path = HashSet::new();
    let mut output = Vec::new();
    while let Some(frame) = pending.pop() {
        match frame {
            Frame::Enter(node) => {
                if !path.insert(node.entity_id()) {
                    return Err(ChartError::InvalidData(
                        "a treemap node contains itself: the tree has a cycle".into(),
                    ));
                }
                ids.push(node.entity_id());
                let children = node.children();
                let items = children.as_ref().map(|c| c.to_vec()).unwrap_or_default();
                pending.push(Frame::Exit(node, items.len(), children.is_some()));
                pending.extend(items.into_iter().rev().map(Frame::Enter));
            }
            Frame::Exit(node, count, internal) => {
                path.remove(&node.entity_id());
                let children = output.split_off(output.len() - count);
                output.push(ferroui_livecharts::TreemapNode {
                    name: node.name(),
                    value: node.value(),
                    children: internal.then_some(children),
                });
            }
        }
    }
    Ok(output)
}

/// The identities of the nodes (and links) of a Sankey or treemap series at the point indexes of
/// its engine, so that a transition follows a node when the collections are reordered: a Sankey
/// node at its index, a link after the nodes, a treemap node at its preorder index.
pub(crate) fn insert_entity_keys(
    series_index: usize,
    model: &Ref<ChartSeries>,
    keys: &mut HashMap<PointId, (u64, u64)>,
) {
    let entity = model.entity_id();
    let mut insert = |point: usize, id: u64| {
        keys.insert(
            PointId {
                series: series_index,
                point,
            },
            (entity, id),
        );
    };
    if let Some(sankey) = model.cast::<SankeySeries>() {
        let nodes = sankey.nodes().to_vec();
        for (index, node) in nodes.iter().enumerate() {
            insert(index, node.entity_id());
        }
        for (index, link) in sankey.links().to_vec().iter().enumerate() {
            insert(nodes.len() + index, link.entity_id());
        }
    } else if let Some(treemap) = model.cast::<TreemapSeries>() {
        let mut ids = Vec::new();
        if treemap_snapshot(&treemap.nodes(), &mut ids).is_ok() {
            for (index, id) in ids.into_iter().enumerate() {
                insert(index, id);
            }
        }
    }
}

/// Observes the nodes and links of a Sankey or treemap series: `changed` when a node or a link
/// changes, `restructured` when a collection, a child list or a link end changes (the
/// subscriptions are then made again). Other series have none.
pub(crate) fn observe_nodes(
    model: &Ref<ChartSeries>,
    changed: Rc<dyn Fn()>,
    restructured: Rc<dyn Fn()>,
) -> Vec<Subscription> {
    let mut subscriptions = Vec::new();
    if let Some(sankey) = model.cast::<SankeySeries>() {
        let nodes: SankeyNodes = sankey.nodes();
        subscriptions.push(observe_list(&nodes, restructured.clone()));
        for node in nodes.to_vec() {
            let changed = changed.clone();
            subscriptions.push(Subscription::new(node.property_changed(move |_| changed())));
        }
        let links: SankeyLinks = sankey.links();
        subscriptions.push(observe_list(&links, restructured.clone()));
        for link in links.to_vec() {
            let changed = changed.clone();
            subscriptions.push(Subscription::new(link.property_changed(move |_| changed())));
        }
    } else if let Some(treemap) = model.cast::<TreemapSeries>() {
        let mut visited = HashSet::new();
        let mut pending = vec![treemap.nodes()];
        while let Some(list) = pending.pop() {
            subscriptions.push(observe_list(&list, restructured.clone()));
            for node in list.to_vec() {
                if !visited.insert(node.entity_id()) {
                    continue;
                }
                let (changed, restructured) = (changed.clone(), restructured.clone());
                subscriptions.push(Subscription::new(node.property_changed(
                    move |change: &FerroPropertyChangedEventArgs<'_>| {
                        if change.property() == TreemapNode::children_property().as_property() {
                            restructured();
                        } else {
                            changed();
                        }
                    },
                )));
                if let Some(children) = node.children() {
                    pending.push(children);
                }
            }
        }
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
    fn links_follow_their_node_objects_and_missing_ends_match_no_node() {
        let _scope = Dispatcher::unit_test_scope();
        let a = SankeyNode::with_name("A");
        let b = SankeyNode::with_name("B");
        assert_ne!(a.entity_id(), b.entity_id());
        let link = SankeyLink::with_nodes(&a, &b, 7.0);
        let snapshot = link.snapshot();
        assert_eq!((snapshot.source, snapshot.target), (a.entity_id(), b.entity_id()));
        assert_eq!(snapshot.weight, 7.0);
        link.set_target(None);
        assert_eq!(link.snapshot().target, 0);
    }

    #[test]
    fn tree_snapshots_keep_leaves_apart_from_empty_internal_nodes() {
        let _scope = Dispatcher::unit_test_scope();
        let leaf = TreemapNode::with_value(10.0, "leaf");
        let empty = TreemapNode::with_children("empty", TreemapNodes::new());
        let root = TreemapNode::with_children(
            "root",
            TreemapNodes::from_items([leaf.clone(), empty.clone(), leaf.clone()]),
        );
        let mut ids = Vec::new();
        let roots = TreemapNodes::from_items([root.clone()]);
        let snapshot = treemap_snapshot(&roots, &mut ids).unwrap();
        assert_eq!(snapshot.len(), 1);
        let children = snapshot[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 3);
        assert!(children[0].children.is_none());
        assert_eq!(children[1].children, Some(Vec::new()));
        // A node shared twice is copied, in preorder.
        assert_eq!(
            ids,
            vec![
                root.entity_id(),
                leaf.entity_id(),
                empty.entity_id(),
                leaf.entity_id()
            ]
        );
    }

    #[test]
    fn a_tree_that_contains_itself_is_an_error() {
        let _scope = Dispatcher::unit_test_scope();
        let root = TreemapNode::with_value(10.0, "cyclic");
        root.set_children(Some(TreemapNodes::from_items([root.clone()])));
        let error = treemap_snapshot(&TreemapNodes::from_items([root.clone()]), &mut Vec::new())
            .unwrap_err();
        assert!(error.to_string().contains("cycle"));
        root.set_children(None);
    }

    #[test]
    fn observation_reports_changes_and_restructuring_and_ends_on_drop() {
        let _scope = Dispatcher::unit_test_scope();
        let leaf = TreemapNode::with_value(10.0, "leaf");
        let children = TreemapNodes::from_items([leaf.clone()]);
        let root = TreemapNode::with_children("root", children.clone());
        let series = TreemapSeries::new();
        series.nodes().add(root.clone());
        let changes = Rc::new(Cell::new(0));
        let rebinds = Rc::new(Cell::new(0));
        let (c, r) = (changes.clone(), rebinds.clone());
        let subscriptions = observe_nodes(
            series.upcast_ref::<ChartSeries>(),
            Rc::new(move || c.set(c.get() + 1)),
            Rc::new(move || r.set(r.get() + 1)),
        );
        leaf.set_numeric_value(20.0);
        assert_eq!((changes.get(), rebinds.get()), (1, 0));
        children.add(TreemapNode::with_value(5.0, "next"));
        assert_eq!((changes.get(), rebinds.get()), (1, 1));
        root.set_children(None);
        assert_eq!((changes.get(), rebinds.get()), (1, 2));
        drop(subscriptions);
        leaf.set_numeric_value(30.0);
        series.nodes().clear();
        assert_eq!((changes.get(), rebinds.get()), (1, 2));
    }
}
