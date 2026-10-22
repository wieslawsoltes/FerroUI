//! Captures the actual WinUI-themed gallery with Skia on the headless platform.
//! These are rendering smoke artifacts, not upstream pixel-parity baselines.

use ferroui_base::media::imaging::PngBitmapEncoderOptions;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Rect, Ref};
use ferroui_controls::{
    AppBuilder, Application, AutoSuggestBox, ComboBox, Control, Image, ScrollViewer, Window,
};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessWindowExtensions,
};
use ferroui_skia::SkiaApplicationExtensions;
use std::error::Error;
use std::path::PathBuf;
use winui_gallery::{App, GalleryView};

fn main() -> Result<(), Box<dyn Error>> {
    let mut pages = vec!["Button".to_string()];
    let mut output_dir = PathBuf::from("gallery-captures");
    let mut dark = false;
    let mut query = None;
    let mut control_name = None;
    let mut explicit_pages = false;
    let mut all_pages = false;
    for argument in std::env::args().skip(1) {
        if let Some(value) = argument.strip_prefix("--pages=") {
            pages = value.split(',').map(str::to_string).collect();
            explicit_pages = true;
        } else if let Some(value) = argument.strip_prefix("--output-dir=") {
            output_dir = value.into();
        } else if argument == "--dark" {
            dark = true;
        } else if let Some(value) = argument.strip_prefix("--query=") {
            query = Some(value.to_string());
        } else if let Some(value) = argument.strip_prefix("--control=") {
            if value.is_empty()
                || value.len() > 128
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
            {
                return Err(
                    "--control must contain 1–128 ASCII letters, digits, underscores or hyphens."
                        .into(),
                );
            }
            control_name = Some(value.to_string());
        } else if argument == "--all" {
            all_pages = true;
            pages = winui_gallery::CATALOG
                .iter()
                .map(|page| page.id.to_string())
                .collect();
        } else {
            return Err(format!("Unknown argument: {argument}. Use --pages=Button,NumberBox, --all, --dark, --query=Button, --control=NAME or --output-dir=PATH.").into());
        }
    }
    if control_name.is_some() && (!explicit_pages || all_pages || pages.len() != 1) {
        return Err("--control requires exactly one explicitly selected --pages=PAGE and cannot be used with --all.".into());
    }
    for page in &pages {
        if winui_gallery::catalog::find(page).is_none() {
            return Err(format!("Unknown catalog page: {page}").into());
        }
    }
    std::fs::create_dir_all(&output_dir)?;
    let _builder = AppBuilder::configure::<App>()
        .use_harfbuzz()
        .use_skia()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..FerroHeadlessPlatformOptions::default()
        })
        .setup_without_starting();
    Application::current()
        .expect("application initialized")
        .set_requested_theme_variant(Some(if dark {
            ThemeVariant::dark()
        } else {
            ThemeVariant::light()
        }));
    let gallery = GalleryView::with_preview_options(
        ferroui_markup_xaml_loader::winui::WinUIXamlOptions::for_winui_theme(),
    )?;
    gallery
        .root()
        .get_control::<ComboBox>("Theme")
        .set_selected_index(if dark { 2 } else { 1 });
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(900.0);
    window.set_content(Some(Control::boxed(gallery.root())));
    window.show();
    for page in pages {
        eprintln!("Rendering {page}");
        gallery.navigate(&page);
        if let Some(error) = gallery.native_example_error() {
            return Err(format!("{page}: native example failed to load: {error}").into());
        }
        if page == "AutoSuggestBox" {
            if let Some(query) = &query {
                let native = gallery
                    .native_example()
                    .ok_or("AutoSuggestBox example missing")?;
                let search = native.get_control::<AutoSuggestBox>("ControlSuggestions");
                search.set_text(query);
                search.submit_query();
                if native.get_control::<Control>("ControlDetails").is_visible()
                    && native
                        .get_control::<Image>("ControlImage")
                        .source()
                        .is_none()
                {
                    return Err(
                        "The control detail image could not be decoded by the real renderer."
                            .into(),
                    );
                }
            }
        }
        let capture_target = if let Some(name) = &control_name {
            // Realize the sample's native template before looking for its control.
            // Restrict the search to the native sample, excluding Gallery chrome
            // and original-source viewers with unrelated names.
            window.update_layout();
            let native = gallery.native_example().ok_or_else(|| {
                format!("{page}: there is no native sample for --control={name}.")
            })?;
            let matches: Vec<Ref<Control>> = native
                .get_self_and_visual_descendants()
                .filter_map(|visual| visual.cast::<Control>())
                .filter(|control| control.name().as_deref() == Some(name.as_str()))
                .collect();
            if matches.len() != 1 {
                return Err(format!("{page}: --control={name} matched {} rendered native controls; exactly one is required.", matches.len()).into());
            }
            let target = matches.into_iter().next().expect("one matching control");
            if !target.is_effectively_visible() {
                return Err(format!("{page}: --control={name} is hidden; activate its sample state before capturing it.").into());
            }
            // A native ScrollContentPresenter handles a request when its offset
            // changes. Repeat after layout so an inner viewer can settle before
            // the same request reaches its outer viewer. Never set offsets here.
            let scroll_ancestors: Vec<_> = target
                .get_visual_ancestors()
                .filter_map(|visual| visual.cast::<ScrollViewer>())
                .collect();
            for _ in 0..=scroll_ancestors.len() {
                let before: Vec<_> = scroll_ancestors
                    .iter()
                    .map(|viewer| viewer.offset())
                    .collect();
                target.bring_into_view();
                window.update_layout();
                if scroll_ancestors
                    .iter()
                    .map(|viewer| viewer.offset())
                    .eq(before)
                {
                    break;
                }
            }
            Some(target)
        } else {
            None
        };
        let frame = window
            .capture_rendered_frame()
            .ok_or("The renderer produced no frame.")?;
        if let Some(target) = &capture_target {
            verify_target_visible(target, &window)?;
        }
        let file_name = match &control_name {
            Some(name) => format!("{page}--{name}.png"),
            None => format!("{page}.png"),
        };
        let path = output_dir.join(file_name);
        frame.save_to_file(
            path.to_str().ok_or("The output path is not valid UTF-8.")?,
            &PngBitmapEncoderOptions::default().into(),
        )?;
        println!("{}: {}", page, path.display());
    }
    window.close();
    Ok(())
}

/// Verify that native scrolling brought some real pixels of the requested control
/// through every rectangular ancestor clip and into the captured window. Oversize
/// examples can occupy more than one viewport; this is not a full-page screenshot.
fn verify_target_visible(target: &Control, window: &Window) -> Result<(), Box<dyn Error>> {
    let transform = target
        .transform_to_visual(window)
        .ok_or("The requested control is not attached to the capture window.")?;
    let bounds = Rect::from_size(target.bounds().size()).transform_to_aabb(transform);
    if !bounds.x.is_finite()
        || !bounds.y.is_finite()
        || !bounds.width.is_finite()
        || !bounds.height.is_finite()
        || target.opacity() <= 0.0
    {
        return Err("The requested control has invalid bounds or is fully transparent.".into());
    }
    let mut visible = bounds.intersect(Rect::from_size(window.bounds().size()));
    let mut offsets = Vec::new();
    for ancestor in target.get_visual_ancestors() {
        if ancestor.opacity() <= 0.0 {
            return Err("An ancestor of the requested control is fully transparent.".into());
        }
        if ancestor.clip_to_bounds() {
            let transform = ancestor
                .transform_to_visual(window)
                .ok_or("A clipping ancestor is detached from the capture window.")?;
            let clip = Rect::from_size(ancestor.bounds().size()).transform_to_aabb(transform);
            visible = visible.intersect(clip);
        }
        if let Some(scroll) = ancestor.cast::<ScrollViewer>() {
            offsets.push(scroll.offset());
        }
    }
    if !visible.width.is_finite()
        || !visible.height.is_finite()
        || visible.width < 1.0
        || visible.height < 1.0
    {
        return Err(format!("The requested control remained outside the rendered viewport after BringIntoView: bounds={bounds:?}, visible={visible:?}.").into());
    }
    eprintln!("Capture target {}: bounds={bounds:?}, visible={visible:?}, ancestor scroll offsets={offsets:?}", target.name().as_deref().unwrap_or("<unnamed>"));
    Ok(())
}
