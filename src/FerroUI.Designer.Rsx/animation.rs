//! Original Rust source for a native animation projected from a known view builder.

mod box_shadows;
mod brush;
mod color;
mod effect;
mod effect_constructor;
mod color_integer;
mod color_text;
mod keyframe;
pub(crate) mod program_comment;
mod relative;
mod retime;
mod structural;
pub(crate) mod syntax;
mod text_source;
mod transform_list;
mod value;

use ferroui_designer_animation::{native_key_time, Easing, KeyframeEdit, ValueKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
pub(crate) use keyframe::{BUILTINS as BUILTIN_PROPERTIES, property as view_property};
pub(crate) use value::{ValueShape, ValueSource, rust_literal};

pub(crate) const TRACK_TARGET: u64 = 1;

#[derive(Clone)]
pub(crate) struct Property {
    pub(crate) name: String,
    pub(crate) shape: ValueShape,
    pub(crate) owner: Option<TargetType>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TargetType {
    pub(crate) name: String,
    pub(crate) namespace: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RsxAnimationIssue {
    pub declaration: SourceSpan,
    pub reason: &'static str,
}

/// Source mapping for one clip. Its keys address only authored Rust literals.
#[derive(Clone, Debug)]
pub struct RsxAnimation {
    pub declaration: SourceSpan,
    pub(crate) xaml: String,
    duration: f64,
    factory_end: usize,
    first_method: Option<usize>,
    delay: Option<DelaySource>,
    target_type: Option<TargetType>,
    keys: Vec<KeySource>,
    pub(crate) attachment: Option<program_comment::AnimationAttachment>,
}

#[derive(Clone, Debug)]
struct DelaySource {
    millis: u64,
    literal: SourceSpan,
    explicit_type: bool,
}

#[derive(Clone, Debug)]
struct KeySource {
    property: String,
    getter: String,
    time: f64,
    cue: FloatSource,
    value: ValueSource,
    spline: Option<[FloatSource; 4]>,
    spline_method: Option<SourceSpan>,
    shared: bool,
    end: usize,
    frame: SourceSpan,
    setter: SourceSpan,
}

impl KeySource {
    fn easing(&self) -> Easing {
        self.spline.map_or(Easing::Linear, |points| {
            let [x1, y1, x2, y2] = points.map(|point| point.value);
            Easing::CubicBezier { x1, y1, x2, y2 }
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct FloatSource {
    value: f64,
    literal: SourceSpan,
    minus: Option<SourceSpan>,
    explicit_type: bool,
}

impl FloatSource {
    fn plan(self, value: f64, patches: &mut Vec<TextPatch>) {
        if self.value == value {
            return;
        }
        let prefix = if self.minus.is_none() && value.is_sign_negative() {
            "-"
        } else {
            ""
        };
        let suffix = if self.explicit_type { "f64" } else { "" };
        patches.push(TextPatch {
            span: self.literal,
            replacement: format!("{prefix}{:?}{suffix}", value.abs()),
        });
        if let Some(minus) = self.minus.filter(|_| !value.is_sign_negative()) {
            patches.push(TextPatch {
                span: minus,
                replacement: String::new(),
            });
        }
    }
}

impl RsxAnimation {
    /// The whole clip is an inline style attachment with checked source edits.
    pub fn can_edit_program(&self) -> bool {
        self.attachment.is_some()
    }

    pub(crate) fn delay_patch(&self, delay: f64) -> Result<Option<TextPatch>, String> {
        let millis = (native_key_time(delay).map_err(|error| error.to_string())? * 1000.0).round();
        if millis / 1000.0 != delay {
            return Err("Use a clip delay in whole milliseconds for Rust animations".into());
        }
        if self.delay.as_ref().map_or(0, |delay| delay.millis) == millis as u64 {
            return Ok(None);
        }
        let patch = match &self.delay {
            Some(delay) => TextPatch {
                span: delay.literal,
                replacement: format!("{millis:.0}{}", if delay.explicit_type { "u64" } else { "" }),
            },
            None => TextPatch {
                span: SourceSpan::new(self.factory_end, self.factory_end),
                replacement: format!(".delay(ferroui_view::ms({millis:.0}))"),
            },
        };
        Ok(Some(patch))
    }

    /// Whether a native track can be edited back to this declaration.
    pub fn maps_key(&self, property: &str, time: f64, kind: ValueKind) -> bool {
        self.keys.iter().any(|key| {
            key.property == property && key.time == time && key.value.value.kind() == kind
        })
    }

    /// Map a validated native keyframe edit back to its original Rust spelling.
    /// The host checks the accepted document and native keyframe plan first.
    pub fn patch_keyframe(
        &self,
        property: &str,
        edit: KeyframeEdit<'_>,
    ) -> Result<Vec<TextPatch>, String> {
        let KeyframeEdit::Upsert {
            previous: Some(previous),
            time,
            value,
            easing,
        } = edit
        else {
            return Err("Select an existing mapped Rust keyframe".into());
        };
        let key = self
            .keys
            .iter()
            .find(|key| key.property == property && key.time == previous)
            .ok_or("Choose an authored Rust keyframe")?;
        if !time.is_finite() || !(0.0..=self.duration).contains(&time) {
            return Err("Choose a key time within the animation duration".into());
        }
        let easing = Easing::parse(easing).map_err(|error| error.to_string())?;
        let points = match easing {
            Easing::Linear => None,
            Easing::CubicBezier { x1, y1, x2, y2 } => Some([x1, y1, x2, y2]),
            _ => return Err("Choose linear or cubic Bézier easing for this Rust keyframe".into()),
        };
        let mapped_time = native_key_time(time).map_err(|error| error.to_string())?;
        if key.shared
            && (key.time != mapped_time
                || points != key.spline.map(|spline| spline.map(|point| point.value)))
        {
            return Err("This Rust keyframe shares its cue and easing with other setters. Edit those fields in source.".into());
        }
        if self.keys.iter().any(|other| {
            other.property == key.property && other.time != key.time && other.time == mapped_time
        }) {
            return Err("Another keyframe occupies the destination time".into());
        }
        let mut patches = Vec::new();
        if key.time != mapped_time {
            key.cue.plan(mapped_time / self.duration, &mut patches);
        }
        key.value.plan(value, &mut patches)?;
        if let Some(existing) = key.spline {
            for (literal, point) in existing
                .into_iter()
                .zip(points.unwrap_or([0.0, 0.0, 1.0, 1.0]))
            {
                literal.plan(point, &mut patches);
            }
        } else if let Some([x1, y1, x2, y2]) = points {
            patches.push(TextPatch {
                span: SourceSpan::new(key.end, key.end),
                replacement: format!(".key_spline({x1:?}, {y1:?}, {x2:?}, {y2:?})"),
            });
        }
        patches.sort_by_key(|patch| patch.span.start);
        Ok(patches)
    }
}
