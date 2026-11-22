use crate::document::DesignDocument;
use crate::edits::editable_attribute;
use crate::property_fields::can_add_property;
use ferroui_designer_core::{MarkupValue, Node, NodeKind};

pub(crate) fn image_source_properties(design: &DesignDocument<'_>, node: &Node) -> Vec<String> {
    if design.rsx.is_some() {
        return design
            .rsx_node(node.id)
            .map_or_else(Vec::new, |(_, rsx)| rsx.editable_image_properties());
    }
    design
        .schema()
        .image_property_names(node.local_name(), node.namespace_uri.as_deref())
        .into_iter()
        .filter(|name| editable_source(design, node, name))
        .map(str::to_owned)
        .collect()
}

fn editable_source(design: &DesignDocument<'_>, node: &Node, name: &str) -> bool {
    // Bindings, owner-qualified assignments, and object values keep their authored form.
    let object_source = node
        .children
        .iter()
        .filter_map(|id| design.xaml.ast().node(*id))
        .any(|child| {
            child.kind == NodeKind::PropertyElement
                && child
                    .local_name()
                    .rsplit_once('.')
                    .is_some_and(|(_, property)| property == name)
        });
    !object_source
        && node
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name == name
                    || attribute
                        .name
                        .rsplit_once('.')
                        .is_some_and(|(_, property)| property == name)
            })
            .all(|attribute| {
                attribute.name == name && matches!(attribute.markup(), Ok(MarkupValue::Literal(_)))
            })
        && (editable_attribute(design, node, name) || can_add_property(design, node, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        projects::Project,
        protocol::{Language, Source},
    };
    use ferroui_designer_language::{PropertyInfo, Schema, TypeInfo};
    use ferroui_designer_rsx::RsxCatalog;
    use std::sync::Arc;

    #[test]
    fn assets_offer_registered_raster_members_and_preserve_other_assignments() {
        let raster = |name: &str| PropertyInfo {
            name: name.into(),
            type_name: "Option<Rc<dyn IImage>>".into(),
            ..Default::default()
        };
        let mut schema = Schema::ferroui();
        schema.types.insert(
            "urn:photos|PhotoCard".into(),
            TypeInfo {
                namespace: "urn:photos".into(),
                name: "PhotoCard".into(),
                base: Some("Image".into()),
                properties: ["HeroImage", "BadgeImage", "HeaderImage"]
                    .map(|name| (name.into(), raster(name)))
                    .into(),
                ..Default::default()
            },
        );
        let project = Project {
            root: Default::default(),
            schema: Arc::new(schema),
            rsx: RsxCatalog::built_in().clone(),
            assets: Vec::new(),
            toolbox: Vec::new(),
            resources: Default::default(),
        };
        let mut input = Source {
            language: Language::Xaml, uri: "file:///Card.xaml".into(), caret: Default::default(), project: None,
            source: r#"<p:PhotoCard xmlns:p="urn:photos" xmlns="https://github.com/ferroui"
                BadgeImage="{Binding Badge}" HeroImage="Assets/Old.png">
                <p:PhotoCard.HeaderImage><Bitmap>Assets/Header.png</Bitmap></p:PhotoCard.HeaderImage>
            </p:PhotoCard>"#.into(),
        };
        let sources = |input: &Source| {
            let design = DesignDocument::parse(input, Some(&project)).unwrap();
            let node = design.xaml.ast().node(design.xaml.ast().root()).unwrap();
            image_source_properties(&design, node)
        };
        assert_eq!(sources(&input), ["HeroImage", "Source"]);
        input.source = input.source.replace(
            "BadgeImage=\"{Binding Badge}\"",
            "BadgeImage=\"Assets/Badge.png\" Image.Source=\"Assets/Inherited.png\"",
        );
        assert_eq!(sources(&input), ["BadgeImage", "HeroImage"]);
    }
}
