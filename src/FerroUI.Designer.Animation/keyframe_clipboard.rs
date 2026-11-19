use crate::{
    AnimationError, Keyframe, KeyframeLocation, Result, Timeline, Track, ValueKind,
    native_key_time, require,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const FORMAT: &str = "ferroui.keyframes.v1";
const MAX_KEYS: usize = 4096;
const MAX_BYTES: usize = 1024 * 1024;

/// The portable clipboard has typed key values and relative times. It carries no
/// source text, document identity, or live control IDs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyframeClipboard {
    pub format: String,
    pub tracks: Vec<CopiedKeyframeTrack>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopiedKeyframeTrack {
    pub property: String,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyframeDestination {
    pub target: u64,
    pub property: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyframePaste {
    pub clipboard: KeyframeClipboard,
    pub destinations: Vec<KeyframeDestination>,
    pub time: f64,
    #[serde(default)]
    pub reverse: bool,
}

impl KeyframeClipboard {
    pub fn from_text(text: &str) -> Result<Self> {
        require(
            text.len() <= MAX_BYTES,
            "The keyframe clipboard exceeds 1 MiB",
        )?;
        let clipboard: Self = serde_json::from_str(text).map_err(|_| {
            AnimationError("Copy FerroUI keyframes from the Timeline before pasting".into())
        })?;
        clipboard.validate()?;
        Ok(clipboard)
    }

    pub fn to_text(&self) -> Result<String> {
        let text =
            serde_json::to_string(self).map_err(|error| AnimationError(error.to_string()))?;
        require(
            text.len() <= MAX_BYTES,
            "The keyframe clipboard exceeds 1 MiB",
        )?;
        Ok(text)
    }

    pub fn validate(&self) -> Result<()> {
        require(
            self.format == FORMAT,
            "The clipboard does not contain FerroUI keyframes",
        )?;
        require(
            !self.tracks.is_empty()
                && self.tracks.len() <= MAX_KEYS
                && self
                    .tracks
                    .iter()
                    .map(|track| track.keyframes.len())
                    .sum::<usize>()
                    <= MAX_KEYS,
            "Copy between 1 and 4096 keyframes at a time",
        )?;
        let mut starts_at_zero = false;
        for track in &self.tracks {
            let first = track
                .keyframes
                .first()
                .ok_or_else(|| AnimationError("A copied track is empty".into()))?;
            let end = track.keyframes.last().expect("nonempty track").time;
            native_key_time(end)?;
            Track {
                target: 1,
                property: track.property.clone(),
                keyframes: track.keyframes.clone(),
                base_value: None,
                muted: false,
            }
            .validate(end)?;
            starts_at_zero |= first.time == 0.0;
        }
        require(starts_at_zero, "Copied key times must start at zero")
    }
}

impl CopiedKeyframeTrack {
    pub fn compatible_with(&self, track: &Track) -> bool {
        let Some(kind) = self.keyframes.first().map(|key| key.value.kind()) else {
            return false;
        };
        track
            .keyframes
            .first()
            .is_some_and(|key| key.value.kind() == kind)
            && (kind != ValueKind::Discrete || track.property == self.property)
    }
}

impl Timeline {
    pub fn copy_keyframes(&self, locations: &[KeyframeLocation]) -> Result<KeyframeClipboard> {
        let tracks = selected_tracks(self, locations)?;
        let first = locations
            .iter()
            .map(|key| key.time)
            .min_by(f64::total_cmp)
            .expect("validated selection");
        let clipboard = KeyframeClipboard {
            format: FORMAT.into(),
            tracks: tracks
                .into_iter()
                .map(|((_, property), mut keyframes)| {
                    for key in &mut keyframes {
                        key.time -= first;
                    }
                    CopiedKeyframeTrack {
                        property: property.into(),
                        keyframes,
                    }
                })
                .collect(),
        };
        clipboard.validate()?;
        clipboard.to_text()?;
        Ok(clipboard)
    }

    /// Paste at the earliest copied time. Destinations follow clipboard track order.
    /// The returned replacement is independent of the source clipboard's document.
    pub fn with_pasted_keyframes(
        &self,
        paste: &KeyframePaste,
    ) -> Result<(Self, Vec<KeyframeLocation>)> {
        let KeyframePaste {
            clipboard,
            destinations,
            time,
            reverse,
        } = paste;
        clipboard.validate()?;
        require(
            (0.0..=self.duration).contains(time),
            "Choose a paste position within the animation duration",
        )?;
        require(
            destinations.len() == clipboard.tracks.len()
                && destinations.iter().collect::<BTreeSet<_>>().len() == destinations.len(),
            "Choose one distinct destination for each copied track",
        )?;
        let last = clipboard
            .tracks
            .iter()
            .filter_map(|track| track.keyframes.last())
            .map(|key| key.time)
            .fold(0.0, f64::max);
        let mut result = self.clone();
        let mut locations = Vec::new();
        for (copied, destination) in clipboard.tracks.iter().zip(destinations) {
            let track = result
                .track_mut(destination.target, &destination.property)
                .ok_or_else(|| AnimationError("Choose a current destination track".into()))?;
            require(
                copied.compatible_with(track),
                format!(
                    "{} cannot receive keys from {}",
                    track.property, copied.property
                ),
            )?;
            let mut keys = copied.keyframes.clone();
            if *reverse {
                crate::keyframe_mirror::reverse_easings(&mut keys)?;
            }
            for key in &mut keys {
                key.time = destination_time(
                    time + if *reverse { last - key.time } else { key.time },
                    self.duration,
                )?;
                locations.push(KeyframeLocation {
                    target: track.target,
                    property: track.property.clone(),
                    time: key.time,
                });
            }
            insert_keys(track, keys)?;
        }
        result.validate()?;
        Ok((result, locations))
    }
}

pub(super) fn selected_tracks<'a>(
    timeline: &'a Timeline,
    locations: &[KeyframeLocation],
) -> Result<BTreeMap<(u64, &'a str), Vec<Keyframe>>> {
    require(
        !locations.is_empty() && locations.len() <= MAX_KEYS,
        "Select between 1 and 4096 keyframes",
    )?;
    let mut tracks = BTreeMap::<_, Vec<Keyframe>>::new();
    let mut seen = BTreeSet::new();
    for location in locations {
        let track = timeline
            .track(location.target, &location.property)
            .ok_or_else(|| AnimationError("The selected track no longer exists".into()))?;
        let time = if location.time == 0.0 {
            0.0
        } else {
            location.time
        };
        let at = track
            .keyframes
            .binary_search_by(|key| key.time.total_cmp(&time))
            .map_err(|_| AnimationError("A selected keyframe no longer exists".into()))?;
        require(
            seen.insert((track.target, track.property.as_str(), time.to_bits())),
            "Select each keyframe only once",
        )?;
        tracks
            .entry((track.target, track.property.as_str()))
            .or_default()
            .push(track.keyframes[at].clone());
    }
    for keys in tracks.values_mut() {
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
    }
    Ok(tracks)
}

pub(super) fn destination_time(time: f64, duration: f64) -> Result<f64> {
    let tolerance = (2.0 * f64::EPSILON * duration.max(1.0)).min(0.00000005);
    require(
        (-tolerance..=duration + tolerance).contains(&time),
        "The keyframes would leave the animation duration",
    )?;
    let time = native_key_time(time.clamp(0.0, duration))?;
    require(
        time <= duration,
        "The native key time would leave the animation duration",
    )?;
    Ok(time)
}

pub(super) fn insert_keys(track: &mut Track, incoming: Vec<Keyframe>) -> Result<()> {
    let mut occupied = track
        .keyframes
        .iter()
        .map(|key| native_key_time(key.time).map(f64::to_bits))
        .collect::<Result<BTreeSet<_>>>()?;
    for key in &incoming {
        require(
            occupied.insert(key.time.to_bits()),
            format!(
                "A keyframe already occupies {} at {} s",
                track.property, key.time
            ),
        )?;
    }
    track.keyframes.extend(incoming);
    track.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(())
}
