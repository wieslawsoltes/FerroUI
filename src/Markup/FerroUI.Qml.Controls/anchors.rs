//! Parent-relative anchors run inside native measure and arrange. Fill temporarily
//! overrides explicit dimensions at a higher value-store priority; removing the
//! anchor exposes the current local width/height and their bindings again.
use super::*;
use ferroui_base::{
    data::BindingPriority,
    ferro_class, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{Layoutable, LayoutableImpl},
    FerroObjectImpl, Rect, Size, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Canvas, CanvasImpl, CanvasImplExt, ControlImpl, PanelImpl};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Anchor {
    Fill { margins: f64 },
    Center { dx: f64, dy: f64 },
}
pub(super) type Rules = HashMap<Ref<Control>, Anchor>;
type Overrides = [Option<Rc<dyn IDisposable>>; 2];

#[repr(C)]
pub struct AnchorCanvas {
    base: Canvas,
    rules: RefCell<Rules>,
    overrides: RefCell<HashMap<Ref<Control>, Overrides>>,
}

ferro_class!(AnchorCanvas: Canvas);
ferro_impl_classes!(AnchorCanvas: FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, PanelImpl);

impl LayoutableImpl for AnchorCanvas {
    fn measure_override(this: &Self, available: Size) -> Size {
        for child in this.children().snapshot().iter() {
            let anchor = this.rules.borrow().get(child).copied();
            let constraint = match anchor {
                Some(Anchor::Fill { margins }) => inset(available, margins),
                _ => Size::new(f64::INFINITY, f64::INFINITY),
            };
            child.measure(constraint);
        }
        Size::default()
    }
}

impl CanvasImpl for AnchorCanvas {
    fn arrange_child(this: &Self, child: &Ref<Control>, size: Size) {
        let anchor = this.rules.borrow().get(child).copied();
        match anchor {
            Some(Anchor::Fill { margins }) => {
                let inner = inset(size, margins);
                child.arrange(Rect::new(margins, margins, inner.width, inner.height));
            }
            Some(Anchor::Center { dx, dy }) => {
                let desired = child.desired_size();
                child.arrange(Rect::new(
                    (size.width - desired.width) * 0.5 + dx,
                    (size.height - desired.height) * 0.5 + dy,
                    desired.width,
                    desired.height,
                ));
            }
            None => Self::parent_arrange_child(this, child, size),
        }
    }
}

fn inset(size: Size, margins: f64) -> Size {
    Size::new(
        (size.width - 2.0 * margins).max(0.0),
        (size.height - 2.0 * margins).max(0.0),
    )
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

    pub(super) fn update(&self, next: Rules) {
        if *self.rules.borrow() == next {
            return;
        }
        let mut old = self.overrides.take();
        let mut active = HashMap::new();
        for (child, rule) in &next {
            if !matches!(rule, Anchor::Fill { .. }) {
                continue;
            }
            let dimensions = old.remove(child).unwrap_or_else(|| {
                [
                    child.set_value_with_priority(
                        Layoutable::width_property(),
                        f64::NAN,
                        BindingPriority::Animation,
                    ),
                    child.set_value_with_priority(
                        Layoutable::height_property(),
                        f64::NAN,
                        BindingPriority::Animation,
                    ),
                ]
            });
            active.insert(child.clone(), dimensions);
        }
        *self.rules.borrow_mut() = next;
        *self.overrides.borrow_mut() = active;
        for dimensions in old.into_values() {
            for value in dimensions.into_iter().flatten() {
                value.dispose();
            }
        }
        self.invalidate_measure();
    }
}

pub(super) fn read(
    instance: &Instance,
    child: ObjectId,
) -> Result<Option<(ObjectId, Anchor)>, Error> {
    let fill = instance.get(child, "anchors.fill")?;
    let center = instance.get(child, "anchors.centerIn")?;
    let scalar = |name| match instance.get(child, name)? {
        Value::Number(n) if n.is_finite() => Ok(n),
        _ => Err(Error::new(format!("{name} must be finite"))),
    };
    match (fill, center) {
        (Value::Null, Value::Null) => Ok(None),
        (Value::Object(id), Value::Null) => Ok(Some((
            id,
            Anchor::Fill {
                margins: scalar("anchors.margins")?,
            },
        ))),
        (Value::Null, Value::Object(id)) => Ok(Some((
            id,
            Anchor::Center {
                dx: scalar("anchors.horizontalCenterOffset")?,
                dy: scalar("anchors.verticalCenterOffset")?,
            },
        ))),
        _ => Err(Error::new(
            "anchors.fill and anchors.centerIn cannot both be active",
        )),
    }
}

impl Drop for AnchorCanvas {
    fn drop(&mut self) {
        // A Rust caller may retain a child after the QML parent is released.
        for (_, dimensions) in self.overrides.get_mut().drain() {
            for value in dimensions.into_iter().flatten() {
                value.dispose();
            }
        }
    }
}
