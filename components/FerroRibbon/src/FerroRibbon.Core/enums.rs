
// Integer newtypes preserve unnamed CLR enum values without unsafe code.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonItemSize(i32);

#[allow(non_upper_case_globals)]
impl RibbonItemSize {
    pub const Large: Self = Self(0);

    pub const Medium: Self = Self(1);

    pub const Small: Self = Self(2);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonItemSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Large => f.write_str("Large"),

            Self::Medium => f.write_str("Medium"),

            Self::Small => f.write_str("Small"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonGroupState(i32);

#[allow(non_upper_case_globals)]
impl RibbonGroupState {
    pub const Large: Self = Self(0);

    pub const Medium: Self = Self(1);

    pub const Small: Self = Self(2);

    pub const Collapsed: Self = Self(3);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonGroupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Large => f.write_str("Large"),

            Self::Medium => f.write_str("Medium"),

            Self::Small => f.write_str("Small"),

            Self::Collapsed => f.write_str("Collapsed"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonDisplayMode(i32);

#[allow(non_upper_case_globals)]
impl RibbonDisplayMode {
    pub const Classic: Self = Self(0);

    pub const Simplified: Self = Self(1);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Classic => f.write_str("Classic"),

            Self::Simplified => f.write_str("Simplified"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonVisibilityMode(i32);

#[allow(non_upper_case_globals)]
impl RibbonVisibilityMode {
    pub const AlwaysShow: Self = Self(0);

    pub const TabsOnly: Self = Self(1);

    pub const FullScreen: Self = Self(2);

    pub const PanelButtons: Self = Self(3);

    pub const PanelTitles: Self = Self(4);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonVisibilityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::AlwaysShow => f.write_str("AlwaysShow"),

            Self::TabsOnly => f.write_str("TabsOnly"),

            Self::FullScreen => f.write_str("FullScreen"),

            Self::PanelButtons => f.write_str("PanelButtons"),

            Self::PanelTitles => f.write_str("PanelTitles"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonMinimizeBehavior(i32);

#[allow(non_upper_case_globals)]
impl RibbonMinimizeBehavior {
    pub const Tabs: Self = Self(0);

    pub const PanelTitles: Self = Self(1);

    pub const PanelButtons: Self = Self(2);

    pub const CycleAll: Self = Self(3);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonMinimizeBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Tabs => f.write_str("Tabs"),

            Self::PanelTitles => f.write_str("PanelTitles"),

            Self::PanelButtons => f.write_str("PanelButtons"),

            Self::CycleAll => f.write_str("CycleAll"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonQuickAccessPosition(i32);

#[allow(non_upper_case_globals)]
impl RibbonQuickAccessPosition {
    pub const AboveRibbon: Self = Self(0);

    pub const BelowRibbon: Self = Self(1);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonQuickAccessPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::AboveRibbon => f.write_str("AboveRibbon"),

            Self::BelowRibbon => f.write_str("BelowRibbon"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonDensity(i32);

#[allow(non_upper_case_globals)]
impl RibbonDensity {
    pub const Comfortable: Self = Self(0);

    pub const Compact: Self = Self(1);

    pub const Touch: Self = Self(2);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonDensity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Comfortable => f.write_str("Comfortable"),

            Self::Compact => f.write_str("Compact"),

            Self::Touch => f.write_str("Touch"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonSimplifiedVisibility(i32);

#[allow(non_upper_case_globals)]
impl RibbonSimplifiedVisibility {
    pub const Auto: Self = Self(0);

    pub const Pinned: Self = Self(1);

    pub const Overflow: Self = Self(2);

    pub const Hidden: Self = Self(3);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonSimplifiedVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Auto => f.write_str("Auto"),

            Self::Pinned => f.write_str("Pinned"),

            Self::Overflow => f.write_str("Overflow"),

            Self::Hidden => f.write_str("Hidden"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonGroupItemsLayout(i32);

#[allow(non_upper_case_globals)]
impl RibbonGroupItemsLayout {
    pub const Columns: Self = Self(0);

    pub const Rows: Self = Self(1);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonGroupItemsLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Columns => f.write_str("Columns"),

            Self::Rows => f.write_str("Rows"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonContextualActivation(i32);

#[allow(non_upper_case_globals)]
impl RibbonContextualActivation {
    pub const None: Self = Self(0);

    pub const SelectOnShow: Self = Self(1);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonContextualActivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::None => f.write_str("None"),

            Self::SelectOnShow => f.write_str("SelectOnShow"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonMergeAction(i32);

#[allow(non_upper_case_globals)]
impl RibbonMergeAction {
    pub const Merge: Self = Self(0);

    pub const Replace: Self = Self(1);

    pub const Remove: Self = Self(2);

    pub const Add: Self = Self(3);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonMergeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Merge => f.write_str("Merge"),

            Self::Replace => f.write_str("Replace"),

            Self::Remove => f.write_str("Remove"),

            Self::Add => f.write_str("Add"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonBackstagePlacement(i32);

#[allow(non_upper_case_globals)]
impl RibbonBackstagePlacement {
    pub const Top: Self = Self(0);

    pub const Bottom: Self = Self(1);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonBackstagePlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Top => f.write_str("Top"),

            Self::Bottom => f.write_str("Bottom"),

            _ => write!(f, "{}", self.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonToolBarOrientation(i32);

#[allow(non_upper_case_globals)]
impl RibbonToolBarOrientation {
    pub const Horizontal: Self = Self(0);

    pub const Vertical: Self = Self(1);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RibbonToolBarOrientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Horizontal => f.write_str("Horizontal"),

            Self::Vertical => f.write_str("Vertical"),

            _ => write!(f, "{}", self.0),
        }
    }
}

// Kept as they are: a value outside the named members stays representable.
ferroui_system::styled_value!(
    RibbonItemSize,
    RibbonGroupState,
    RibbonDisplayMode,
    RibbonVisibilityMode,
    RibbonMinimizeBehavior,
    RibbonQuickAccessPosition,
    RibbonDensity,
    RibbonSimplifiedVisibility,
    RibbonGroupItemsLayout,
    RibbonContextualActivation,
    RibbonMergeAction,
    RibbonBackstagePlacement,
    RibbonToolBarOrientation,
);
