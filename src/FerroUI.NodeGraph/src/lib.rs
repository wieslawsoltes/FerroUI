//! **FerroUI NodeGraph** (`ferroui-nodegraph`) — A Rhino Grasshopper-inspired parametric visual
//! programming engine and interactive hardware-accelerated FerroUI canvas control.
//!
//! Designed to build **anything** — parametric CAD/geometry, live physics simulations, procedural
//! UI, audio/DSP, shaders, or data pipelines:
//! - True **Grasshopper DataTrees** (`{0;1;2}`) with `Flatten`, `Graft`, `Reverse`, `Simplify`,
//!   `FlipMatrix`, and automatic `LongestList` / `ShortestList` / `CrossReference` lacing
//! - Extensible [`NodeComponent`] trait + [`NodeRegistry`] with fuzzy search and reusable
//!   [`ClusterComponent`] sub-graphs
//! - Incremental dirty-propagating topological DAG evaluator with microsecond per-node profiling
//! - 45+ built-in standard library nodes + recursive-descent math expression parser
//! - Zero-dependency JSON serialization (`graph_to_json` / `graph_from_json`)
//! - Optional `ferroui` feature: interactive [`WireBackdropLayer`] control + [`node_graph_editor`] view
//!   with Fancy Wires, inline sliders/toggles/panels, colored groups, marquee selection, minimap,
//!   and undo/redo
//! - Optional `physics` feature: parametric physics node library compiling directly into live
//!   `ferroui-physics` worlds!

pub mod cluster;
pub mod data_tree;
pub mod eval;
pub mod graph;
pub mod node;
pub mod serialization;
pub mod stdlib;
pub mod types;

#[cfg(feature = "physics")]
pub mod physics_nodes;

#[cfg(feature = "ferroui")]
pub mod canvas;
#[cfg(feature = "ferroui")]
pub mod editor_view;
#[cfg(feature = "ferroui")]
pub mod node_visual;
#[cfg(feature = "ferroui")]
pub mod widgets;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::cluster::{
        cluster_port, ClusterComponent, ClusterInputBinding, ClusterOutputBinding,
    };
    pub use crate::data_tree::{lace_item_inputs, DataPath, DataTree, Value};
    pub use crate::eval::{evaluate_graph, topological_order, EvalReport};
    pub use crate::graph::{NodeGraph, NodeGroup, NodeInstance, Wire};
    pub use crate::node::{ComponentMeta, EvalContext, FnComponent, NodeComponent, NodeRegistry};
    pub use crate::serialization::{graph_from_json, graph_to_json, try_graph_to_json};
    pub use crate::stdlib::{eval_math_expression, register_standard_nodes};
    pub use crate::types::{
        AlignMode, DataMatching, GraphCurveKind, GroupId, NodeCategory, NodeId, NodeKind,
        NodeState, ParamAccess, ParamModifier, ParamWidget, PortDirection, PortSpec, PortType,
        WireDataKind, WireDisplay, WireId,
    };

    #[cfg(feature = "physics")]
    pub use crate::physics_nodes::{
        extract_physics_blueprint, register_physics_nodes, BodySpec, BuiltPhysicsScene, ClothSpec,
        CompiledPhysicsBlueprint, JointSpec, PhysicsGraphTemplate, SoftBodySpec, WorldConfigSpec,
        GRAPH_ARENA_WALL_TAG,
    };

    #[cfg(feature = "ferroui")]
    pub use crate::canvas::{ActiveWireDrag, VisualGraphState, WireBackdropLayer};
    #[cfg(feature = "ferroui")]
    pub use crate::editor_view::node_graph_editor;
    #[cfg(feature = "ferroui")]
    pub use crate::node_visual::build_node_visual;
    #[cfg(feature = "ferroui")]
    pub use crate::widgets::{GraphMapperControl, MdSliderControl};
}

/// Common graph types are available at the crate root as well as in [`prelude`].
pub use prelude::*;
