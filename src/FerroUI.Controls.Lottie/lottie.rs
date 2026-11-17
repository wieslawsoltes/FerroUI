use crate::playback::normalize_elapsed;
use crate::{CompositionAnimatedControl, CompositionAnimation, NormalizeResult};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::platform::IAssetLoader;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::UriRef;
use ferroui_base::{
    DirectProperty, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, instantiate,
};
use ferroui_controls::{ControlImpl, ControlImplExt};
use ferroui_markup_xaml::ServiceProviderExtensions;
use skia_safe::{AutoCanvasRestore, Canvas, skottie::Animation};
use std::cell::Cell;
use std::io::{self, Read};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// Plays a Lottie JSON animation from a FerroUI asset or a desktop file URL.
#[repr(C)]
pub struct Lottie {
    base: CompositionAnimatedControl,
    base_uri: Option<Url>,
    source_request: Cell<u64>,
}

ferro_class!(Lottie: CompositionAnimatedControl);
ferro_class_info!(Lottie {
    markup: {
        namespace: "FerroUI.Controls.Lottie",
        constructors: [
            (Rc<dyn IServiceProvider>) => Lottie::with_service_provider,
        ],
        content: Path,
    },
});
ferro_impl_classes!(Lottie: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

ferro_properties! {
    impl Lottie {
        pub fn path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Self, _>("Path", None)
        }
        pub fn position_seconds_property() -> DirectProperty<Lottie, f64> {
            FerroProperty::register_direct::<Self, _>(
                "PositionSeconds", Self::position_seconds, Some(Self::set_position_seconds), 0.0,
            )
        }
        pub fn duration_seconds_property() -> DirectProperty<Lottie, f64> {
            FerroProperty::register_direct::<Self, _>("DurationSeconds", Self::duration_seconds, None, 0.0)
        }
    }
}

impl FerroObjectImpl for Lottie {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::path_property().as_property() && this.is_loaded() {
            this.load_path();
        } else if change.property() == CompositionAnimatedControl::position_property().as_property()
        {
            this.raise_seconds(Self::position_seconds_property(), change);
        } else if change.property()
            == CompositionAnimatedControl::source_duration_property().as_property()
        {
            this.raise_seconds(Self::duration_seconds_property(), change);
        }
    }
}

impl ControlImpl for Lottie {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        let request = this.source_request.get();
        Self::parent_on_loaded(this, e);
        if this.source_request.get() == request {
            this.load_path();
        }
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        this.next_request();
        Self::parent_on_unloaded(this, e);
        this.set_animation(None, false);
    }
}

impl Lottie {
    pub const INFINITY: i32 = CompositionAnimatedControl::INFINITY;

    pub fn construct(base_uri: Option<Url>) -> Self {
        Self {
            base: CompositionAnimatedControl::construct(),
            base_uri,
            source_request: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        Self::with_base_uri(None)
    }

    pub fn with_base_uri(base_uri: Option<Url>) -> Ref<Self> {
        instantiate(Self::construct(base_uri))
    }

    pub fn with_service_provider(services: Rc<dyn IServiceProvider>) -> Ref<Self> {
        Self::with_base_uri(services.get_context_base_uri())
    }

    pub fn path(&self) -> Option<String> {
        self.get_value(Self::path_property())
    }

    pub fn set_path(&self, path: Option<String>) {
        self.set_value(Self::path_property(), path);
    }

    pub fn play(&self) {
        self.start();
    }

    pub fn position_seconds(&self) -> f64 {
        self.position().as_secs_f64()
    }

    pub fn set_position_seconds(&self, seconds: f64) {
        self.seek_seconds(seconds);
    }

    pub fn duration_seconds(&self) -> f64 {
        self.source_duration().as_secs_f64()
    }

    pub fn seek_seconds(&self, seconds: f64) {
        if seconds.is_finite() {
            // A finite input larger than Duration can represent still seeks to the end.
            let position = Duration::try_from_secs_f64(seconds.max(0.0)).unwrap_or(Duration::MAX);
            self.seek(position);
        }
    }

    pub fn seek_progress(&self, progress: f64) {
        if progress.is_finite() && !self.source_duration().is_zero() {
            self.seek(self.source_duration().mul_f64(progress.clamp(0.0, 1.0)));
        }
    }

    fn raise_seconds(
        &self,
        property: &'static DirectProperty<Self, f64>,
        change: &FerroPropertyChangedEventArgs<'_>,
    ) {
        let (old, new) = change.get_old_and_new_value::<Duration>();
        self.raise_property_changed(
            property.as_property(),
            Some(&old.as_secs_f64()),
            &new.as_secs_f64(),
            BindingPriority::LocalValue,
            true,
        );
    }

    fn next_request(&self) -> u64 {
        let request = self.source_request.get().wrapping_add(1);
        self.source_request.set(request);
        request
    }

    fn load_path(&self) {
        let request = self.next_request();
        self.set_animation(None, false);
        if self.source_request.get() != request {
            return;
        }
        let Some(path) = self.path().filter(|path| !path.is_empty()) else {
            return;
        };
        let uri = match UriRef::parse(&path) {
            Ok(uri) => uri,
            Err(error) => {
                self.publish_load_error(&error.to_string());
                return;
            }
        };
        self.publish_status(Some("Loading animation…".into()));
        if self.source_request.get() != request {
            return;
        }

        let absolute = uri.as_url().cloned().or_else(|| {
            self.base_uri
                .as_ref()
                .and_then(|base| uri.resolve(base).ok())
        });
        if let Some(file_url) = absolute.filter(|absolute| absolute.scheme() == "file") {
            let Ok(file_path) = file_url.to_file_path() else {
                self.publish_load_error("Invalid file URL");
                return;
            };
            let weak = self.to_ref().downgrade();
            let read = ferroui_system::threading::tasks::run(move || {
                let bytes: Arc<[u8]> = std::fs::read(file_path)?.into();
                Ok(bytes)
            });
            Dispatcher::ui_thread().spawn_local(async move {
                let result = read.flatten().await;
                if let Some(control) = weak
                    .upgrade()
                    .filter(|control| control.source_request.get() == request)
                {
                    match result {
                        Ok(bytes) => control.load_animation(bytes),
                        Err(error) => control.publish_load_error(&error.to_string()),
                    }
                }
            });
        } else {
            match self.read_asset(&uri) {
                Ok(bytes) => self.load_animation(bytes),
                Err(error) => self.publish_load_error(&error.to_string()),
            }
        }
    }

    fn read_asset(&self, uri: &UriRef) -> io::Result<Arc<[u8]>> {
        let loader = FerroLocator::current()
            .get_service::<dyn IAssetLoader>()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "No asset loader is available")
            })?;
        let mut stream = loader.open(uri, self.base_uri.as_ref())?;
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        Ok(bytes.into())
    }

    fn load_animation(&self, bytes: Arc<[u8]>) {
        self.set_animation(Some(Arc::new(move || SkottieAnimation::load(&bytes))), true);
    }

    fn publish_load_error(&self, error: &str) {
        self.publish_status(Some(format!("Could not load Lottie animation. {error}")));
    }
}

struct SkottieAnimation {
    animation: Animation,
    size: Size,
    duration: Duration,
}

impl SkottieAnimation {
    fn load(bytes: &[u8]) -> Result<Box<dyn CompositionAnimation>, String> {
        let animation = Animation::from_bytes(bytes).ok_or_else(|| {
            "Could not load Lottie animation. Invalid animation JSON.".to_string()
        })?;
        let size = animation.size();
        let duration = Duration::try_from_secs_f32(animation.duration())
            .map_err(|_| "Could not load Lottie animation. Invalid duration.".to_string())?;
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            return Err("Could not load Lottie animation. Invalid dimensions.".into());
        }
        Ok(Box::new(Self {
            animation,
            size: Size::new(f64::from(size.width), f64::from(size.height)),
            duration,
        }))
    }
}

impl CompositionAnimation for SkottieAnimation {
    fn source_size(&self) -> Size {
        self.size
    }

    fn duration(&self) -> Option<Duration> {
        Some(self.duration)
    }

    fn on_normalize_elapsed(&mut self, elapsed: Duration) -> NormalizeResult {
        normalize_elapsed(elapsed, self.duration)
    }

    fn on_render(
        &mut self,
        canvas: &Canvas,
        destination: Rect,
        position: Duration,
        _running: bool,
    ) {
        let position = position.min(self.duration.saturating_sub(Duration::from_nanos(1)));
        self.animation.seek_frame_time(position.as_secs_f64());
        let _restore = AutoCanvasRestore::guard(canvas, true);
        canvas.translate((destination.x as f32, destination.y as f32));
        canvas.scale((
            (destination.width / self.size.width) as f32,
            (destination.height / self.size.height) as f32,
        ));
        self.animation.render(canvas, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ANIMATION: &[u8] = br##"{
      "v":"5.5.7","fr":20,"ip":0,"op":40,"w":32,"h":16,
      "layers":[{"ty":1,"ind":1,"ip":0,"op":40,"st":0,"sw":8,"sh":8,"sc":"#ff0000",
        "ks":{"a":{"a":0,"k":[0,0,0]},"p":{"a":1,"k":[{"t":0,"s":[0,4,0],"h":1},{"t":20,"s":[24,4,0]}]},
        "s":{"a":0,"k":[100,100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}}]
    }"##;

    #[test]
    fn decodes_size_duration_and_rendered_positions() {
        let mut animation = SkottieAnimation::load(TEST_ANIMATION).unwrap();
        assert_eq!(animation.source_size(), Size::new(32.0, 16.0));
        assert_eq!(animation.duration(), Some(Duration::from_secs(2)));

        let mut surface = skia_safe::surfaces::raster_n32_premul((64, 16)).unwrap();
        let destination = Rect::new(0.0, 0.0, 64.0, 16.0);
        for (position, x) in [(Duration::ZERO, 3), (Duration::from_secs(2), 51)] {
            surface.canvas().clear(skia_safe::Color::TRANSPARENT);
            animation.on_render(surface.canvas(), destination, position, false);
            let mut bitmap = skia_safe::Bitmap::new();
            bitmap.alloc_n32_pixels((64, 16), false);
            assert!(surface.read_pixels_to_bitmap(&bitmap, (0, 0)));
            assert_eq!(bitmap.get_color((x, 8)), skia_safe::Color::RED);
            assert_eq!(bitmap.get_color((32, 8)), skia_safe::Color::TRANSPARENT);
        }
    }

    #[test]
    fn invalid_animation_returns_an_error() {
        assert!(SkottieAnimation::load(b"invalid").is_err());
    }

    #[test]
    fn clipping_and_opacity_apply_to_the_full_destination() {
        use crate::composition_animated_custom_visual_handler::draw_animation;

        let mut animation = SkottieAnimation::load(TEST_ANIMATION).unwrap();
        let mut surface = skia_safe::surfaces::raster_n32_premul((32, 16)).unwrap();
        surface.canvas().clear(skia_safe::Color::TRANSPARENT);
        surface.canvas().translate((2.0, 0.0));
        draw_animation(
            surface.canvas(),
            Rect::new(2.0, 0.0, 24.0, 16.0),
            Rect::new(-4.0, 0.0, 64.0, 16.0),
            0.5,
            &mut *animation,
            Duration::ZERO,
            false,
        );
        let mut bitmap = skia_safe::Bitmap::new();
        bitmap.alloc_n32_pixels((32, 16), false);
        assert!(surface.read_pixels_to_bitmap(&bitmap, (0, 0)));
        assert_eq!(bitmap.get_color((3, 8)), skia_safe::Color::TRANSPARENT);
        assert_eq!(
            bitmap.get_color((4, 8)),
            skia_safe::Color::from_argb(128, 255, 0, 0)
        );
        assert_eq!(bitmap.get_color((14, 8)), skia_safe::Color::TRANSPARENT);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn metal_skia_lease_matches_raster() {
        use crate::composition_animated_custom_visual_handler::{draw_animation, draw_with_lease};
        use ferroui_base::PixelSize;
        use ferroui_base::media::Colors;
        use ferroui_base::platform::surfaces::IPlatformRenderSurface;
        use ferroui_base::platform::{
            IPlatformGraphics, IPlatformRenderInterface, RenderTargetSceneInfo,
        };
        use ferroui_base::rendering::composition::CompositionTransparencyLevel;
        use ferroui_metal::offscreen::{
            OffscreenFramePresentation, OffscreenMetalGraphics, OffscreenMetalSurface,
        };
        use ferroui_skia::{ISkiaApiLeaseFeature, PlatformRenderInterface};
        use std::any::TypeId;

        let Some(graphics) = OffscreenMetalGraphics::try_new() else {
            eprintln!("Metal device unavailable");
            return;
        };
        let interface = PlatformRenderInterface::new(None, None);
        let context = interface.create_backend_context(Some(graphics.create_context()));
        let size = PixelSize::new(64, 16);
        let surface = OffscreenMetalSurface::new(size, 1.0, OffscreenFramePresentation::Kept, None);
        let surfaces = [surface.clone() as Arc<dyn IPlatformRenderSurface>];
        assert!(context.is_ready_to_create_render_target(&surfaces));
        let target = context.create_render_target(&surfaces);
        let info = RenderTargetSceneInfo::new(size, 1.0, CompositionTransparencyLevel::None);
        let viewport = Rect::new(0.0, 0.0, 64.0, 16.0);
        let destination = Rect::new(4.0, 0.0, 56.0, 16.0);
        let mut animation = SkottieAnimation::load(TEST_ANIMATION).unwrap();
        let mut raster = skia_safe::surfaces::raster_n32_premul((64, 16)).unwrap();
        let image_info = skia_safe::ImageInfo::new(
            (64, 16),
            skia_safe::ColorType::BGRA8888,
            skia_safe::AlphaType::Premul,
            None,
        );
        let mut raster_pixels = vec![0u8; 64 * 16 * 4];
        for position in [Duration::ZERO, Duration::from_secs(1)] {
            let (mut drawing, _) = target.try_create_drawing_context(&info).unwrap();
            drawing.clear(Colors::TRANSPARENT);
            let feature = drawing
                .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
                .unwrap();
            let feature = feature
                .downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>()
                .unwrap();
            draw_with_lease(
                &**feature,
                viewport,
                destination,
                &mut *animation,
                position,
                false,
            );
            drawing.dispose();

            raster.canvas().clear(skia_safe::Color::TRANSPARENT);
            draw_animation(
                raster.canvas(),
                viewport,
                destination,
                1.0,
                &mut *animation,
                position,
                false,
            );
            assert!(raster.read_pixels(&image_info, &mut raster_pixels, 64 * 4, (0, 0)));
            let (_, gpu_pixels) = surface.last_frame().unwrap();
            assert_eq!(gpu_pixels.len(), raster_pixels.len());
            assert!(raster_pixels.iter().any(|&channel| channel == 255));
            assert!(
                gpu_pixels
                    .iter()
                    .zip(&raster_pixels)
                    .all(|(gpu, raster)| gpu.abs_diff(*raster) <= 4),
                "GPU and raster differ at {position:?}",
            );
        }
        target.dispose();
    }
}
