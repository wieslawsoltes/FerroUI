//! Tests of the units an application keeps ([`super`]). Not tests of upstream: small graphs of
//! fake units, rooted the ways an application roots them.

use super::*;
use ferroui_build_scan::model::{RustType, TypeModel};
use ferroui_build_scan::xaml_metadata::ThemeUnitModel;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// A unit: its name, the keys and type keys it defines, the keys it references, the types it
/// constructs.
fn unit(index: u32, name: &str, keys: &[&str], type_keys: &[&str], references: &[&str], types: &[&str]) -> ThemeUnitModel {
    ThemeUnitModel {
        name: name.to_string(),
        index,
        documents: vec![name.to_string()],
        keys: strings(keys),
        type_keys: strings(type_keys),
        references: strings(references),
        types: strings(types),
    }
}

/// A theme of the shapes the WinUI theme has: the ambient unit, control documents with type keys
/// and the resources they reference, a style based on another, and a template that constructs a
/// control of another document.
fn theme() -> ThemeUnitsModel {
    ThemeUnitsModel {
        class: "Tests.Theme".to_string(),
        table_path: "::tests::compiled_xaml::THEME_UNITS".to_string(),
        units: vec![
            unit(0, "ambient", &["SystemAccentColor"], &["Window"], &["TextFillColorPrimaryBrush"], &["Window", "ContentPresenter"]),
            unit(1, "Resources/Common_themeresources.xaml", &["TextFillColorPrimaryBrush", "AccentFillColorDefaultBrush"], &[], &["SystemAccentColor"], &["SolidColorBrush"]),
            unit(2, "Resources/Button_themeresources.xaml", &["ButtonBackground"], &[], &["AccentFillColorDefaultBrush"], &["SolidColorBrush"]),
            unit(3, "Controls/Button.xaml", &["DefaultButtonStyle", "AccentButtonStyle"], &["Button"], &["ButtonBackground", "DefaultButtonStyle"], &["Button", "ContentPresenter", "Border"]),
            unit(4, "Controls/TextBlock.xaml", &["CaptionTextBlockStyle"], &["TextBlock"], &[], &["TextBlock"]),
            unit(5, "Controls/ComboBox.xaml", &["DefaultComboBoxStyle"], &["ComboBox"], &[], &["ComboBox", "Popup", "ToggleButton"]),
            unit(6, "Controls/ToggleButton.xaml", &[], &["ToggleButton"], &["DefaultButtonStyle"], &["ToggleButton"]),
            unit(7, "Controls/Popup.xaml", &[], &["Popup"], &[], &["Popup"]),
            unit(8, "Controls/CalendarView.xaml", &[], &["CalendarView"], &[], &["CalendarView", "Button"]),
        ],
    }
}

fn kept(roots: &Roots) -> Vec<u32> {
    kept_units(&theme(), roots, &TypeGraph::default(), false)
}

#[test]
fn the_ambient_unit_and_what_it_references_are_always_kept() {
    assert_eq!(kept(&Roots::default()), [0, 1]);
}

#[test]
fn a_type_roots_its_type_keyed_unit_and_the_resources_it_references() {
    let roots = Roots { types: set(&["Button"]), ..Roots::default() };
    // Button, its resources, and through them the common resources.
    assert_eq!(kept(&roots), [0, 1, 2, 3]);
}

#[test]
fn a_key_the_documents_name_roots_the_unit_that_defines_it() {
    let roots = Roots { keys: set(&["CaptionTextBlockStyle", "Hello", "World"]), ..Roots::default() };
    assert_eq!(kept(&roots), [0, 1, 4]);
}

#[test]
fn based_on_chains_keep_the_unit_of_the_base_style() {
    // `ToggleButton.xaml` references `DefaultButtonStyle` (its `BasedOn`): the unit of the button
    // is kept for it, and with it what that unit references.
    let roots = Roots { types: set(&["ToggleButton"]), ..Roots::default() };
    assert_eq!(kept(&roots), [0, 1, 2, 3, 6]);
}

#[test]
fn types_templates_construct_root_their_own_control_themes() {
    // The template of the combo box constructs a popup and a toggle button.
    let roots = Roots { types: set(&["ComboBox"]), ..Roots::default() };
    assert_eq!(kept(&roots), [0, 1, 2, 3, 5, 6, 7]);
}

#[test]
fn a_subclass_roots_the_themes_of_its_base_classes() {
    let bases = TypeGraph::from_pairs(&[("MyButton", "Button"), ("Button", "ButtonBase")]);
    assert_eq!(bases.chain("MyButton"), ["MyButton", "Button", "ButtonBase"]);
    let roots = Roots { types: set(&["MyButton"]), ..Roots::default() };
    assert_eq!(kept_units(&theme(), &roots, &bases, false), [0, 1, 2, 3]);
}

#[test]
fn explicit_roots_name_units_types_or_keys() {
    // A unit by the path of its document, a type, a key.
    let roots = Roots { names: set(&["Controls/Popup.xaml", "CalendarView", "CaptionTextBlockStyle"]), ..Roots::default() };
    // The calendar view constructs buttons.
    assert_eq!(kept(&roots), [0, 1, 2, 3, 4, 7, 8]);
}

#[test]
fn every_unit_is_kept_for_all_or_with_full_themes() {
    let all = Roots { all: true, ..Roots::default() };
    assert_eq!(kept(&all), (0..9).collect::<Vec<u32>>());
    assert_eq!(kept_units(&theme(), &Roots::default(), &TypeGraph::default(), true), (0..9).collect::<Vec<u32>>());
}

#[test]
fn roots_added_by_a_build_script_join_the_roots_of_the_documents() {
    let mut roots = Roots { types: set(&["TextBlock"]), ..Roots::default() };
    roots.extend(Roots { names: set(&["Popup"]), ..Roots::default() });
    assert_eq!(kept(&roots), [0, 1, 4, 7]);
}

#[test]
fn the_static_names_and_paths_follow_the_class_and_the_table() {
    assert_eq!(parts_static_name("FerroUI.Themes.WinUI.WinUITheme"), "THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME");
    assert_eq!(unit_static_path("::ferroui_themes_winui::compiled_xaml::THEME_UNITS", 12), "::ferroui_themes_winui::compiled_xaml::THEME_UNIT_12");
}

#[test]
fn the_manifest_states_the_opt_in_and_the_roots() {
    let manifest = r#"
[package]
name = "app"
metadata.other = 1

[dependencies]
ferroui-base = { workspace = true, features = ["markup-functions"] }

[package.metadata.ferroui]
# Trim the themes.
theme-trimming = true # the default, stated
theme-roots = [
    "CalendarView", # created in code
    'AccentButtonStyle',
]

[package.metadata.release]
group = "samples"
"#;
    let trimming = ThemeTrimming::from_manifest(manifest).expect("a manifest");
    assert_eq!(trimming, ThemeTrimming { enabled: true, all: false, roots: strings(&["CalendarView", "AccentButtonStyle"]) });

    let all = ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-trimming = true\ntheme-roots = \"all\"\n").expect("a manifest");
    assert_eq!(all, ThemeTrimming { enabled: true, all: true, roots: Vec::new() });

    // Without the table, the themes are trimmed (the default); `false` turns it off.
    assert_eq!(ThemeTrimming::from_manifest("[package]\nname = \"app\"\n"), Ok(ThemeTrimming::default()));
    assert!(ThemeTrimming::default().enabled);
    let off = ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-trimming = false\n").expect("a manifest");
    assert_eq!(off, ThemeTrimming { enabled: false, all: false, roots: Vec::new() });
    assert_eq!(
        ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-trimming = \"yes\"\n"),
        Err("[package.metadata.ferroui] theme-trimming must be true or false".to_string())
    );
    assert_eq!(
        ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-roots = 3\n"),
        Err("[package.metadata.ferroui] theme-roots must be \"all\" or an array of names".to_string())
    );
}

/// A class of a model: its name, its base and the classes it creates.
fn class(name: &str, base: Option<&str>, creates: &[&str]) -> TypeModel {
    let mut type_ = TypeModel::new(name, TypeKind::Class, RustType::resolved(&format!("::tests::{name}")), "tests");
    type_.namespace = "Tests.Controls".to_string();
    type_.object_model = true;
    type_.base = base.map(|base| RustType::resolved(&format!("::tests::{base}")));
    type_.creates = strings(creates);
    type_
}

#[test]
fn a_live_type_makes_the_classes_it_creates_live_transitively() {
    // A list creates its items, the items a toggle button; a subclass of the list creates them
    // through its base class.
    let graph = TypeGraph::from_pairs(&[("MyList", "List")]).with_creates(&[("List", "Item"), ("Item", "ToggleButton")]);
    assert_eq!(graph.live("MyList"), set(&["Item", "List", "MyList", "ToggleButton"]));
    assert_eq!(graph.live("Item"), set(&["Item", "ToggleButton"]));
    // A cycle of edges ends.
    let cycle = TypeGraph::default().with_creates(&[("A", "B"), ("B", "A")]);
    assert_eq!(cycle.live("A"), set(&["A", "B"]));

    // The toggle button keeps its own control theme and, through its `BasedOn`, the button's.
    let roots = Roots { types: set(&["MyList"]), ..Roots::default() };
    assert_eq!(kept_units(&theme(), &roots, &graph, false), [0, 1, 2, 3, 6]);
    // A type a kept unit constructs makes what it creates live too: the calendar view's buttons
    // create a popup.
    let graph = TypeGraph::default().with_creates(&[("Button", "Popup")]);
    let roots = Roots { types: set(&["CalendarView"]), ..Roots::default() };
    assert_eq!(kept_units(&theme(), &roots, &graph, false), [0, 1, 2, 3, 7, 8]);
}

#[test]
fn the_type_graph_reads_bases_and_creates_from_the_models() {
    let mut model = AssemblyModel::new("Tests", "tests");
    model.types = vec![
        class("List", Some("Control"), &["Tests.Controls.Item"]),
        class("Item", Some("ContentControl"), &["Other.Primitives.Handle", "Tests.Controls.Item"]),
        class("MyList", Some("List"), &[]),
    ];
    let graph = TypeGraph::from_models([&model]);
    assert_eq!(graph.chain("MyList"), ["MyList", "List", "Control"]);
    // By local name, the class itself left out.
    assert_eq!(graph.live("MyList"), set(&["Control", "ContentControl", "Handle", "Item", "List", "MyList"]));
    assert!(graph.is_class("Item") && !graph.is_class("Handle"));
}

#[test]
fn the_sources_root_type_keys_classes_and_keys() {
    let graph = TypeGraph::default().with_creates(&[("Canvas", "Popup")]);
    let references = SourceReferences {
        // A type key, a class of the models that is no type key, and names that are neither.
        names: set(&["CalendarView", "Canvas", "Orientation", "Vertical"]),
        // A key, and strings that are none.
        strings: set(&["CaptionTextBlockStyle", "Hello, World!", "CalendarView"]),
    };
    let theme = theme();
    let roots = source_roots(&references, [&theme], &graph);
    assert_eq!(roots, Roots { types: set(&["CalendarView", "Canvas"]), keys: set(&["CaptionTextBlockStyle"]), ..Roots::default() });
    // The calendar view and its buttons, the popup the canvas creates, the caption style.
    assert_eq!(kept_units(&theme, &roots, &graph, false), [0, 1, 2, 3, 4, 7, 8]);
}

/// Not from upstream (#603): compiled bindings are the default; the manifest turns them off for
/// the crate (`compile-bindings = false`), or states the default, and refuses anything else.
#[test]
fn the_manifest_states_whether_bindings_compile() {
    assert_eq!(compile_bindings_from_manifest("[package]\nname = \"app\"\n"), Ok(None));
    assert_eq!(compile_bindings_from_manifest("[package.metadata.ferroui]\ntheme-trimming = false\n"), Ok(None));
    assert_eq!(compile_bindings_from_manifest("[package.metadata.ferroui]\ncompile-bindings = false # reflection bindings\n"), Ok(Some(false)));
    assert_eq!(compile_bindings_from_manifest("[package.metadata.ferroui]\ncompile-bindings = true\n"), Ok(Some(true)));
    // Another table with the same key says nothing about the crate.
    assert_eq!(compile_bindings_from_manifest("[package.metadata.other]\ncompile-bindings = false\n"), Ok(None));
    assert_eq!(
        compile_bindings_from_manifest("[package.metadata.ferroui]\ncompile-bindings = \"no\"\n"),
        Err("[package.metadata.ferroui] compile-bindings must be true or false".to_string())
    );
    // The key does not change the trimming of the themes.
    assert_eq!(ThemeTrimming::from_manifest("[package.metadata.ferroui]\ncompile-bindings = false\n"), Ok(ThemeTrimming::default()));
}

/// Not from upstream (#849): the established name of a parts table is kept where no other class has
/// it (every theme of the framework); classes whose full names differ in case or punctuation only
/// get names disambiguated from their full names, the same whatever the order.
#[test]
fn parts_tables_of_classes_that_collide_get_two_names() {
    let themes = ["FerroUI.Themes.WinUI.WinUITheme", "FerroUI.Themes.Fluent.FluentTheme", "A.B_C", "A_B.C", "a.b.c"];
    let names = parts_static_names(&themes);
    assert_eq!(names[0], "THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME");
    assert_eq!(names[1], "THEME_PARTS_FERROUI_THEMES_FLUENT_FLUENTTHEME");
    for name in &names[2..] {
        assert!(name.starts_with("THEME_PARTS_A_B_C_") && name.len() == "THEME_PARTS_A_B_C_".len() + 8, "{name}");
        assert_eq!(name.to_ascii_uppercase(), *name);
    }
    assert_ne!(names[2], names[3]);
    assert_ne!(names[3], names[4]);
    assert_ne!(names[2], names[4]);
    let reversed = parts_static_names(&["a.b.c", "A_B.C", "A.B_C"]);
    assert_eq!(reversed, [names[4].clone(), names[3].clone(), names[2].clone()]);
    // A class alone keeps its established name.
    assert_eq!(parts_static_names(&["A.B_C"]), ["THEME_PARTS_A_B_C"]);
}

/// A model of the crate `themes` with the class `Tests.Theme` (units, and the public functions
/// `functions` of its type, as names and numbers of parameters) and the class `Tests.App` (units,
/// and `new()` only).
fn model_with(functions: &[(&str, usize, bool)]) -> AssemblyModel {
    use ferroui_build_scan::model::{FunctionModel, FunctionsModel};
    let mut model = AssemblyModel::new("Tests", "themes");
    for (name, module) in [("Theme", "themes::theme"), ("App", "themes::app")] {
        let rust_path = RustType::resolved(&format!("::{module}::{name}"));
        let mut type_ = TypeModel::new(name, TypeKind::Class, rust_path, module);
        type_.namespace = "Tests".to_string();
        type_.public_path = Some(format!("::themes::{name}"));
        model.types.push(type_);
        model.theme_units.push(ThemeUnitsModel {
            class: format!("Tests.{name}"),
            table_path: format!("::themes::compiled_{}::THEME_UNITS", name.to_lowercase()),
            units: vec![unit(0, "ambient", &[], &[], &[], &[])],
        });
    }
    let function = |name: &str, parameters: usize, receiver: bool| FunctionModel {
        name: name.to_string(),
        receiver,
        parameters,
        declared_by: None,
    };
    model.functions = vec![
        FunctionsModel {
            owner: "::themes::theme::Theme".to_string(),
            public_path: "::themes::Theme".to_string(),
            functions: functions.iter().map(|(name, count, receiver)| function(name, *count, *receiver)).collect(),
        },
        FunctionsModel {
            owner: "::themes::app::App".to_string(),
            public_path: "::themes::App".to_string(),
            functions: vec![function("new", 0, false)],
        },
    ];
    model
}

/// Not from upstream (#849): another crate constructs a class from selected units only when its
/// type model states a public `with_parts` of two parameters without `self`; an ordinary class
/// whose markup is partitioned into units, a class of no crate's units and a function of another
/// shape are not that capability.
#[test]
fn a_class_is_constructed_from_units_only_with_its_constructor_from_units() {
    let models = [model_with(&[("new", 0, false), ("with_parts", 2, false)])];
    assert_eq!(parts_constructor(&models, "Tests.Theme"), Some("::themes::Theme::with_parts".to_string()));
    assert_eq!(parts_constructor(&models, "Tests.App"), None);
    assert_eq!(parts_constructor(&models, "Tests.Missing"), None);
    for functions in [&[("from_parts", 2, false)][..], &[("with_parts", 1, false)], &[("with_parts", 2, true)]] {
        assert_eq!(parts_constructor(&[model_with(functions)], "Tests.Theme"), None, "{functions:?}");
    }
}

/// An inherent function of the type `Tests.Theme` of [`model_with`], as the scanner reads it.
fn inherent(name: &str, visibility: &str, parameters: &[&str], return_type: &str) -> InherentFunction {
    InherentFunction {
        owner: "::themes::theme::Theme".to_string(),
        name: name.to_string(),
        visibility: visibility.to_string(),
        receiver: false,
        parameters: strings(parameters),
        return_type: Some(return_type.to_string()),
        declared_by: None,
        module: "themes::theme".to_string(),
        file: std::path::PathBuf::from("theme.rs"),
        line: 1,
    }
}

/// Not from upstream (#849): the build of the crate of a class verifies the signature of its
/// constructor from units, whatever the paths and the white space are written with; a public
/// `with_parts` of two parameters and another signature is an error, since other crates would call
/// it; no such function, or a private one, is an ordinary class.
#[test]
fn the_build_of_a_class_verifies_its_constructor_from_units() {
    let model = model_with(&[]);
    let verify = |functions: &[InherentFunction]| verify_parts_constructor(&model, functions, "Tests.Theme");
    let sp = "Option<Rc<dyn IServiceProvider>>";
    assert_eq!(verify(&[inherent("with_parts", "pub", &[sp, "rt::ThemeParts"], "Ref<Self>")]), Ok(true));
    let qualified = [
        "::core::option::Option< ::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider> >",
        "ferroui_markup_xaml::xaml_il::runtime::compiled::ThemeParts",
    ];
    assert_eq!(verify(&[inherent("with_parts", "pub", &qualified, "ferroui_base::Ref<Theme>")]), Ok(true));
    assert_eq!(verify(&[]), Ok(false));
    assert_eq!(verify(&[inherent("with_parts", "", &[sp, "rt::ThemeParts"], "Ref<Self>")]), Ok(false));
    assert_eq!(verify(&[inherent("from_parts", "pub", &[sp, "rt::ThemeParts"], "Ref<Self>")]), Ok(false));
    let wrong = verify(&[inherent("with_parts", "pub", &[sp, "Vec<u32>"], "Ref<Self>")]);
    let named = |reason: &String| reason.starts_with("`Tests.Theme` has `pub fn with_parts(");
    assert!(wrong.as_ref().is_err_and(named), "{wrong:?}");
    // A class of no type of the model.
    assert_eq!(verify_parts_constructor(&model, &[], "Tests.Missing"), Ok(false));
    assert_eq!(last_segments("Option < Rc < dyn crate::IServiceProvider > >"), "Option<Rc<dynIServiceProvider>>");
}

/// Not from upstream (#850): the table is read as TOML. Equivalent forms of
/// `[package.metadata.ferroui]` give the same options: a table header, dotted keys under a parent
/// table, an inline table, and dotted keys at the top level.
#[test]
fn equivalent_forms_of_the_manifest_table_give_the_same_options() {
    let expected = ThemeTrimming { enabled: false, all: true, roots: Vec::new() };
    let forms = [
        "[package.metadata.ferroui]\ntheme-trimming = false\ntheme-roots = \"all\"\ncompile-bindings = false\n",
        "[package.metadata]\nferroui.theme-trimming = false\nferroui.theme-roots = \"all\"\n\
         ferroui.compile-bindings = false\n",
        "[package]\nname = \"app\"\n\
         metadata.ferroui = { theme-trimming = false, theme-roots = \"all\", compile-bindings = false }\n",
        "package.metadata.ferroui.theme-trimming = false\npackage.metadata.ferroui.theme-roots = 'all'\n\
         package.metadata.ferroui.compile-bindings = false\n",
    ];
    for form in forms {
        assert_eq!(ThemeTrimming::from_manifest(form), Ok(expected.clone()), "{form}");
        assert_eq!(compile_bindings_from_manifest(form), Ok(Some(false)), "{form}");
    }
}

/// Not from upstream (#850): strings are TOML strings, with their escapes, and a bracket inside a
/// string is text, also in an array that spans lines.
#[test]
fn the_strings_of_the_manifest_are_toml_strings() {
    let manifest = "[package.metadata.ferroui]\ntheme-roots = [\n  \"Accent\\u0042uttonStyle\", # a comment\n\
                    \"Accent[Brush\",\n  'C:\\path',\n]\n";
    let trimming = ThemeTrimming::from_manifest(manifest).expect("the table");
    assert_eq!(trimming.roots, strings(&["AccentButtonStyle", "Accent[Brush", "C:\\path"]));
}

/// Not from upstream (#850): a top-level table whose quoted name is `package.metadata.ferroui` is
/// another table, and says nothing about the crate.
#[test]
fn a_quoted_table_name_is_one_key() {
    let manifest = "[\"package.metadata.ferroui\"]\ntheme-trimming = false\ncompile-bindings = false\n";
    assert_eq!(ThemeTrimming::from_manifest(manifest), Ok(ThemeTrimming::default()));
    assert_eq!(compile_bindings_from_manifest(manifest), Ok(None));
}

/// Not from upstream (#850): a value of a wrong type is refused with the option it is for, and a
/// manifest that is not TOML is refused.
#[test]
fn a_wrong_value_names_its_option() {
    let roots = ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-roots = [\"A\", 1]\n");
    assert_eq!(roots, Err("[package.metadata.ferroui] theme-roots must be \"all\" or an array of names".to_string()));
    let other = ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-roots = \"some\"\n");
    assert_eq!(other, Err("[package.metadata.ferroui] theme-roots must be \"all\" or an array of names".to_string()));
    assert!(ThemeTrimming::from_manifest("[package\nname = 1").is_err());
}

/// Not from upstream (#848): two types of one local name in two assemblies keep their own base
/// classes and creates edges, whatever the order of the models; a root that names the local name
/// is live with the edges of both.
#[test]
fn types_of_one_local_name_keep_their_own_edges() {
    let model = |assembly: &str, namespace: &str, base: &str, created: &str| {
        let mut model = AssemblyModel::new(assembly, &assembly.to_lowercase());
        let crate_name = assembly.to_lowercase();
        let mut twin = TypeModel::new("Twin", TypeKind::Class, RustType::resolved(&format!("::{crate_name}::Twin")), &crate_name);
        twin.namespace = namespace.to_string();
        twin.object_model = true;
        twin.base = Some(RustType::resolved(&format!("::{crate_name}::{base}")));
        twin.creates = vec![format!("{namespace}.{created}")];
        let mut base_type = TypeModel::new(base, TypeKind::Class, RustType::resolved(&format!("::{crate_name}::{base}")), &crate_name);
        base_type.namespace = namespace.to_string();
        base_type.object_model = true;
        model.types = vec![twin, base_type];
        model
    };
    let left = model("Left", "Left", "LeftBase", "LeftPart");
    let right = model("Right", "Right", "RightBase", "RightPart");
    for graph in [TypeGraph::from_models([&left, &right]), TypeGraph::from_models([&right, &left])] {
        assert_eq!(graph.live("Twin"), set(&["LeftBase", "LeftPart", "RightBase", "RightPart", "Twin"]));
        assert_eq!(graph.live("LeftBase"), set(&["LeftBase"]));
        assert!(graph.is_class("Twin") && graph.is_class("RightBase") && !graph.is_class("LeftPart"));
    }
}

/// A theme of one unit besides the ambient one, for the tests of the closure across themes.
fn small_theme(class: &str, units: Vec<ThemeUnitModel>) -> ThemeUnitsModel {
    ThemeUnitsModel { class: class.to_string(), table_path: format!("::{}::THEME_UNITS", class.to_lowercase()), units }
}

/// Not from upstream (#848): themes used together are closed over together. A key that a kept unit
/// of one theme references and only another theme defines keeps the unit that defines it, a type a
/// kept unit of one theme constructs keeps the control theme another supplies, and references in a
/// cycle between two themes end.
#[test]
fn themes_used_together_are_closed_over_together() {
    let first = small_theme(
        "Tests.First",
        vec![
            unit(0, "ambient", &[], &[], &[], &[]),
            unit(1, "First/Button.xaml", &[], &["Button"], &["SecondBrush"], &["Popup"]),
            unit(2, "First/Other.xaml", &["FirstBrush"], &[], &["SecondOther"], &[]),
        ],
    );
    let second = small_theme(
        "Tests.Second",
        vec![
            unit(0, "ambient", &[], &[], &[], &[]),
            unit(1, "Second/Brushes.xaml", &["SecondBrush"], &[], &["FirstBrush"], &[]),
            unit(2, "Second/Popup.xaml", &[], &["Popup"], &[], &[]),
            unit(3, "Second/Unused.xaml", &["SecondOther"], &[], &[], &[]),
        ],
    );
    let roots = Roots { types: set(&["Button"]), ..Roots::default() };
    let graph = TypeGraph::default();
    // Alone, each theme keeps what the roots name in it.
    assert_eq!(kept_units(&first, &roots, &graph, false), [0, 1]);
    assert_eq!(kept_units(&second, &roots, &graph, false), [0]);
    // Together: the brush the button references and the popup it constructs, and, through the
    // cycle `SecondBrush` -> `FirstBrush` -> `SecondOther`, the rest.
    let kept = kept_units_across(&[&first, &second], &roots, &graph, false);
    assert_eq!(kept, [vec![0, 1, 2], vec![0, 1, 2, 3]]);
    // The order of the themes does not change what each keeps.
    let reversed = kept_units_across(&[&second, &first], &roots, &graph, false);
    assert_eq!(reversed, [vec![0, 1, 2, 3], vec![0, 1, 2]]);
}

/// Not from upstream (#847): the binaries of a package are read from its manifest and discovered as
/// Cargo discovers them: explicit `[[bin]]` paths, a `[[bin]]` without a path by its name, and,
/// unless `autobins = false`, `src/main.rs` and `src/bin`. Only files that exist count.
#[test]
fn the_binaries_of_a_package_are_its_production_targets() {
    let root = std::env::temp_dir().join(format!("ferroui-binary-roots-{}", std::process::id()));
    let files =
        ["program.rs", "src/main.rs", "src/bin/tool.rs", "src/bin/other/main.rs", "src/bin/named.rs", "examples/a.rs"];
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().expect("a directory")).expect("the directory");
        std::fs::write(&path, "fn main() {}\n").expect("a file");
    }
    let manifest = "[package]\nname = \"app\"\nautobins = false # explicit only\n\n\
                    [[bin]]\nname = \"app\"\npath = \"program.rs\"\n\n[[bin]]\nname = \"named\"\n\n\
                    [[bin]]\nname = \"missing\"\npath = \"missing.rs\"\n\n[dependencies]\npath = \"x\"\n";
    let relative = |paths: Vec<std::path::PathBuf>| -> Vec<String> {
        let below = |path: &std::path::PathBuf| path.strip_prefix(&root).expect("below the root").display().to_string();
        paths.iter().map(|path| below(path).replace('\\', "/")).collect()
    };
    assert_eq!(relative(binary_roots(&root, manifest)), ["program.rs", "src/bin/named.rs"]);
    let discovered = "[package]\nname = \"app\"\n\n[[bin]]\nname = \"app\"\npath = \"program.rs\"\n";
    assert_eq!(
        relative(binary_roots(&root, discovered)),
        ["program.rs", "src/main.rs", "src/bin/named.rs", "src/bin/other/main.rs", "src/bin/tool.rs"]
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Not from upstream (#852): the explained closure keeps the units [`kept_units`] keeps, with the
/// first reason for each (stated, constructed, referenced or conservative) and the live share of
/// its keys; the listing has the totals.
#[test]
fn every_kept_unit_has_a_reason() {
    let graph = TypeGraph::from_pairs(&[("MyButton", "Button")]);
    let documents = Roots { types: set(&["MyButton"]), keys: set(&["CaptionStyle"]), ..Roots::default() };
    let manifest = Roots { names: set(&["Controls/Popup.xaml"]), ..Roots::default() };
    let button_keys = ["ButtonPadding", "ButtonUnused"];
    let theme = ThemeUnitsModel {
        class: "Tests.Explained".to_string(),
        table_path: "::tests::compiled_xaml::THEME_UNITS".to_string(),
        units: vec![
            unit(0, "ambient", &["AmbientBrush"], &[], &[], &[]),
            unit(1, "Controls/Button.xaml", &button_keys, &["Button"], &["AccentBrush"], &["TextBlock"]),
            unit(2, "Resources/Accent.xaml", &["AccentBrush", "AccentUnused"], &[], &[], &[]),
            unit(3, "Controls/TextBlock.xaml", &[], &["TextBlock"], &[], &[]),
            unit(4, "Resources/Caption.xaml", &["CaptionStyle"], &[], &[], &[]),
            unit(5, "Controls/Popup.xaml", &[], &["Popup"], &[], &[]),
            unit(6, "Controls/Unused.xaml", &[], &["Unused"], &[], &[]),
        ],
    };
    let sources: [(&str, &Roots); 2] = [("manifest", &manifest), ("documents", &documents)];
    let explained = explain_kept_units(&theme, &sources, &graph, false);
    let mut union = documents.clone();
    union.extend(manifest.clone());
    let indices: Vec<u32> = explained.iter().map(|unit| unit.index).collect();
    assert_eq!(indices, kept_units(&theme, &union, &graph, false));
    assert_eq!(indices, [0, 1, 2, 3, 4, 5]);
    let reason = |index: u32| explained.iter().find(|unit| unit.index == index).expect("kept").clone();
    assert_eq!(reason(0).reason, "the ambient unit, always kept");
    assert!(reason(1).reason.contains("defines the theme of Button") && reason(1).reason.contains("makes Button live"));
    assert!(reason(1).conservative, "a type of the documents is conservative");
    assert_eq!(reason(2).reason, "defines AccentBrush: the key AccentBrush, referenced by Controls/Button.xaml");
    assert!(!reason(2).conservative);
    assert_eq!((reason(2).keys, reason(2).live_keys), (2, 1));
    let constructed = "defines the theme of TextBlock: the type TextBlock, constructed by Controls/Button.xaml";
    assert_eq!(reason(3).reason, constructed);
    assert!(reason(4).conservative);
    assert!(reason(4).reason.starts_with("defines CaptionStyle: the key CaptionStyle, a root of the documents"));
    assert_eq!(reason(5).reason, "named by the manifest");
    let listing = why_kept_listing("Tests.Explained", &explained, theme.units.len());
    let totals = "Tests.Explained: 6 of 7 units; 2 of 6 keys of the kept units are live\n";
    assert!(listing.starts_with(totals), "{listing}");
    assert!(listing.contains("Resources/Accent.xaml\t1 of 2 keys live\texact\tdefines AccentBrush"), "{listing}");
}

/// What the documents of an ordinary class of a dependency name (#1002): it roots nothing where
/// nothing constructs the class, and roots the themes where the application names the class (by
/// the type or its builder, which the scan records in the case of the type), where a live type
/// creates it, or where the document of another live class constructs it.
#[test]
fn the_references_of_an_ordinary_class_root_only_where_the_class_is_live() {
    use ferroui_build_scan::xaml_metadata::ClassReferencesModel;
    let class = |class: &str, types: &[&str], keys: &[&str]| ClassReferencesModel {
        class: class.to_string(),
        types: strings(types),
        keys: strings(keys),
    };
    let about = class("Lib.About", &["Button", "Window"], &["AboutTitle"]);
    let credits = class("Lib.Credits", &["Expander"], &[]);
    let host = class("Lib.Host", &["Credits"], &[]);
    let classes = [&about, &credits, &host];
    let graph = TypeGraph::from_pairs(&[("Lib.About", "Window"), ("Lib.Credits", "UserControl"), ("Lib.Host", "Window")])
        .with_creates(&[("Lib.Shell", "Lib.Host")]);
    let names = |classes: &[&ClassReferencesModel]| -> Vec<String> { classes.iter().map(|class| class.class.clone()).collect() };
    let roots = Roots { types: set(&["Window", "TextBlock"]), ..Roots::default() };

    // Nothing constructs a class: a base class of it being live does not make it live.
    assert!(live_class_references(&classes, &roots, &BTreeSet::new(), &graph).is_empty());
    // The application names the class (`About::new()`, or `about()`, recorded as `About`).
    let named = live_class_references(&classes, &roots, &set(&["About"]), &graph);
    assert_eq!(names(&named), ["Lib.About"]);
    // A live type creates the class, and the document of the created class constructs another.
    let with_shell = Roots { types: set(&["Shell"]), ..Roots::default() };
    let created = live_class_references(&classes, &with_shell, &BTreeSet::new(), &graph);
    assert_eq!(names(&created), ["Lib.Credits", "Lib.Host"]);
    // A full name of the sources names the class too.
    let full = live_class_references(&classes, &Roots::default(), &set(&["Lib.Credits"]), &graph);
    assert_eq!(names(&full), ["Lib.Credits"]);
}

/// The reason a unit is kept for a live class names the class (#1002).
#[test]
fn a_unit_kept_for_a_live_class_says_which_class() {
    let graph = TypeGraph::default();
    let theme = ThemeUnitsModel {
        class: "Tests.Theme".to_string(),
        table_path: "::theme::compiled_xaml::THEME_UNITS".to_string(),
        units: vec![unit(0, "ambient", &[], &[], &[], &[]), unit(1, "Controls/Button.xaml", &[], &["Button"], &[], &[])],
    };
    let class_roots = Roots { types: set(&["Button"]), ..Roots::default() };
    let sources: [(&str, &Roots); 1] = [("referenced by the document of the live class Lib.About", &class_roots)];
    let explained = explain_kept_units(&theme, &sources, &graph, false);
    let button = explained.iter().find(|unit| unit.index == 1).expect("the unit of Button is kept");
    assert_eq!(
        button.reason,
        "defines the theme of Button: the type Button, referenced by the document of the live class Lib.About"
    );
    assert!(button.conservative);
}
