//! Checked source actions for color declarations and resource setters.
use super::{
    RsxStyle,
    edit::{insert, qualified_getter},
    resources::valid_key,
    string_literal,
};
use crate::{RsxProjection, animation::rust_literal, blocks::keep_comments};
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::TextPatch;

impl RsxProjection<'_> {
    pub(super) fn add_style_color(
        &self,
        style: &RsxStyle,
        key: &str,
        value: &str,
    ) -> Result<Vec<TextPatch>, String> {
        if !valid_key(key) {
            return Err("Enter a nonempty resource key without surrounding whitespace or control characters".into());
        }
        if style.resources.iter().any(|resource| resource.key == key) {
            return Err("This rule already declares that resource key".into());
        }
        let color =
            AnimationValue::parse(value, ValueKind::Color).map_err(|error| error.to_string())?;
        let value = rust_literal(&color)?;
        Ok(vec![insert(
            style.declaration.end,
            format!(".resource({key:?}, {value})"),
        )])
    }

    pub(super) fn change_style_color(
        &self,
        style: &RsxStyle,
        resource: usize,
        value: &str,
    ) -> Result<Vec<TextPatch>, String> {
        let authored = style
            .resources
            .iter()
            .find(|entry| entry.declaration.start == resource)
            .and_then(|entry| entry.color.as_ref())
            .ok_or("Choose an authored color resource in this rule")?;
        let mut patches = Vec::new();
        authored.plan(value, &mut patches)?;
        Ok(patches)
    }

    pub(super) fn plan_resource_setter(
        &self,
        style: &RsxStyle,
        key: &str,
        setter: Option<usize>,
        getter: Option<&str>,
    ) -> Result<Vec<TextPatch>, String> {
        if !self
            .style_resource_choices(style.declaration.start)
            .iter()
            .any(|entry| entry.key == key)
        {
            return Err("Choose a mapped color resource from this style's scope".into());
        }
        match (setter, getter) {
            (Some(setter), None) => {
                let member = style
                    .setters
                    .iter()
                    .find(|member| member.declaration.start == setter)
                    .filter(|member| member.value.is_some())
                    .filter(|member| {
                        member
                            .field
                            .as_ref()
                            .is_some_and(|field| field.accepts_color_resource())
                    })
                    .ok_or("Choose a mapped color or brush setter")?;
                if member.resource_key() == Some(key) {
                    return Ok(Vec::new());
                }
                let mut patches = Vec::new();
                let previous = &self.source[member.source.range()];
                let replacement = if member.resource_key().is_some() {
                    string_literal(previous, key)
                } else {
                    patches.push(TextPatch {
                        span: member.method,
                        replacement: "set_resource".into(),
                    });
                    let comments = keep_comments(previous)?;
                    format!("{comments}{key:?}")
                };
                patches.push(TextPatch {
                    span: member.source,
                    replacement,
                });
                Ok(patches)
            }
            (None, Some(getter)) => {
                let field = self.new_style_field(style, getter)?;
                if !field.accepts_color_resource() {
                    return Err("Choose a color or brush property for this resource".into());
                }
                let getter = qualified_getter(field);
                Ok(vec![insert(
                    style.declaration.end,
                    format!(".set_resource({getter}(), {key:?})"),
                )])
            }
            _ => Err("Choose an existing setter or a registered property to add".into()),
        }
    }
}
