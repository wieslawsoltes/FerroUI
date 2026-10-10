//! The accessibility nodes of the automation peers (the `Automation`
//! directory of the reference): per provider contract of a peer, what a
//! node of the system says of it and which actions it performs.

pub(crate) mod expand_collapse_node_info_provider;
pub(crate) mod i_node_info_provider;
pub(crate) mod invoke_node_info_provider;
pub(crate) mod node_info;
pub(crate) mod node_info_provider;
pub(crate) mod range_value_node_info_provider;
pub(crate) mod scroll_node_info_provider;
pub(crate) mod selection_item_node_info_provider;
pub(crate) mod toggle_node_info_provider;
pub(crate) mod value_node_info_provider;

pub use i_node_info_provider::{INodeInfoProvider, NodeActionArguments, NodeActionError};
pub use node_info::{NodeInfo, RangeInfo};
