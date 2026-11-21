use super::State;
use crate::{image_source, literal::Literal};
use crate::tree::Element;
use ferroui_designer_core::SourceSpan;
use std::collections::BTreeSet;
use syn::{spanned::Spanned, Expr};

pub(super) struct PropertyField<'a> {
    pub field: Option<SourceSpan>,
    pub fallback: SourceSpan,
    pub expression: Option<&'a Expr>,
}

impl State<'_> {
    pub(super) fn project_property(
        &mut self,
        index: usize,
        result: &mut Element,
        seen: &mut BTreeSet<String>,
        key: String,
        text: &str,
        source: PropertyField<'_>,
    ) {
        let operand = source.expression.map_or(source.fallback, |value| {
            let range = value.span().byte_range();
            SourceSpan::new(range.start, range.end)
        });
        let node = &mut self.nodes[index];
        let omitted = if !seen.insert(key.clone()) {
            result.properties.remove(&key);
            node.properties.remove(&key);
            node.fields.remove(&key);
            format!("Repeated {}.{key}", node.class)
        } else if let Some((field, text)) = source.field.zip(
            node.class
                .literal(&key)
                .and_then(|kind| {
                    let expression = source.expression?;
                    match kind {
                        Literal::EmbeddedImage => image_source::projected_reference(
                            expression, text, self.modules, &self.image_includes,
                        ),
                        _ => kind.read_rust(expression),
                    }
                }),
        ) {
            node.properties.insert(key.clone(), operand);
            node.fields.insert(key.clone(), field);
            result.properties.insert(key, text);
            return;
        } else {
            format!("{}.{key} expression", node.class)
        };
        node.expressions.insert(key, operand);
        self.omit(omitted);
    }
}

#[cfg(test)]
mod tests {
    use crate::{analyze_rsx, project_rust};
    use ferroui_designer_core::TextPatch;

    #[test]
    fn property_reset_keeps_comments_before_within_and_after_a_builder_call() {
        let source = concat!(
            "rsx! { Button {\r\n",
            "    // preferred width\r\n",
            "    width(/* minimum 🧪 */ 112.0,) /* logical */,\r\n",
            "    content(\"Next\"),\r\n",
            "} }",
        );
        let projection = project_rust(source, 0).unwrap();
        let patch = projection.edit_property(0, "Width", None).unwrap();
        let edited = TextPatch::apply_all(source, vec![patch]).unwrap();
        for trivia in ["// preferred width", "/* minimum 🧪 */", "/* logical */"] {
            assert!(edited.contains(trivia));
        }
        assert!(!edited.contains("width("));
        assert!(edited.contains("content(\"Next\"),\r\n"));
        let result = project_rust(&edited, 0).unwrap();
        assert_eq!(result.omitted_count, 0);
        assert!(!result.nodes[0].properties.contains_key("Width"));
    }

    #[test]
    fn an_ambiguous_or_nonliteral_builder_call_cannot_be_reset_or_overwritten() {
        for field in [
            "width(ideal_width())",
            "width()",
            "width(12.0, 24.0)",
            "width(12.0), width: 18.0",
            "width: 12.0, width(18.0)",
        ] {
            let source = format!("rsx! {{ Button {{ {field} }} }}");
            let projection = project_rust(&source, 0).unwrap();
            assert!(!projection.nodes[0].properties.contains_key("Width"));
            assert!(projection.nodes[0].expressions.contains_key("Width"));
            assert!(projection.edit_property(0, "Width", None).is_err());
            assert!(projection.edit_property(0, "Width", Some("23")).is_err());
            let analysis = serde_json::to_value(analyze_rsx(&source)).unwrap();
            assert!(analysis["nodes"][0]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .all(|field| field["literal"] == false));
        }
    }

    #[test]
    fn image_source_call_updates_only_the_original_include() {
        let source = r##"rsx! { Image {
            source(::ferroui_view::embedded_image(
                /* license 🎨 */ ::core::include_bytes!(r#"../art/Łódź #%.png"#),
            )),
        } }"##
            .replace('\n', "\r\n");
        let projection = project_rust(&source, 0).unwrap();
        assert!(projection.nodes[0].can_set_image_source());
        assert!(projection
            .xaml()
            .contains("../art/%C5%81%C3%B3d%C5%BA%20%23%25.png"));
        let patch = projection
            .edit_property(0, "Source", Some("../art/Next%20image.png"))
            .unwrap();
        assert_eq!(
            TextPatch::apply_all(&source, vec![patch]).unwrap(),
            source.replace("Łódź #%.png", "Next image.png"),
        );
    }

    #[test]
    fn attached_layout_call_preserves_spelling_and_shares_its_xaml_property() {
        let source = r#"rsx! { Grid { Button { grid_row(/* lane */ 1), content("Next") } } }"#;
        let projection = project_rust(source, 0).unwrap();
        assert_eq!(projection.omitted_count, 0);
        let draft = projection
            .xaml()
            .replace("Grid.Row=\"1\"", "Grid.Row=\"2\"");
        assert_eq!(
            TextPatch::apply_all(source, projection.apply_xaml(&draft).unwrap()).unwrap(),
            source.replace("/* lane */ 1", "/* lane */ 2"),
        );
        for previous in ["Grid.row: 0", "grid_row: 0"] {
            let duplicate = source.replace("grid_row(", &format!("{previous}, grid_row("));
            let projection = project_rust(&duplicate, 0).unwrap();
            assert!(projection.nodes[1].expressions.contains_key("Grid.Row"));
            assert!(projection.edit_property(1, "Grid.Row", Some("2")).is_err());
        }
    }
}
