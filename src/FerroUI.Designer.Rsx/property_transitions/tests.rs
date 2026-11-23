use crate::{project_rust, RsxPropertyTransitionEdit as Edit};
use ferroui_designer_animation::{Easing, PropertyTransitionList, PropertyTransitionTiming};
use ferroui_designer_core::{Document, TextPatch};

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

#[test]
fn edits_only_the_accepted_rust_collection_and_preserves_comments_and_suffixes() {
    let original = r#"use ferroui_view::{ms, transition as fade, transition_eased, transitions as group};
use ferroui_base::{Visual, layout::Layoutable, animation::easings::Easing};
fn another() -> Element { rsx! { TextBlock { text: "Unaffected" } } }
fn account() -> Element {
    rsx! {
        Border { name: "Account", transitions: group([
            fade(Visual::opacity_property(), ms(/* hold 🌱 */ 260u64)),
            transition_eased(Layoutable::margin_property(), std::time::Duration::from_secs(/* duration */ 1u64),
                Easing::parse(r"0.25,0.3,0.75,1.2").unwrap())
        ]), TextBlock { text: computed_title() } }
    }
}"#.replace('\n', "\r\n");
    let project = |source: &str| source.find("name:").unwrap();
    let projection = project_rust(&original, project(&original)).unwrap();
    let list = projection.property_transitions(0).unwrap();
    let timing = PropertyTransitionTiming {
        duration: Some(0.32),
        easing: Some(Easing::parse("0.2,0.4,0.7,1.25").unwrap()),
        ..Default::default()
    };
    let patches = projection
        .edit_property_transitions(
            0,
            Edit::Timing {
                source: list.entries[1].source,
                timing: &timing,
            },
        )
        .unwrap();
    let draft = TextPatch::apply_all(&original, patches).unwrap();
    assert!(draft.contains("::ferroui_view::ms(/* duration */ 320u64),"));
    assert!(draft.contains(r#"Easing::parse(r"0.2,0.4,0.7,1.25").unwrap()"#));
    assert!(draft.contains("/* hold 🌱 */ 260u64"));
    assert_eq!(
        draft.matches("\r\n").count(),
        original.matches("\r\n").count()
    );

    let projection = project_rust(&draft, project(&draft)).unwrap();
    let first = projection.property_transitions(0).unwrap().entries[0].source;
    let patches = projection
        .edit_property_transitions(
            0,
            Edit::Retarget {
                source: first,
                getter: "ferroui_controls::Border::background_property",
            },
        )
        .unwrap();
    let draft = TextPatch::apply_all(&draft, patches).unwrap();
    assert!(draft.contains(
        "fade(::ferroui_controls::Border::background_property(), ms(/* hold 🌱 */ 260u64))"
    ));
    let projection = project_rust(&draft, project(&draft)).unwrap();
    let added_easing = PropertyTransitionTiming {
        easing: Some(Easing::Named("CubicEaseOut".into())),
        ..Default::default()
    };
    let patches = projection
        .edit_property_transitions(
            0,
            Edit::Add {
                getter: "ferroui_controls::Border::corner_radius_property",
                timing: &added_easing,
            },
        )
        .unwrap();
    let draft = TextPatch::apply_all(&draft, patches).unwrap();
    let projection = project_rust(&draft, project(&draft)).unwrap();
    let list = projection.property_transitions(0).unwrap();
    let patches = projection
        .edit_property_transitions(
            0,
            Edit::Move {
                source: list.entries[2].source,
                index: 0,
            },
        )
        .unwrap();
    let draft = TextPatch::apply_all(&draft, patches).unwrap();
    let projection = project_rust(&draft, project(&draft)).unwrap();
    let list = projection.property_transitions(0).unwrap();
    assert_eq!(list.entries[0].type_name, "CornerRadiusTransition");
    let patches = projection
        .edit_property_transitions(
            0,
            Edit::Remove {
                source: list.entries[1].source,
            },
        )
        .unwrap();
    let draft = TextPatch::apply_all(&draft, patches).unwrap();
    let projection = project_rust(&draft, project(&draft)).unwrap();
    assert_eq!(projection.property_transitions(0).unwrap().entries.len(), 2);
    assert!(draft.contains("/* hold 🌱 */"));
    assert!(draft.contains("TextBlock { text: computed_title() }"));
    assert!(
        draft.contains(r#"fn another() -> Element { rsx! { TextBlock { text: "Unaffected" } } }"#)
    );
}

#[test]
fn starts_one_collection_on_a_control_with_a_transition_setter() {
    let source = r#"fn view() -> Element { rsx! { Border { TextBlock { text: "Card" } } } }"#;
    let projection = project_rust(source, 0).unwrap();
    let choice = "ferroui_base::Visual::opacity_property";
    let timing = PropertyTransitionTiming {
        duration: Some(0.45),
        ..Default::default()
    };
    let patches = projection
        .edit_property_transitions(
            0,
            Edit::Add {
                getter: choice,
                timing: &timing,
            },
        )
        .unwrap();
    let draft = TextPatch::apply_all(source, patches).unwrap();
    let projection = project_rust(&draft, 0).unwrap();
    let list = projection.property_transitions(0).unwrap();
    assert_eq!(list.entries[0].duration, 0.45);
    assert!(draft.contains("TextBlock { text: \"Card\" }"));
    assert!(projection
        .edit_property_transitions(
            1,
            Edit::Remove {
                source: list.entries[0].source
            }
        )
        .is_err());
}

#[test]
fn edits_spring_parameters_without_rewriting_the_rust_constructor() {
    let source = r#"fn view() -> Element {
        rsx! { Border { transitions: ferroui_view::transitions([
            ferroui_view::transition_eased(ferroui_base::Visual::opacity_property(), ferroui_view::ms(400),
                ferroui_base::animation::easings::SpringEasing::with_values(
                    1.0, /* stiffness */ 170.0f64, 18.0, -/* velocity */ 2.0,
                )),
        ]) } }
    }"#;
    let projection = project_rust(source, 0).unwrap();
    let entry = &projection.property_transitions(0).unwrap().entries[0];
    assert!(entry.markup.contains("SpringEasing"));
    let next = PropertyTransitionTiming {
        easing: Some(Easing::Spring {
            mass: 1.0,
            stiffness: 190.0,
            damping: 18.0,
            initial_velocity: 1.5,
        }),
        ..Default::default()
    };
    let patches = projection
        .edit_property_transitions(
            0,
            Edit::Timing {
                source: entry.source,
                timing: &next,
            },
        )
        .unwrap();
    let updated = TextPatch::apply_all(source, patches).unwrap();
    assert_eq!(
        updated,
        source
            .replace("170.0f64", "190.0f64")
            .replace("-/* velocity */ 2.0", "/* velocity */ 1.5")
    );
}
