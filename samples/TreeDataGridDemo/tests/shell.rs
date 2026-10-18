//! Not a port: the demo window, rendered through the compositor of the test with Skia by the shell
//! of the samples, shows every page: each tab, when selected, shows a TreeDataGrid that realizes
//! rows and draws them. With `FERROUI_SCREENSHOT_DIR` the frame of each page is saved there as a
//! PPM image (`<n>-<header>.ppm`).

use crate::views::MainView;
use crate::{App, MainWindow, SAMPLE};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::{Control, TabItem};
use ferroui_controls_treedatagrid::TreeDataGrid;
use sample_testing::Shell;
use std::path::PathBuf;

/// The size of the window of the test.
const SIZE: (f64, f64) = (1200.0, 760.0);

/// The headers of the tabs, in their order.
const PAGES: [&str; 10] = [
    "Template Column Reuse",
    "People (v12 XAML)",
    "Countries",
    "Find Displayed Row",
    "BringIntoView (Non-uniform rows)",
    "Files",
    "Wikipedia",
    "Drag/Drop",
    "Core",
    // The tab that is selected when the window opens is shown again last.
    "Countries",
];

#[test]
fn every_page_shows_a_grid_that_realizes_and_draws_rows() {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
    let window = MainWindow::new();
    window.set_width(SIZE.0);
    window.set_height(SIZE.1);
    window.show();
    shell.settle();

    let view = window.get_control::<MainView>("mainView");
    let tabs = view.tabs();
    let items: Vec<Ref<TabItem>> =
        tabs.items().view().to_vec().iter().filter_map(from_markup_value::<Ref<TabItem>>).collect();
    let headers: Vec<String> = items.iter().filter_map(|tab| from_markup_value::<String>(&tab.header())).collect();
    assert_eq!(PAGES[..PAGES.len() - 1].to_vec(), headers, "the tabs of the window");

    for (n, page) in PAGES.iter().enumerate() {
        let index = headers.iter().position(|header| header == page).expect("the tab of the page");
        tabs.set_selected_index(index as i32);
        shell.settle();

        let content = from_markup_value::<Ref<Control>>(&items[index].content()).expect("the content of the tab");
        let grid = content
            .get_logical_descendants()
            .find_map(|x| x.cast::<TreeDataGrid>())
            .unwrap_or_else(|| panic!("{page}: a TreeDataGrid"));
        assert!(grid.is_effectively_visible(), "{page}: the grid is shown");
        // The Wikipedia page has no rows until its articles are loaded.
        if *page != "Wikipedia" {
            let rows = grid.rows_presenter().unwrap_or_else(|| panic!("{page}: the rows presenter"));
            assert!(!rows.get_realized_elements().is_empty(), "{page}: the grid realizes rows");
        }

        assert!(shell.frames(0) > 0, "{page}: the compositor drew the window");
        let frame = shell.last_frame(0);
        let rect = Shell::frame_rect_of(&window, &grid);
        assert!(frame.colors(rect) > 2, "{page}: the grid is drawn");
        save(&shell, &format!("{n:02}-{}.ppm", file_name(page)));
    }

    window.close();
}

/// The header of a page as a file name.
fn file_name(header: &str) -> String {
    header.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect()
}

/// Saves the last frame of the window below `FERROUI_SCREENSHOT_DIR`, when the variable is set.
fn save(shell: &Shell, file_name: &str) {
    let Some(root) = std::env::var("FERROUI_SCREENSHOT_DIR").ok().filter(|root| !root.trim().is_empty()).map(PathBuf::from)
    else {
        return;
    };
    std::fs::create_dir_all(&root).expect("the directory of the screenshots");
    shell.last_frame(0).save_ppm(&root.join(file_name)).expect("the screenshot is saved");
}

