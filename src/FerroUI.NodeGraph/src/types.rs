//! Core identifiers, port definitions, categories, and inline widget models for `ferroui-nodegraph`.

use std::fmt;

/// Unique identifier for a node inside a [`crate::NodeGraph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "N{}", self.0)
    }
}

/// Unique identifier for a wire inside a [`crate::NodeGraph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WireId(pub u32);

/// Unique identifier for a colored Grasshopper node group container.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupId(pub u32);

/// Port direction on a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortDirection {
    Input,
    Output,
}

/// Grasshopper parameter access level controlling how data-tree lacing invokes a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ParamAccess {
    /// Component runs once per matched item across branches (Grasshopper default).
    #[default]
    Item,
    /// Component receives each branch's full list at once (e.g., `ListLength`, `ReverseList`).
    List,
    /// Component receives the entire `DataTree` untouched (e.g., `FlattenTree`, `GraftTree`, `FlipMatrix`).
    Tree,
}

/// Grasshopper per-port data-tree modifier badge (`F`, `G`, `R`, `S`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ParamModifier {
    pub flatten: bool,
    pub graft: bool,
    pub reverse: bool,
    pub simplify: bool,
}

impl ParamModifier {
    pub const NONE: Self = Self { flatten: false, graft: false, reverse: false, simplify: false };

    pub fn is_empty(self) -> bool {
        !self.flatten && !self.graft && !self.reverse && !self.simplify
    }

    /// Compact Grasshopper-style badge string shown on port pills (e.g. `"F"`, `"G↑"`, `"R"`).
    pub fn badge(self) -> String {
        let mut s = String::new();
        if self.flatten {
            s.push('F');
        }
        if self.graft {
            s.push('G');
        }
        if self.reverse {
            s.push('R');
        }
        if self.simplify {
            s.push('S');
        }
        s
    }
}

/// Grasshopper data matching (lacing) strategy when input lists have different lengths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum DataMatching {
    /// Repeat the last item of shorter lists to match the longest list (Grasshopper default).
    #[default]
    LongestList,
    /// Stop at the length of the shortest list.
    ShortestList,
    /// Cartesian product across all combinations of inputs.
    CrossReference,
}

/// Semantic type of a port, used for color-coding, tooltips, and smart connection hints.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PortType {
    Any,
    Bool,
    Int,
    Float,
    Vec2,
    Vec3,
    Color,
    String,
    Domain,
    Curve,
    Mesh,
    /// Domain-specific custom payload (e.g., `"PhysicsShape"`, `"BodySpec"`, `"JointSpec"`, `"ForceField"`).
    Custom(String),
}

impl PortType {
    pub fn label(&self) -> &str {
        match self {
            Self::Any => "Any",
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Float => "Number",
            Self::Vec2 => "Point2D",
            Self::Vec3 => "Point3D",
            Self::Color => "Color",
            Self::String => "Text",
            Self::Domain => "Domain",
            Self::Curve => "Curve",
            Self::Mesh => "Mesh",
            Self::Custom(tag) => tag.as_str(),
        }
    }

    /// Distinctive ARGB accent color for port sockets & wires.
    pub fn color_argb(&self) -> u32 {
        match self {
            Self::Any => 0xFF94A3B8,
            Self::Bool => 0xFFF472B6,
            Self::Int => 0xFF34D399,
            Self::Float => 0xFF38BDF8,
            Self::Vec2 => 0xFFFBBF24,
            Self::Vec3 => 0xFFFB923C,
            Self::Color => 0xFFE879F9,
            Self::String => 0xFFA3E635,
            Self::Domain => 0xFF2DD4BF,
            Self::Curve => 0xFF60A5FA,
            Self::Mesh => 0xFF818CF8,
            Self::Custom(tag) => match tag.as_str() {
                "PhysicsShape" => 0xFF22D3EE,
                "BodySpec" => 0xFF38BDF8,
                "JointSpec" => 0xFFFDE047,
                "ForceField" => 0xFFC084FC,
                "SoftBodySpec" => 0xFFFB7185,
                "ClothSpec" => 0xFF34D399,
                "WorldConfig" => 0xFF94A3B8,
                _ => 0xFFA78BFA,
            },
        }
    }

    /// Returns true if an output of type `self` can connect to an input of type `target`.
    /// Grasshopper allows generous implicit coercion between numeric/text/vector types!
    pub fn is_compatible_with(&self, target: &Self) -> bool {
        if matches!(self, Self::Any) || matches!(target, Self::Any) || self == target {
            return true;
        }
        match (self, target) {
            // Numbers & bools & strings convert freely in Grasshopper
            (Self::Bool | Self::Int | Self::Float, Self::Bool | Self::Int | Self::Float | Self::String) => true,
            (Self::Vec2, Self::Vec3 | Self::String) | (Self::Vec3, Self::Vec2 | Self::String) => true,
            (Self::Domain, Self::Float | Self::Vec2 | Self::String) => true,
            (_, Self::String) => true,
            _ => false,
        }
    }
}

/// Definition of one input or output socket on a node component.
#[derive(Clone, Debug)]
pub struct PortSpec {
    /// Short Grasshopper nickname shown on the node capsule (e.g. `"A"`, `"B"`, `"Pt"`, `"R"`).
    pub nickname: String,
    /// Full descriptive name shown in tooltips and inspector (e.g. `"Radius"`).
    pub name: String,
    pub port_type: PortType,
    pub access: ParamAccess,
    pub optional: bool,
    pub default_value: crate::data_tree::Value,
}

impl PortSpec {
    pub fn new(
        nickname: impl Into<String>,
        name: impl Into<String>,
        port_type: PortType,
        default_value: crate::data_tree::Value,
    ) -> Self {
        Self {
            nickname: nickname.into(),
            name: name.into(),
            port_type,
            access: ParamAccess::Item,
            optional: true,
            default_value,
        }
    }

    pub fn with_access(mut self, access: ParamAccess) -> Self {
        self.access = access;
        self
    }

    pub fn required(mut self) -> Self {
        self.optional = false;
        self
    }
}

/// Top-level ribbon category for organizing components (like Grasshopper's tabs).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeCategory {
    Params,
    Math,
    Sets,
    Vector,
    Geometry,
    Logic,
    Physics,
    Cluster,
    Custom(String),
}

impl NodeCategory {
    pub fn label(&self) -> &str {
        match self {
            Self::Params => "Params",
            Self::Math => "Math",
            Self::Sets => "Sets",
            Self::Vector => "Vector",
            Self::Geometry => "Geometry",
            Self::Logic => "Logic",
            Self::Physics => "Physics",
            Self::Cluster => "Cluster",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn accent_argb(&self) -> u32 {
        match self {
            Self::Params => 0xFF10B981,
            Self::Math => 0xFF38BDF8,
            Self::Sets => 0xFFA78BFA,
            Self::Vector => 0xFFF59E0B,
            Self::Geometry => 0xFF60A5FA,
            Self::Logic => 0xFFF472B6,
            Self::Physics => 0xFF06B6D4,
            Self::Cluster => 0xFFEAB308,
            Self::Custom(_) => 0xFF94A3B8,
        }
    }
}

/// Visual & behavioral archetype of a node on the Grasshopper canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum NodeKind {
    /// Standard 3-column Grasshopper component (left inputs | center capsule | right outputs).
    #[default]
    Component,
    /// Inline draggable horizontal numeric slider (Grasshopper's iconic Number Slider!).
    NumberSlider,
    /// Inline clickable True/False toggle pill.
    BoolToggle,
    /// Yellow/slate live multiline DataTree inspection panel.
    Panel,
    /// Inline clickable color preview swatch.
    ColorSwatch,
    /// Inline momentary trigger / expression input.
    ExpressionBox,
    /// Compact inline wire reroute dot (`Relay`).
    Relay,
    /// Frameless typography annotation (`Scribble`).
    Scribble,
}

/// Persistent editable widget state stored on a [`crate::NodeInstance`].
#[derive(Clone, Debug, PartialEq)]
pub enum ParamWidget {
    None,
    Slider {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        integer: bool,
    },
    Toggle {
        value: bool,
    },
    Color {
        argb: u32,
    },
    Text {
        text: String,
    },
}

impl Default for ParamWidget {
    fn default() -> Self {
        Self::None
    }
}

/// Runtime evaluation status of a node (drives Grasshopper's capsule color tinting!).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum NodeState {
    #[default]
    Normal,
    Warning(String),
    Error(String),
    Disabled,
}

/// Wire visual display style (Grasshopper Fancy Wires + per-wire override).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WireDisplay {
    #[default]
    Default,
    Faint,
    Hidden,
}

/// Data complexity classification of a wire for Grasshopper Fancy Wire rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireDataKind {
    Empty,
    Item,
    List,
    Tree,
}
