//! `TreeDataGridSelectionMode.cs`: how the rows or cells of a grid are selected.

/// The selection mode of a [`TreeDataGrid`](crate::TreeDataGrid) (`[Flags] TreeDataGridSelectionMode`):
/// rows or cells, one or many.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TreeDataGridSelectionMode(pub u32);

impl TreeDataGridSelectionMode {
    /// Rows are selected.
    pub const ROW: TreeDataGridSelectionMode = TreeDataGridSelectionMode(0x01);
    /// Cells are selected.
    pub const CELL: TreeDataGridSelectionMode = TreeDataGridSelectionMode(0x02);
    /// More than one row or cell can be selected.
    pub const MULTIPLE: TreeDataGridSelectionMode = TreeDataGridSelectionMode(0x04);

    /// Whether every flag of `flag` is set.
    pub fn contains(self, flag: TreeDataGridSelectionMode) -> bool {
        self.0 & flag.0 == flag.0
    }

    /// Whether any flag of `flag` is set (`(mode & flag) != 0`).
    pub fn intersects(self, flag: TreeDataGridSelectionMode) -> bool {
        self.0 & flag.0 != 0
    }

    /// The mode parsed from its markup form: flag names separated by commas
    /// (`"Cell, Multiple"`), or a number.
    pub fn parse(text: &str) -> Option<TreeDataGridSelectionMode> {
        let mut value = 0u32;
        for part in text.split(',') {
            let part = part.trim();
            value |= match part {
                p if p.eq_ignore_ascii_case("Row") => Self::ROW.0,
                p if p.eq_ignore_ascii_case("Cell") => Self::CELL.0,
                p if p.eq_ignore_ascii_case("Multiple") => Self::MULTIPLE.0,
                p => p.parse::<u32>().ok()?,
            };
        }
        Some(TreeDataGridSelectionMode(value))
    }
}

impl Default for TreeDataGridSelectionMode {
    fn default() -> Self {
        Self::ROW
    }
}

impl std::ops::BitOr for TreeDataGridSelectionMode {
    type Output = TreeDataGridSelectionMode;

    fn bitor(self, rhs: TreeDataGridSelectionMode) -> TreeDataGridSelectionMode {
        TreeDataGridSelectionMode(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for TreeDataGridSelectionMode {
    type Output = TreeDataGridSelectionMode;

    fn bitand(self, rhs: TreeDataGridSelectionMode) -> TreeDataGridSelectionMode {
        TreeDataGridSelectionMode(self.0 & rhs.0)
    }
}

impl std::fmt::Display for TreeDataGridSelectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut names = Vec::new();
        if self.contains(Self::ROW) {
            names.push("Row");
        }
        if self.contains(Self::CELL) {
            names.push("Cell");
        }
        if self.contains(Self::MULTIPLE) {
            names.push("Multiple");
        }
        if names.is_empty() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "{}", names.join(", "))
        }
    }
}
