use crate::project_rust;
use ferroui_designer_animation::{Easing, PropertyTransitionList};
use ferroui_designer_core::Document;

#[test]
fn projects_the_selected_inline_rust_transition_collection() {
    let source = r#"
use ferroui_view::{ms, transition as motion, transition_eased, transitions as group};
use ferroui_base::{Visual, layout::Layoutable, animation::easings::Easing};

fn another() -> Element {
    rsx! { TextBlock { text: "Other invocation" } }
}
fn card() -> Element {
    rsx! {
        Border { name: "Motion", transitions: group([
            motion(Visual::opacity_property(), ms(/* hold 🌱 */ 260u64)),
            transition_eased(Layoutable::margin_property(), std::time::Duration::from_secs(1),
                Easing::parse(r"0.25,0.3,0.75,1.2").unwrap()),
        ]),
            TextBlock { text: "Account activity" }
        }
    }
}"#;
    let projection = project_rust(source, source.find("name:").unwrap()).unwrap();
    assert_eq!(projection.omitted_count, 0, "{:?}", projection.omissions);
    assert_eq!(projection.nodes.len(), 2);
    let transitions = projection.property_transitions(0).unwrap();
    assert_eq!(transitions.entries[0].property, "Opacity");
    assert_eq!(transitions.entries[0].duration, 0.26);
    assert!(source[transitions.entries[0].source.range()].contains("/* hold 🌱 */ 260u64"));
    assert_eq!(transitions.entries[1].type_name, "ThicknessTransition");
    assert_eq!(transitions.entries[1].duration, 1.0);
    assert!(matches!(
        transitions.entries[1].easing,
        Some(Easing::CubicBezier { .. })
    ));
    let xml = Document::new(projection.xaml()).unwrap();
    let collection = PropertyTransitionList::read(&xml, xml.ast().root()).unwrap();
    assert_eq!(collection.entries.len(), 2);
    assert_eq!(
        collection.entries[1].property.as_deref(),
        Some("{x:Static Layoutable.MarginProperty}")
    );
    assert!(!projection.can_replace_structure());
    assert!(project_rust(source, 0)
        .unwrap()
        .property_transitions(0)
        .is_none());
}

#[test]
fn leaves_computed_and_incompatible_rust_transition_sources_opaque() {
    let source = r#"
fn view() -> Element {
    rsx! {
        StackPanel {
            Border { transitions: fade_from_settings(), TextBlock { text: "Computed" } }
            Border { transitions: ferroui_view::transitions([
                ferroui_view::transition(ferroui_controls::TextBlock::font_size_property(), ferroui_view::ms(100)),
            ]), TextBlock { text: "Wrong owner" } }
            Border { transitions: ferroui_view::transitions([
                ferroui_view::transition(ferroui_base::Visual::opacity_property(), ferroui_view::ms(100)),
                computed_transition()
            ]), TextBlock { text: "Mixed" } }
        }
    }
}"#;
    let projection = project_rust(source, 0).unwrap();
    assert_eq!(projection.omitted_count, 3);
    assert!((0..projection.nodes.len()).all(|node| projection.property_transitions(node).is_none()));
    assert!(!projection.xaml().contains("<Transitions>"));
    assert!(projection.xaml().contains("Wrong owner"));
    assert!(!projection.can_replace_structure());
    assert!(projection.edit_property(1, "Transitions", None).is_err());
}
