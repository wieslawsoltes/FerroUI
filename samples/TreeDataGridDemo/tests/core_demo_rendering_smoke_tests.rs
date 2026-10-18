//! Port of `TreeDataGridCoreDemo.Tests/RenderingSmokeTests.cs`: the whole core demo, shown in a
//! window that renders through the compositor of the test with Skia, has a core model and no
//! classic source in every grid, follows the nested expansion state of the people, and renders
//! the countries and the people pages to frames that are saved.
//!
//! The reference's test application is its own headless application with the Fluent theme and
//! the theme of the grid; here it is the application of the demo ([`App`]: `App.xaml`, the same
//! themes), started by the shell of the samples (`sample_testing::Shell`), whose windows render
//! frames into memory. The shell starts the application of the test itself, in place of
//! `super::support::start_application()`, so that the window draws frames. The reference saves
//! the frames as PNG images below the directory its environment names (or the directory of the
//! test assembly); the port saves them as PPM images below `FERROUI_SCREENSHOT_DIR` (or the
//! temporary directory).

use crate::view_models::CoreDemoViewModel;
use crate::views::CoreDemoPage;
use crate::{App, SAMPLE};
use ferroui_controls::{Control, TabControl, Window};
use ferroui_controls_treedatagrid::TreeDataGrid;
use ferroui_treedatagrid_core::ITreeDataGridSource;
use sample_testing::Shell;
use std::path::PathBuf;

/// The size of the window of the core demo (`Width="1180" Height="760"`).
const SIZE: (f64, f64) = (1180.0, 760.0);

/// The grids of the pages of the demo, in the order of the tabs.
const GRIDS: [&str; 8] =
    ["CountriesGrid", "FindGrid", "VariableGrid", "FilesGrid", "WikipediaGrid", "DragDropGrid", "PeopleGrid", "TemplateGrid"];

/// Closes the window at the end of the test, as the reference's `finally`.
struct ClosesWindow(ferroui_base::Ref<Window>);

impl Drop for ClosesWindow {
    fn drop(&mut self) {
        self.0.close();
    }
}

#[test]
fn full_core_demo_renders_without_legacy_source_adapters() {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
    let view_model = CoreDemoViewModel::new(false);
    let page = CoreDemoPage::with_view_model(view_model.clone());
    let window = Window::new();
    window.set_width(SIZE.0);
    window.set_height(SIZE.1);
    window.set_title(Some("TreeDataGrid Core API Demo".to_string()));
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    let window = ClosesWindow(window);

    shell.settle();
    let grids: Vec<_> = GRIDS.iter().map(|name| page.get_control::<TreeDataGrid>(name)).collect();
    assert_eq!(8, grids.len());
    for (grid, name) in grids.iter().zip(GRIDS) {
        assert!(grid.model().is_some(), "{name}: the model of the grid");
        assert!(grid.source().is_none(), "{name}: the grid has no classic source");
        assert!(grid.presentation().is_some(), "{name}: the presentation of the model");
    }

    let tabs = page.get_control::<TabControl>("Tabs");
    for (i, (grid, name)) in grids.iter().zip(GRIDS).enumerate() {
        tabs.set_selected_index(i as i32);
        shell.settle();
        assert!(grid.rows_presenter().is_some(), "{name}: the rows presenter of the selected page");
    }

    let people_source = view_model.people_source();
    let people = people_source.items().get(0);
    let expanded_count = ITreeDataGridSource::rows(&*people_source).count();
    people.expansion().set_is_expanded(false);
    assert!(ITreeDataGridSource::rows(&*people_source).count() < expanded_count);
    people.expansion().set_is_expanded(true);
    assert_eq!(expanded_count, ITreeDataGridSource::rows(&*people_source).count());

    tabs.set_selected_index(0);
    shell.settle();
    save(&shell, "core-demo-countries.ppm");
    tabs.set_selected_index(6);
    shell.settle();
    save(&shell, "core-demo-people.ppm");
    tabs.set_selected_index(0);
    shell.settle();

    drop(window);
}

/// Saves the last frame of the window of the test as `file_name` below the directory of the
/// screenshots.
fn save(shell: &Shell, file_name: &str) {
    assert!(shell.frames(0) > 0, "No rendered frame was captured.");
    let frame = shell.last_frame(0);
    let root = std::env::var("FERROUI_SCREENSHOT_DIR")
        .ok()
        .filter(|root| !root.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("headless-screenshots"));
    std::fs::create_dir_all(&root).expect("the directory of the screenshots");
    let path = root.join(file_name);
    frame.save_ppm(&path).expect("the screenshot is saved");
    assert!(path.exists());
    assert!(std::fs::metadata(&path).expect("the screenshot").len() > 0);
}
