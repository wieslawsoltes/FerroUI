//! Resolve each axis in dependency order without changing the native child stack.
//! Per-axis animation-priority overrides preserve the authored dimensions for unanchoring.
use super::*;
use ferroui_base::{
    data::BindingPriority,
    ferro_class, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{Layoutable, LayoutableImpl, LayoutableImplExt},
    FerroObjectImpl, Rect, Size, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Canvas, CanvasImpl, ControlImpl, PanelImpl, TextBlock};
use ferroui_qml::AnchorEdge;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Start,
    End,
    Center,
    Baseline,
}
impl Side {
    fn mirrored(self, mirror: bool) -> Self {
        match (self, mirror) {
            (Side::Start, true) => Side::End,
            (Side::End, true) => Side::Start,
            _ => self,
        }
    }
}
fn axis_side(edge: AnchorEdge) -> (usize, Side) {
    use AnchorEdge::*;
    match edge {
        Left => (0, Side::Start),
        Right => (0, Side::End),
        HorizontalCenter => (0, Side::Center),
        Top => (1, Side::Start),
        Bottom => (1, Side::End),
        VerticalCenter => (1, Side::Center),
        Baseline => (1, Side::Baseline),
    }
}
#[derive(Clone, PartialEq)]
struct Line {
    target: Option<Ref<Control>>,
    side: Side,
    offset: f64,
    target_text: Option<Ref<TextBlock>>,
    child_text: Option<Ref<TextBlock>>,
    align: bool,
}
#[derive(Clone, Default, PartialEq)]
struct Axis {
    start: Option<Line>,
    end: Option<Line>,
    center: Option<Line>,
}
impl Axis {
    fn lines(&self) -> impl Iterator<Item = &Line> {
        [&self.start, &self.end, &self.center].into_iter().flatten()
    }
    fn stretches(&self) -> bool {
        self.lines().count() >= 2
    }
}
#[derive(Clone, PartialEq)]
pub(super) struct Anchor {
    axes: [Axis; 2],
    align: bool,
}
#[derive(Default)]
pub(super) struct Rules {
    rules: HashMap<Ref<Control>, Anchor>,
    order: [Vec<Ref<Control>>; 2],
}
impl Rules {
    pub(super) fn insert(&mut self, child: Ref<Control>, anchor: Anchor) {
        self.rules.insert(child, anchor);
    }
    pub(super) fn replace(&mut self, child: Ref<Control>, anchor: Option<Anchor>) {
        if let Some(anchor) = anchor {
            self.insert(child, anchor);
        } else {
            self.rules.remove(&child);
        }
    }
    pub(super) fn validate(&mut self) -> Result<(), Error> {
        for axis in 0..2 {
            let mut degrees = HashMap::new();
            let mut users: HashMap<_, Vec<_>> = HashMap::new();
            let mut ready = vec![];
            for (child, rule) in &self.rules {
                let mut count = 0usize;
                for line in rule.axes[axis].lines() {
                    if let Some(target) =
                        line.target.as_ref().filter(|t| self.rules.contains_key(*t))
                    {
                        users.entry(target.clone()).or_default().push(child.clone());
                        count += 1;
                    }
                }
                if count == 0 {
                    ready.push(child.clone());
                }
                degrees.insert(child.clone(), count);
            }
            let mut order = vec![];
            while let Some(child) = ready.pop() {
                if let Some(list) = users.get(&child) {
                    for next in list {
                        let count = degrees.get_mut(next).expect("recorded child");
                        *count -= 1;
                        if *count == 0 {
                            ready.push(next.clone());
                        }
                    }
                }
                order.push(child);
            }
            if order.len() != self.rules.len() {
                return Err(Error::new("cycle between sibling anchor lines"));
            }
            self.order[axis] = order;
        }
        Ok(())
    }
}

type Overrides = [Option<Rc<dyn IDisposable>>; 2];
type Frames = HashMap<Ref<Control>, Rect>;

#[repr(C)]
pub struct AnchorCanvas {
    base: Canvas,
    rules: RefCell<Rules>,
    overrides: RefCell<HashMap<Ref<Control>, Overrides>>,
}
ferro_class!(AnchorCanvas: Canvas);
ferro_impl_classes!(AnchorCanvas: FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl CanvasImpl for AnchorCanvas {}

fn half(length: f64, align: bool) -> f64 {
    if align && length.is_finite() {
        (length.trunc() / 2.).ceil()
    } else {
        length / 2.
    }
}
fn component(rect: Rect, axis: usize) -> (f64, f64) {
    if axis == 0 {
        (rect.x, rect.width)
    } else {
        (rect.y, rect.height)
    }
}
fn resolve(line: &Line, frames: &Frames, parent: Rect, axis: usize) -> f64 {
    let rect = line
        .target
        .as_ref()
        .and_then(|target| frames.get(target))
        .copied()
        .unwrap_or(parent);
    let (pos, extent) = component(rect, axis);
    pos + match line.side {
        Side::Start | Side::Baseline => 0.,
        Side::End => extent,
        Side::Center => half(extent, line.align),
    } + line.offset
        + text_baseline(&line.target_text)
        - text_baseline(&line.child_text)
}
fn text_baseline(text: &Option<Ref<TextBlock>>) -> f64 {
    text.as_ref().map_or(0., |text| {
        text.padding().top + text.text_layout().baseline()
    })
}

fn solve(rules: &Rules, frames: &mut Frames, size: Size, axis: usize) {
    let parent = Rect::new(0., 0., size.width, size.height);
    for child in &rules.order[axis] {
        let Some(frame) = frames.get(child) else {
            continue;
        };
        let (_, natural) = component(*frame, axis);
        let anchor = &rules.rules[child];
        let constraint = &anchor.axes[axis];
        let coord =
            |target: &Option<Line>| target.as_ref().map(|l| resolve(l, frames, parent, axis));
        let (pos, extent) = match (
            coord(&constraint.start),
            coord(&constraint.end),
            coord(&constraint.center),
        ) {
            (Some(start), Some(end), _) => (start, (end - start).max(0.)),
            (Some(start), _, Some(center)) => (start, (2. * (center - start)).max(0.)),
            (_, Some(end), Some(center)) => {
                let n = (2. * (end - center)).max(0.);
                (end - n, n)
            }
            (Some(start), _, _) => (start, natural),
            (_, Some(end), _) => (end - natural, natural),
            (_, _, Some(center)) => (center - half(natural, anchor.align), natural),
            _ => continue,
        };
        let frame = frames.get_mut(child).unwrap();
        if axis == 0 {
            frame.x = pos;
            frame.width = extent;
        } else {
            frame.y = pos;
            frame.height = extent;
        }
    }
}
impl AnchorCanvas {
    fn frames(&self, size: Size) -> Frames {
        self.children()
            .snapshot()
            .iter()
            .map(|c| {
                let desired = c.desired_size();
                let x = origin(
                    Canvas::get_left(c),
                    Canvas::get_right(c),
                    size.width,
                    desired.width,
                );
                let y = origin(
                    Canvas::get_top(c),
                    Canvas::get_bottom(c),
                    size.height,
                    desired.height,
                );
                (c.clone(), Rect::new(x, y, desired.width, desired.height))
            })
            .collect()
    }
}
fn origin(start: f64, end: f64, size: f64, desired: f64) -> f64 {
    if start.is_finite() {
        start
    } else if end.is_finite() {
        size - end - desired
    } else {
        0.
    }
}
impl LayoutableImpl for AnchorCanvas {
    fn measure_override(this: &Self, available: Size) -> Size {
        let children = this.children().snapshot();
        for c in children.iter() {
            c.measure(Size::new(f64::INFINITY, f64::INFINITY));
        }
        let rules = this.rules.borrow();
        if !rules.rules.is_empty() {
            let mut frames = this.frames(available);
            if available.width.is_finite() {
                solve(&rules, &mut frames, available, 0);
            }
            // Width constraints can change intrinsic height (for example, wrapped text).
            for (child, rule) in &rules.rules {
                if rule.axes[0].stretches() && available.width.is_finite() {
                    if let Some(rect) = frames.get_mut(child) {
                        child.measure(Size::new(rect.width, f64::INFINITY));
                        rect.height = child.desired_size().height;
                    }
                }
            }
            if available.height.is_finite() {
                solve(&rules, &mut frames, available, 1);
            }
            for (child, rule) in &rules.rules {
                if rule.axes[1].stretches() && available.height.is_finite() {
                    if let Some(rect) = frames.get(child) {
                        let width = if rule.axes[0].stretches() && available.width.is_finite() {
                            rect.width
                        } else {
                            f64::INFINITY
                        };
                        child.measure(Size::new(width, rect.height));
                    }
                }
            }
        }
        Size::default()
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        let rules = this.rules.borrow();
        if rules.rules.is_empty() {
            return Self::parent_arrange_override(this, size);
        }
        let mut frames = this.frames(size);
        solve(&rules, &mut frames, size, 0);
        solve(&rules, &mut frames, size, 1);
        for child in this.children().snapshot().iter() {
            child.arrange(frames[child]);
        }
        size
    }
}
impl AnchorCanvas {
    pub(super) fn create() -> Ref<Canvas> {
        instantiate(Self {
            base: Canvas::construct(),
            rules: RefCell::default(),
            overrides: RefCell::default(),
        })
        .upcast()
    }
    pub(super) fn snapshot_rules(&self) -> Rules {
        Rules {
            rules: self.rules.borrow().rules.clone(),
            order: Default::default(),
        }
    }
    pub(super) fn update(&self, next: Rules) {
        if self.rules.borrow().rules == next.rules {
            return;
        }
        let mut old = self.overrides.take();
        let mut active = HashMap::new();
        for (child, anchor) in &next.rules {
            let mut dimensions = old.remove(child).unwrap_or_default();
            for (axis, dimension) in dimensions.iter_mut().enumerate() {
                if anchor.axes[axis].stretches() {
                    if dimension.is_none() {
                        let property = if axis == 0 {
                            Layoutable::width_property()
                        } else {
                            Layoutable::height_property()
                        };
                        *dimension = child.set_value_with_priority(
                            property,
                            f64::NAN,
                            BindingPriority::Animation,
                        );
                    }
                } else if let Some(d) = dimension.take() {
                    d.dispose();
                }
            }
            if dimensions.iter().any(Option::is_some) {
                active.insert(child.clone(), dimensions);
            }
        }
        *self.rules.borrow_mut() = next;
        *self.overrides.borrow_mut() = active;
        for dims in old.into_values() {
            for d in dims.into_iter().flatten() {
                d.dispose();
            }
        }
        self.invalidate_measure();
    }
}

pub(super) fn read(
    instance: &Instance,
    child: ObjectId,
    parents: &BTreeMap<ObjectId, ObjectId>,
    nodes: &tree::Peers,
) -> Result<Option<Anchor>, Error> {
    let field = |name: &str| instance.get(child, &format!("anchors.{name}"));
    let optional_target = |name| match field(name)? {
        Value::Object(o) => Ok(Some(o)),
        Value::Undefined | Value::Null => Ok(None),
        _ => Err(Error::new("invalid anchor target")),
    };
    let scalar = |name, default| match field(name)? {
        Value::Number(n) if n.is_finite() => Ok(n),
        Value::Undefined => Ok(default),
        _ => Err(Error::new(format!("anchors.{name} must be finite"))),
    };
    let fill = optional_target("fill")?;
    let center = optional_target("centerIn")?;
    if fill.is_some() && center.is_some() {
        return Err(Error::new(
            "anchors.fill and anchors.centerIn cannot both be active",
        ));
    }
    let parent = parents.get(&child).copied();
    let align = |id| instance.get(id, "anchors.alignWhenCentered").ok() != Some(Value::Bool(false));
    let margin = scalar("margins", 0.)?;
    let mirrored = instance.get(child, "LayoutMirroring.enabled")? == Value::Bool(true);
    let baseline = |id| match instance.get(id, "baselineOffset")? {
        Value::Number(n) if n.is_finite() => Ok(n),
        _ => Err(Error::new("baselineOffset must be finite")),
    };
    let mut anchor = Anchor {
        axes: Default::default(),
        align: align(child),
    };
    let mut anchored = false;
    for edge in AnchorEdge::ALL {
        let (axis, side) = axis_side(edge);
        let explicit = match field(edge.name())? {
            Value::Undefined | Value::Null => None,
            Value::AnchorLine(id, target_edge) => Some((id, target_edge)),
            _ => return Err(Error::new("expected an Item anchor line")),
        };
        let shorthand = match side {
            Side::Center => center,
            Side::Baseline => None,
            _ => fill,
        };
        if explicit.is_some() && (fill.is_some() || center.is_some()) {
            return Err(Error::new(
                "cannot combine explicit anchor lines with fill or centerIn",
            ));
        }
        let Some((id, target_edge)) = explicit.or(shorthand.map(|id| (id, edge))) else {
            continue;
        };
        let (target_axis, target_side) = axis_side(target_edge);
        if target_axis != axis {
            return Err(Error::new("anchor lines must use the same axis"));
        }
        let target = if Some(id) == parent {
            None
        } else if parent.is_some() && child != id && parents.get(&id) == parent.as_ref() {
            Some(
                nodes
                    .get(&id)
                    .ok_or_else(|| Error::new("anchor target has no native peer"))?
                    .control(),
            )
        } else {
            return Err(Error::new(
                "anchor target must be the item's visual parent or sibling",
            ));
        };
        let mirror = mirrored && axis == 0;
        let authored_offset = match side {
            Side::Start => scalar(if axis == 0 { "leftMargin" } else { "topMargin" }, margin)?,
            Side::End => -scalar(
                if axis == 0 {
                    "rightMargin"
                } else {
                    "bottomMargin"
                },
                margin,
            )?,
            Side::Center => scalar(
                if axis == 0 {
                    "horizontalCenterOffset"
                } else {
                    "verticalCenterOffset"
                },
                0.,
            )?,
            Side::Baseline => {
                if anchor.axes[axis].lines().next().is_some() {
                    return Err(Error::new(
                        "baseline cannot be combined with another vertical anchor",
                    ));
                }
                scalar("baselineOffset", 0.)? - baseline(child)?
            }
        };
        let offset = if mirror {
            -authored_offset
        } else {
            authored_offset
        };
        let slot = match side.mirrored(mirror) {
            Side::Start | Side::Baseline => &mut anchor.axes[axis].start,
            Side::End => &mut anchor.axes[axis].end,
            Side::Center => &mut anchor.axes[axis].center,
        };
        let measured_text = |id, side| {
            if side == Side::Baseline && !instance.is_assigned(id, "baselineOffset") {
                nodes.get(&id)?.control().cast::<TextBlock>()
            } else {
                None
            }
        };
        *slot = Some(Line {
            target_text: measured_text(id, target_side),
            child_text: measured_text(child, side),
            target,
            side: target_side.mirrored(mirror),
            offset: if target_side == Side::Baseline {
                offset + baseline(id)?
            } else {
                offset
            },
            align: align(id),
        });
        anchored = true;
    }
    if anchor.axes.iter().any(|axis| axis.lines().count() > 2) {
        return Err(Error::new("an axis accepts at most two anchor lines"));
    }
    Ok(anchored.then_some(anchor))
}
impl Drop for AnchorCanvas {
    fn drop(&mut self) {
        for (_, dims) in self.overrides.get_mut().drain() {
            for d in dims.into_iter().flatten() {
                d.dispose();
            }
        }
    }
}
