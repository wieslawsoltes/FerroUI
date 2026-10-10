use super::node_info::NodeInfo;
use ferroui_controls::automation::ElementNotEnabledException;

/// The arguments of an action of a node: what the access helper reads of
/// the bundle of the system.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeActionArguments {
    /// The text of `ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE`.
    pub set_text_char_sequence: Option<String>,
}

/// Why an action of a node was not performed. The access helper answers
/// the system with "not performed" for each of them.
#[derive(Debug)]
pub enum NodeActionError {
    /// The element of the peer is not enabled.
    ElementNotEnabled(ElementNotEnabledException),
    /// The peer cannot do what was asked in its state.
    InvalidOperation(String),
    /// The peer does not support what was asked.
    NotSupported(String),
}

impl From<ElementNotEnabledException> for NodeActionError {
    fn from(error: ElementNotEnabledException) -> Self {
        Self::ElementNotEnabled(error)
    }
}

impl std::fmt::Display for NodeActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ElementNotEnabled(_) => f.write_str("Element not enabled."),
            Self::InvalidOperation(message) | Self::NotSupported(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for NodeActionError {}

pub trait INodeInfoProvider {
    fn virtual_view_id(&self) -> i32;

    fn perform_node_action(&self, action: i32, arguments: Option<&NodeActionArguments>) -> Result<bool, NodeActionError>;

    fn populate_node_info(&self, node_info: &mut NodeInfo) -> Result<(), NodeActionError>;
}
