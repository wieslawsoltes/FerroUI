use super::{Project, NS};
use crate::{apply_text_edits, FileMoveDraft, LanguageService};
use serde_json::json;

fn string(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

fn stage_set(uri: &str) -> String {
    format!(
        r#"{{"version":3, "active":0, "stages":[{{"name":"Review 🪴","source_uri":{},"environment":{{"width":800,"height":520,"theme":"light","flow_direction":"inherit"}},"target":{{"kind":"document"}},"layout":"fixed","design_time_data":true,"states":[],"pinned":true,"auto_update":true,"mode":"select"}}]}}"#,
        string(uri)
    )
}

#[test]
fn reviews_saved_stage_uris_and_scenario_paths_with_exact_json_patches() {
    let project = Project::new();
    let label = "Overview & 🪴.xaml";
    let destination = "Themes/Archive/Sales % 🎯.xaml";
    let from = project.uri(label);
    let to = project.uri(destination);
    project.write(label, &format!("<Border {NS}/>"));
    project.write("Alias.xaml", &format!("<Border {NS}/>"));
    let stage = stage_set(&from).replace(", ", ",\r\n  ");
    project.write("Customer.ferrostages", &stage);
    project.write("Review.json", &format!("\u{feff}{stage}\r\n"));
    let alt = "memory://demo/stable-key";
    let scenario_base = serde_json::to_string_pretty(&json!({
        "version":1,"name":format!("Keep the label {label}"),
        "document":format!("./{label}"),
        "prototype":{"stageSet":"Customer.ferrostages","flow":"Review","sources":{
            (&from):{"path":"Alias.xaml"},
            (alt):{"path":label, "caret":{"line":1,"character":0}}
        }},
        "steps":[{"action":"expectStage","name":"Review 🪴"}]
    }))
    .unwrap()
    .replace('\n', "\r\n");
    let scenario = scenario_base.replace(
        &string(&from),
        &string(&from).replacen("file:", "\\u0066ile:", 1),
    );
    project.write("Customer.ferroui-scenario.json", &scenario);
    let unrelated = format!(
        r#"{{"source_uri":{},"document":{},"label":"A different kind of project JSON"}}"#,
        string(&from),
        string(label)
    );
    project.write("unrelated.json", &unrelated);
    let mut service = LanguageService::default();
    project.index(&mut service);

    let draft = FileMoveDraft {
        uri: project.uri("Customer.ferroui-scenario.json"),
        version: 18,
        source: scenario.replace("Keep the label", "Preserve this unsaved label"),
    };
    let first = service
        .plan_file_move_with_drafts(&from, &to, &[draft])
        .unwrap();
    assert_eq!(first.documents.len(), 4);
    let document = |file: &str| {
        first
            .documents
            .iter()
            .find(|doc| doc.edit.uri == project.uri(file))
            .unwrap()
    };
    assert_eq!(
        document("Customer.ferrostages").after,
        stage_set(&to).replace(", ", ",\r\n  ")
    );
    let alias = document("Review.json");
    assert_eq!(
        alias.after,
        format!("\u{feff}{}\r\n", stage_set(&to).replace(", ", ",\r\n  "))
    );
    let case = document("Customer.ferroui-scenario.json");
    assert_eq!(case.edit.version, 18);
    assert!(case.after.contains("Preserve this unsaved label"));
    assert!(case
        .after
        .contains(&format!(r#""document": "./{destination}""#)));
    assert!(case.after.contains(&format!(r#""path": "{destination}""#)));
    assert!(case.after.contains(&format!("{}: {{\r\n", string(&to))));
    assert!(case.after.contains(&format!("{}: {{\r\n", string(alt))));
    assert!(case.after.contains(r#""path": "Alias.xaml""#));
    assert_eq!(case.edit.edits.len(), 3);
    for doc in &first.documents {
        assert_eq!(
            apply_text_edits(&doc.before, &doc.edit.edits).unwrap(),
            doc.after
        );
        assert!(!doc.after.replace("\r\n", "").contains('\n'));
    }
    assert_ne!(
        service.plan_file_move(&from, &to).unwrap().review,
        first.review
    );
    assert!(project.0.join(label).is_file());
    assert!(!project.0.join(destination).exists());
}

#[test]
fn rejects_remap_collisions_and_incomplete_saved_design_drafts() {
    let project = Project::new();
    let label = "Overview.xaml";
    let from = project.uri(label);
    let to = project.uri("Themes/Archive/New Overview.xaml");
    project.write(label, &format!("<Border {NS}/>"));
    let board = stage_set(&from);
    project.write("review.json", &board);
    let scenario = json!({
        "version":1, "name":"Review","document":"Fallback.xaml",
        "prototype":{"stageSet":"review.json","flow":"Review","sources":{
            (&from):{"path":"Fallback.xaml"}
        }},
        "steps":[{"action":"expectStage", "name":"Review"}]
    })
    .to_string();
    for file in ["Scenario.ferroui-scenario.json", "Scenario.json"] {
        project.write(file, &scenario);
    }
    project.write("Settings.json", r#"{"unrelated":true}"#);
    let mut service = LanguageService::default();
    project.index(&mut service);
    for file in [
        "review.json",
        "Scenario.ferroui-scenario.json",
        "Scenario.json",
    ] {
        let broken = FileMoveDraft {
            uri: project.uri(file),
            version: 3,
            source: "{\"version\": 1,\n".into(),
        };
        let error = service
            .plan_file_move_with_drafts(&from, &to, &[broken])
            .unwrap_err();
        assert!(
            error.0.contains("Repair") && error.0.contains(file),
            "{error}"
        );
    }
    let unrelated = FileMoveDraft {
        uri: project.uri("Settings.json"),
        version: 4,
        source: format!(r#"{{"note":"{label}","#),
    };
    assert!(service
        .plan_file_move_with_drafts(&from, &to, &[unrelated])
        .is_ok());

    project.write(
        "Conflict.ferroui-scenario.json",
        &json!({
            "version":1, "name":"Conflicting remaps","document":label,
            "prototype":{"stageSet":"review.json","flow":"Review","sources":{
                (&from):{"path":label},
                (&to):{"path":"Another.xaml"}
            }},
            "steps":[{"action":"expectStage", "name":"Review"}]
        })
        .to_string(),
    );
    assert!(service
        .plan_file_move(&from, &to)
        .unwrap_err()
        .0
        .contains("duplicate prototype source remap"));
    assert_eq!(
        std::fs::read_to_string(project.0.join("review.json")).unwrap(),
        board
    );
    assert!(project.0.join(label).is_file());
}
