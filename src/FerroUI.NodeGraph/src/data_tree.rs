//! Rhino Grasshopper-style **DataTree** (`{0;1;2}` branched collections), universal [`Value`],
//! and automatic **LongestList / ShortestList / CrossReference** lacing engine.

use crate::types::{DataMatching, ParamModifier, WireDataKind};
use indexmap::IndexMap;
use smallvec::SmallVec;
use std::any::Any;
use std::fmt;
use std::sync::Arc;

/// Hierarchical Grasshopper branch address such as `{0}`, `{0;1}`, or `{2;0;4}`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DataPath(pub SmallVec<[u32; 4]>);

impl DataPath {
    pub fn root() -> Self {
        let mut v = SmallVec::new();
        v.push(0);
        Self(v)
    }

    pub fn from_slice(indices: &[u32]) -> Self {
        if indices.is_empty() {
            Self::root()
        } else {
            Self(SmallVec::from_slice(indices))
        }
    }

    pub fn append(&self, idx: u32) -> Self {
        let mut next = self.0.clone();
        next.push(idx);
        Self(next)
    }

    pub fn shift(&self, offset: isize) -> Self {
        if offset == 0 || self.0.len() <= 1 {
            return self.clone();
        }
        let n = self.0.len();
        if offset < 0 {
            let drop = (-offset as usize).min(n - 1);
            Self(SmallVec::from_slice(&self.0[..n - drop]))
        } else {
            let drop = (offset as usize).min(n - 1);
            Self(SmallVec::from_slice(&self.0[drop..]))
        }
    }

    /// Parses Grasshopper notation like `"{0;1;2}"` or `"0;1"`.
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim().trim_start_matches('{').trim_end_matches('}');
        if trimmed.is_empty() {
            return Some(Self::root());
        }
        let mut out = SmallVec::new();
        for part in trimmed.split(';') {
            let v: u32 = part.trim().parse().ok()?;
            out.push(v);
        }
        Some(Self(out))
    }
}

impl fmt::Display for DataPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, seg) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ";")?;
            }
            write!(f, "{seg}")?;
        }
        write!(f, "}}")
    }
}

/// Extensible typed payload carried inside [`Value::Custom`].
#[derive(Clone)]
pub struct CustomPayload {
    pub type_tag: String,
    pub summary: String,
    pub data: Arc<dyn Any + Send + Sync>,
}

impl fmt::Debug for CustomPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Custom<{}>({})", self.type_tag, self.summary)
    }
}

/// Universal parametric value flowing through Grasshopper wires and DataTrees.
#[derive(Clone, Debug, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Vec2([f64; 2]),
    Vec3([f64; 3]),
    Color(u32),
    String(String),
    Domain(f64, f64),
    Curve(Vec<[f64; 2]>),
    Mesh {
        vertices: Vec<[f64; 2]>,
        indices: Vec<[u32; 3]>,
    },
    Custom(CustomPayload),
}

impl Value {
    pub fn custom<T: Any + Send + Sync>(
        type_tag: impl Into<String>,
        summary: impl Into<String>,
        value: T,
    ) -> Self {
        Self::Custom(CustomPayload {
            type_tag: type_tag.into(),
            summary: summary.into(),
            data: Arc::new(value),
        })
    }

    pub fn downcast_custom<T: Any + Send + Sync>(&self) -> Option<&T> {
        match self {
            Self::Custom(p) => p.data.downcast_ref::<T>(),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f64),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::Domain(a, b) => Some(b - a),
            Self::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            Self::Float(v) => Some(v.round() as i64),
            Self::Bool(b) => Some(if *b { 1 } else { 0 }),
            Self::String(s) => s.trim().parse::<i64>().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            Self::Int(i) => Some(*i != 0),
            Self::Float(f) => Some(f.abs() > 1e-9),
            Self::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                "false" | "0" | "no" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_vec2(&self) -> Option<[f64; 2]> {
        match self {
            Self::Vec2(v) => Some(*v),
            Self::Vec3(v) => Some([v[0], v[1]]),
            Self::Domain(a, b) => Some([*a, *b]),
            Self::Float(x) => Some([*x, *x]),
            Self::Int(x) => Some([*x as f64, *x as f64]),
            _ => None,
        }
    }

    pub fn as_vec3(&self) -> Option<[f64; 3]> {
        match self {
            Self::Vec3(v) => Some(*v),
            Self::Vec2(v) => Some([v[0], v[1], 0.0]),
            Self::Float(x) => Some([*x, *x, *x]),
            Self::Int(x) => Some([*x as f64, *x as f64, *x as f64]),
            _ => None,
        }
    }

    pub fn as_color(&self) -> Option<u32> {
        match self {
            Self::Color(c) => Some(*c),
            Self::Int(i) => Some(*i as u32),
            _ => None,
        }
    }

    pub fn as_domain(&self) -> Option<(f64, f64)> {
        match self {
            Self::Domain(a, b) => Some((*a, *b)),
            Self::Vec2([a, b]) => Some((*a, *b)),
            Self::Float(x) => Some((0.0, *x)),
            Self::Int(x) => Some((0.0, *x as f64)),
            _ => None,
        }
    }

    /// Compact single-line formatting for Panel nodes and port hover tooltips.
    pub fn short_summary(&self) -> String {
        match self {
            Self::Null => "<null>".into(),
            Self::Bool(b) => if *b { "True".into() } else { "False".into() },
            Self::Int(i) => format!("{i}"),
            Self::Float(v) => {
                if (v - v.round()).abs() < 1e-6 && v.abs() < 1e9 {
                    format!("{:.0}", v)
                } else {
                    format!("{v:.3}")
                }
            }
            Self::Vec2([x, y]) => format!("({x:.2}, {y:.2})"),
            Self::Vec3([x, y, z]) => format!("({x:.2}, {y:.2}, {z:.2})"),
            Self::Color(c) => format!("#{:06X}", c & 0x00FF_FFFF),
            Self::String(s) => s.clone(),
            Self::Domain(a, b) => format!("{a:.2} To {b:.2}"),
            Self::Curve(pts) => format!("Curve({} pts)", pts.len()),
            Self::Mesh { vertices, indices } => format!("Mesh({}v, {}f)", vertices.len(), indices.len()),
            Self::Custom(p) => p.summary.clone(),
        }
    }
}

/// Rhino Grasshopper-style hierarchical DataTree (`{path} -> [item0, item1, ...]`).
#[derive(Clone, Debug, Default)]
pub struct DataTree {
    branches: IndexMap<DataPath, Vec<Value>>,
}

impl DataTree {
    pub fn new() -> Self {
        Self { branches: IndexMap::new() }
    }

    pub fn single(value: Value) -> Self {
        let mut t = Self::new();
        t.branches.insert(DataPath::root(), vec![value]);
        t
    }

    pub fn from_list(items: Vec<Value>) -> Self {
        let mut t = Self::new();
        t.branches.insert(DataPath::root(), items);
        t
    }

    pub fn insert_branch(&mut self, path: DataPath, items: Vec<Value>) {
        self.branches.insert(path, items);
    }

    pub fn push(&mut self, path: DataPath, item: Value) {
        self.branches.entry(path).or_default().push(item);
    }

    pub fn branches(&self) -> &IndexMap<DataPath, Vec<Value>> {
        &self.branches
    }

    pub fn branches_mut(&mut self) -> &mut IndexMap<DataPath, Vec<Value>> {
        &mut self.branches
    }

    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    pub fn item_count(&self) -> usize {
        self.branches.values().map(|v| v.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.item_count() == 0
    }

    /// Returns the first non-null item if any exists.
    pub fn first_item(&self) -> Option<&Value> {
        self.branches.values().flat_map(|v| v.iter()).next()
    }

    /// Collects every item in branch order into a flat slice.
    pub fn all_items(&self) -> Vec<Value> {
        let mut out = Vec::with_capacity(self.item_count());
        for list in self.branches.values() {
            out.extend(list.iter().cloned());
        }
        out
    }

    /// Classifies tree structure for Grasshopper Fancy Wire styling.
    pub fn wire_data_kind(&self) -> WireDataKind {
        let n = self.item_count();
        if n == 0 {
            WireDataKind::Empty
        } else if self.branches.len() > 1 {
            WireDataKind::Tree
        } else if n > 1 {
            WireDataKind::List
        } else {
            WireDataKind::Item
        }
    }

    /// Grasshopper **Flatten**: collapses all branches into `{0}`.
    pub fn flatten(&self) -> Self {
        Self::from_list(self.all_items())
    }

    /// Grasshopper **Graft**: promotes every individual item into its own branch `{path;i}`.
    pub fn graft(&self) -> Self {
        let mut out = Self::new();
        for (path, items) in &self.branches {
            for (i, item) in items.iter().enumerate() {
                out.insert_branch(path.append(i as u32), vec![item.clone()]);
            }
        }
        out
    }

    /// Grasshopper **Reverse**: reverses item order inside every branch.
    pub fn reverse(&self) -> Self {
        let mut out = Self::new();
        for (path, items) in &self.branches {
            let mut rev = items.clone();
            rev.reverse();
            out.insert_branch(path.clone(), rev);
        }
        out
    }

    /// Grasshopper **Simplify**: strips leading path segments shared identically by every branch.
    pub fn simplify(&self) -> Self {
        if self.branches.len() <= 1 {
            return if let Some((_, items)) = self.branches.first() {
                Self::from_list(items.clone())
            } else {
                Self::new()
            };
        }
        let min_len = self.branches.keys().map(|p| p.0.len()).min().unwrap_or(1);
        let mut common_prefix = 0usize;
        while common_prefix + 1 < min_len {
            let first_val = self.branches.keys().next().unwrap().0[common_prefix];
            if self.branches.keys().all(|p| p.0[common_prefix] == first_val) {
                common_prefix += 1;
            } else {
                break;
            }
        }
        if common_prefix == 0 {
            return self.clone();
        }
        let mut out = Self::new();
        for (path, items) in &self.branches {
            out.insert_branch(DataPath::from_slice(&path.0[common_prefix..]), items.clone());
        }
        out
    }

    /// Grasshopper **Flip Matrix**: transposes rows (branches) and columns (item indices).
    pub fn flip_matrix(&self) -> Self {
        let max_len = self.branches.values().map(|v| v.len()).max().unwrap_or(0);
        let mut out = Self::new();
        for col in 0..max_len {
            let mut col_items = Vec::with_capacity(self.branches.len());
            for list in self.branches.values() {
                if let Some(v) = list.get(col) {
                    col_items.push(v.clone());
                }
            }
            out.insert_branch(DataPath::from_slice(&[col as u32]), col_items);
        }
        out
    }

    /// Grasshopper **Shift Paths**: strips leading (`offset > 0`) or trailing (`offset < 0`) path indices.
    pub fn shift_paths(&self, offset: isize) -> Self {
        let mut out = Self::new();
        for (path, items) in &self.branches {
            let shifted = path.shift(offset);
            out.branches.entry(shifted).or_default().extend(items.iter().cloned());
        }
        out
    }

    /// Grasshopper **Path Mapper**: remaps branch paths & item indices using lexical masks such as
    /// `"{A;B}(i)" -> "{B;A}"`, `"{A;B}" -> "{0}"`, `"{A}(i)" -> "{A;i}"`, or `"{A;B}" -> "{A}"`.
    pub fn map_paths(&self, source_mask: &str, target_mask: &str) -> Result<Self, String> {
        let (src_vars, src_item_var) = parse_path_mask(source_mask)?;
        let tgt_tokens = parse_target_mask(target_mask)?;
        let mut out = Self::new();

        for (path, items) in &self.branches {
            for (item_idx, val) in items.iter().enumerate() {
                let mut new_segs: SmallVec<[u32; 4]> = SmallVec::new();
                for tok in &tgt_tokens {
                    let seg = match tok {
                        TargetToken::Literal(n) => *n,
                        TargetToken::Var(name) => {
                            if src_item_var.as_deref() == Some(name.as_str()) || name == "i" {
                                item_idx as u32
                            } else if let Some(pos) = src_vars.iter().position(|v| v == name) {
                                path.0.get(pos).copied().unwrap_or(0)
                            } else {
                                0
                            }
                        }
                    };
                    new_segs.push(seg);
                }
                let new_path = if new_segs.is_empty() {
                    DataPath::root()
                } else {
                    DataPath(new_segs)
                };
                out.push(new_path, val.clone());
            }
        }
        Ok(out)
    }

    /// Formats Grasshopper **Param Viewer** topology summary (`{0;0} (N = 6)` per branch).
    pub fn format_param_viewer(&self, max_branches: usize) -> String {
        if self.branches.is_empty() {
            return "Empty tree (0 branches)".into();
        }
        let mut lines = Vec::new();
        lines.push(format!(
            "Data: {} branch{}, {} item{}",
            self.branches.len(),
            if self.branches.len() == 1 { "" } else { "es" },
            self.item_count(),
            if self.item_count() == 1 { "" } else { "s" }
        ));
        for (idx, (p, items)) in self.branches.iter().enumerate() {
            if idx >= max_branches {
                lines.push(format!("… +{} more branches", self.branches.len() - idx));
                break;
            }
            lines.push(format!("{p}  (N = {})", items.len()));
        }
        lines.join("\n")
    }

    /// Merges another `DataTree` into `self` preserving matching branch paths.
    pub fn merge_from(&mut self, other: &DataTree) {
        for (path, items) in &other.branches {
            self.branches.entry(path.clone()).or_default().extend(items.iter().cloned());
        }
    }

    /// Applies per-port Grasshopper modifiers (`Flatten`, `Graft`, `Reverse`, `Simplify`).
    pub fn apply_modifier(&self, m: ParamModifier) -> Self {
        if m.is_empty() {
            return self.clone();
        }
        let mut t = self.clone();
        if m.flatten {
            t = t.flatten();
        }
        if m.graft {
            t = t.graft();
        }
        if m.reverse {
            t = t.reverse();
        }
        if m.simplify {
            t = t.simplify();
        }
        t
    }

    /// Renders a Grasshopper Panel-style text view of branches & items (capped for snappy UI).
    pub fn format_panel(&self, max_lines: usize) -> String {
        if self.is_empty() {
            return "<empty>".into();
        }
        let mut lines = Vec::new();
        for (path, items) in &self.branches {
            if self.branches.len() > 1 || items.len() > 1 {
                lines.push(format!("{path} (N={})", items.len()));
            }
            for (i, item) in items.iter().enumerate() {
                if lines.len() >= max_lines {
                    lines.push(format!("… ({} total items)", self.item_count()));
                    return lines.join("\n");
                }
                if self.branches.len() == 1 && items.len() == 1 {
                    lines.push(item.short_summary());
                } else {
                    lines.push(format!(" {i}: {}", item.short_summary()));
                }
            }
        }
        lines.join("\n")
    }
}

/// Runs Grasshopper item-level lacing over multiple input `DataTree`s, invoking `f` for each
/// matched tuple of items and placing outputs into aligned branches.
pub fn lace_item_inputs(
    inputs: &[DataTree],
    matching: DataMatching,
    mut f: impl FnMut(&[Value]) -> Result<Vec<Value>, String>,
    num_outputs: usize,
) -> Result<Vec<DataTree>, String> {
    let mut outputs: Vec<DataTree> = (0..num_outputs).map(|_| DataTree::new()).collect();
    if inputs.is_empty() {
        let vals = f(&[])?;
        for (o, v) in vals.into_iter().enumerate() {
            if o < num_outputs {
                outputs[o].push(DataPath::root(), v);
            }
        }
        return Ok(outputs);
    }

    // Determine branch count (longest branch count, repeating last branch of shorter trees)
    let max_branches = inputs.iter().map(|t| t.branch_count()).max().unwrap_or(0);
    if max_branches == 0 {
        return Ok(outputs);
    }

    // Pick branch paths from the tree with the most branches
    let primary_tree = inputs
        .iter()
        .max_by_key(|t| t.branch_count())
        .unwrap();
    let primary_paths: Vec<DataPath> = primary_tree.branches.keys().cloned().collect();

    for b_idx in 0..max_branches {
        let path = primary_paths
            .get(b_idx)
            .cloned()
            .unwrap_or_else(|| DataPath::from_slice(&[b_idx as u32]));

        let branch_slices: Vec<&[Value]> = inputs
            .iter()
            .map(|t| {
                if t.branches.is_empty() {
                    &[][..]
                } else {
                    let idx = b_idx.min(t.branches.len() - 1);
                    t.branches.get_index(idx).map(|(_, v)| v.as_slice()).unwrap_or(&[])
                }
            })
            .collect();

        if branch_slices.iter().any(|s| s.is_empty()) {
            continue;
        }

        match matching {
            DataMatching::LongestList => {
                let count = branch_slices.iter().map(|s| s.len()).max().unwrap_or(0);
                let mut args = Vec::with_capacity(inputs.len());
                for i in 0..count {
                    args.clear();
                    for s in &branch_slices {
                        args.push(s[i.min(s.len() - 1)].clone());
                    }
                    let vals = f(&args)?;
                    for (o, v) in vals.into_iter().enumerate() {
                        if o < num_outputs {
                            outputs[o].push(path.clone(), v);
                        }
                    }
                }
            }
            DataMatching::ShortestList => {
                let count = branch_slices.iter().map(|s| s.len()).min().unwrap_or(0);
                let mut args = Vec::with_capacity(inputs.len());
                for i in 0..count {
                    args.clear();
                    for s in &branch_slices {
                        args.push(s[i].clone());
                    }
                    let vals = f(&args)?;
                    for (o, v) in vals.into_iter().enumerate() {
                        if o < num_outputs {
                            outputs[o].push(path.clone(), v);
                        }
                    }
                }
            }
            DataMatching::CrossReference => {
                // Cartesian product across input lists in this branch
                let mut combos: Vec<Vec<Value>> = vec![Vec::with_capacity(inputs.len())];
                for s in &branch_slices {
                    let mut next = Vec::with_capacity(combos.len() * s.len());
                    for prefix in &combos {
                        for item in *s {
                            let mut row = prefix.clone();
                            row.push(item.clone());
                            next.push(row);
                        }
                    }
                    combos = next;
                }
                for args in &combos {
                    let vals = f(args)?;
                    for (o, v) in vals.into_iter().enumerate() {
                        if o < num_outputs {
                            outputs[o].push(path.clone(), v);
                        }
                    }
                }
            }
        }
    }

    Ok(outputs)
}

enum TargetToken {
    Literal(u32),
    Var(String),
}

fn parse_path_mask(mask: &str) -> Result<(Vec<String>, Option<String>), String> {
    let trimmed = mask.trim();
    let (brace_part, item_part) = if let Some(idx) = trimmed.find('(') {
        (&trimmed[..idx], Some(trimmed[idx..].trim()))
    } else {
        (trimmed, None)
    };
    let inner = brace_part.trim().trim_start_matches('{').trim_end_matches('}');
    let vars: Vec<String> = inner
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let item_var = item_part.map(|p| p.trim_start_matches('(').trim_end_matches(')').trim().to_string());
    Ok((vars, item_var))
}

fn parse_target_mask(mask: &str) -> Result<Vec<TargetToken>, String> {
    let inner = mask.trim().trim_start_matches('{').trim_end_matches('}');
    if inner.is_empty() {
        return Ok(vec![TargetToken::Literal(0)]);
    }
    let mut out = Vec::new();
    for seg in inner.split(';') {
        let s = seg.trim();
        if s.is_empty() {
            continue;
        }
        if let Ok(n) = s.parse::<u32>() {
            out.push(TargetToken::Literal(n));
        } else {
            out.push(TargetToken::Var(s.to_string()));
        }
    }
    Ok(out)
}
