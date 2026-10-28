//! Real native icon audit: geometry extent, compact content budgets, presenter templates,
//! raster output at multiple DPIs, and isolation from the application's own icon theme.
use super::*;
use ferroui_base::layout::Orientation;
use ferroui_base::media::{FontWeight, SolidColorBrush, Stretch};
use ferroui_base::{Point, Ref, Thickness, Vector};
use ferroui_controls::primitives::UniformGrid;
use ferroui_controls::shapes::Path as NativePath;
use ferroui_controls::{Border, PathIcon, RowDefinitions, StackPanel, TextBlock};
use std::collections::BTreeSet;

/// Inventory the actual vectors used by the shell and every dynamic icon provider.
/// Keeping their sources here also captures newly added variants in these providers;
/// this is an audit atlas of production artwork, not a second copy of that artwork.
fn builtin_paths() -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for (name, source) in [
        ("Shell", include_str!("Workspace.xaml")),
        ("Actions", include_str!("icons.rs")),
        ("Assets", include_str!("asset_toolbox.rs")),
        ("Layers", include_str!("inspector_controls.rs")),
        ("Library", include_str!("component_browser.rs")),
        ("Properties", include_str!("property_controls.rs")),
        ("Alignment", include_str!("editors.rs")),
        ("Grid tracks", include_str!("grid_track_editor.rs")),
        ("Layout", include_str!("workspace_layout.rs")),
        ("Timeline", include_str!("workspace_timeline.rs")),
    ] {
        let mut ordinal = 0;
        for value in source.split('"') {
            if !value.starts_with("F1 M") && !value.starts_with("F0 M") {
                continue;
            }
            ordinal += 1;
            if seen.insert(value.to_owned()) {
                result.push((format!("{name} {ordinal:02}"), value.to_owned()));
            }
        }
    }
    assert!(
        result.len() >= 70,
        "The audit must cover all production icon families"
    );
    // Custom vectors may have nonzero or negative origins and arbitrary aspect ratios.
    result.push(("Offset origin".into(), "F1 M120 80H160V100H120Z".into()));
    result.push((
        "Negative origin".into(),
        "F0 M-18 -8H-2V8H-18Z M-14 -4V4H-6V-4Z".into(),
    ));
    result
}

fn assert_icon_fits(icon: &PathIcon) {
    let path = icon
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<NativePath>())
        .expect("Every designer icon has a native vector path");
    assert_eq!(path.name().as_deref(), Some("DesignerIconPath"));
    assert_eq!(path.stretch(), Stretch::Uniform);
    let geometry = path
        .rendered_geometry()
        .expect("The icon has real geometry");
    let ink = geometry
        .bounds()
        .transform_to_aabb(path.transform_to_visual(icon).unwrap());
    let frame = Rect::from_size(icon.bounds().size());
    assert!(
        ink.width > 0.0 && ink.height > 0.0,
        "An icon may not be blank: {ink:?}"
    );
    assert!(
        ink.x >= 0.99
            && ink.y >= 0.99
            && ink.right() <= frame.right() - 0.99
            && ink.bottom() <= frame.bottom() - 0.99,
        "The entire glyph must fit inside its optical gutter: ink={ink:?}, frame={frame:?}"
    );
    let center = Point::new(ink.x + ink.width / 2.0, ink.y + ink.height / 2.0);
    assert!(
        (center.x - frame.width / 2.0).abs() <= 0.51
            && (center.y - frame.height / 2.0).abs() <= 0.51,
        "Non-square/offset paths must remain centered: ink={ink:?}, frame={frame:?}"
    );
    let mut parent = icon.visual_parent();
    while let Some(ancestor) = parent {
        if let Some(button) = ancestor.cast::<Button>() {
            let button_ink = geometry
                .bounds()
                .transform_to_aabb(path.transform_to_visual(&button).unwrap());
            let padding = button.padding();
            let border = button.border_thickness();
            assert!(
                button_ink.x >= padding.left + border.left - 0.01
                    && button_ink.y >= padding.top + border.top - 0.01
                    && button_ink.right()
                        <= button.bounds().width - padding.right - border.right + 0.01
                    && button_ink.bottom()
                        <= button.bounds().height - padding.bottom - border.bottom + 0.01,
                "The button must allocate room for its glyph rather than clipping it: ink={button_ink:?}, button={:?}, padding={padding:?}",
                button.bounds()
            );
            break;
        }
        parent = ancestor.visual_parent();
    }
}

fn settle(window: &Window, clock: &RenderClock) {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
}

#[test]
fn native_designer_icon_atlas_preserves_all_geometry_and_compact_presenter_bounds() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let clock = Rc::new(RenderClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
        now: Cell::new(0.0),
    });
    let mut services = TestServices::styled_window();
    services.global_clock = Some(clock.clone());
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1600.0, 1200.0);
        native.max_auto_size_hint.set(Size::new(4096.0, 4096.0));
        let screen = PixelRect::new(0, 0, 4096, 4096);
        native.setup_feature::<dyn IScreenImpl>(MockScreenImpl::new(vec![mock_screen(
            1.0, screen, screen, true,
        )]));
        native.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        native
    }));
    services.render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>();
    services.font_manager_impl = FerroLocator::current().get_service::<dyn IFontManagerImpl>();
    services.text_shaper_impl = FerroLocator::current().get_service::<dyn ITextShaperImpl>();
    services.theme = Some(Rc::new(|| {
        ferroui_themes_winui::WinUITheme::new().as_style()
    }));
    let app = UnitTestApplication::start(services);
    ferroui_themes_winui::register_types();
    let paths = builtin_paths();
    let panel = Grid::new();
    panel.classes().add("designer-chrome");
    panel.styles().add(crate::designer_chrome_style().unwrap());
    panel.set_background(Some(SolidColorBrush::from_uint32(0xff202126).into()));
    panel.set_row_definitions(RowDefinitions::parse("58,*").unwrap());
    let heading = TextBlock::new();
    heading.set_text(Some(&format!(
        "Designer vector audit · {} paths · 14 / 24 px · dark / light",
        paths.len()
    )));
    heading.set_font_size(18.0);
    heading.set_font_weight(FontWeight::SemiBold);
    heading.set_margin(Thickness::new(16.0, 18.0, 0.0, 0.0));
    panel.children().add(heading.upcast());
    let grid = UniformGrid::new();
    grid.set_columns(8);
    grid.set_margin(Thickness::uniform(8.0));
    Grid::set_row(&grid, 1);
    panel.children().add(grid.clone().upcast());
    let mut icons: Vec<Ref<PathIcon>> = Vec::new();
    for (name, data) in &paths {
        let card = StackPanel::new();
        card.set_spacing(6.0);
        card.set_margin(Thickness::uniform(6.0));
        let label = TextBlock::new();
        label.set_text(Some(name));
        label.set_font_size(10.0);
        card.children().add(label.upcast());
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(8.0);
        for light in [false, true] {
            let background = Border::new();
            background.set_background(Some(
                SolidColorBrush::from_uint32(if light { 0xfff2f3f5 } else { 0xff202126 }).into(),
            ));
            background.set_padding(Thickness::uniform(3.0));
            let examples = StackPanel::new();
            examples.set_orientation(Orientation::Horizontal);
            examples.set_spacing(5.0);
            for size in [14.0, 24.0] {
                let icon = crate::icons::vector_icon(data, size);
                icon.set_foreground(Some(
                    SolidColorBrush::from_uint32(if light { 0xff202126 } else { 0xfff2f3f5 })
                        .into(),
                ));
                examples.children().add(icon.clone().upcast());
                icons.push(icon);
            }
            background.set_child(Some(examples.upcast()));
            row.children().add(background.upcast());
        }
        card.children().add(row.upcast());
        grid.children().add(card.upcast());
    }
    let size = Size::new(1152.0, 74.0 + paths.len().div_ceil(8) as f64 * 62.0);
    let window = Window::new();
    window.set_width(size.width);
    window.set_height(size.height);
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    settle(&window, &clock);
    for icon in &icons {
        assert_icon_fits(icon);
    }
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    for scale in [1.0, 1.25, 2.0] {
        let width = (size.width * scale).ceil() as i32;
        let height = (size.height * scale).ceil() as i32;
        let bitmap = RenderTargetBitmap::with_dpi(
            PixelSize::new(width, height),
            Vector::new(96.0 * scale, 96.0 * scale),
        );
        bitmap.render(&panel.clone().upcast());
        let mut pixels = vec![0; width as usize * height as usize * 4];
        bitmap.copy_pixels(PixelRect::new(0, 0, width, height), &mut pixels, width * 4);
        for icon in &icons {
            let rect = Rect::from_size(icon.bounds().size())
                .transform_to_aabb(icon.transform_to_visual(&panel).unwrap());
            let x0 = (rect.x * scale).floor() as i32;
            let y0 = (rect.y * scale).floor() as i32;
            let x1 = (rect.right() * scale).ceil() as i32;
            let y1 = (rect.bottom() * scale).ceil() as i32;
            let mut colors = BTreeSet::new();
            for y in y0.max(0)..y1.min(height) {
                for x in x0.max(0)..x1.min(width) {
                    let i = (y as usize * width as usize + x as usize) * 4;
                    colors.insert((pixels[i], pixels[i + 1], pixels[i + 2]));
                }
            }
            assert!(
                colors.len() >= 2,
                "Every icon must produce visible native ink at {scale}x: {rect:?}"
            );
        }
        if scale == 1.0 {
            bitmap
                .save_to_file(
                    directory.join("designer-icon-atlas.png").to_str().unwrap(),
                    &PngBitmapEncoderOptions::default().into(),
                )
                .unwrap();
        }
        bitmap.dispose();
    }
    window.close();
    drop(window);

    let workspace = DesignerWorkspace::new(r#"<StackPanel xmlns="https://github.com/ferroui"><PathIcon Name="ApplicationIcon" Width="22" Height="22" Data="M2 2H20V20H2Z"/></StackPanel>"#).unwrap();
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    settle(&window, &clock);
    let mut checked = 0;
    for visual in workspace.control().get_visual_descendants() {
        let Some(icon) = visual.cast::<PathIcon>() else {
            continue;
        };
        if icon.classes().contains("designer-icon")
            && icon.is_effectively_visible()
            && icon.bounds().width > 0.0
        {
            assert_icon_fits(&icon);
            checked += 1;
        }
    }
    assert!(
        checked >= 25,
        "Commandbar, shelf, toolbar and inspector icons were visited ({checked})"
    );
    let application_icon = workspace
        .control()
        .get_visual_descendants()
        .find_map(|v| {
            v.cast::<PathIcon>()
                .filter(|i| i.name().as_deref() == Some("ApplicationIcon"))
        })
        .unwrap();
    assert!(!application_icon.classes().contains("designer-icon"));
    assert!(!application_icon.get_visual_descendants().any(|v| {
        v.cast::<NativePath>()
            .is_some_and(|p| p.name().as_deref() == Some("DesignerIconPath"))
    }));
    println!(
        "Native designer icon audit: {} production/custom paths, {} attached chrome icons, 1x/1.25x/2x raster coverage",
        paths.len(),
        checked
    );
    window.close();
    drop(window);
    drop(workspace);
    drop(app);
    locator.dispose();
}
