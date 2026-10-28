use super::*;
use crate::{PreviewEnvironment, PreviewStageLayout};

fn fixture() -> (Document, PreviewStageConfiguration) {
    let document =
        Document::new(r#"<Grid xmlns="https://github.com/ferroui" Name="Root"/>"#).unwrap();
    let config = PreviewStageConfiguration::new(
        "Screen",
        "memory://matrix/Screen.xaml",
        PreviewEnvironment::default(),
    );
    (document, config)
}

#[test]
fn product_is_deterministic_and_preserves_untouched_configuration() {
    let (document, mut base) = fixture();
    base.pinned = true;
    base.auto_update = false;
    base.layout = PreviewStageLayout::FixedWidth;
    let matrix = PreviewStageMatrix {
        sizes: vec![
            PreviewStageMatrixSize {
                name: "Phone".into(),
                width: 390.0,
                height: 844.0,
            },
            PreviewStageMatrixSize {
                name: "Desktop".into(),
                width: 1280.0,
                height: 800.0,
            },
        ],
        themes: vec![PreviewTheme::Light, PreviewTheme::Dark],
        directions: vec![
            PreviewFlowDirection::LeftToRight,
            PreviewFlowDirection::RightToLeft,
        ],
        design_time_data: vec![true, false],
        ..Default::default()
    };
    assert_eq!(matrix.count().unwrap(), 16);
    let product = matrix.expand(&base, &document, 16).unwrap();
    assert_eq!(product.len(), 16);
    assert_eq!(
        product[0].name,
        "Screen · Phone · Light · LTR · Design data"
    );
    assert_eq!(
        product[15].name,
        "Screen · Desktop · Dark · RTL · Runtime data"
    );
    for config in product {
        assert!(config.pinned);
        assert!(!config.auto_update);
        assert_eq!(config.layout, base.layout);
        assert_eq!(config.target, base.target);
    }
    assert_eq!(document.revision(), 0);
    assert!(matrix
        .expand(&base, &document, 15)
        .unwrap_err()
        .contains("16 stages"));
}

#[test]
fn axes_reject_duplicate_invalid_and_excessive_values_before_expansion() {
    let (document, base) = fixture();
    let mut matrix = PreviewStageMatrix {
        themes: vec![PreviewTheme::Dark, PreviewTheme::Dark],
        ..Default::default()
    };
    assert!(matrix.count().is_err());
    matrix.themes.clear();
    matrix.sizes.push(PreviewStageMatrixSize {
        name: "Invalid".into(),
        width: f64::INFINITY,
        height: 40.0,
    });
    assert!(matrix.count().is_err());
    matrix.sizes[0].width = 200.0;
    matrix.sizes.push(PreviewStageMatrixSize {
        name: "Duplicate dimensions".into(),
        width: 200.0,
        height: 40.0,
    });
    assert!(matrix.count().is_err());
    assert!(PreviewStageMatrix::default()
        .expand(&base, &document, 0)
        .is_err());
    assert_eq!(
        PreviewStageMatrix::default()
            .expand(&base, &document, 1)
            .unwrap()[0]
            .name,
        "Screen · Copy"
    );
    assert!(serde_json::from_str::<PreviewStageMatrix>(r#"{"unknown":[]}"#).is_err());
}

#[test]
fn long_unicode_names_remain_distinct_bounded_and_valid() {
    let (document, mut base) = fixture();
    base.name = "视图".repeat(20);
    let matrix = PreviewStageMatrix {
        themes: vec![PreviewTheme::Light, PreviewTheme::Dark],
        directions: vec![
            PreviewFlowDirection::LeftToRight,
            PreviewFlowDirection::RightToLeft,
        ],
        ..Default::default()
    };
    let product = matrix.expand(&base, &document, 4).unwrap();
    let names = product
        .iter()
        .map(|config| {
            config.validate().unwrap();
            assert!(config.name.len() <= 128);
            config.name.as_str()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), 4);
}

#[test]
fn authored_state_axes_validate_owner_and_names_without_constructing_controls() {
    let (mut document, base) = fixture();
    let owner = document.ast().root();
    let group = ferroui_designer_animation::VisualStateGroup {
        owner,
        name: "Screen".into(),
        transitions: vec![],
        states: ["Loading", "Content"]
            .into_iter()
            .map(|name| ferroui_designer_animation::VisualState {
                name: name.into(),
                setters: vec![],
            })
            .collect(),
    };
    document
        .apply("Fixture states", group.plan(&document).unwrap())
        .unwrap();
    let axis = PreviewStageStateAxis {
        owner: PreviewSourceAnchor::capture(&document, document.ast().root()).unwrap(),
        group: "Screen".into(),
        states: vec!["Loading".into(), "Content".into()],
    };
    let mut matrix = PreviewStageMatrix {
        themes: vec![PreviewTheme::Light, PreviewTheme::Dark],
        state_groups: vec![axis],
        ..Default::default()
    };
    let product = matrix.expand(&base, &document, 4).unwrap();
    assert_eq!(product.len(), 4);
    assert_eq!(product[0].states[0].state, "Loading");
    assert_eq!(product[3].states[0].state, "Content");
    matrix.state_groups[0].states.push("Missing".into());
    assert!(matrix
        .expand(&base, &document, 16)
        .unwrap_err()
        .contains("Missing"));
}

#[test]
fn matrix_rejects_ineligible_definitions_and_stale_retained_states_before_creation() {
    let (document, mut base) = fixture();
    let anchor = PreviewSourceAnchor::capture(&document, document.ast().root()).unwrap();
    let matrix = PreviewStageMatrix {
        themes: vec![PreviewTheme::Light, PreviewTheme::Dark],
        ..Default::default()
    };
    base.target = PreviewStageTarget::Definition {
        anchor: anchor.clone(),
        options: Default::default(),
    };
    assert!(matrix
        .expand(&base, &document, 2)
        .unwrap_err()
        .contains("authored component"));
    base.target = PreviewStageTarget::Document;
    base.states.push(PreviewStageState {
        owner: anchor,
        group: "RemovedGroup".into(),
        state: "OldState".into(),
    });
    assert!(matrix
        .expand(&base, &document, 2)
        .unwrap_err()
        .contains("RemovedGroup"));
}
