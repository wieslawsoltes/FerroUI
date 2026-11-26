use crate::{AnimationValue, CurveChannel, Track, TrackCurveChannel, ValueKind};
use std::num::NonZeroUsize;

const STOP: &str = "gradient_stop/";
const FIELD: &str = "gradient/";
const STOP_CHANNELS: &[CurveChannel] = &[
    CurveChannel::StopOffset,
    CurveChannel::Red,
    CurveChannel::Green,
    CurveChannel::Blue,
    CurveChannel::Alpha,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrushCurveChannel {
    Stop {
        index: NonZeroUsize,
        component: CurveChannel,
    },
    Field {
        field: BrushField,
        component: CurveChannel,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrushField {
    Start,
    End,
    Center,
    GradientOrigin,
    TransformOrigin,
    RadiusX,
    RadiusY,
    Angle,
    Opacity,
}

impl BrushCurveChannel {
    pub(crate) fn for_track(track: &Track) -> Option<Vec<TrackCurveChannel>> {
        if track.keyframes.first()?.value.kind() != ValueKind::Brush {
            return None;
        }
        let mut count = 0;
        let mut geometry = Vec::new();
        for value in track
            .keyframes
            .iter()
            .map(|key| &key.value)
            .chain(&track.base_value)
        {
            let AnimationValue::Brush(brush) = value else {
                continue;
            };
            if let Some(gradient) = brush.as_gradient() {
                count = count.max(gradient.stops.len());
                for (name, value) in gradient
                    .shape
                    .fields()
                    .into_iter()
                    .chain([("TransformOrigin", gradient.transform_origin.value())])
                {
                    let field = BrushField::from_xaml(name).expect("gradient field");
                    for &component in field.components() {
                        if component.relative_unit().is_none()
                            || component.relative_unit() == value.relative_unit()
                        {
                            let channel = Self::Field { field, component };
                            if !geometry.contains(&channel) {
                                geometry.push(channel);
                            }
                        }
                    }
                }
            }
        }
        let stops = (1..=count).filter_map(NonZeroUsize::new).flat_map(|index| {
            STOP_CHANNELS
                .iter()
                .map(move |&component| Self::Stop { index, component })
        });
        let opacity = Self::Field {
            field: BrushField::Opacity,
            component: CurveChannel::Value,
        };
        Some(
            stops
                .chain(geometry)
                .chain([opacity])
                .map(TrackCurveChannel::Brush)
                .collect(),
        )
    }

    pub fn component(self) -> CurveChannel {
        match self {
            Self::Stop { component, .. } | Self::Field { component, .. } => component,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Stop { index, component } => format!("Stop {index} · {}", component.label()),
            Self::Field {
                field,
                component: CurveChannel::Value,
            } => field.label().into(),
            Self::Field {
                field: BrushField::Angle,
                component,
            } => component.label().into(),
            Self::Field { field, component } => {
                format!("{} · {}", field.label(), component.label())
            }
        }
    }

    pub(super) fn parse(value: &str) -> Option<Self> {
        if let Some(path) = value.strip_prefix(STOP) {
            let (index, component) = path.split_once('/')?;
            let component = component.parse().ok()?;
            return STOP_CHANNELS.contains(&component).then_some(Self::Stop {
                index: index.parse().ok()?,
                component,
            });
        }
        let (field, component) = value.strip_prefix(FIELD)?.split_once('/')?;
        let field = BrushField::ALL
            .iter()
            .copied()
            .find(|candidate| candidate.wire() == field)?;
        let component = component.parse().ok()?;
        field
            .components()
            .contains(&component)
            .then_some(Self::Field { field, component })
    }

    pub(super) fn path(self, component: &str) -> String {
        match self {
            Self::Stop { index, .. } => format!("{STOP}{index}/{component}"),
            Self::Field { field, .. } => format!("{FIELD}{}/{component}", field.wire()),
        }
    }
}

impl BrushField {
    const ALL: [Self; 9] = [
        Self::Start,
        Self::End,
        Self::Center,
        Self::GradientOrigin,
        Self::TransformOrigin,
        Self::RadiusX,
        Self::RadiusY,
        Self::Angle,
        Self::Opacity,
    ];

    fn from_xaml(name: &str) -> Option<Self> {
        Some(match name {
            "StartPoint" => Self::Start,
            "EndPoint" => Self::End,
            "Center" => Self::Center,
            "GradientOrigin" => Self::GradientOrigin,
            "TransformOrigin" => Self::TransformOrigin,
            "RadiusX" => Self::RadiusX,
            "RadiusY" => Self::RadiusY,
            "Angle" => Self::Angle,
            _ => return None,
        })
    }

    fn wire(self) -> &'static str {
        match self {
            Self::Start => "start_point",
            Self::End => "end_point",
            Self::Center => "center",
            Self::GradientOrigin => "gradient_origin",
            Self::TransformOrigin => "transform_origin",
            Self::RadiusX => "radius_x",
            Self::RadiusY => "radius_y",
            Self::Angle => "angle",
            Self::Opacity => "opacity",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Start => "Start point",
            Self::End => "End point",
            Self::Center => "Center",
            Self::GradientOrigin => "Gradient origin",
            Self::TransformOrigin => "Transform origin",
            Self::RadiusX => "Radius X",
            Self::RadiusY => "Radius Y",
            Self::Angle => "Angle",
            Self::Opacity => "Brush opacity",
        }
    }

    fn components(self) -> &'static [CurveChannel] {
        match self {
            Self::Angle => &[CurveChannel::Angle],
            Self::Opacity => &[CurveChannel::Value],
            Self::RadiusX | Self::RadiusY => CurveChannel::for_kind(ValueKind::RelativeScalar),
            _ => CurveChannel::for_kind(ValueKind::RelativePoint),
        }
    }
}
