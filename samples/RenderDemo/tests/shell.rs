//! The real application headless: the application of the sample ([`App`]: `App.xaml`, the
//! Fluent theme, the resources of the hamburger menu) with its main window, which renders
//! through the compositor of the test with Skia into memory, and the input of a mouse.
//!
//! Not ports: the upstream sample has no tests. Every page of the main window is selected,
//! laid out and rendered, and each test looks at what the page drew: the content of its
//! rectangle of the frame, an animation that advances between two ticks of the clock, the
//! colours a custom control draws with.

use crate::pages::*;
use crate::{App, MainWindow, SAMPLE};
use control_samples::HamburgerMenu;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ObjectType, Point, Ref, Visual};
use ferroui_controls::{Control, Slider, TabItem, TextBlock};
use sample_testing::{Frame, Shell};

/// The size of the main window: the size its view model gives it.
const WIDTH: f64 = 800.0;
const HEIGHT: f64 = 600.0;

/// The headers of the pages of the main window, as its document declares them.
const PAGES: [&str; 20] = [
    "Animations",
    "Animation Speed",
    "Transitions",
    "Custom Animator",
    "Spring Animation",
    "Clipping",
    "Drawing",
    "Hit Testing",
    "Geometry Hit Testing",
    "SkCanvas",
    "RenderTargetBitmap",
    "WriteableBitmap",
    "GlyphRun",
    "FormattedText",
    "TextFormatter",
    "LineBounds",
    "Path Measurement",
    "Brushes",
    "3D Transformation",
    "Resize Pattern",
];

/// The main window of the sample, shown under the application of the sample.
struct Demo {
    // Declared before the shell: closed and dropped before it.
    window: Ref<MainWindow>,
    shell: Shell,
}

impl Demo {
    fn start() -> Demo {
        let shell = Shell::start(&SAMPLE, WIDTH, HEIGHT, || App::new().upcast());
        let window = MainWindow::new();
        window.show();
        let demo = Demo { window, shell };
        demo.shell.settle();
        demo
    }

    fn menu(&self) -> Ref<HamburgerMenu> {
        from_markup_value::<Ref<HamburgerMenu>>(&self.window.content()).expect("the content of the main window is the hamburger menu")
    }

    fn tabs(&self) -> Vec<Ref<TabItem>> {
        self.menu().items().view().to_vec().iter().filter_map(from_markup_value::<Ref<TabItem>>).collect()
    }

    fn header(tab: &TabItem) -> String {
        tab.header().map(|header| ValueTypes::to_display_string(Some(&header))).unwrap_or_default()
    }

    /// Selects the page with the header `header` and returns it, laid out and rendered.
    fn show(&self, header: &str) -> Ref<Control> {
        let tabs = self.tabs();
        let index = tabs.iter().position(|tab| Self::header(tab) == header).unwrap_or_else(|| panic!("the menu has no page {header}"));
        self.menu().set_selected_index(index as i32);
        self.shell.settle();
        let page = from_markup_value::<Ref<Control>>(&tabs[index].content()).unwrap_or_else(|| panic!("{header}: the content of the tab is a control"));
        assert!(page.is_attached_to_visual_tree(), "{header}: the page is in the tree of the window");
        let bounds = page.bounds();
        assert!(bounds.width > 0.0 && bounds.height > 0.0, "{header}: the page is laid out ({bounds:?})");
        page
    }

    /// The rectangle of a visual in the frames of the window.
    fn rect(&self, visual: &Visual) -> (f64, f64, f64, f64) {
        Shell::frame_rect_of(&self.window, visual)
    }

    fn frame(&self) -> Frame {
        self.shell.last_frame(0)
    }

    /// The position of a point of a visual in the window.
    fn position(&self, visual: &Visual, point: Point) -> Point {
        visual.translate_point(point, &self.window).expect("the visual is in the tree of the window")
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

/// The texts of the text blocks below `root` that are shown.
fn texts(root: &Visual) -> Vec<String> {
    descendants::<TextBlock>(root)
        .into_iter()
        .filter(|text_block| text_block.is_effectively_visible())
        .filter_map(|text_block| text_block.text())
        .collect()
}

/// The menu sorts its pages by their headers when it is loaded, as the upstream control does,
/// and every page of the document is a page of the menu.
#[test]
fn the_menu_shows_every_page_sorted_by_header() {
    let demo = Demo::start();
    let headers: Vec<String> = demo.tabs().iter().map(|tab| Demo::header(tab)).collect();
    let mut expected: Vec<String> = PAGES.iter().map(|page| page.to_string()).collect();
    expected.sort_by_key(|header| header.to_uppercase());
    assert_eq!(headers, expected);
    // The header of the selected page is the title the menu shows.
    let page = demo.show("Brushes");
    assert!(page.cast::<BrushesPage>().is_some());
    assert!(texts(&demo.menu()).iter().any(|text| text == "Brushes"));
}

/// Opaque colours of a frame (red, green, blue, alpha).
const BLACK: [u8; 4] = [0, 0, 0, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 128, 0, 255];
const LIME: [u8; 4] = [0, 255, 0, 255];
const FUCHSIA: [u8; 4] = [255, 0, 255, 255];
const DARK_BLUE: [u8; 4] = [0, 0, 139, 255];
const PURPLE: [u8; 4] = [128, 0, 128, 255];
const LIGHT_BLUE: [u8; 4] = [173, 216, 230, 255];
const ORANGE_RED: [u8; 4] = [255, 69, 0, 255];

/// The pages whose content changes from frame to frame without any input: an animation of a
/// style on the global clock, a composition animation, a control that asks for its next
/// frame while it draws.
const MOVING: [&str; 8] =
    ["Animations", "Custom Animator", "Spring Animation", "Clipping", "Hit Testing", "SkCanvas", "RenderTargetBitmap", "WriteableBitmap"];

/// The pixels of the rectangle of the page that change within `frames` frames.
fn moved(demo: &Demo, page: &Visual, frames: usize) -> usize {
    let rect = demo.rect(page);
    let before = demo.frame();
    for _ in 0..frames {
        demo.shell.frame();
    }
    demo.frame().difference(&before, rect)
}

/// Every page is selected, laid out and rendered through the compositor: its rectangle of
/// the frame shows more than its background, and the pages that animate without input draw
/// another frame after a few ticks of the clock. No binding of a page reports an error.
#[test]
fn every_page_is_laid_out_and_rendered() {
    let demo = Demo::start();
    let frames_before = demo.shell.frames(0);
    for header in PAGES {
        let page = demo.show(header);
        let rect = demo.rect(&page);
        let colors = demo.frame().colors(rect);
        assert!(colors > 1, "{header}: the page drew nothing into {rect:?}");
        if MOVING.contains(&header) {
            assert!(moved(&demo, &page, 5) > 0, "{header}: the page draws the same frame after five ticks");
        }
    }
    assert!(demo.shell.frames(0) > frames_before, "the compositor drew frames of the window");
    let reports: Vec<(String, String, usize)> =
        demo.shell.take_binding_reports().into_iter().map(|(report, count)| ("tour".to_string(), report.line(), count)).collect();
    sample_testing::assert_accepted(&reports, ACCEPTED_BINDING_REPORTS);
}

/// The binding errors the tour of the pages is known to report, each with its count. The
/// managed original makes each of them too: the template of the hamburger menu binds members
/// of the selected item of the menu, the menu has no selected item while it sorts its pages
/// (it clears its items and adds them again when it is loaded), and a null in the middle of
/// a path is an error of the binding upstream too (`ExpressionNode.ValidateNonNullSource`,
/// logged by `BindingExpression.OnNodeError` at the level `Warning`).
const ACCEPTED_BINDING_REPORTS: &[(&str, &str, usize)] = &[
    // HamburgerMenu.xaml: the scroll bars of the content take the attached properties of the selected item.
    ("tour", "ScrollViewer.HorizontalScrollBarVisibility <- $templatedParent.SelectedItem.HorizontalScrollBarVisibility at SelectedItem: Value is null.", 1),
    ("tour", "ScrollViewer.VerticalScrollBarVisibility <- $templatedParent.SelectedItem.VerticalScrollBarVisibility at SelectedItem: Value is null.", 1),
    // HamburgerMenu.xaml: the title is the header of the selected item (with an empty fallback value).
    ("tour", "TextBlock.Text <- $parent[TabControl].SelectedItem.Header at SelectedItem: Value is null.", 1),
];

/// The animation speed page: its sliders write their values to the view model when they are
/// bound (`Mode=OneWayToSource`), so the speed ratio of the animations is zero until a slider
/// is moved, as in the managed original; with a speed ratio the texts rotate.
#[test]
fn the_animation_speed_page_rotates_its_texts_at_the_speed_of_the_slider() {
    let demo = Demo::start();
    let page = demo.show("Animation Speed");
    let view_model = from_markup_value::<std::rc::Rc<crate::view_models::AnimationSpeedPageViewModel>>(&page.data_context())
        .expect("the view model of the page");
    assert_eq!(view_model.speed_ratio(), 0.0);
    assert_eq!(moved(&demo, &page, 5), 0, "nothing rotates at a speed ratio of zero");

    let sliders = descendants::<Slider>(&page);
    assert_eq!(sliders.len(), 3);
    sliders[0].set_value(ferroui_controls::primitives::RangeBase::value_property(), 1.0);
    demo.shell.settle();
    assert_eq!(view_model.speed_ratio(), 1.0);
    assert!(moved(&demo, &page, 3) > 0, "the texts rotate at a speed ratio of one");
}

/// The custom animator page: the text of the page is what the animator of the sample makes
/// of the progress of the animation, a growing start of the digits.
#[test]
fn the_custom_animator_page_animates_its_text_with_the_animator_of_the_sample() {
    let demo = Demo::start();
    let page = demo.show("Custom Animator");
    let text_block = descendants::<TextBlock>(&page).into_iter().next().expect("the text block of the page");
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..12 {
        demo.shell.frame();
        let text = text_block.text().unwrap_or_default();
        assert!("0123456789".starts_with(&text), "{text:?} is a start of the digits");
        seen.insert(text);
    }
    assert!(seen.len() >= 8, "the text grows with the clock: {seen:?}");
    // Leaving the page ends the animation with its value at a progress of one (R002).
    demo.show("Brushes");
}

/// The transitions page: its elements change when the pointer is over them, over the time of
/// their transitions.
#[test]
fn the_transitions_page_changes_under_the_pointer() {
    let demo = Demo::start();
    let page = demo.show("Transitions");
    let rect = demo.rect(&page);
    let still = demo.frame();
    let mut changed = 0;
    for row in 1..6 {
        for column in 1..6 {
            let point = Point::new(rect.0 + (rect.2 - rect.0) * f64::from(column) / 6.0, rect.1 + (HEIGHT - rect.1) * f64::from(row) / 6.0);
            demo.shell.pointer_move(0, point);
            demo.shell.frame();
            if demo.frame().difference(&still, rect) > 0 {
                changed += 1;
            }
        }
    }
    assert!(changed > 0, "no element of the page changed under the pointer");
}

/// The hit testing page: four thousand cells, a fifth of them moved by composition
/// animations, and the statistics of the hit tests the page runs with every composition
/// update.
#[test]
fn the_hit_testing_page_builds_its_scene_and_animates_it_in_the_compositor() {
    let demo = Demo::start();
    let page = demo.show("Hit Testing");
    assert!(page.cast::<HitTestingPage>().is_some());
    let statistics = texts(&page).into_iter().find(|text| text.starts_with("Visuals: ")).expect("the statistics of the page");
    assert!(statistics.starts_with("Visuals: 4000 (800 animated), Hit tests/frame: 256, "), "{statistics}");
    assert!(descendants::<ferroui_controls::Border>(&page).len() >= 4000);
    assert!(moved(&demo, &page, 3) > 0, "the composition animations move the cells");
}

/// The geometry hit testing page: the probe follows the pointer over the scene, and the
/// status names the shapes the geometry of the probe intersects.
#[test]
fn the_geometry_hit_testing_page_reports_the_shapes_under_the_probe() {
    let demo = Demo::start();
    let page = demo.show("Geometry Hit Testing");
    let status = page.get_control::<TextBlock>("Status");
    let scene = page.get_control::<ferroui_controls::Grid>("Scene");
    assert_eq!(status.text().as_deref(), Some("No intersection"));

    let bounds = scene.bounds();
    let mut reported = std::collections::BTreeSet::new();
    for row in 1..8 {
        for column in 1..8 {
            let point = Point::new(bounds.width * f64::from(column) / 8.0, bounds.height * f64::from(row) / 8.0);
            demo.shell.pointer_move(0, demo.position(&scene, point));
            reported.insert(status.text().unwrap_or_default());
        }
    }
    let intersecting: Vec<&String> = reported.iter().filter(|text| text.starts_with("Intersecting, ")).collect();
    assert!(!intersecting.is_empty(), "the probe intersects no shape anywhere: {reported:?}");
    for shape in ["Ellipse", "Rectangle", "Star", "Line"] {
        assert!(intersecting.iter().any(|text| text.contains(shape)), "the probe never intersects {shape}: {reported:?}");
    }

    demo.shell.pointer_leave(0);
    assert_eq!(status.text().as_deref(), Some("No intersection"));
}

/// The pages that draw through the drawing context with fixed colours: each colour of the
/// control is in the frame.
#[test]
fn the_custom_drawn_pages_draw_their_colours() {
    let demo = Demo::start();
    let count = |page: &Ref<Control>, color: [u8; 4]| demo.frame().count(demo.rect(page), color, 0);

    // The fuchsia square of the render target bitmap, rotated by the time.
    let page = demo.show("RenderTargetBitmap");
    assert!(page.cast::<RenderTargetBitmapPage>().is_some());
    assert!(count(&page, FUCHSIA) > 5000, "the bitmap shows its square");

    // The line of the line bounds control and the rectangle of its bounds.
    let page = demo.show("LineBounds");
    assert!(count(&page, GREEN) > 1000, "the green line");
    // A black pen of one unit between the pixels: shades of grey.
    let (frame, rect) = (demo.frame(), demo.rect(&page));
    let mut outline = 0;
    for y in rect.1 as usize..(rect.3.min(HEIGHT) as usize) {
        for x in rect.0 as usize..rect.2 as usize {
            let [red, green, blue, _] = frame.pixel(x, y);
            if red == green && green == blue && red < 200 {
                outline += 1;
            }
        }
    }
    assert!(outline > 100, "the rectangle of the bounds of the line: {outline} grey pixels");

    // The path, three segments of it and the bounds of its stroke, drawn into a bitmap.
    let page = demo.show("Path Measurement");
    for (color, name) in [(DARK_BLUE, "the path"), (PURPLE, "the first segment"), (GREEN, "the second segment"), (LIGHT_BLUE, "the third segment")] {
        assert!(count(&page, color) > 100, "{name}");
    }

    // The resize pattern: the background, the frame at the edges, the circles.
    let page = demo.show("Resize Pattern");
    let rect = demo.rect(&page);
    let area = ((rect.2 - rect.0) * (rect.3.min(HEIGHT) - rect.1)) as usize;
    assert!(count(&page, [16, 16, 24, 255]) > area / 2, "the background");
    assert!(count(&page, LIME) > 1000, "the frame at the edges");
    assert!(count(&page, ORANGE_RED) > 1000, "the circles");
    assert!(descendants::<crate::controls::ResizePattern>(&page).len() == 1);

    // The glyph runs: one drawn as a glyph run, one as its geometry.
    let page = demo.show("GlyphRun");
    assert!(count(&page, BLACK) > 0, "the glyph run");
    assert!(count(&page, GREEN) > 0, "the geometry of the glyph run");

    // The formatted text, its geometry in the gradient and its highlight geometry.
    let page = demo.show("FormattedText");
    assert!(count(&page, BLACK) > 100, "the text");
    assert!(demo.frame().colors(demo.rect(&page)) > 1000, "the gradient of the run, the geometry and the highlight");

    // The canvas of the backend: gradients, noise and a blur.
    let page = demo.show("SkCanvas");
    assert!(demo.frame().colors(demo.rect(&page)) > 10000, "the shaders of the Skia canvas");
}

/// The writeable bitmap page draws the same green three ways over red: a bitmap that is not
/// premultiplied, a premultiplied one, and a brush with an opacity. The three squares are one
/// colour at every alpha.
#[test]
fn the_writeable_bitmap_page_blends_its_three_squares_alike() {
    let demo = Demo::start();
    let page = demo.show("WriteableBitmap");
    assert!(page.cast::<WriteableBitmapPage>().is_some());
    let origin = demo.position(&page, Point::new(0.0, 0.0));
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..6 {
        demo.shell.frame();
        let frame = demo.frame();
        let pixel = |x: f64| frame.pixel((origin.x + x) as usize, (origin.y + 100.0) as usize);
        let (unpremultiplied, premultiplied, brush) = (pixel(100.0), pixel(356.0), pixel(530.0));
        for channel in 0..4 {
            assert!(unpremultiplied[channel].abs_diff(premultiplied[channel]) <= 2, "{unpremultiplied:?} and {premultiplied:?}");
            assert!(unpremultiplied[channel].abs_diff(brush[channel]) <= 2, "{unpremultiplied:?} and {brush:?}");
        }
        assert_eq!(unpremultiplied[2], 0, "red and green only");
        seen.insert(unpremultiplied);
    }
    assert!(seen.len() > 1, "the alpha follows the time: {seen:?}");
    assert!(seen.iter().any(|pixel| *pixel != RED), "the squares are blended over the red");
}

/// The text formatter page formats a line of two texts with a button between them, and
/// arranges the button where the line has it.
#[test]
fn the_text_formatter_page_lays_out_a_control_inside_a_line() {
    let demo = Demo::start();
    let page = demo.show("TextFormatter");
    assert!(texts(&page).iter().any(|text| text == "ClickMe"));
    let button = descendants::<ferroui_controls::Button>(&page).into_iter().next().expect("the button of the line");
    let bounds = button.bounds();
    assert!(bounds.x > 20.0, "the button follows the first text of the line: {bounds:?}");
    assert!(bounds.width > 0.0 && bounds.height > 0.0);
    // The text before and the text after the button are drawn.
    let rect = demo.rect(&page);
    let frame = demo.frame();
    assert!(frame.count((rect.0, rect.1, rect.0 + bounds.x, rect.1 + 40.0), BLACK, 0) > 0, "the text before the button");
    assert!(frame.count((rect.0 + bounds.x + bounds.width, rect.1, rect.2, rect.1 + 40.0), BLACK, 0) > 0, "the text after the button");
}

/// The view model of the main window switches the debug overlays of the renderer (the menu
/// of the flyout of the window toggles its properties) and gives the window its size.
#[test]
fn the_view_model_of_the_window_switches_the_debug_overlays() {
    use ferroui_base::rendering::RendererDebugOverlays;
    let demo = Demo::start();
    let view_model = from_markup_value::<std::rc::Rc<crate::view_models::MainWindowViewModel>>(&demo.window.data_context())
        .expect("the view model of the window");
    // The frame counter is on from the start.
    assert_eq!(demo.window.renderer_diagnostics().debug_overlays(), RendererDebugOverlays::FPS);
    view_model.set_draw_dirty_rects(true);
    view_model.set_draw_fps(false);
    assert_eq!(demo.window.renderer_diagnostics().debug_overlays(), RendererDebugOverlays::DIRTY_RECTS);
    demo.shell.settle();
    view_model.set_draw_dirty_rects(false);
    assert_eq!(demo.window.renderer_diagnostics().debug_overlays(), RendererDebugOverlays::NONE);
    // The window has the size of its view model, both ways.
    assert_eq!((demo.window.width(), demo.window.height()), (WIDTH, HEIGHT));
    view_model.set_width(820.0);
    assert_eq!(demo.window.width(), 820.0);
}
