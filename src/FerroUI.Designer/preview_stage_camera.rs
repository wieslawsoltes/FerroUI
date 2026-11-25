use serde::{Deserialize, Serialize};

pub const PREVIEW_STAGE_ZOOM_LEVELS: [u16; 6] = [25, 50, 75, 100, 150, 200];

/// Presentation of one stage inside an editor. Offsets are in viewport pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageCamera {
    /// None fits the preview without enlarging its native pixels.
    pub zoom_percent: Option<u16>,
    pub x: f64,
    pub y: f64,
}

impl PreviewStageCamera {
    pub fn validate(self) -> Result<(), String> {
        if self.zoom_percent.is_some_and(|zoom| !PREVIEW_STAGE_ZOOM_LEVELS.contains(&zoom)) {
            return Err("Choose a supported stage zoom".into());
        }
        if !self.x.is_finite() || !self.y.is_finite() || self.x < 0.0 || self.y < 0.0 {
            return Err("Stage offsets must be finite nonnegative numbers".into());
        }
        if self.zoom_percent.is_none() && (self.x != 0.0 || self.y != 0.0) {
            return Err("Choose a fixed zoom to pan a stage".into());
        }
        Ok(())
    }
}
