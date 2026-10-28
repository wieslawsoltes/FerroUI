//! Local canvas aids expressed in authored logical units. These are workspace preferences,
//! never XAML properties. A scope owns its guides; a camera only changes their projection.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanvasGuideAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasGuide {
    pub id: u64,
    pub axis: CanvasGuideAxis,
    pub position: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasGuideSettings {
    pub rulers_visible: bool,
    pub guides_visible: bool,
    pub grid_visible: bool,
    pub snap_to_grid: bool,
    pub snap_to_guides: bool,
    pub snap_to_objects: bool,
    pub grid_spacing: f64,
    /// Screen logical pixels, independent of the canvas zoom.
    pub snap_tolerance: f64,
    pub small_nudge: f64,
    pub large_nudge: f64,
    pub guides: Vec<CanvasGuide>,
}

impl Default for CanvasGuideSettings {
    fn default() -> Self {
        Self {
            rulers_visible: false,
            guides_visible: true,
            grid_visible: false,
            snap_to_grid: true,
            snap_to_guides: true,
            snap_to_objects: true,
            grid_spacing: 8.0,
            snap_tolerance: 6.0,
            small_nudge: 1.0,
            large_nudge: 10.0,
            guides: Vec::new(),
        }
    }
}

impl CanvasGuideSettings {
    pub const MAX_GUIDES: usize = 256;
    pub const MAX_POSITION: f64 = 1_000_000.0;

    pub fn validate(&self) -> Result<(), String> {
        if self.guides.len() > Self::MAX_GUIDES {
            return Err(format!(
                "Use at most {} guides in one canvas scope",
                Self::MAX_GUIDES
            ));
        }
        for (name, value, minimum, maximum) in [
            ("Grid spacing", self.grid_spacing, 0.1, 100_000.0),
            ("Snap tolerance", self.snap_tolerance, 0.0, 32.0),
            ("Small nudge", self.small_nudge, 0.01, 10_000.0),
            ("Large nudge", self.large_nudge, 0.01, 100_000.0),
        ] {
            if !value.is_finite() || !(minimum..=maximum).contains(&value) {
                return Err(format!("{name} must be between {minimum} and {maximum}"));
            }
        }
        let mut ids = BTreeSet::new();
        for guide in &self.guides {
            if guide.id == 0 || !ids.insert(guide.id) {
                return Err("Each guide must have a unique, nonzero identity".into());
            }
            Self::validate_position(guide.position)?;
        }
        Ok(())
    }

    pub fn validate_position(position: f64) -> Result<(), String> {
        if !position.is_finite() || position.abs() > Self::MAX_POSITION {
            Err(
                "Guide positions must be finite and between -1000000 and 1000000 logical units"
                    .into(),
            )
        } else {
            Ok(())
        }
    }

    pub fn add(&mut self, axis: CanvasGuideAxis, position: f64) -> Result<u64, String> {
        self.validate()?;
        Self::validate_position(position)?;
        if self.guides.len() == Self::MAX_GUIDES {
            return Err(format!(
                "Use at most {} guides in one canvas scope",
                Self::MAX_GUIDES
            ));
        }
        let id = (1..=Self::MAX_GUIDES as u64 + 1)
            .find(|id| self.guides.iter().all(|guide| guide.id != *id))
            .expect("A bounded guide set always has an available identity");
        self.guides.push(CanvasGuide { id, axis, position });
        Ok(id)
    }

    pub(crate) fn snap_settings(&self, zoom: f64) -> ferroui_designer_tools::SnapSettings {
        let mut settings = ferroui_designer_tools::SnapSettings {
            grid: self.snap_to_grid.then_some(self.grid_spacing),
            tolerance: self.snap_tolerance / zoom.max(crate::CanvasViewport::MIN_ZOOM),
            guides_x: Vec::new(),
            guides_y: Vec::new(),
        };
        if self.snap_to_guides {
            for guide in &self.guides {
                match guide.axis {
                    CanvasGuideAxis::Horizontal => settings.guides_y.push(guide.position),
                    CanvasGuideAxis::Vertical => settings.guides_x.push(guide.position),
                }
            }
        }
        settings
    }
}

/// At most one screen's worth of ticks, even at extreme camera offsets and scales.
pub(crate) fn ruler_ticks(offset: f64, zoom: f64, length: f64) -> Vec<(f64, f64, bool)> {
    if !offset.is_finite()
        || !zoom.is_finite()
        || zoom <= 0.0
        || !length.is_finite()
        || length <= 0.0
    {
        return Vec::new();
    }
    let raw = 80.0 / zoom;
    let power = 10.0f64.powf(raw.log10().floor());
    let normalized = raw / power;
    let major = power
        * if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };
    let minor = major / 5.0;
    let first = ((-offset / zoom) / minor).ceil();
    let count = (length / (minor * zoom)).ceil().clamp(0.0, 1024.0) as usize + 1;
    (0..count)
        .filter_map(|index| {
            let tick = first + index as f64;
            let value = tick * minor;
            let pixel = value * zoom + offset;
            (pixel >= 0.0 && pixel <= length && value.is_finite()).then_some((
                pixel,
                value,
                (tick / 5.0 - (tick / 5.0).round()).abs() < 0.00001,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guides_validate_before_changes_and_keep_snap_visibility_independent() {
        let mut settings = CanvasGuideSettings::default();
        let x = settings.add(CanvasGuideAxis::Vertical, 73.5).unwrap();
        settings.add(CanvasGuideAxis::Horizontal, -18.25).unwrap();
        settings.guides_visible = false;
        assert_eq!(settings.snap_settings(2.0).guides_x, vec![73.5]);
        assert_eq!(settings.snap_settings(2.0).guides_y, vec![-18.25]);
        assert_eq!(settings.snap_settings(2.0).tolerance, 3.0);
        let before = settings.clone();
        assert!(settings.add(CanvasGuideAxis::Vertical, f64::NAN).is_err());
        assert_eq!(settings, before);
        settings.guides.push(CanvasGuide {
            id: x,
            axis: CanvasGuideAxis::Horizontal,
            position: 4.0,
        });
        assert!(settings.validate().is_err());
        settings.guides.pop();
        settings.grid_spacing = 0.0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rulers_stay_bounded_and_track_real_authored_coordinates() {
        for zoom in [0.01, 0.125, 1.0, 3.75, 16.0] {
            for offset in [-1.0e12, -85.5, 0.0, 210.25, 1.0e12] {
                let ticks = ruler_ticks(offset, zoom, 1600.0);
                assert!(!ticks.is_empty());
                assert!(ticks.len() < 200);
                for (pixel, value, _) in ticks {
                    assert!((pixel - (value * zoom + offset)).abs() < 0.001);
                }
            }
        }
        assert!(ruler_ticks(0.0, f64::NAN, 100.0).is_empty());
    }
}
