use ferroui_base::{
    media::{immutable::ImmutableSolidColorBrush, text_formatting::TextShaperOptions, *},
    platform::*,
    utilities::ReadOnlyMemory,
    FerroLocator, Matrix, PixelSize, Point, Rect, RoundedRect, Vector,
};
use ferroui_rendering_benchmarks::{distribution, Backend};
use serde_json::{json, Value};
use std::{hint::black_box, path::PathBuf, rc::Rc, sync::Arc, time::Instant};

fn arg(name: &str, default: &str) -> String {
    let a: Vec<_> = std::env::args().collect();
    a.windows(2)
        .find(|x| x[0] == name)
        .map_or(default.into(), |x| x[1].clone())
}
fn main() {
    if std::env::args().any(|a| a == "--list") {
        println!("{}", Backend::available().join(","));
        return;
    }
    if let Err(e) = run() {
        eprintln!("benchmark failed: {e}");
        std::process::exit(1);
    }
}
#[cfg(not(target_os = "macos"))]
fn run() -> Result<(), String> {
    Err("The initial GPU surface adapter is macOS Metal; add a platform surface adapter to run GPU workloads elsewhere".into())
}
#[cfg(target_os = "macos")]
fn run() -> Result<(), String> {
    use ferroui_base::platform::surfaces::IPlatformRenderSurface;
    use ferroui_metal::offscreen::{
        OffscreenFramePresentation, OffscreenMetalGraphics, OffscreenMetalSurface,
    };
    let output = PathBuf::from(arg("--output", "target/rendering-benchmarks/run.json"));
    let samples: usize = arg("--samples", "200")
        .parse()
        .map_err(|_| "invalid samples")?;
    let warmup: usize = arg("--warmup", "30")
        .parse()
        .map_err(|_| "invalid warmup")?;
    let scale: f64 = arg("--scale", "2").parse().map_err(|_| "invalid scale")?;
    if samples < 10 || warmup < 1 || !scale.is_finite() || scale <= 0. || scale > 4. {
        return Err("Use samples >= 10, warmup >= 1 and scale in (0,4]".into());
    }
    let only = arg("--workload", "all");
    let backend_name = arg("--backend", "skia");
    let start = Instant::now();
    let backend = Backend::new(&backend_name)?;
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(backend.renderer.clone())
        .bind::<dyn IFontManagerImpl>()
        .to_constant(backend.fonts.clone())
        .bind::<dyn ITextShaperImpl>()
        .to_constant(backend.shaper.clone());
    let graphics = OffscreenMetalGraphics::try_new().ok_or("Metal device unavailable")?;
    let device = graphics.create_context();
    let context = backend.renderer.create_backend_context(Some(device));
    if context.is_lost() {
        return Err("backend context lost".into());
    }
    let setup_ms = start.elapsed().as_secs_f64() * 1000.;
    let fonts = Fonts::new(&backend)?;
    let size = PixelSize::new((1280. * scale) as i32, (800. * scale) as i32);
    let info = RenderTargetSceneInfo::new(
        size,
        scale,
        ferroui_base::rendering::composition::CompositionTransparencyLevel::None,
    );
    let mut rows = Vec::new();
    for name in [
        "rounded-rects",
        "geometry",
        "bitmaps",
        "text-latin",
        "text-arabic",
        "text-hebrew",
        "shadows",
        "opacity-layers",
    ] {
        if only != "all" && only != name {
            continue;
        }
        let scene = Scene::new(&backend, &fonts, name)?;
        // Read back once, outside every timed interval, and reject blank/incorrect frames.
        let kept = OffscreenMetalSurface::new(
            size,
            scale,
            OffscreenFramePresentation::Kept,
            renderer_wait(&backend_name),
        );
        let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![kept.clone()];
        if !context.is_ready_to_create_render_target(&surfaces) {
            return Err(format!(
                "{backend_name} cannot draw to Metal; no software fallback accepted"
            ));
        }
        let target = context.create_render_target(&surfaces);
        frame(&*target, &info, &scene, scale, 0)?;
        let (_, pixels) = kept.last_frame().ok_or("missing validation frame")?;
        if pixels.get(..4) != Some(&[0, 0, 255, 255]) {
            return Err(format!("{name}: missing red validation marker"));
        }
        let ink = pixels
            .chunks_exact(4)
            .enumerate()
            .filter(|(index, p)| {
                let x = index % size.width as usize;
                let y = index / size.width as usize;
                (x as f64 >= 16. * scale || y as f64 >= 16. * scale)
                    && (p[0] < 240 || p[1] < 240 || p[2] < 240)
            })
            .count();
        if ink < 500 {
            return Err(format!("{name}: empty validation scene"));
        }
        if arg("--capture", "false") == "true" {
            let path = output.with_file_name(format!("{backend_name}-{name}.bgra"));
            if let Some(p) = path.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            std::fs::write(path, &pixels).map_err(|e| e.to_string())?;
        }
        target.dispose();
        let surface = OffscreenMetalSurface::new(
            size,
            scale,
            OffscreenFramePresentation::Drawn,
            renderer_wait(&backend_name),
        );
        let target =
            context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
        for i in 0..warmup {
            frame(&*target, &info, &scene, scale, i)?;
        }
        let before = surface.frames();
        let mut completed = Vec::with_capacity(samples);
        let mut cpu = Vec::with_capacity(samples);
        let mut waits = Vec::with_capacity(samples);
        for i in 0..samples {
            let w = surface.waited();
            let t = Instant::now();
            frame(&*target, &info, &scene, scale, i)?;
            let total = t.elapsed();
            let wait = surface.waited() - w;
            completed.push(total.as_secs_f64() * 1000.);
            cpu.push(total.saturating_sub(wait).as_secs_f64() * 1000.);
            waits.push(wait.as_secs_f64() * 1000.);
        }
        if surface.frames() - before != samples as u32
            || context.is_lost()
            || target.platform_render_target_state().is_corrupted
        {
            return Err(format!("{name}: dropped frames or lost GPU context"));
        }
        rows.push(json!({"name":name,"kind":"render","operations_per_frame":scene.operations(),"validation_ink_pixels":ink,"completed":distribution(&completed),"cpu_before_wait":distribution(&cpu),"wait":distribution(&waits),"samples_ms":completed,"cpu_samples_ms":cpu}));
        target.dispose();
    }
    // Time shaping with preloaded fixed font bytes and UTF-16 input, not font discovery or fallback.
    for case in &fonts.0 {
        if only != "all" && only != format!("shape-{}", case.name) {
            continue;
        }
        let options =
            TextShaperOptions::with_all(case.face.clone(), 16., case.bidi, None, 0., 0., None);
        let text = ReadOnlyMemory::from_str(case.text);
        let shaped = backend.shaper.shape_text(&text, &options);
        let glyphs = shaped.glyph_infos();
        if glyphs.is_empty()
            || glyphs.iter().any(|g| {
                g.glyph_index == 0
                    || g.glyph_cluster < 0
                    || g.glyph_cluster as usize >= case.text.encode_utf16().count()
            })
        {
            return Err(format!("{}: missing glyphs/invalid clusters", case.name));
        }
        let count = glyphs.len();
        drop(glyphs);
        for _ in 0..warmup {
            black_box(backend.shaper.shape_text(&text, &options));
        }
        let mut times = Vec::new();
        for _ in 0..samples {
            let t = Instant::now();
            for _ in 0..32 {
                black_box(
                    backend
                        .shaper
                        .shape_text(black_box(&text), black_box(&options)),
                );
            }
            times.push(t.elapsed().as_secs_f64() * 1000. / 32.);
        }
        rows.push(json!({"name":format!("shape-{}",case.name),"kind":"shape","glyphs":count,"utf16_units":case.text.encode_utf16().count(),"batch_size":32,"completed":distribution(&times),"samples_ms":times}));
    }
    if rows.is_empty() {
        return Err(format!("Unknown workload: {only}"));
    }
    #[cfg(feature = "quartz")]
    let diagnostics = if backend_name.starts_with("quartz") {
        let s = ferroui_quartz::metal_statistics();
        if s[6] != 0 {
            return Err(format!("GPU errors: {s:?}"));
        }
        json!(s)
    } else {
        Value::Null
    };
    #[cfg(not(feature = "quartz"))]
    let diagnostics = Value::Null;
    let report = json!({"schema":1,"backend":backend_name,"text_backend":backend.text_backend,"surface":"Metal offscreen, GPU-completed, private storage","pixels":[size.width,size.height],"scale":scale,"warmup_frames":warmup,"samples":samples,"backend_context_setup_ms":setup_ms,"text_hinting":"Light","text_antialiasing":"Antialias","readback":"one untimed validation frame per scene; none in measured frames","quartz_counters":diagnostics,"rows":rows});
    if let Some(p) = output.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    std::fs::write(&output, serde_json::to_vec_pretty(&report).unwrap())
        .map_err(|e| e.to_string())?;
    context.dispose();
    scope.dispose();
    println!("{}", output.display());
    Ok(())
}
#[cfg(target_os = "macos")]
fn renderer_wait(name: &str) -> Option<Arc<dyn Fn() + Send + Sync>> {
    #[cfg(feature = "vello")]
    if name.starts_with("vello-") {
        return Some(Arc::new(
            ferroui_vello::gpu::VelloWgpuDevice::wait_for_shared_device,
        ));
    }
    let _ = name;
    None
}
struct TextCase {
    name: &'static str,
    text: &'static str,
    bidi: i8,
    face: Rc<GlyphTypeface>,
}
struct Fonts(Vec<TextCase>);
impl Fonts {
    fn new(b: &Backend) -> Result<Self, String> {
        let mut cases = Vec::new();
        for (name, text, bidi, data) in [
            (
                "latin",
                "The quick brown fox: office affine efficient 0123456789.",
                0,
                include_bytes!("../../src/Skia/FerroUI.Skia/test_assets/assets/Inter-Regular.ttf")
                    as &[u8],
            ),
            (
                "arabic",
                "مرحبا بالعالم هذا اختبار للنص العربي وتشكيل الحروف",
                1,
                include_bytes!(
                    "../../src/Skia/FerroUI.Skia/test_assets/assets/NotoSansArabic-Regular.ttf"
                ) as &[u8],
            ),
            (
                "hebrew",
                "שלום עולם בדיקת טקסט בעברית ומספרים 0123456789",
                1,
                include_bytes!(
                    "../../src/Skia/FerroUI.Skia/test_assets/assets/NotoSansHebrew-Regular.ttf"
                ) as &[u8],
            ),
        ] {
            let platform = b
                .fonts
                .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
                .ok_or("font loading failed")?;
            let face = GlyphTypeface::new(platform, FontSimulations::None)
                .map_err(|e| format!("{e:?}"))?;
            cases.push(TextCase {
                name,
                text,
                bidi,
                face,
            });
        }
        Ok(Self(cases))
    }
}
struct Scene {
    name: String,
    runs: Vec<Arc<dyn IGlyphRunImpl>>,
    geometry: Arc<dyn IGeometryImpl>,
    bitmap: Arc<SharedBitmapImpl>,
}
impl Scene {
    fn new(b: &Backend, fonts: &Fonts, name: &str) -> Result<Self, String> {
        let mut runs = Vec::new();
        if let Some(script) = name.strip_prefix("text-") {
            let c = fonts.0.iter().find(|c| c.name == script).unwrap();
            let shaped = b.shaper.shape_text(
                &ReadOnlyMemory::from_str(c.text),
                &TextShaperOptions::with_all(c.face.clone(), 16., c.bidi, None, 0., 0., None),
            );
            for i in 0..96 {
                runs.push(b.renderer.create_glyph_run(
                    &c.face,
                    16.,
                    &shaped.glyph_infos(),
                    Point::new(24. + (i % 2) as f64 * 620., 32. + (i / 2) as f64 * 16.),
                ));
            }
        }
        let mut builder = b.renderer.create_stream_geometry_builder(None);
        builder.begin_figure(Point::new(0., 0.), true);
        builder.cubic_bezier_to(
            Point::new(15., -15.),
            Point::new(45., 55.),
            Point::new(60., 30.),
            true,
        );
        builder.line_to(Point::new(30., 55.), true);
        builder.end_figure(true);
        let geometry = builder.to_geometry();
        builder.dispose();
        let pixels: Vec<_> = (0..64 * 64)
            .flat_map(|i| [(i % 64 * 4) as u8, (i / 64 * 4) as u8, 80, 255])
            .collect();
        let bitmap = b.renderer.load_bitmap_from_pixels(
            PixelFormat::BGRA8888,
            AlphaFormat::Premul,
            &pixels,
            PixelSize::new(64, 64),
            Vector::new(96., 96.),
            256,
        );
        Ok(Self {
            name: name.into(),
            runs,
            geometry,
            bitmap,
        })
    }
    fn operations(&self) -> usize {
        match self.name.as_str() {
            "rounded-rects" | "geometry" => 256,
            "bitmaps" => 96,
            "shadows" | "opacity-layers" => 24,
            _ => self.runs.len(),
        }
    }
    fn draw(&self, c: &mut dyn IDrawingContextImpl, scale: f64, index: usize) {
        let fill = ImmutableSolidColorBrush::new(Color::from_rgb(40, 130, 210));
        let ink = ImmutableSolidColorBrush::new(Colors::BLACK);
        let empty = BoxShadows::default();
        c.clear(Colors::WHITE);
        c.set_transform(Matrix::create_scale(scale, scale));
        for i in 0..self.operations() {
            let x = 20. + (i % 16) as f64 * 77. + (index % 3) as f64 * 0.25;
            let y = 24. + (i / 16) as f64 * 46.;
            match self.name.as_str() {
                "geometry" => {
                    c.set_transform(
                        Matrix::create_translation(x, y) * Matrix::create_scale(scale, scale),
                    );
                    c.draw_geometry(Some(&fill), None, &*self.geometry);
                }
                "bitmaps" => c.draw_bitmap(
                    &*self.bitmap,
                    1.,
                    Rect::new(0., 0., 64., 64.),
                    Rect::new(x, y, 60., 42.),
                ),
                "shadows" => c.draw_rectangle(
                    Some(&fill),
                    None,
                    RoundedRect::from_radius_xy(
                        Rect::new(
                            32. + (i % 6) as f64 * 200.,
                            32. + (i / 6) as f64 * 180.,
                            160.,
                            120.,
                        ),
                        12.,
                        12.,
                    ),
                    &BoxShadows::new(BoxShadow {
                        offset_x: 3.,
                        offset_y: 4.,
                        blur: 12.,
                        color: Color::from_argb(120, 0, 0, 0),
                        ..Default::default()
                    }),
                ),
                "opacity-layers" => {
                    c.push_opacity(0.7, Some(Rect::new(x, y, 140., 90.)));
                    for j in 0..4 {
                        c.draw_rectangle(
                            Some(&fill),
                            None,
                            RoundedRect::from_rect(Rect::new(
                                x + j as f64 * 12.,
                                y + j as f64 * 9.,
                                70.,
                                50.,
                            )),
                            &empty,
                        );
                    }
                    c.pop_opacity();
                }
                n if n.starts_with("text-") => {
                    c.push_text_options(TextOptions {
                        text_hinting_mode: TextHintingMode::Light,
                        text_rendering_mode: TextRenderingMode::Antialias,
                        ..Default::default()
                    });
                    c.draw_glyph_run(Some(&ink), &*self.runs[i]);
                    c.pop_text_options();
                }
                _ => c.draw_rectangle(
                    Some(&fill),
                    None,
                    RoundedRect::from_radius_xy(Rect::new(x, y, 65., 35.), 8., 8.),
                    &empty,
                ),
            }
        }
        c.set_transform(Matrix::create_scale(scale, scale));
        c.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Colors::RED)),
            None,
            RoundedRect::from_rect(Rect::new(0., 0., 16., 16.)),
            &empty,
        );
    }
}
fn frame(
    t: &dyn IRenderTarget,
    info: &RenderTargetSceneInfo,
    s: &Scene,
    scale: f64,
    i: usize,
) -> Result<(), String> {
    let (mut c, _) = t
        .try_create_drawing_context(info)
        .map_err(|e| e.to_string())?;
    s.draw(&mut *c, scale, i);
    c.dispose();
    Ok(())
}
