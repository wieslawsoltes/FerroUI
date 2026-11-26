use crate::xaml::native_ticks;
use crate::{Result, require};
use serde::{Deserialize, Serialize};

/// An editor work area in seconds on the selected preview clock.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRange {
    pub start: f64,
    pub end: f64,
    #[serde(rename = "loop")]
    pub looping: bool,
}

impl PreviewRange {
    pub fn validate(self, limit: Option<f64>) -> Result<()> {
        native_ticks(self.start)?;
        native_ticks(self.end)?;
        require(
            self.start < self.end && limit.is_none_or(|limit| self.end <= limit),
            "Choose an In and Out time within the preview clock, with In before Out",
        )
    }

    /// Keep an accepted work area when a replacement clock becomes shorter.
    pub fn fit(self, limit: Option<f64>) -> Option<Self> {
        self.validate(None).ok()?;
        let range = Self {
            end: limit.map_or(self.end, |limit| self.end.min(limit)),
            ..self
        };
        range.validate(limit).ok()?;
        Some(range)
    }

    pub fn start_at(self, playhead: f64) -> f64 {
        if (self.start..self.end).contains(&playhead) {
            playhead
        } else {
            self.start
        }
    }

    /// Return the next pose and whether its one-shot playback reached Out.
    pub fn advance(self, playhead: f64, seconds: f64) -> Result<(f64, bool)> {
        require(
            seconds.is_finite() && seconds >= 0.0 && playhead.is_finite(),
            "Playback time must be finite and the delta nonnegative",
        )?;
        self.validate(None)?;
        let playhead = playhead.clamp(self.start, self.end);
        let remaining = self.end - playhead;
        if seconds < remaining {
            return Ok(((playhead + seconds).min(self.end), false));
        }
        if self.looping {
            let offset = (seconds - remaining) % (self.end - self.start);
            Ok(((self.start + offset).min(self.end), false))
        } else {
            Ok((self.end, true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_and_hold_positions_use_the_work_area() {
        let range = PreviewRange {
            start: 0.25,
            end: 0.75,
            looping: true,
        };
        assert_eq!(range.start_at(0.75), 0.25);
        assert_eq!(range.start_at(0.5), 0.5);
        assert_eq!(range.advance(0.625, 1.25).unwrap(), (0.375, false));
        assert_eq!(range.advance(0.625, 0.125).unwrap(), (0.25, false));
        assert_eq!(
            PreviewRange {
                looping: false,
                ..range
            }
            .advance(0.625, 1.25)
            .unwrap(),
            (0.75, true)
        );
        assert_eq!(range.fit(Some(0.5)).unwrap().end, 0.5);
        assert!(range.fit(Some(0.25)).is_none());
        assert!(
            PreviewRange {
                end: f64::NAN,
                ..range
            }
            .fit(Some(2.0))
            .is_none()
        );
        assert!(range.advance(0.5, -0.1).is_err());
    }
}
