use super::*;
use crate::Easing;

#[test]
fn discovers_control_style_theme_and_resource_collections() {
    let document = Document::new(r#"<UserControl xmlns="https://github.com/ferroui"
      xmlns:s="using:FerroUI.Styling" xmlns:a="using:FerroUI.Animation"
      xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:foreign="urn:app">
        <UserControl.Resources><ResourceDictionary>
          <a:Transitions x:Key="Shared"><a:ThicknessTransition Property="Margin"/></a:Transitions>
          <s:ControlTheme x:Key="Card" TargetType="Border">
            <s:Setter Property="Transitions"><s:Setter.Value><a:Transitions>
              <a:BrushTransition Property="Background" Duration="0:0:.25"/>
            </a:Transitions></s:Setter.Value></s:Setter>
          </s:ControlTheme>
        </ResourceDictionary></UserControl.Resources>
        <UserControl.Styles><s:Style Selector="Border.card">
          <s:Style.Setters><s:Setter Property="Transitions" Value="{StaticResource Shared}"/></s:Style.Setters>
        </s:Style></UserControl.Styles>
        <Border Name="Card">
          <a:Animatable.Transitions><a:Transitions>
            <a:DoubleTransition Property="Opacity" Easing="{StaticResource Ease}"/>
            <foreign:DoubleTransition Property="Opacity"/>
          </a:Transitions></a:Animatable.Transitions>
        </Border>
    </UserControl>"#).unwrap();
    let control = PropertyTransitionList::read(&document, by_name(&document, "Border")).unwrap();
    assert_eq!(control.entries.len(), 2);
    assert_eq!(control.entries[0].property.as_deref(), Some("Opacity"));
    assert_eq!(control.entries[0].editable_timing, ["Duration", "Delay"]);
    assert!(control.entries[1].editable_timing.is_empty());
    let theme =
        PropertyTransitionList::read(&document, by_name(&document, "ControlTheme")).unwrap();
    assert_eq!(theme.entries[0].duration.as_deref(), Some("0:0:.25"));
    let style = PropertyTransitionList::read(&document, by_name(&document, "Style")).unwrap();
    assert!(style.collection.is_none());
    assert_eq!(
        &document.source()[style.source.unwrap().range()],
        "{StaticResource Shared}"
    );
    let resource =
        PropertyTransitionList::read(&document, by_name(&document, "Transitions")).unwrap();
    assert_eq!(resource.entries[0].type_name, "ThicknessTransition");
}

#[test]
fn patches_a_single_collection_and_preserves_timing_spelling_and_comments() {
    let source = "<Border xmlns=\"https://github.com/ferroui\">\r\n  <Border.Transitions>\r\n    <Transitions>\r\n      <!-- keep this note -->\r\n      <DoubleTransition Property='Opacity' Duration='0:0:.25' Easing='{StaticResource Ease}'/>\r\n    </Transitions>\r\n  </Border.Transitions>\r\n</Border>";
    let mut document = Document::new(source).unwrap();
    let owner = document.ast().root();
    let id = by_name(&document, "DoubleTransition");
    let changes = PropertyTransitionTiming {
        duration: Some(0.25),
        delay: Some(0.05),
        ..Default::default()
    };
    let edits = PropertyTransitionList::plan_timing(&document, owner, id, &changes).unwrap();
    assert_eq!(edits.len(), 1);
    document.apply("Edit transition timing", edits).unwrap();
    assert!(document
        .source()
        .contains("Duration='0:0:.25' Easing='{StaticResource Ease}'"));
    assert!(document.source().contains("Delay='0:0:0.0500000'"));
    assert!(document.source().contains("<!-- keep this note -->\r\n"));
    let invalid_easing = PropertyTransitionTiming {
        easing: Some(Easing::Linear),
        ..Default::default()
    };
    assert!(PropertyTransitionList::plan_timing(&document, owner, id, &invalid_easing).is_err());
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn checks_group_timing_scope_and_writes_native_spring_in_prefixed_xaml() {
    let mut document = Document::new(
        r#"<a:Transitions xmlns:a="using:FerroUI.Animation">
      <a:DoubleTransition Easing="LinearEasing" Property="Opacity"/>
      <a:DoubleTransition Property="Width"/>
    </a:Transitions>"#,
    )
    .unwrap();
    let owner = document.ast().root();
    let id = by_name(&document, "DoubleTransition");
    let update = PropertyTransitionTiming {
        duration: Some(0.6),
        delay: Some(0.2),
        easing: Some(Easing::Spring {
            mass: 1.0,
            stiffness: 190.0,
            damping: 18.0,
            initial_velocity: 0.0,
        }),
    };
    assert!(PropertyTransitionList::plan_timing(&document, id, id, &update).is_err());
    let overflow = PropertyTransitionTiming {
        duration: Some(500_000_000_000.0),
        delay: Some(500_000_000_000.0),
        easing: None,
    };
    assert!(PropertyTransitionList::plan_timing(&document, owner, id, &overflow).is_err());
    let edits = PropertyTransitionList::plan_timing(&document, owner, id, &update).unwrap();
    document.apply("Edit transition timing", edits).unwrap();
    let result = PropertyTransitionList::read(&document, owner).unwrap();
    assert_eq!(result.entries[0].duration.as_deref(), Some("0:0:0.6000000"));
    assert_eq!(result.entries[0].editable_timing, ["Duration", "Delay"]);
    assert!(result.entries[1].duration.is_none());
    let spring = document
        .ast()
        .nodes()
        .find(|node| node.local_name() == "SpringEasing")
        .unwrap();
    assert_eq!(spring.namespace_uri.as_deref(), Some(NATIVE_XMLNS));
}

fn by_name(document: &Document, name: &str) -> NodeId {
    document
        .ast()
        .elements()
        .find(|node| node.local_name() == name)
        .unwrap()
        .id
}
