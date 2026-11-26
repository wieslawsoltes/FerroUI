use super::*;
use crate::ResolvedLayout;
use ferroui_designer_core::Document;
use std::collections::BTreeMap;

#[test]
fn wrapping_explicit_canvas_children_preserves_namespaces_text_and_identity() {
    let source = "<ui:Canvas xmlns=\"using:Review\" xmlns:ui=\"https://github.com/ferroui\">\r\n\
                  \t<ui:Canvas.Children>\r\n\
                  \t\t<ui:Border Name='a' ui:Canvas.Left='10' Width='20'>\r\n\
                  \t\t\t<ui:TextBlock xml:space='preserve'>\r\n keep this text  </ui:TextBlock>\r\n\
                  \t\t</ui:Border>\r\n\
                  \t\t<!-- inside 🍃 -->\r\n\
                  \t\t<Chip Name='b' ui:Canvas.Right='8' Width='30'/>\r\n\
                  \t</ui:Canvas.Children>\r\n\
                  </ui:Canvas>";
    let mut document = Document::new(source).unwrap();
    let root = document.ast().root();
    let a = document.ast().find_name("a").unwrap();
    let b = document.ast().find_name("b").unwrap();
    let content = document.ast().node(a).unwrap().parent.unwrap();
    let slots = BTreeMap::from([
        (
            a,
            ResolvedLayout {
                slot: Some(Rect::new(10.0, 20.0, 20.0, 40.0)),
                ..Default::default()
            },
        ),
        (
            b,
            ResolvedLayout {
                slot: Some(Rect::new(70.0, 76.0, 38.0, 40.0)),
                ..Default::default()
            },
        ),
    ]);
    let plan = plan_selection_wrap(
        &LayoutContext::new(&document, &BTreeMap::new()).with_resolved_layout(&slots),
        root,
        &[b, a],
        WrapKind::Canvas,
    )
    .unwrap();
    assert_eq!(plan.children, [a, b]);
    let patches = plan.xaml_patches(&document).unwrap();
    document.apply_text_patches(0, "Wrap", patches).unwrap();
    let wrapper = document.ast().node(a).unwrap().parent.unwrap();
    assert_eq!(document.ast().element_children(content), [wrapper]);
    assert_eq!(document.ast().element_children(wrapper), [a, b]);
    assert_eq!(document.ast().node(wrapper).unwrap().name, "ui:Canvas");
    assert_eq!(
        document.ast().node(a).unwrap().attribute("ui:Canvas.Left"),
        Some("0")
    );
    assert_eq!(
        document.ast().node(b).unwrap().attribute("ui:Canvas.Left"),
        Some("60")
    );
    assert!(document
        .ast()
        .node(b)
        .unwrap()
        .attribute("ui:Canvas.Right")
        .is_none());
    assert_eq!(
        document.ast().node(b).unwrap().namespace_uri.as_deref(),
        Some("using:Review")
    );
    assert!(document
        .source()
        .contains("\r\n\t\t\t<!-- inside 🍃 -->\r\n"));
    assert!(document
        .source()
        .contains("xml:space='preserve'>\r\n keep this text  </ui:TextBlock>"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
    assert!(!document.can_undo());
}

#[test]
fn wrapping_requires_one_source_slot_and_protects_grid_and_binding_anchors() {
    let document = Document::new(
        r#"<Grid>
      <Border Name="a" Grid.Row="1" Grid.Column="1" Grid.RowSpan="2" />
      <Border Name="b" Grid.Row="1" Grid.Column="1" Grid.RowSpan="2" />
      <Border Name="c" Grid.Row="0" />
      <Border Name="d" Grid.Row="{Binding Slot}" />
    </Grid>"#,
    )
    .unwrap();
    let [a, b, c, d] = ["a", "b", "c", "d"].map(|name| document.ast().find_name(name).unwrap());
    let bounds = BTreeMap::new();
    let context = LayoutContext::new(&document, &bounds);
    let parent = document.ast().root();
    let plan = plan_selection_wrap(&context, parent, &[a, b], WrapKind::HorizontalStack).unwrap();
    assert!(plan
        .wrapper_properties
        .contains(&("Grid.RowSpan".into(), "2".into())));
    assert_eq!(plan.child_edits().len(), 6);
    for (nodes, kind, reason) in [
        (&[a, b][..], WrapKind::Border, "one child"),
        (&[a, c][..], WrapKind::Grid, "consecutive"),
        (&[b, c][..], WrapKind::Grid, "share the same Grid.Row"),
        (&[d][..], WrapKind::VerticalStack, "expression"),
    ] {
        assert!(plan_selection_wrap(&context, parent, nodes, kind)
            .unwrap_err()
            .to_string()
            .contains(reason));
    }
}
