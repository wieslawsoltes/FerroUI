//! Tests of the export plan (#857): what a plan registers, the errors of a plan, the loader
//! branches and the report.

use super::*;

fn document(uri: &str, build: Option<&str>, populate: Option<&str>, public: bool) -> DocumentModel {
    DocumentModel {
        uri: uri.to_string(),
        root_type: "FerroUI.Controls.UserControl".to_string(),
        class_rust_path: None,
        build_path: build.map(str::to_string),
        populate_path: populate.map(str::to_string),
        public,
    }
}

fn documents() -> Vec<DocumentModel> {
    vec![
        document("ferres://app/Views/Settings.xaml", Some("::app::compiled_xaml::build_views_settings_xaml"), None, true),
        document("ferres://app/Views/About.xaml", Some("::app::compiled_xaml::build_views_about_xaml"), None, true),
        document("ferres://app/Styles/Internal.xaml", Some("::app::compiled_xaml::build_styles_internal_xaml"), None, false),
        document("ferres://app/MainWindow.xaml", None, Some("::app::compiled_main_window::populate"), true),
    ]
}

fn assets() -> Vec<(String, PathBuf)> {
    vec![
        ("/Assets/logo.png".to_string(), PathBuf::from("/crate/Assets/logo.png")),
        ("/Assets/unused.png".to_string(), PathBuf::from("/crate/Assets/unused.png")),
    ]
}

fn exported() -> Vec<String> {
    vec!["compiled_xaml".to_string(), "compiled_main_window".to_string()]
}

#[test]
fn without_a_plan_everything_is_registered() {
    assert!(!ExportPlan::all().is_closed());
    assert_eq!(ExportPlan::default(), ExportPlan::all());
    let applied = apply(&ExportPlan::all(), "app", &documents(), &exported(), &assets()).expect("no plan");
    assert_eq!(applied.documents, None);
    assert_eq!(applied.assets, assets());
    // Adding to a plan of everything changes nothing.
    assert_eq!(ExportPlan::all().document("ferres://app/Views/About.xaml").asset("/Assets/logo.png"), ExportPlan::all());
}

#[test]
fn a_closed_plan_answers_its_documents_and_holds_its_assets() {
    let plan = ExportPlan::closed()
        .document("FERRES://app/views/settings.xaml")
        .document("ferres://app/MainWindow.xaml")
        .asset("/assets/LOGO.png")
        .asset("/Assets/logo.png");
    assert!(plan.is_closed());
    let applied = apply(&plan, "app", &documents(), &exported(), &assets()).expect("the plan");
    // URIs and paths are compared without regard to case; the documents keep their own URI.
    assert_eq!(
        applied.documents,
        Some(vec![
            (
                "ferres://app/Views/Settings.xaml".to_string(),
                Answer::Build("super::compiled_xaml::build_views_settings_xaml".to_string())
            ),
            ("ferres://app/MainWindow.xaml".to_string(), Answer::Module("compiled_main_window".to_string())),
        ])
    );
    assert_eq!(applied.assets, vec![assets()[0].clone()]);
    // An empty closed plan registers nothing.
    let empty = apply(&ExportPlan::closed(), "app", &documents(), &exported(), &assets()).expect("the empty plan");
    assert_eq!(empty.documents, Some(Vec::new()));
    assert!(empty.assets.is_empty());
}

#[test]
fn a_plan_that_lists_what_the_crate_does_not_have_is_an_error() {
    let plan = ExportPlan::closed()
        .document("ferres://app/Styles/Internal.xaml")
        .document("ferres://app/Missing.xaml")
        .asset("/Assets/missing.png");
    let errors = apply(&plan, "app", &documents(), &exported(), &assets()).expect_err("the plan lists what is not there");
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert!(errors[0].contains("`ferres://app/Styles/Internal.xaml`, which is not a public document"), "{errors:?}");
    assert!(errors[2].contains("`/Assets/missing.png`, which the crate does not embed"), "{errors:?}");
    // A document of a group that is not exported is not answered either.
    let errors = apply(&ExportPlan::closed().document("ferres://app/Views/About.xaml"), "app", &documents(), &[], &assets())
        .expect_err("the group is not exported");
    assert_eq!(errors.len(), 1);
}

#[test]
fn the_loader_branches_build_each_document_or_ask_the_loader_of_its_class() {
    let documents = vec![
        ("ferres://app/Views/Settings.xaml".to_string(), Answer::Build("super::compiled_xaml::build_views_settings_xaml".to_string())),
        ("ferres://app/MainWindow.xaml".to_string(), Answer::Module("compiled_main_window".to_string())),
    ];
    let text = loader_branches(&documents, |text| format!("{text:?}"));
    assert!(text.starts_with(
        "        if ::ferroui_markup_xaml::xaml_il::runtime::compiled::uri_equals(uri, \"ferres://app/Views/Settings.xaml\", \"\") {\n\
         \x20           let provider = ::ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(\n"
    ), "{text}");
    assert!(text.contains("            let root = super::compiled_xaml::build_views_settings_xaml(::core::option::Option::Some(provider))?;\n"), "{text}");
    assert!(text.contains("            return super::compiled_main_window::try_load(service_provider, uri);\n"), "{text}");
    assert!(!text.contains("About"), "{text}");
}

#[test]
fn the_report_lists_the_documents_and_the_assets_with_what_the_plan_does() {
    let plan = ExportPlan::closed().document("ferres://app/Views/Settings.xaml").asset("/Assets/logo.png");
    let applied = apply(&plan, "app", &documents(), &exported(), &assets()).expect("the plan");
    let text = report(&plan, &documents(), &applied, &assets());
    assert!(text.starts_with("# The export plan of the crate (#857): closed.\n"), "{text}");
    assert!(text.contains("document ferres://app/Views/Settings.xaml public answered ::app::compiled_xaml::build_views_settings_xaml\n"), "{text}");
    assert!(text.contains("document ferres://app/Views/About.xaml public not answered "), "{text}");
    assert!(text.contains("document ferres://app/Styles/Internal.xaml internal not answered "), "{text}");
    // The files do not exist here: their size is 0.
    assert!(text.contains("asset /Assets/logo.png 0 registered\n"), "{text}");
    assert!(text.contains("asset /Assets/unused.png 0 left out\n"), "{text}");
    let all = apply(&ExportPlan::all(), "app", &documents(), &exported(), &assets()).expect("no plan");
    let text = report(&ExportPlan::all(), &documents(), &all, &assets());
    assert!(text.contains("document ferres://app/Views/About.xaml public answered "), "{text}");
    assert!(text.contains("asset /Assets/unused.png 0 registered\n"), "{text}");
}

/// The file the crate includes with a closed plan: the checked-in tables first, then the
/// documents of the plan; no loader table of a module and only the assets of the plan.
#[test]
fn the_crate_file_of_a_closed_plan_names_only_what_the_plan_lists() {
    let module = |name: &str, file: bool| crate::Module { name: name.to_string(), path: PathBuf::from(format!("/out/xaml/{name}.rs")), exported: true, file };
    let modules = vec![module("compiled_xaml", false), module("compiled_main_window", true)];
    let plan = ExportPlan::closed().document("ferres://app/Views/Settings.xaml").asset("/Assets/logo.png");
    let applied = apply(&plan, "app", &documents(), &exported(), &assets()).expect("the plan");
    let text = crate::crate_file_with_plan("App", &modules, &["crate::compiled_checked_in::try_load".to_string()], &applied, &[]);
    assert!(!text.contains("super::compiled_xaml::try_load"), "{text}");
    assert!(!text.contains("super::compiled_main_window::try_load"), "{text}");
    assert!(text.contains("super::compiled_xaml::build_views_settings_xaml("), "{text}");
    let checked_in = text.find("crate::compiled_checked_in::try_load(service_provider, uri)?").expect("the checked-in table");
    let planned = text.find("\"ferres://app/Views/Settings.xaml\"").expect("the planned document");
    assert!(checked_in < planned, "{text}");
    assert!(text.contains("(\"/Assets/logo.png\", include_bytes!(\"/crate/Assets/logo.png\")),"), "{text}");
    assert!(!text.contains("unused.png"), "{text}");
    // The modules themselves are still declared.
    assert!(text.contains("pub mod compiled_xaml {\n"), "{text}");
    // Without a plan the loader tables of the modules are asked, as before.
    let all = apply(&ExportPlan::all(), "app", &documents(), &exported(), &assets()).expect("no plan");
    let text = crate::crate_file_with_plan("App", &modules, &[], &all, &[]);
    assert!(text.contains("super::compiled_xaml::try_load(service_provider, uri)?"), "{text}");
    assert!(text.contains("unused.png"), "{text}");
}
