use super::{AnimationBrush, AnimationGradient, BrushForm};

impl AnimationBrush {
    pub fn to_xaml(&self) -> String {
        self.write_object("").unwrap_or_else(|| match self {
            Self::Solid { argb, .. } => format!("#{:08X}", u32::from_be_bytes(*argb)),
            Self::Gradient { .. } => unreachable!(),
        })
    }

    pub(crate) fn setter_object_xaml(&self) -> Option<String> {
        self.write_object(r#" xmlns="https://github.com/ferroui""#)
    }

    fn write_object(&self, ns: &str) -> Option<String> {
        let (object, wrapped) = match self {
            Self::Gradient { value, wrapped } => (value.object_xaml(ns), *wrapped),
            Self::Solid {
                form: BrushForm::Literal,
                ..
            } => return None,
            Self::Solid {
                argb,
                opacity,
                form,
            } => {
                let argb = u32::from_be_bytes(*argb);
                (
                    format!(r##"<SolidColorBrush{ns} Color="#{argb:08X}" Opacity="{opacity}" />"##),
                    *form == BrushForm::PropertyElement,
                )
            }
        };
        Some(if wrapped {
            format!("<Setter.Value{ns}>{object}</Setter.Value>")
        } else {
            object
        })
    }
}

impl AnimationGradient {
    fn object_xaml(&self, ns: &str) -> String {
        use std::fmt::Write;
        let class = self.shape.class_name();
        let opacity = self.opacity;
        let spread = self.spread.name();
        let origin = self.transform_origin.native();
        let mut result = format!(
            r#"<{class}{ns} Opacity="{opacity}" SpreadMethod="{spread}" TransformOrigin="{origin}""#
        );
        for (name, value) in self.shape.fields() {
            write!(result, r#" {name}="{}""#, value.to_xaml()).unwrap();
        }
        for (name, transform) in [
            ("Transform", &self.transform),
            ("RelativeTransform", &self.relative_transform),
        ] {
            if let Some(transform) = transform {
                let text = crate::xaml::escape(&transform.to_xaml());
                write!(result, r#" {name}="{text}""#).unwrap();
            }
        }
        result.push('>');
        for stop in &self.stops {
            let offset = stop.offset;
            let argb = u32::from_be_bytes(stop.argb);
            write!(
                result,
                r##"<GradientStop Offset="{offset}" Color="#{argb:08X}" />"##
            )
            .unwrap();
        }
        write!(result, "</{class}>").unwrap();
        result
    }
}
