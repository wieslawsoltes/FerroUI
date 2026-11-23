use super::*;
use crate::Easing;

fn target(property: &str, type_name: &'static str) -> XamlTransitionTarget {
    XamlTransitionTarget {
        property: property.into(),
        type_name,
        namespaces: Vec::new(),
    }
}

fn edit(document: &mut Document, owner: NodeId, edit: PropertyTransitionCollectionEdit<'_>) {
    let edits = PropertyTransitionList::plan_collection_edit(document, owner, edit).unwrap();
    document.apply("Edit transition collection", edits).unwrap();
}

#[test]
fn adds_native_collections_to_controls_and_style_setters() {
    let source = "<c:Border xmlns:c=\"using:FerroUI.Controls\">\r\n  <c:TextBlock Text='Preserve 🌱'/>\r\n</c:Border>";
    let mut document = Document::new(source).unwrap();
    let owner = document.ast().root();
    let timing = PropertyTransitionTiming {
        duration: Some(0.43),
        easing: Some(Easing::Spring {
            mass: 1.0,
            stiffness: 180.0,
            damping: 18.0,
            initial_velocity: 0.0,
        }),
        ..Default::default()
    };
    edit(
        &mut document,
        owner,
        PropertyTransitionCollectionEdit::Add {
            target: &target("Opacity", "DoubleTransition"),
            timing: &timing,
        },
    );
    let list = PropertyTransitionList::read(&document, owner).unwrap();
    assert_eq!(list.entries[0].duration_seconds, Some(0.43));
    let collection = list.collection.unwrap();
    assert_eq!(
        PropertyTransitionList::read(&document, collection)
            .unwrap()
            .target_owner,
        Some(owner)
    );
    assert!(document.source().contains("<c:Border.Transitions>"));
    assert!(document
        .source()
        .contains("\r\n  <c:TextBlock Text='Preserve 🌱'/>"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);

    let mut style = Document::new(
        r#"<Style xmlns="https://github.com/ferroui" Selector="Border">
        <Style.Setters><Setter Property="Opacity" Value="1"/></Style.Setters>
    </Style>"#,
    )
    .unwrap();
    let owner = style.ast().root();
    edit(
        &mut style,
        owner,
        PropertyTransitionCollectionEdit::Add {
            target: &target("Margin", "ThicknessTransition"),
            timing: &Default::default(),
        },
    );
    let list = PropertyTransitionList::read(&style, owner).unwrap();
    assert_eq!(list.entries[0].property.as_deref(), Some("Margin"));
    assert_eq!(
        PropertyTransitionList::read(&style, list.collection.unwrap())
            .unwrap()
            .target_owner,
        Some(owner)
    );
    assert!(style
        .source()
        .contains("<Setter Property=\"Opacity\" Value=\"1\"/>"));
}

#[test]
fn retargets_the_native_type_and_preserves_the_authored_timing_object() {
    let source = r#"<a:Transitions xmlns:a="using:FerroUI.Animation" xmlns:f="https://github.com/ferroui">
        <a:DoubleTransition Property='Opacity' Duration='0:0:.8'>
            <!-- Easing stays exactly as it was. -->
            <a:DoubleTransition.Easing><f:CubicEaseOut/></a:DoubleTransition.Easing>
        </a:DoubleTransition>
        <a:DoubleTransition Property='Width' Duration='0:0:.2'/>
    </a:Transitions>"#;
    let mut document = Document::new(source).unwrap();
    let owner = document.ast().root();
    let list = PropertyTransitionList::read(&document, owner).unwrap();
    assert_eq!(list.target_owner, None);
    let mut target = target("{x:Static f:Border.BackgroundProperty}", "BrushTransition");
    target.namespaces.push(("x".into(), XAML.into()));
    edit(
        &mut document,
        owner,
        PropertyTransitionCollectionEdit::Retarget {
            transition: list.entries[0].id,
            target: &target,
        },
    );
    let new_source = document.source();
    assert!(new_source.contains(
        "<a:BrushTransition Property='{x:Static f:Border.BackgroundProperty}' Duration='0:0:.8'"
    ));
    assert!(new_source.contains("<!-- Easing stays exactly as it was. -->"));
    assert!(new_source
        .contains("<a:BrushTransition.Easing><f:CubicEaseOut/></a:BrushTransition.Easing>"));
    assert!(new_source.contains("</a:BrushTransition>"));
    assert!(new_source.contains("<a:DoubleTransition Property='Width' Duration='0:0:.2'/>"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn reorders_overrides_and_leaves_bound_collections_intact() {
    let source = r#"<Border xmlns="https://github.com/ferroui">
    <Border.Transitions><Transitions>
        <DoubleTransition Property="Opacity" Duration="0:0:.2"/>
        <!-- Preserve this note between entries. -->
        <DoubleTransition Property="Opacity" Duration="0:0:.7"/>
    </Transitions></Border.Transitions>
    <Border Name="Other" Transitions="{StaticResource Fade}"/>
    </Border>"#;
    let mut document = Document::new(source).unwrap();
    let owner = document.ast().root();
    let before = PropertyTransitionList::read(&document, owner).unwrap();
    let first = before.entries[0].id;
    assert!(PropertyTransitionList::plan_collection_edit(
        &document,
        owner,
        PropertyTransitionCollectionEdit::Move {
            transition: first,
            index: 2
        }
    )
    .is_err());
    let other = document
        .ast()
        .elements()
        .find(|node| node.attribute("Name") == Some("Other"))
        .unwrap()
        .id;
    assert!(!PropertyTransitionList::read(&document, other)
        .unwrap()
        .can_add());
    assert!(PropertyTransitionList::plan_collection_edit(
        &document,
        other,
        PropertyTransitionCollectionEdit::Remove { transition: first }
    )
    .is_err());
    edit(
        &mut document,
        owner,
        PropertyTransitionCollectionEdit::Move {
            transition: first,
            index: 1,
        },
    );
    let after = PropertyTransitionList::read(&document, owner).unwrap();
    assert_eq!(after.entries[0].duration_seconds, Some(0.7));
    assert_eq!(after.entries[1].id, first);
    edit(
        &mut document,
        owner,
        PropertyTransitionCollectionEdit::Remove { transition: first },
    );
    assert_eq!(
        PropertyTransitionList::read(&document, owner)
            .unwrap()
            .entries
            .len(),
        1
    );
    assert!(document
        .source()
        .contains("<!-- Preserve this note between entries. -->"));
    assert!(document
        .source()
        .contains("Transitions=\"{StaticResource Fade}\""));
    document.undo().unwrap();
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}
