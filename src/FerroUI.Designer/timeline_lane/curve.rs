use super::*;
use ferroui_designer_animation::{CurveChannel, CurveMetric, CurvePoint, CurveWindow, RelativeUnit, TrackCurveChannel};

pub(super) const HEIGHT: f64 = 88.0;
const VALUE_INSET: f64 = 12.0;
const KEY_ROW: f64 = 20.0;
type Sample = Box<dyn Fn(CurveWindow) -> Result<Vec<CurvePoint>, String>>;
type Edit = Rc<dyn Fn(f64, f64, f64)>;

pub(super) struct Plot {
    metric: CurveMetric,
    channel: TrackCurveChannel,
    keys: Vec<CurvePoint>,
    samples: Vec<CurvePoint>,
    scale: Option<Scale>,
    sample: Sample,
    edit: Edit,
}

impl Plot {
    fn edits_values(&self) -> bool {
        self.metric == CurveMetric::Value && self.channel.is_scalar()
    }

    fn height(&self, height: f64) -> f64 {
        if self.edits_values() {
            height
        } else {
            height - KEY_ROW
        }
    }
}

#[derive(Clone, Copy)]
struct Scale {
    factor: f64,
    low: f64,
    span: f64,
}

impl Scale {
    fn from_values(values: impl IntoIterator<Item = f64>) -> Option<Self> {
        let mut values = values.into_iter().filter(|value| value.is_finite());
        let first = values.next()?;
        let (low, high) = values.fold((first, first), |(low, high), value| {
            (low.min(value), high.max(value))
        });
        let factor =
            low.abs()
                .max(high.abs())
                .max(if low == high { 1.0 } else { f64::MIN_POSITIVE });
        let low = low / factor;
        let high = high / factor;
        Some(if low == high {
            Self {
                factor,
                low: low - 0.1,
                span: 0.2,
            }
        } else {
            Self {
                factor,
                low,
                span: high - low,
            }
        })
    }

    fn y(self, value: f64, height: f64) -> f64 {
        let ratio = (value / self.factor - self.low) / self.span;
        VALUE_INSET + (1.0 - ratio) * plot_height(height)
    }

    fn shift(self, value: f64, pixels: f64, height: f64) -> Option<f64> {
        let next = (value / self.factor - pixels / plot_height(height) * self.span) * self.factor;
        next.is_finite().then_some(next)
    }
}

#[derive(Clone)]
pub(super) struct ValueDrag {
    origin: f64,
    pub(super) current: f64,
    scale: Scale,
}

impl ValueDrag {
    pub(super) fn update(&mut self, offset: f64, height: f64) {
        self.current = if offset.abs() > 2.0 {
            self.scale
                .shift(self.origin, offset, height)
                .unwrap_or(self.origin)
        } else {
            self.origin
        };
    }

    pub(super) fn changed(&self) -> bool {
        self.origin != self.current
    }
}

impl TimelineLane {
    pub(crate) fn configure_curve(
        &self,
        metric: CurveMetric,
        channel: impl Into<TrackCurveChannel>,
        keys: Vec<CurvePoint>,
        sample: impl Fn(CurveWindow) -> Result<Vec<CurvePoint>, String> + 'static,
        edit: impl Fn(f64, f64, f64) + 'static,
    ) {
        let channel = channel.into();
        self.set_height(
            if metric == CurveMetric::Velocity || !channel.is_scalar() {
                HEIGHT + KEY_ROW
            } else {
                HEIGHT
            },
        );
        *self.curve.borrow_mut() = Some(Plot {
            metric,
            channel,
            keys,
            samples: Vec::new(),
            scale: None,
            sample: Box::new(sample),
            edit: Rc::new(edit),
        });
        self.refresh_curve();
    }

    pub(super) fn refresh_curve(&self) {
        let (start, end) = self.view_range();
        let mut curve = self.curve.borrow_mut();
        let Some(plot) = curve.as_mut() else { return };
        let result = (plot.sample)(CurveWindow { start, end });
        let help = match &result {
            Ok(_) if plot.metric == CurveMetric::Velocity => "Velocity in property units per authored second. Drag the bottom markers to retime keys. Edit incoming easing to shape the rate of change.".into(),
            Ok(_) if !plot.edits_values() => "Read the selected property component. Drag a marker to retime its key or select it to edit the full value and easing.".into(),
            Ok(_) => "Drag a key to edit time and value together. Select a key to edit its time, value, and easing in the inspector. Alt+Left or Right selects adjacent keys. Ctrl+wheel zooms and Shift+wheel pans.".into(),
            Err(error) => error.clone(),
        };
        plot.samples = result.unwrap_or_default();
        let color_value = plot.metric == CurveMetric::Value && plot.channel.component().is_color() && !plot.samples.is_empty();
        plot.scale = Scale::from_values(
            plot.samples
                .iter()
                .map(|point| point.value)
                .chain(
                    plot.keys
                        .iter()
                        .filter(|key| plot.edits_values() && (start..=end).contains(&key.time))
                        .map(|key| key.value),
                )
                .chain(
                    (plot.metric == CurveMetric::Velocity && !plot.samples.is_empty())
                        .then_some(0.0),
                )
                .chain(color_value.then_some([0.0, 1.0]).into_iter().flatten()),
        );
        ToolTip::set_tip(self, Some(Rc::new(help)));
        self.invalidate_visual();
    }

    fn curve_value(&self, time: f64) -> Option<f64> {
        self.curve
            .borrow()
            .as_ref()?
            .keys
            .iter()
            .find(|key| key.time == time)
            .map(|key| key.value)
    }

    pub(super) fn curve_y(&self, value: f64) -> Option<f64> {
        Some(
            self.curve
                .borrow()
                .as_ref()?
                .scale?
                .y(value, self.bounds().height),
        )
    }

    pub(super) fn key_y(&self, time: f64) -> Option<f64> {
        if !self.curve.borrow().as_ref()?.edits_values() {
            return Some(self.bounds().height - KEY_ROW * 0.5);
        }
        self.curve_y(self.curve_value(time)?)
    }

    pub(super) fn key_at_position(&self, position: Point) -> Option<f64> {
        if let Some(band) = self.clip.get() {
            return self.clip_key_at(position, band);
        }
        if self.curve.borrow().is_none() {
            return self.key_at(position.x);
        }
        let keys = self.keys.borrow();
        let (start, end) = self.view_range();
        let edge = self.time(position.x - 8.0, KeyModifiers::ALT).max(start);
        keys[keys.partition_point(|key| *key < edge)..]
            .iter()
            .copied()
            .take_while(|time| *time <= end && self.x(*time) <= position.x + 8.0)
            .filter_map(|time| {
                let dx = self.x(time) - position.x;
                let dy = self.key_y(time)? - position.y;
                let distance = dx * dx + dy * dy;
                (distance <= 64.0).then_some((distance, time))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, time)| time)
    }

    pub(super) fn dragged_time(
        &self,
        time: f64,
        start: Point,
        position: Point,
        modifiers: KeyModifiers,
    ) -> f64 {
        let offset = position.x - start.x;
        if offset.abs() <= 2.0 {
            time
        } else {
            self.time(self.x(time) + offset, modifiers)
        }
    }

    pub(super) fn start_curve_drag(&self, time: f64) -> Option<ValueDrag> {
        let origin = self.curve_value(time)?;
        let curve = self.curve.borrow();
        let plot = curve
            .as_ref()
            .filter(|plot| plot.edits_values() && !plot.samples.is_empty())?;
        Some(ValueDrag {
            origin,
            current: origin,
            scale: plot.scale?,
        })
    }

    pub(super) fn commit_curve_drag(&self, from: f64, time: f64, value: f64) {
        let edit = self.curve.borrow().as_ref().map(|plot| plot.edit.clone());
        if let Some(edit) = edit {
            edit(from, time, value);
        }
    }

    pub(super) fn draw_curve(&self, context: &mut DrawingContext, bounds: Rect) -> bool {
        let curve = self.curve.borrow();
        let Some(plot) = curve.as_ref() else {
            return false;
        };
        if let Some(scale) = plot.scale {
            let grid: Rc<dyn IPen> = Pen::from_uint32(0xff3b3d46, 1.0).into();
            let component = plot.channel.component();
            let color = match component {
                CurveChannel::Red => 0xffff958f,
                CurveChannel::Green => 0xffa3d891,
                CurveChannel::Alpha => 0xffc9cce0,
                _ => 0xff9bbaff,
            };
            let trace: Rc<dyn IPen> = Pen::from_uint32(color, 1.5).into();
            let plot_bounds = plot.height(bounds.height);
            let height = plot_height(plot_bounds);
            let axis_units = component.relative_unit().or_else(|| {
                matches!(component, CurveChannel::BlurRadius | CurveChannel::OffsetX | CurveChannel::OffsetY | CurveChannel::Spread | CurveChannel::ShadowDepth)
                    .then_some(RelativeUnit::Absolute)
            });
            let unit = match (axis_units, plot.metric) {
                (Some(RelativeUnit::Relative), CurveMetric::Value) => "%",
                (Some(RelativeUnit::Relative), CurveMetric::Velocity) => " pp/s",
                (Some(RelativeUnit::Absolute), CurveMetric::Value) => " px",
                (Some(RelativeUnit::Absolute), CurveMetric::Velocity) => " px/s",
                (None, CurveMetric::Velocity) if component == CurveChannel::Direction => " °/s",
                (None, CurveMetric::Value) if component == CurveChannel::Direction => "°",
                (None, CurveMetric::Velocity) => " /s",
                (None, CurveMetric::Value) => "",
            };
            if plot.metric == CurveMetric::Velocity
                && scale.low < 0.0
                && scale.low + scale.span > 0.0
            {
                let y = scale.y(0.0, plot_bounds);
                context.draw_line(
                    &grid,
                    Point::new(INSET, y),
                    Point::new(INSET + self.plot_width(), y),
                );
            }
            for (ratio, value) in [(0.0, scale.low + scale.span), (1.0, scale.low)] {
                let y = VALUE_INSET + ratio * height;
                context.draw_line(
                    &grid,
                    Point::new(INSET, y),
                    Point::new(INSET + self.plot_width(), y),
                );
                let label = text(
                    &format!("{}{unit}", format_value(value * scale.factor)),
                    9.0,
                    0xffa9adba,
                );
                let label_y = if ratio == 0.0 { 0.0 } else { y + 1.0 };
                context.draw_text(
                    &label,
                    Point::new((bounds.width - label.width() - 3.0).max(0.0), label_y),
                );
            }
            let position = |point: CurvePoint| {
                Point::new(self.x(point.time), scale.y(point.value, plot_bounds))
            };
            for pair in plot.samples.windows(2).filter(|pair| !pair[1].break_before) {
                context.draw_line(&trace, position(pair[0]), position(pair[1]));
            }
            for (index, point) in plot.samples.iter().enumerate() {
                if (index == 0 || point.break_before)
                    && plot.samples.get(index + 1).is_none_or(|next| next.break_before)
                {
                    let at = position(*point);
                    context.draw_line(&trace, Point::new(at.x - 1.0, at.y), Point::new(at.x + 1.0, at.y));
                }
            }
        }
        if plot.samples.is_empty() {
            context.draw_text(
                &text("Curve unavailable", 10.0, 0xffa9adba),
                Point::new(INSET, 2.0),
            );
        }
        true
    }
}

fn plot_height(height: f64) -> f64 {
    (height - 2.0 * VALUE_INSET).max(1.0)
}

pub(super) fn format_value(value: f64) -> String {
    if value != 0.0 && !(0.001..10000.0).contains(&value.abs()) {
        format!("{value:.2e}")
    } else {
        format!("{value:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}
