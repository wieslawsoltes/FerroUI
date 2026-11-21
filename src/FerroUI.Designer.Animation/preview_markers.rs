use crate::{Result, require, xaml::native_ticks};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A named position on one preview clock, stored independently of source.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PreviewMarker {
    pub name: String,
    pub time: f64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreviewMarkerEdit {
    Set { name: String, time: f64 },
    Remove { name: String },
}

#[derive(Default)]
pub struct PreviewMarkers(BTreeMap<String, f64>);

impl PreviewMarkers {
    /// List markers usable on the current clock in time and name order.
    pub fn list(&self, end: Option<f64>) -> Vec<PreviewMarker> {
        let mut markers: Vec<_> = self
            .0
            .iter()
            .filter(|(_, time)| end.is_none_or(|end| **time <= end))
            .map(|(name, &time)| PreviewMarker {
                name: name.clone(),
                time,
            })
            .collect();
        markers.sort_by(|left, right| {
            left.time
                .total_cmp(&right.time)
                .then(left.name.cmp(&right.name))
        });
        markers
    }

    pub fn find(&self, name: &str, end: Option<f64>) -> Option<f64> {
        self.0
            .get(name.trim())
            .copied()
            .filter(|time| end.is_none_or(|end| *time <= end))
    }

    pub fn apply(&mut self, edit: PreviewMarkerEdit, end: Option<f64>) -> Result<()> {
        match edit {
            PreviewMarkerEdit::Set { name, time } => {
                let name = marker_name(&name)?;
                require(
                    native_ticks(time).is_ok() && end.is_none_or(|end| time <= end),
                    "Choose a marker time within the preview clock",
                )?;
                require(
                    self.0.len() < 64 || self.0.contains_key(name),
                    "Remove a marker before adding another",
                )?;
                self.0.insert(name.to_owned(), time);
            }
            PreviewMarkerEdit::Remove { name } => {
                let removed = self.0.remove(marker_name(&name)?);
                require(removed.is_some(), "Choose a marker from this preview clock")?;
            }
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn marker_name(name: &str) -> Result<&str> {
    let name = name.trim();
    require(
        !name.is_empty()
            && name.encode_utf16().count() <= 48
            && !name.bytes().any(|byte| byte.is_ascii_control()),
        "Enter a marker name of up to 48 characters",
    )?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(name: &str, time: f64) -> PreviewMarkerEdit {
        PreviewMarkerEdit::Set {
            name: name.into(),
            time,
        }
    }

    #[test]
    fn markers_validate_and_sort_on_the_available_clock() {
        let mut markers = PreviewMarkers::default();
        assert!(markers.apply(set("Bad\tname", 0.0), None).is_err());
        assert!(
            markers
                .apply(set("🚀".repeat(25).as_str(), 0.0), None)
                .is_err()
        );
        assert!(markers.apply(set("Infinite", f64::INFINITY), None).is_err());
        assert!(markers.apply(set("Outside", 2.0), Some(1.0)).is_err());
        markers.apply(set(" Later ", 0.75), Some(1.0)).unwrap();
        markers.apply(set("Earlier", 0.25), Some(1.0)).unwrap();
        assert_eq!(markers.list(Some(0.5))[0].name, "Earlier");
        assert_eq!(markers.find("Later", Some(0.5)), None);
        assert_eq!(markers.find("Later", None), Some(0.75));
        markers.apply(set("Later", 0.15), None).unwrap();
        assert_eq!(markers.list(None)[0].name, "Later");
        markers
            .apply(
                PreviewMarkerEdit::Remove {
                    name: "Later".into(),
                },
                Some(0.0),
            )
            .unwrap();
        assert_eq!(markers.list(None).len(), 1);
    }
}
