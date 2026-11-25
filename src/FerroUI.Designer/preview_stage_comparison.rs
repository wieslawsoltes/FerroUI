use crate::preview_environment::{DESKTOP_SIZE, PHONE_SIZE, TABLET_SIZE};
use crate::{
    PreviewEnvironment, PreviewFlowDirection, PreviewSizePreset, PreviewStageConfiguration,
    PreviewTheme,
};
use serde::{Deserialize, Serialize};

const SIZES: [(&str, PreviewSizePreset); 3] = [
    ("Phone", PHONE_SIZE),
    ("Tablet", TABLET_SIZE),
    ("Desktop", DESKTOP_SIZE),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStageComparison {
    Sizes,
    Themes,
    Orientations,
    Directions,
}

impl PreviewStageComparison {
    pub fn variants(
        self,
        configuration: &PreviewStageConfiguration,
    ) -> Vec<(String, PreviewEnvironment)> {
        let base = configuration.environment;
        let size = |width, height| PreviewEnvironment {
            width,
            height,
            ..base
        };
        let name = |label| {
            let suffix = format!(" · {label}");
            let mut end = configuration.name.len().min(128 - suffix.len());
            while !configuration.name.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}{suffix}", configuration.name[..end].trim_end())
        };
        match self {
            Self::Sizes => SIZES
                .into_iter()
                .map(|(label, preset)| (label.into(), size(preset.width, preset.height)))
                .collect(),
            Self::Themes => vec![
                (
                    name("Light"),
                    PreviewEnvironment {
                        theme: PreviewTheme::Light,
                        ..base
                    },
                ),
                (
                    name("Dark"),
                    PreviewEnvironment {
                        theme: PreviewTheme::Dark,
                        ..base
                    },
                ),
            ],
            Self::Orientations => {
                let short = base.width.min(base.height);
                let long = base.width.max(base.height);
                vec![
                    (name("Portrait"), size(short, long)),
                    (name("Landscape"), size(long, short)),
                ]
            }
            Self::Directions => vec![
                (
                    name("LTR"),
                    PreviewEnvironment {
                        flow_direction: PreviewFlowDirection::LeftToRight,
                        ..base
                    },
                ),
                (
                    name("RTL"),
                    PreviewEnvironment {
                        flow_direction: PreviewFlowDirection::RightToLeft,
                        ..base
                    },
                ),
            ],
        }
    }
}
