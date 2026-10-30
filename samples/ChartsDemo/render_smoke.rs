//! A real CPU backend and real glyph shaping, with a mock window host and
//! a manual animation clock so captures show completed theme transitions.
//! Images are evidence of native rendering, not upstream pixel comparisons.

use crate::ChartsDemo;
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, PixelSize, Rect, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, UserControl, Window};
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_themes_winui::WinUITheme;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

struct CaptureClock {
    subject: LightweightSubject<TimeSpan>,
    seconds: Cell<i32>,
    state: Cell<PlayState>,
}

impl CaptureClock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            subject: LightweightSubject::new(),
            seconds: Cell::new(0),
            state: Cell::new(PlayState::Run),
        })
    }

    fn finish_transitions(&self) {
        // The first pulse initializes new animation subscriptions; the second
        // advances beyond the WinUI theme's brief brush transitions.
        for _ in 0..2 {
            self.subject
                .on_next(TimeSpan::from_seconds(f64::from(self.seconds.get())));
            self.seconds.set(self.seconds.get() + 1);
            Dispatcher::ui_thread().run_jobs(None);
        }
    }
}

impl IObservable<TimeSpan> for CaptureClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for CaptureClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}

impl IGlobalClock for CaptureClock {}

fn render(view: &UserControl, path: &Path) -> Vec<u8> {
    let size = Size::new(1120.0, 880.0);
    view.measure(size);
    view.arrange(Rect::from_size(size));
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1120, 880));
    bitmap.render(&view.to_ref().upcast());
    let mut pixels = vec![0; 1120 * 880 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 1120, 880), &mut pixels, 1120 * 4);
    let colored = pixels
        .chunks_exact(4)
        .filter(|p| p[0].abs_diff(p[1]) > 24 || p[1].abs_diff(p[2]) > 24)
        .count();
    assert!(colored > 1500, "The actual backend image must contain the colored chart paths, fills and markers ({colored} pixels)");
    bitmap
        .save_to_file(
            path.to_str().unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    println!("Rendered {}", path.display());
    pixels
}

#[test]
fn charts_demo_renders_native_light_dark_and_live_mutations() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let mut services = TestServices::styled_window();
    // Carry the real services into the test application's nested locator scope.
    // An absent render service explicitly masks the outer scope's registration.
    services.render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>();
    services.font_manager_impl = FerroLocator::current().get_service::<dyn IFontManagerImpl>();
    services.text_shaper_impl = FerroLocator::current().get_service::<dyn ITextShaperImpl>();
    let clock = CaptureClock::new();
    services.global_clock = Some(clock.clone());
    services.theme = Some(Rc::new(|| WinUITheme::new().as_style()));
    let _app = UnitTestApplication::start(services);
    let demo = ChartsDemo::new();
    let view = demo.view();
    let window = Window::new();
    window.set_width(1120.0);
    window.set_height(880.0);
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    window.update_layout();
    clock.finish_transitions();

    let directory = std::env::var_os("FERROUI_CHARTS_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-charts-render"));
    std::fs::create_dir_all(&directory).unwrap();
    let light = render(&view, &directory.join("charts-light.png"));
    demo.toggle_theme();
    clock.finish_transitions();
    let dark = render(&view, &directory.join("charts-dark.png"));
    assert_ne!(
        light, dark,
        "A native theme change must alter rendered pixels"
    );
    demo.add_sample();
    demo.toggle_bars();
    demo.toggle_legend();
    clock.finish_transitions();
    let changed = render(&view, &directory.join("charts-live-mutation.png"));
    assert_ne!(
        dark, changed,
        "Live samples, orientation and legends must alter rendered pixels"
    );
    assert_eq!(demo.sample_count(), 7);
    window.close();
    drop(window);
    drop(view);
    drop(demo);
    drop(_app);
    locator.dispose();
}
