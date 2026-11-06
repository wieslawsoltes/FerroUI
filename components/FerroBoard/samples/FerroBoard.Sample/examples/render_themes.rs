//! Opt-in real-renderer check of the compiled sample with isolated local data.
use ferroboard_core::{keys::KeyboardUiMode, settings::ThemeMode};
use ferroboard_sample::{
    App,
    composition::SampleSession,
    sample_options::SampleOptions,
    sample_storage::{FileSampleStoragePlatform, SampleStorage},
};
use ferroui_base::{
    Ref, Visual,
    controls::NameScope,
    media::{
        Color,
        imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions},
    },
    threading::Dispatcher,
};
use ferroui_controls::{AppBuilder, TextBlock, TextBox, TopLevel};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    HeadlessWindowExtensions,
};
use ferroui_system::threading::CancellationToken;
use serde_json::json;
use std::{fs, path::PathBuf, rc::Rc, sync::Arc};

fn collect_header_colors(root: &Ref<Visual>, colors: &mut Vec<Color>) {
    let header = root
        .cast::<TextBlock>()
        .filter(|part| part.name().as_deref() == Some("Header"));
    if let Some(header) = header {
        let foreground = header.foreground().expect("settings header foreground");
        colors.push(
            foreground
                .as_solid_color_brush()
                .expect("solid header foreground")
                .color(),
        );
    }
    for index in 0..root.visual_children().count() {
        collect_header_colors(&root.visual_children().get(index), colors);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: render_themes NEW_OUTPUT_DIRECTORY")?,
    );
    if output.exists() {
        return Err("output directory must be new to preserve earlier rendering evidence".into());
    }
    fs::create_dir_all(&output)?;
    let session = HeadlessUnitTestSession::start_new(|| {
        let builder = AppBuilder::configure::<App>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..FerroHeadlessPlatformOptions::default()
            });
        assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("Skia"));
        assert_eq!(
            builder.text_shaping_subsystem_name().as_deref(),
            Some("HarfBuzz")
        );
        builder
    });
    session.dispatch(move || {
        let storage = SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(output.join("data"))));
        let sample = SampleSession::with_options(storage, Arc::new(SampleOptions::default()))
            .expect("compiled sample session");
        sample.host.set_is_animated(false);
        sample.keyboard.settings().set_clipboard_history_enabled(false);
        sample.keyboard.settings().set_clipboard_suggestion(false);
        sample.keyboard.settings().set_learn_words(false);
        sample.bindings.model().set_settings_pane_open(true);
        sample.window.set_width(1440.0);
        sample.window.set_height(1100.0);
        sample.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = NameScope::get_name_scope(&sample.window).expect("compiled window scope");
        let message = scope.find("MessageBox").expect("message input").cast::<TextBox>().expect("native TextBox");
        message.set_text(Some("Hello — café · العربية · 日本語 · 👩🏽‍💻"));
        assert!(message.focus(), "the native field must accept focus");
        Dispatcher::ui_thread().run_jobs(None);
        assert!(sample.keyboard.engine().target().is_some(), "host must attach the focused target");
        let original_target = sample.keyboard.engine().target().unwrap();
        let top_level = sample.window.cast::<TopLevel>().expect("sample TopLevel");
        let mut frames = Vec::new();
        let mut capture = |name: &str| {
            Dispatcher::ui_thread().run_jobs(None);
            assert!(message.is_focused(), "rendering and panel changes must retain field focus");
            assert!(Rc::ptr_eq(&original_target, &sample.keyboard.engine().target().expect("editing target")));
            let mut header_colors = Vec::new();
            collect_header_colors(&top_level.cast::<Visual>().unwrap(), &mut header_colors);
            assert!(header_colors.len() >= 3, "all settings header families must be present");
            let expected = Color::parse(if sample.keyboard.settings().theme_mode() == ThemeMode::Dark {
                "#FFFFFFFF"
            } else {
                "#E4000000"
            }).unwrap();
            for color in &header_colors {
                assert_eq!(*color, expected, "settings header must follow the active WinUI theme");
            }
            let bitmap = top_level.capture_rendered_frame().expect("real Skia rendered frame");
            let path = output.join(format!("{name}.png"));
            let options = BitmapEncoderOptions::Png(PngBitmapEncoderOptions::default());
            let mut file = fs::File::create(&path).expect("create frame output");
            bitmap.save(&mut file, &options).expect("save actual rendered frame");
            frames.push(json!({"name": name, "file": format!("{name}.png"), "pixel_size": format!("{:?}", bitmap.pixel_size()), "settings_headers_checked": header_colors.len(), "editing_target_and_focus_preserved": true}));
        };
        for (name, mode) in [("light-keyboard", ThemeMode::Light), ("dark-keyboard", ThemeMode::Dark)] {
            sample.keyboard.settings().set_theme_mode(mode);
            capture(name);
        }
        // Seed only synthetic history in this example's isolated storage, using public engine operations.
        sample.keyboard.engine().input_emoji(&"😀".into()).expect("insert synthetic emoji");
        sample.keyboard.engine().input_emoji(&"👩🏽‍💻".into()).expect("insert synthetic emoji sequence");
        sample.keyboard.engine().input_emoji(&"🇺🇳".into()).expect("insert synthetic flag sequence");
        sample.keyboard.engine().set_ui_mode(KeyboardUiMode::Media).expect("show actual emoji panel");
        capture("dark-emoji");
        sample.keyboard.settings().set_theme_mode(ThemeMode::Light);
        capture("light-emoji");
        sample.window.close();
        sample.release();
        fs::write(output.join("result.json"), serde_json::to_vec_pretty(&json!({
            "renderer": "Skia", "shaper": "HarfBuzz", "windowing": "Headless", "frames": frames,
            "compiled_sample": true, "isolated_data": "data", "clipboard_history": false,
            "operating_system": std::env::consts::OS, "architecture": std::env::consts::ARCH,
            "ferroboard_version": env!("CARGO_PKG_VERSION"),
            "native_os_input_and_accessibility": "unverified", "pixel_goldens": "not asserted; system fonts may vary"
        })).expect("serialize rendering report")).expect("write rendering report");
    }, CancellationToken::none()).wait().expect("Headless rendering task");
    println!("Compiled sample frames rendered with Skia/HarfBuzz and Headless windowing.");
    Ok(())
}
