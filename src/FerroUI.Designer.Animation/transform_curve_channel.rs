use crate::{AnimationValue, CurveChannel, Track, TrackCurveChannel, TransformOperationKind};
use serde::Deserialize;
use std::{collections::BTreeSet, num::NonZeroUsize};

const OPERATION: &str = "transform_operation/";
const MATRIX: &str = "transform_matrix/";
const COEFFICIENTS: &[CurveChannel] = &[
    CurveChannel::M11,
    CurveChannel::M12,
    CurveChannel::M21,
    CurveChannel::M22,
    CurveChannel::M31,
    CurveChannel::M32,
];

/// One operation slot or a coefficient of the combined affine transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformCurveChannel {
    Operation {
        index: NonZeroUsize,
        kind: TransformOperationKind,
        component: CurveChannel,
    },
    Matrix(CurveChannel),
}

impl TransformCurveChannel {
    pub(crate) fn for_track(track: &Track) -> Option<Vec<TrackCurveChannel>> {
        if !matches!(
            &track.keyframes.first()?.value,
            AnimationValue::TransformList(_)
        ) {
            return None;
        }
        let mut slots = BTreeSet::new();
        for value in track
            .keyframes
            .iter()
            .map(|key| &key.value)
            .chain(&track.base_value)
        {
            if let AnimationValue::TransformList(list) = value {
                for (index, operation) in list.operations().iter().enumerate() {
                    slots.insert((
                        NonZeroUsize::new(index + 1).expect("one-based index"),
                        operation.kind(),
                    ));
                }
            }
        }
        let channels = slots
            .into_iter()
            .flat_map(|(index, kind)| {
                components(kind)
                    .iter()
                    .map(move |&component| Self::Operation {
                        index,
                        kind,
                        component,
                    })
            })
            .chain(COEFFICIENTS.iter().copied().map(Self::Matrix));
        Some(channels.map(TrackCurveChannel::Transform).collect())
    }

    pub fn component(self) -> CurveChannel {
        match self {
            Self::Operation { component, .. } | Self::Matrix(component) => component,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Operation {
                index,
                kind,
                component,
            } => {
                format!("Operation {index} · {} {}", kind.label(), component.label())
            }
            Self::Matrix(component) => format!("Combined · {}", component.label()),
        }
    }

    pub(super) fn path(self, component: &str) -> String {
        match self {
            Self::Operation { index, kind, .. } => {
                format!("{OPERATION}{index}/{}/{component}", kind.wire_name())
            }
            Self::Matrix(_) => format!("{MATRIX}{component}"),
        }
    }

    pub(super) fn parse(value: &str) -> Option<Self> {
        if let Some(component) = value.strip_prefix(MATRIX) {
            let component = component.parse().ok()?;
            return COEFFICIENTS
                .contains(&component)
                .then_some(Self::Matrix(component));
        }
        let mut path = value.strip_prefix(OPERATION)?.split('/');
        let index = path.next()?.parse().ok()?;
        use serde::de::value::{Error, StrDeserializer};
        let kind = TransformOperationKind::deserialize(StrDeserializer::<Error>::new(path.next()?))
            .ok()?;
        let component = path.next()?.parse().ok()?;
        (path.next().is_none() && components(kind).contains(&component)).then_some(
            Self::Operation {
                index,
                kind,
                component,
            },
        )
    }
}

fn components(kind: TransformOperationKind) -> &'static [CurveChannel] {
    use CurveChannel::*;
    match kind {
        TransformOperationKind::Translate => &[AbsoluteX, AbsoluteY],
        TransformOperationKind::Rotate => &[Angle],
        TransformOperationKind::Scale => &[X, Y],
        TransformOperationKind::Skew => &[AngleX, AngleY],
        TransformOperationKind::Matrix => COEFFICIENTS,
    }
}

impl TransformOperationKind {
    fn label(self) -> &'static str {
        match self {
            Self::Translate => "Translate",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::Skew => "Skew",
            Self::Matrix => "Matrix",
        }
    }

    fn wire_name(self) -> &'static str {
        match self {
            Self::Translate => "translate",
            Self::Rotate => "rotate",
            Self::Scale => "scale",
            Self::Skew => "skew",
            Self::Matrix => "matrix",
        }
    }
}
