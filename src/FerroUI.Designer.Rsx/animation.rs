//! Original Rust source for a native animation projected from a known view builder.

mod syntax;

use ferroui_designer_animation::{
    native_key_time, AnimationValue, Easing, KeyframeEdit, ValueKind,
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::Expr;

pub(crate) const TRACK_TARGET: u64 = 1;

#[derive(Debug)]
pub(crate) struct AnimationStyle {
    pub(crate) selector: String,
    pub(crate) clips: Vec<RsxAnimation>,
}

/// Source mapping for one clip. Its keys address only authored Rust literals.
#[derive(Clone, Debug)]
pub struct RsxAnimation {
    pub declaration: SourceSpan,
    pub(crate) xaml: String,
    duration: f64,
    keys: Vec<KeySource>,
}

#[derive(Clone, Debug)]
struct KeySource {
    property: &'static str,
    time: f64,
    cue: FloatSource,
    value: FloatSource,
    spline: Option<[FloatSource; 4]>,
    end: usize,
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
        let AnimationValue::Number(value) =
            AnimationValue::parse(value, ValueKind::Number).map_err(|error| error.to_string())?
        else {
            unreachable!("specified number kind")
        };
        let easing = Easing::parse(easing).map_err(|error| error.to_string())?;
        let points = match easing {
            Easing::Linear => None,
            Easing::CubicBezier { x1, y1, x2, y2 } => Some([x1, y1, x2, y2]),
            _ => return Err("Choose linear or cubic Bézier easing for this Rust keyframe".into()),
        };
        let mapped_time = native_key_time(time).map_err(|error| error.to_string())?;
        if self.keys.iter().any(|other| {
            other.property == key.property && other.time != key.time && other.time == mapped_time
        }) {
            return Err("Another keyframe occupies the destination time".into());
        }
        let mut patches = Vec::new();
        if key.time != mapped_time {
            key.cue.plan(mapped_time / self.duration, &mut patches);
        }
        key.value.plan(value, &mut patches);
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

/// Find the inline native style builders without resolving arbitrary Rust names.
pub(crate) fn projected_styles(value: &Expr) -> Option<(Vec<AnimationStyle>, usize)> {
    let Expr::Array(array) = value else {
        return None;
    };
    let styles: Vec<_> = array.elems.iter().filter_map(syntax::style).collect();
    if styles.is_empty() {
        return None;
    }
    let omitted = array.elems.len() - styles.len();
    Some((styles, omitted))
}
