use super::*;

fn apply(document: &mut Document, axis: Axis, change: GridTrackChange) {
    let edits = plan_grid_track_change(document, document.ast().root(), axis, change).unwrap();
    document.apply("Edit grid tracks", edits).unwrap();
}

fn value<'a>(document: &'a Document, name: &str, property: &str) -> Option<&'a str> {
    document
        .ast()
        .find_name(name)
        .and_then(|id| document.ast().node(id))
        .and_then(|node| node.attribute(property))
}

#[test]
fn insert_and_remove_update_cells_spans_and_preserve_one_undo() {
    let source = r#"<Grid xmlns="https://github.com/ferroui" RowDefinitions="Auto,*,40">
    <!-- retained -->
    <Button Name="First" Grid.Row="0" />
    <Button Name="Middle" Grid.Row="1" />
    <Button Name="Spanning" Grid.RowSpan="3" />
    <Button Name="Tail" Grid.Row="2" />
    </Grid>"#;
    let mut document = Document::new(source).unwrap();
    let original = document.ast_snapshot();
    apply(
        &mut document,
        Axis::Vertical,
        GridTrackChange::Insert {
            index: 1,
            length: GridLength::Pixels(24.0),
        },
    );
    assert_eq!(value(&document, "First", "Grid.Row"), Some("0"));
    assert_eq!(value(&document, "Middle", "Grid.Row"), Some("2"));
    assert_eq!(value(&document, "Tail", "Grid.Row"), Some("3"));
    assert_eq!(value(&document, "Spanning", "Grid.RowSpan"), Some("4"));
    assert!(document.source().contains("<!-- retained -->"));
    assert_eq!(document.revision(), 1);
    document.undo().unwrap();
    assert_eq!(document.source(), source);
    assert_eq!(original.root(), document.ast().root());
    apply(
        &mut document,
        Axis::Vertical,
        GridTrackChange::Remove { index: 1 },
    );
    assert_eq!(value(&document, "Middle", "Grid.Row"), Some("1"));
    assert_eq!(value(&document, "Tail", "Grid.Row"), Some("1"));
    assert_eq!(value(&document, "Spanning", "Grid.RowSpan"), Some("2"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn explicit_definitions_keep_namespace_constraints_comments_and_children_collection() {
    let source = r#"<p:Grid xmlns:p="https://github.com/ferroui">
    <p:Grid.ColumnDefinitions><!-- widths --><p:ColumnDefinition Width="Auto" MinWidth="16" SharedSizeGroup="A" /><p:ColumnDefinition Width="2*" MaxWidth="300" /></p:Grid.ColumnDefinitions>
    <p:Grid.Children><p:Button Name="Right" p:Grid.Column="1" /><p:Button Name="Span" Grid.ColumnSpan="2" /></p:Grid.Children>
    </p:Grid>"#;
    let mut document = Document::new(source).unwrap();
    apply(
        &mut document,
        Axis::Horizontal,
        GridTrackChange::Set {
            index: 1,
            length: GridLength::Star(3.0),
        },
    );
    assert!(document.source().contains("Width=\"3*\" MaxWidth=\"300\""));
    apply(
        &mut document,
        Axis::Horizontal,
        GridTrackChange::Insert {
            index: 1,
            length: GridLength::Pixels(8.0),
        },
    );
    assert!(
        document
            .source()
            .contains("<p:ColumnDefinition Width=\"8\" />")
    );
    assert!(
        document
            .source()
            .contains("MinWidth=\"16\" SharedSizeGroup=\"A\"")
    );
    assert!(document.source().contains("<!-- widths -->"));
    assert_eq!(value(&document, "Right", "p:Grid.Column"), Some("2"));
    assert_eq!(value(&document, "Span", "Grid.ColumnSpan"), Some("3"));
    assert_eq!(
        grid_track_values(&document, document.ast().root(), Axis::Horizontal).unwrap(),
        ["Auto", "8", "3*"]
    );
    document.undo().unwrap();
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn implicit_tracks_materialize_and_last_track_cannot_be_removed() {
    for source in [
        r#"<Grid><Button Name="Only" /></Grid>"#,
        r#"<Grid><Grid.RowDefinitions><!-- empty --></Grid.RowDefinitions><Button Name="Only" /></Grid>"#,
    ] {
        let mut document = Document::new(source).unwrap();
        let root = document.ast().root();
        assert_eq!(
            grid_track_values(&document, root, Axis::Vertical).unwrap(),
            ["*"]
        );
        assert!(
            plan_grid_track_change(
                &document,
                root,
                Axis::Vertical,
                GridTrackChange::Remove { index: 0 }
            )
            .is_err()
        );
        apply(
            &mut document,
            Axis::Vertical,
            GridTrackChange::Insert {
                index: 0,
                length: GridLength::Auto,
            },
        );
        assert_eq!(
            grid_track_values(&document, root, Axis::Vertical).unwrap(),
            ["Auto", "*"]
        );
        assert_eq!(value(&document, "Only", "Grid.Row"), Some("1"));
        document.undo().unwrap();
        assert_eq!(document.source(), source);
    }
}

#[test]
fn expressions_invalid_indices_and_object_cells_fail_before_any_mutation() {
    for source in [
        r#"<Grid RowDefinitions="{Binding Rows}"><Button /></Grid>"#,
        r#"<Grid RowDefinitions="*,*"><Button Grid.Row="{Binding Row}" /></Grid>"#,
        r#"<Grid RowDefinitions="*,*"><Button><Grid.Row><Binding Path="Row" /></Grid.Row></Button></Grid>"#,
        r#"<Grid RowDefinitions="*,*"><Button Grid.RowSpan="0" /></Grid>"#,
    ] {
        let document = Document::new(source).unwrap();
        assert!(
            plan_grid_track_change(
                &document,
                document.ast().root(),
                Axis::Vertical,
                GridTrackChange::Insert {
                    index: 1,
                    length: GridLength::Star(1.0)
                }
            )
            .is_err(),
            "{source}"
        );
        assert_eq!(document.source(), source);
        assert!(!document.can_undo());
    }
    let document = Document::new("<Grid RowDefinitions=\"*,*\" />").unwrap();
    for change in [
        GridTrackChange::Remove { index: 2 },
        GridTrackChange::Set {
            index: 2,
            length: GridLength::Auto,
        },
        GridTrackChange::Insert {
            index: 3,
            length: GridLength::Auto,
        },
        GridTrackChange::Insert {
            index: 1,
            length: GridLength::Pixels(f64::INFINITY),
        },
    ] {
        assert!(
            plan_grid_track_change(&document, document.ast().root(), Axis::Vertical, change)
                .is_err()
        );
    }
}

#[test]
fn unrelated_bound_definition_survives_insertion_and_noop_does_not_write() {
    let source = r#"<Grid><Grid.RowDefinitions><RowDefinition Height="{Binding Height}" /><RowDefinition Height="*" /></Grid.RowDefinitions></Grid>"#;
    let mut document = Document::new(source).unwrap();
    let root = document.ast().root();
    assert!(
        plan_grid_track_change(
            &document,
            root,
            Axis::Vertical,
            GridTrackChange::Set {
                index: 0,
                length: GridLength::Auto
            }
        )
        .is_err()
    );
    assert!(
        plan_grid_track_change(
            &document,
            root,
            Axis::Vertical,
            GridTrackChange::Set {
                index: 1,
                length: GridLength::Star(1.0)
            }
        )
        .unwrap()
        .is_empty()
    );
    apply(
        &mut document,
        Axis::Vertical,
        GridTrackChange::Insert {
            index: 1,
            length: GridLength::Auto,
        },
    );
    assert!(document.source().contains("Height=\"{Binding Height}\""));
    assert_eq!(
        grid_track_values(&document, root, Axis::Vertical).unwrap(),
        ["{Binding Height}", "Auto", "*"]
    );
}
