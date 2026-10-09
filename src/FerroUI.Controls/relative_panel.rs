// The layout algorithm is adapted from the relative panel of the HandyControl
// project (https://github.com/HandyOrg/HandyControl), MIT License.

use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, BoxedValue, FerroObject,
    FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// A panel that arranges its children relative to each other and to the
/// panel itself.
#[repr(C)]
pub struct RelativePanel {
    base: Panel,
    child_graph: RefCell<Graph>,
}

ferro_class!(RelativePanel: Panel);
ferroui_base::ferro_class_info!(RelativePanel { new: RelativePanel::new });
ferro_impl_classes!(
    RelativePanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferroui_base::ferro_impl_classes!(RelativePanel: FerroObjectImpl);

impl LayoutableImpl for RelativePanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        // The graph is taken out of its cell for the duration of the pass:
        // measuring children runs arbitrary code.
        let mut child_graph = this.child_graph.take();

        child_graph.clear();
        for child in this.children().snapshot().iter() {
            let node = child_graph.add_node(child.clone().upcast());

            let link = child_graph.add_link(node, this.get_dependency_element(Self::align_left_with_property(), child));
            child_graph.nodes[node].align_left_with_node = link;
            let link = child_graph.add_link(node, this.get_dependency_element(Self::align_top_with_property(), child));
            child_graph.nodes[node].align_top_with_node = link;
            let link =
                child_graph.add_link(node, this.get_dependency_element(Self::align_right_with_property(), child));
            child_graph.nodes[node].align_right_with_node = link;
            let link =
                child_graph.add_link(node, this.get_dependency_element(Self::align_bottom_with_property(), child));
            child_graph.nodes[node].align_bottom_with_node = link;

            let link = child_graph.add_link(node, this.get_dependency_element(Self::left_of_property(), child));
            child_graph.nodes[node].left_of_node = link;
            let link = child_graph.add_link(node, this.get_dependency_element(Self::above_property(), child));
            child_graph.nodes[node].above_node = link;
            let link = child_graph.add_link(node, this.get_dependency_element(Self::right_of_property(), child));
            child_graph.nodes[node].right_of_node = link;
            let link = child_graph.add_link(node, this.get_dependency_element(Self::below_property(), child));
            child_graph.nodes[node].below_node = link;

            let link = child_graph
                .add_link(node, this.get_dependency_element(Self::align_horizontal_center_with_property(), child));
            child_graph.nodes[node].align_horizontal_center_with = link;
            let link = child_graph
                .add_link(node, this.get_dependency_element(Self::align_vertical_center_with_property(), child));
            child_graph.nodes[node].align_vertical_center_with = link;
        }

        child_graph.measure(available_size);

        child_graph.reset(false);
        let calc_width = this.width().is_nan() && (this.horizontal_alignment() != HorizontalAlignment::Stretch);
        let calc_height = this.height().is_nan() && (this.vertical_alignment() != VerticalAlignment::Stretch);

        let bounding_size = child_graph.get_bounding_size(calc_width, calc_height);
        child_graph.reset(true);
        child_graph.measure(bounding_size);

        this.child_graph.replace(child_graph);

        bounding_size
    }

    fn arrange_override(this: &Self, arrange_size: Size) -> Size {
        // The slots are computed before any child is arranged: arranging
        // children runs arbitrary code.
        let slots: Vec<(Ref<Layoutable>, Rect)> = this
            .child_graph
            .borrow()
            .nodes
            .iter()
            .map(|node| (node.element.clone(), node.arrange_rect(arrange_size)))
            .collect();
        for (element, rect) in slots {
            element.arrange(rect);
        }
        arrange_size
    }
}

ferroui_base::ferro_properties! { impl RelativePanel, also [
    RelativePanel::above_property,
    RelativePanel::align_bottom_with_panel_property,
    RelativePanel::align_bottom_with_property,
    RelativePanel::align_horizontal_center_with_panel_property,
    RelativePanel::align_horizontal_center_with_property,
    RelativePanel::align_left_with_panel_property,
    RelativePanel::align_left_with_property,
    RelativePanel::align_right_with_panel_property,
    RelativePanel::align_right_with_property,
    RelativePanel::align_top_with_panel_property,
    RelativePanel::align_top_with_property,
    RelativePanel::align_vertical_center_with_panel_property,
    RelativePanel::align_vertical_center_with_property,
    RelativePanel::below_property,
    RelativePanel::left_of_property,
    RelativePanel::right_of_property,
] {} }

impl RelativePanel {
    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<RelativePanel>(true);

        let properties = [
            Self::align_left_with_panel_property().as_property(),
            Self::align_left_with_property().as_property(),
            Self::left_of_property().as_property(),
            Self::align_right_with_panel_property().as_property(),
            Self::align_right_with_property().as_property(),
            Self::right_of_property().as_property(),
            Self::align_top_with_panel_property().as_property(),
            Self::align_top_with_property().as_property(),
            Self::above_property().as_property(),
            Self::align_bottom_with_panel_property().as_property(),
            Self::align_bottom_with_property().as_property(),
            Self::below_property().as_property(),
            Self::align_horizontal_center_with_panel_property().as_property(),
            Self::align_horizontal_center_with_property().as_property(),
            Self::align_vertical_center_with_panel_property().as_property(),
            Self::align_vertical_center_with_property().as_property(),
        ];
        Panel::affects_parent_arrange::<RelativePanel>(&properties);
        Panel::affects_parent_measure::<RelativePanel>(&properties);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct(), child_graph: RefCell::new(Graph::default()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The child of the panel that a relative-positioning property of `child`
    /// refers to.
    ///
    /// Panics if the property refers to an element that is not a child of the
    /// panel.
    fn get_dependency_element(
        &self,
        property: &'static AttachedProperty<Option<BoxedValue>>,
        child: &Ref<Control>,
    ) -> Option<Ref<Layoutable>> {
        let dependency = child.get_value(property)?;
        let dependency = &*dependency;

        let layoutable = match dependency.downcast_ref::<Ref<Control>>() {
            Some(control) => control.clone().upcast::<Layoutable>(),
            None => dependency.downcast_ref::<Ref<Layoutable>>()?.clone(),
        };

        if self.children().snapshot().iter().any(|c| *c == layoutable) {
            return Some(layoutable);
        }

        panic!("RelativePanel error: Element does not exist in the current context: {}", property.name());
    }

    ferro_property!(for RelativePanel;
        /// Defines the `Above` attached property.
        pub fn above_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("Above", None)
        }
    );

    /// Gets the value of the `Above` attached property for an object.
    pub fn get_above(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::above_property())
    }

    /// Sets the value of the `Above` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_above(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::above_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignBottomWithPanel` attached property.
        pub fn align_bottom_with_panel_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignBottomWithPanel", false)
        }
    );

    /// Gets the value of the `AlignBottomWithPanel` attached property for an object.
    pub fn get_align_bottom_with_panel(obj: &FerroObject) -> bool {
        obj.get_value(Self::align_bottom_with_panel_property())
    }

    /// Sets the value of the `AlignBottomWithPanel` attached property for an object.
    pub fn set_align_bottom_with_panel(obj: &FerroObject, value: bool) {
        obj.set_value(Self::align_bottom_with_panel_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignBottomWith` attached property.
        pub fn align_bottom_with_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignBottomWith", None)
        }
    );

    /// Gets the value of the `AlignBottomWith` attached property for an object.
    pub fn get_align_bottom_with(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::align_bottom_with_property())
    }

    /// Sets the value of the `AlignBottomWith` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_align_bottom_with(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::align_bottom_with_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignHorizontalCenterWithPanel` attached property.
        pub fn align_horizontal_center_with_panel_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignHorizontalCenterWithPanel", false)
        }
    );

    /// Gets the value of the `AlignHorizontalCenterWithPanel` attached property for an object.
    pub fn get_align_horizontal_center_with_panel(obj: &FerroObject) -> bool {
        obj.get_value(Self::align_horizontal_center_with_panel_property())
    }

    /// Sets the value of the `AlignHorizontalCenterWithPanel` attached property for an object.
    pub fn set_align_horizontal_center_with_panel(obj: &FerroObject, value: bool) {
        obj.set_value(Self::align_horizontal_center_with_panel_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignHorizontalCenterWith` attached property.
        pub fn align_horizontal_center_with_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignHorizontalCenterWith", None)
        }
    );

    /// Gets the value of the `AlignHorizontalCenterWith` attached property for an object.
    pub fn get_align_horizontal_center_with(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::align_horizontal_center_with_property())
    }

    /// Sets the value of the `AlignHorizontalCenterWith` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_align_horizontal_center_with(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::align_horizontal_center_with_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignLeftWithPanel` attached property.
        pub fn align_left_with_panel_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignLeftWithPanel", false)
        }
    );

    /// Gets the value of the `AlignLeftWithPanel` attached property for an object.
    pub fn get_align_left_with_panel(obj: &FerroObject) -> bool {
        obj.get_value(Self::align_left_with_panel_property())
    }

    /// Sets the value of the `AlignLeftWithPanel` attached property for an object.
    pub fn set_align_left_with_panel(obj: &FerroObject, value: bool) {
        obj.set_value(Self::align_left_with_panel_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignLeftWith` attached property.
        pub fn align_left_with_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignLeftWith", None)
        }
    );

    /// Gets the value of the `AlignLeftWith` attached property for an object.
    pub fn get_align_left_with(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::align_left_with_property())
    }

    /// Sets the value of the `AlignLeftWith` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_align_left_with(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::align_left_with_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignRightWithPanel` attached property.
        pub fn align_right_with_panel_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignRightWithPanel", false)
        }
    );

    /// Gets the value of the `AlignRightWithPanel` attached property for an object.
    pub fn get_align_right_with_panel(obj: &FerroObject) -> bool {
        obj.get_value(Self::align_right_with_panel_property())
    }

    /// Sets the value of the `AlignRightWithPanel` attached property for an object.
    pub fn set_align_right_with_panel(obj: &FerroObject, value: bool) {
        obj.set_value(Self::align_right_with_panel_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignRightWith` attached property.
        pub fn align_right_with_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignRightWith", None)
        }
    );

    /// Gets the value of the `AlignRightWith` attached property for an object.
    pub fn get_align_right_with(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::align_right_with_property())
    }

    /// Sets the value of the `AlignRightWith` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_align_right_with(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::align_right_with_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignTopWithPanel` attached property.
        pub fn align_top_with_panel_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignTopWithPanel", false)
        }
    );

    /// Gets the value of the `AlignTopWithPanel` attached property for an object.
    pub fn get_align_top_with_panel(obj: &FerroObject) -> bool {
        obj.get_value(Self::align_top_with_panel_property())
    }

    /// Sets the value of the `AlignTopWithPanel` attached property for an object.
    pub fn set_align_top_with_panel(obj: &FerroObject, value: bool) {
        obj.set_value(Self::align_top_with_panel_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignTopWith` attached property.
        pub fn align_top_with_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignTopWith", None)
        }
    );

    /// Gets the value of the `AlignTopWith` attached property for an object.
    pub fn get_align_top_with(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::align_top_with_property())
    }

    /// Sets the value of the `AlignTopWith` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_align_top_with(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::align_top_with_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignVerticalCenterWithPanel` attached property.
        pub fn align_vertical_center_with_panel_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignVerticalCenterWithPanel", false)
        }
    );

    /// Gets the value of the `AlignVerticalCenterWithPanel` attached property for an object.
    pub fn get_align_vertical_center_with_panel(obj: &FerroObject) -> bool {
        obj.get_value(Self::align_vertical_center_with_panel_property())
    }

    /// Sets the value of the `AlignVerticalCenterWithPanel` attached property for an object.
    pub fn set_align_vertical_center_with_panel(obj: &FerroObject, value: bool) {
        obj.set_value(Self::align_vertical_center_with_panel_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `AlignVerticalCenterWith` attached property.
        pub fn align_vertical_center_with_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("AlignVerticalCenterWith", None)
        }
    );

    /// Gets the value of the `AlignVerticalCenterWith` attached property for an object.
    pub fn get_align_vertical_center_with(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::align_vertical_center_with_property())
    }

    /// Sets the value of the `AlignVerticalCenterWith` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_align_vertical_center_with(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::align_vertical_center_with_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `Below` attached property.
        pub fn below_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("Below", None)
        }
    );

    /// Gets the value of the `Below` attached property for an object.
    pub fn get_below(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::below_property())
    }

    /// Sets the value of the `Below` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_below(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::below_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `LeftOf` attached property.
        pub fn left_of_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("LeftOf", None)
        }
    );

    /// Gets the value of the `LeftOf` attached property for an object.
    pub fn get_left_of(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::left_of_property())
    }

    /// Sets the value of the `LeftOf` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_left_of(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::left_of_property(), value)
    }

    ferro_property!(for RelativePanel;
        /// Defines the `RightOf` attached property.
        pub fn right_of_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<RelativePanel, Layoutable, _>("RightOf", None)
        }
    );

    /// Gets the value of the `RightOf` attached property for an object.
    pub fn get_right_of(obj: &FerroObject) -> Option<BoxedValue> {
        obj.get_value(Self::right_of_property())
    }

    /// Sets the value of the `RightOf` attached property for an object. The
    /// layout uses the value when it holds a control of the panel; see
    /// [`Control::boxed`].
    pub fn set_right_of(obj: &FerroObject, value: Option<BoxedValue>) {
        obj.set_value(Self::right_of_property(), value)
    }
}

struct GraphNode {
    measured: bool,
    element: Ref<Layoutable>,
    horizontal_offset_flag: bool,
    vertical_offset_flag: bool,
    bounding_size: Size,
    origin_desired_size: Size,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    /// The nodes this node depends on, in the order the links were added.
    outgoing_nodes: Vec<usize>,
    align_left_with_node: Option<usize>,
    align_top_with_node: Option<usize>,
    align_right_with_node: Option<usize>,
    align_bottom_with_node: Option<usize>,
    left_of_node: Option<usize>,
    above_node: Option<usize>,
    right_of_node: Option<usize>,
    below_node: Option<usize>,
    align_horizontal_center_with: Option<usize>,
    align_vertical_center_with: Option<usize>,
}

impl GraphNode {
    fn new(element: Ref<Layoutable>) -> Self {
        Self {
            measured: false,
            element,
            horizontal_offset_flag: false,
            vertical_offset_flag: false,
            bounding_size: Size::default(),
            origin_desired_size: Size::default(),
            left: f64::NAN,
            top: f64::NAN,
            right: f64::NAN,
            bottom: f64::NAN,
            outgoing_nodes: Vec::new(),
            align_left_with_node: None,
            align_top_with_node: None,
            align_right_with_node: None,
            align_bottom_with_node: None,
            left_of_node: None,
            above_node: None,
            right_of_node: None,
            below_node: None,
            align_horizontal_center_with: None,
            align_vertical_center_with: None,
        }
    }

    fn arrange_rect(&self, arrange_size: Size) -> Rect {
        Rect::new(
            self.left,
            self.top,
            max(arrange_size.width - self.left - self.right, 0.0),
            max(arrange_size.height - self.top - self.bottom, 0.0),
        )
    }

    fn reset(&mut self, clear_pos: bool) {
        if clear_pos {
            self.left = f64::NAN;
            self.top = f64::NAN;
            self.right = f64::NAN;
            self.bottom = f64::NAN;
        }

        self.measured = false;
    }
}

/// The nodes a measure pass visits: every node of the graph, or the outgoing
/// nodes of one node.
#[derive(Clone, Copy)]
enum NodeSet {
    All,
    OutgoingOf(usize),
}

/// The dependency graph of the children of a [`RelativePanel`]. Nodes refer
/// to each other by their index in `nodes`, which is their insertion order.
#[derive(Default)]
struct Graph {
    node_dic: HashMap<Ref<Layoutable>, usize>,
    nodes: Vec<GraphNode>,
    available_size: Size,
}

impl Graph {
    fn clear(&mut self) {
        self.available_size = Size::default();
        self.node_dic.clear();
        self.nodes.clear();
    }

    fn reset(&mut self, clear_pos: bool) {
        for node in &mut self.nodes {
            node.reset(clear_pos);
        }
    }

    fn add_link(&mut self, from: usize, to: Option<Ref<Layoutable>>) -> Option<usize> {
        let to = to?;
        let node_to = self.add_node(to);

        if !self.nodes[from].outgoing_nodes.contains(&node_to) {
            self.nodes[from].outgoing_nodes.push(node_to);
        }
        Some(node_to)
    }

    fn add_node(&mut self, value: Ref<Layoutable>) -> usize {
        if let Some(&node) = self.node_dic.get(&value) {
            return node;
        }

        let node = self.nodes.len();
        self.nodes.push(GraphNode::new(value.clone()));
        self.node_dic.insert(value, node);
        node
    }

    fn node_count(&self, set: NodeSet) -> usize {
        match set {
            NodeSet::All => self.nodes.len(),
            NodeSet::OutgoingOf(node) => self.nodes[node].outgoing_nodes.len(),
        }
    }

    fn node_at(&self, set: NodeSet, index: usize) -> usize {
        match set {
            NodeSet::All => index,
            NodeSet::OutgoingOf(node) => self.nodes[node].outgoing_nodes[index],
        }
    }

    fn measure(&mut self, available_size: Size) {
        self.available_size = available_size;
        let mut set = HashSet::new();
        self.measure_nodes(NodeSet::All, &mut set);
    }

    fn measure_nodes(&mut self, nodes: NodeSet, set: &mut HashSet<usize>) {
        for index in 0..self.node_count(nodes) {
            let node = self.node_at(nodes, index);

            if !self.nodes[node].measured && self.nodes[node].outgoing_nodes.is_empty() {
                self.measure_child(node);
                continue;
            }

            if self.nodes[node].outgoing_nodes.iter().all(|&item| self.nodes[item].measured) {
                self.measure_child(node);
                continue;
            }

            if !set.insert(node) {
                panic!("RelativePanel error: Circular dependency detected. Layout could not complete.");
            }

            self.measure_nodes(NodeSet::OutgoingOf(node), set);

            if !self.nodes[node].measured {
                self.measure_child(node);
            }
        }
    }

    fn measure_child(&mut self, index: usize) {
        let available_size = self.available_size;
        let child = self.nodes[index].element.clone();
        child.measure(Size::new(f64::INFINITY, f64::INFINITY));
        self.nodes[index].origin_desired_size = child.desired_size();

        let align_left_with_panel = RelativePanel::get_align_left_with_panel(&child);
        let align_top_with_panel = RelativePanel::get_align_top_with_panel(&child);
        let align_right_with_panel = RelativePanel::get_align_right_with_panel(&child);
        let align_bottom_with_panel = RelativePanel::get_align_bottom_with_panel(&child);

        // The positions of the nodes this node refers to.
        let left_of = |node: Option<usize>| node.map(|n| self.nodes[n].left);
        let top_of = |node: Option<usize>| node.map(|n| self.nodes[n].top);
        let right_of = |node: Option<usize>| node.map(|n| self.nodes[n].right);
        let bottom_of = |node: Option<usize>| node.map(|n| self.nodes[n].bottom);

        let node = &self.nodes[index];
        let align_left_with_left = left_of(node.align_left_with_node);
        let align_top_with_top = top_of(node.align_top_with_node);
        let align_right_with_right = right_of(node.align_right_with_node);
        let align_bottom_with_bottom = bottom_of(node.align_bottom_with_node);
        let below_bottom = bottom_of(node.below_node);
        let right_of_right = right_of(node.right_of_node);
        let left_of_left = left_of(node.left_of_node);
        let above_top = top_of(node.above_node);
        let horizontal_center =
            node.align_horizontal_center_with.map(|n| (self.nodes[n].left, self.nodes[n].right));
        let vertical_center = node.align_vertical_center_with.map(|n| (self.nodes[n].top, self.nodes[n].bottom));

        let node = &mut self.nodes[index];

        if align_left_with_panel {
            node.left = 0.0;
        }
        if align_top_with_panel {
            node.top = 0.0;
        }
        if align_right_with_panel {
            node.right = 0.0;
        }
        if align_bottom_with_panel {
            node.bottom = 0.0;
        }

        if let Some(left) = align_left_with_left {
            node.left = if node.left.is_nan() { left } else { left * 0.5 };
        }

        if let Some(top) = align_top_with_top {
            node.top = if node.top.is_nan() { top } else { top * 0.5 };
        }

        if let Some(right) = align_right_with_right {
            node.right = if node.right.is_nan() { right } else { right * 0.5 };
        }

        if let Some(bottom) = align_bottom_with_bottom {
            node.bottom = if node.bottom.is_nan() { bottom } else { bottom * 0.5 };
        }

        if let Some(bottom) = below_bottom {
            if node.top.is_nan() {
                node.top = available_size.height - bottom;
            }
        }

        if let Some(right) = right_of_right {
            if node.left.is_nan() {
                node.left = available_size.width - right;
            }
        }

        let mut available_height = available_size.height - node.top - node.bottom;
        if available_height.is_nan() {
            available_height = available_size.height;

            if !node.top.is_nan() && node.bottom.is_nan() {
                available_height -= node.top;
            } else if node.top.is_nan() && !node.bottom.is_nan() {
                available_height -= node.bottom;
            }
        }

        let mut available_width = available_size.width - node.left - node.right;
        if available_width.is_nan() {
            available_width = available_size.width;

            if !node.left.is_nan() && node.right.is_nan() {
                available_width -= node.left;
            } else if node.left.is_nan() && !node.right.is_nan() {
                available_width -= node.right;
            }
        }

        child.measure(Size::new(max(available_width, 0.0), max(available_height, 0.0)));
        let child_size = child.desired_size();

        let node = &mut self.nodes[index];

        if let Some(left) = left_of_left {
            if node.left.is_nan() {
                node.left = left - child_size.width;
            }
        }

        if let Some(top) = above_top {
            if node.top.is_nan() {
                node.top = top - child_size.height;
            }
        }

        if let Some(right) = right_of_right {
            if node.right.is_nan() {
                node.right = right - child_size.width;
            }
        }

        if let Some(bottom) = below_bottom {
            if node.bottom.is_nan() {
                node.bottom = bottom - child_size.height;
            }
        }

        if let Some((left, right)) = horizontal_center {
            let half_width_left = (available_size.width + left - right - child_size.width) * 0.5;
            let half_width_right = (available_size.width - left + right - child_size.width) * 0.5;

            if node.left.is_nan() {
                node.left = half_width_left;
            } else {
                node.left = (node.left + half_width_left) * 0.5;
            }

            if node.right.is_nan() {
                node.right = half_width_right;
            } else {
                node.right = (node.right + half_width_right) * 0.5;
            }
        }

        if let Some((top, bottom)) = vertical_center {
            let half_height_top = (available_size.height + top - bottom - child_size.height) * 0.5;
            let half_height_bottom = (available_size.height - top + bottom - child_size.height) * 0.5;

            if node.top.is_nan() {
                node.top = half_height_top;
            } else {
                node.top = (node.top + half_height_top) * 0.5;
            }

            if node.bottom.is_nan() {
                node.bottom = half_height_bottom;
            } else {
                node.bottom = (node.bottom + half_height_bottom) * 0.5;
            }
        }

        if RelativePanel::get_align_horizontal_center_with_panel(&child) {
            let half_sub_width = (available_size.width - child_size.width) * 0.5;

            if node.left.is_nan() {
                node.left = half_sub_width;
            } else {
                node.left = (node.left + half_sub_width) * 0.5;
            }

            if node.right.is_nan() {
                node.right = half_sub_width;
            } else {
                node.right = (node.right + half_sub_width) * 0.5;
            }
        }

        if RelativePanel::get_align_vertical_center_with_panel(&child) {
            let half_sub_height = (available_size.height - child_size.height) * 0.5;

            if node.top.is_nan() {
                node.top = half_sub_height;
            } else {
                node.top = (node.top + half_sub_height) * 0.5;
            }

            if node.bottom.is_nan() {
                node.bottom = half_sub_height;
            } else {
                node.bottom = (node.bottom + half_sub_height) * 0.5;
            }
        }

        if node.left.is_nan() {
            if !node.right.is_nan() {
                node.left = available_size.width - node.right - child_size.width;
            } else {
                node.left = 0.0;
                node.right = available_size.width - child_size.width;
            }
        } else if !node.left.is_nan() && node.right.is_nan() {
            node.right = available_size.width - node.left - child_size.width;
        }

        if node.top.is_nan() {
            if !node.bottom.is_nan() {
                node.top = available_size.height - node.bottom - child_size.height;
            } else {
                node.top = 0.0;
                node.bottom = available_size.height - child_size.height;
            }
        } else if !node.top.is_nan() && node.bottom.is_nan() {
            node.bottom = available_size.height - node.top - child_size.height;
        }

        node.measured = true;
    }

    fn get_bounding_size(&mut self, calc_width: bool, calc_height: bool) -> Size {
        let mut bounding_size = Size::default();

        for node in 0..self.nodes.len() {
            let size = self.get_node_bounding_size(node);
            bounding_size = bounding_size.with_width(max(bounding_size.width, size.width));
            bounding_size = bounding_size.with_height(max(bounding_size.height, size.height));
        }

        let available_width =
            if self.available_size.width.is_infinite() { bounding_size.width } else { self.available_size.width };
        let available_height =
            if self.available_size.height.is_infinite() { bounding_size.height } else { self.available_size.height };

        bounding_size = bounding_size.with_width(if calc_width { bounding_size.width } else { available_width });
        bounding_size = bounding_size.with_height(if calc_height { bounding_size.height } else { available_height });
        bounding_size
    }

    fn get_node_bounding_size(&mut self, index: usize) -> Size {
        let node = &self.nodes[index];
        if node.left < 0.0 || node.top < 0.0 {
            return Size::default();
        }
        if node.measured {
            return node.bounding_size;
        }

        let desired_size = node.element.desired_size();
        let bounding_size = if node.outgoing_nodes.is_empty() {
            desired_size
        } else {
            self.get_bounding_size_of(index, desired_size)
        };

        let node = &mut self.nodes[index];
        node.bounding_size = bounding_size;
        node.measured = true;
        bounding_size
    }

    /// Accumulates into `prev_size` the bounding sizes of the outgoing nodes
    /// of the node `prev_node`.
    fn get_bounding_size_of(&mut self, prev_node: usize, mut prev_size: Size) -> Size {
        for position in 0..self.nodes[prev_node].outgoing_nodes.len() {
            let node = self.nodes[prev_node].outgoing_nodes[position];

            if self.nodes[node].measured || self.nodes[node].outgoing_nodes.is_empty() {
                let node_bounding_size = self.nodes[node].bounding_size;
                let node_origin_desired_size = self.nodes[node].origin_desired_size;
                let node_horizontal_offset_flag = self.nodes[node].horizontal_offset_flag;
                let node_vertical_offset_flag = self.nodes[node].vertical_offset_flag;
                let node_element = self.nodes[node].element.clone();
                let prev = &mut self.nodes[prev_node];

                if prev.left_of_node == Some(node) || prev.right_of_node == Some(node) {
                    prev_size = prev_size.with_width(prev_size.width + node_bounding_size.width);
                    if RelativePanel::get_align_horizontal_center_with_panel(&node_element)
                        || node_horizontal_offset_flag
                    {
                        prev_size = prev_size.with_width(prev_size.width + prev.origin_desired_size.width);
                        prev.horizontal_offset_flag = true;
                    }

                    if node_vertical_offset_flag {
                        prev.vertical_offset_flag = true;
                    }
                }

                if prev.above_node == Some(node) || prev.below_node == Some(node) {
                    prev_size = prev_size.with_height(prev_size.height + node_bounding_size.height);
                    if RelativePanel::get_align_vertical_center_with_panel(&node_element) || node_vertical_offset_flag {
                        prev_size = prev_size.with_height(prev_size.height + node_origin_desired_size.height);
                        prev.vertical_offset_flag = true;
                    }

                    if node_horizontal_offset_flag {
                        prev.horizontal_offset_flag = true;
                    }
                }
            } else {
                return self.get_bounding_size_of(node, prev_size);
            }
        }

        prev_size
    }
}
