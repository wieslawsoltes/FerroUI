//! Original Rust source for a native animation projected from a known view builder.

mod syntax;
mod value;

use ferroui_designer_animation::{native_key_time, Easing, KeyframeEdit, ValueKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::Expr;
pub(crate) use value::ValueShape;
use value::ValueSource;

pub(crate) const TRACK_TARGET: u64 = 1;

#[derive(Clone)]
pub(crate) struct Property {
    pub(crate) name: String,
    pub(crate) shape: ValueShape,
    pub(crate) owner: Option<TargetType>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TargetType {
    pub(crate) name: String,
    pub(crate) namespace: Option<String>,
}

#[derive(Debug)]
pub(crate) struct AnimationStyle {
    pub(crate) selector: String,
    pub(crate) clips: Vec<RsxAnimation>,
}

#[derive(Clone, Debug)]
pub struct RsxAnimationIssue {
    pub declaration: SourceSpan,
    pub reason: &'static str,
}

pub(crate) struct ProjectedStyles {
    pub(crate) styles: Vec<AnimationStyle>,
    pub(crate) unavailable: Vec<RsxAnimationIssue>,
    pub(crate) omitted: usize,
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
    property: String,
    time: f64,
    cue: FloatSource,
    value: ValueSource,
    spline: Option<[FloatSource; 4]>,
    shared: bool,
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

/// Find the inline native style builders without resolving arbitrary Rust names.
pub(crate) fn projected_styles(
    value: &Expr,
    catalog: &crate::RsxCatalog,
) -> Option<ProjectedStyles> {
    let Expr::Array(array) = value else {
        return None;
    };
    let mut result = ProjectedStyles {
        styles: Vec::new(),
        unavailable: Vec::new(),
        omitted: 0,
    };
    for expr in &array.elems {
        match syntax::style(expr, catalog) {
            Some((style, mut issues)) => {
                result.omitted += issues.len();
                result.unavailable.append(&mut issues);
                if !style.clips.is_empty() {
                    result.styles.push(style);
                }
            }
            None => result.omitted += 1,
        }
    }
    Some(result)
}
