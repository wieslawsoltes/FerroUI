//! The text of the ProGPU backend beside the same text of the Skia backend: one scene of glyph
//! runs drawn by both into offscreen targets, written as images with their difference, and the
//! numbers of the difference.
//!
//! ```text
//! cargo run -p ferroui-progpu --example progpu_text_quality -- <output folder>
//! ```
//!
//! The scene is Latin text of 9 to 32 pixels in a regular and a bold face, Arabic, Hebrew and
//! Japanese text, and text in a colour, once dark on light and once light on dark; below it,
//! text of simulated styles, which is counted apart. Three renderings are made:
//!
//! - `skia`: the Skia backend with its font manager and the HarfBuzz shaper (the reference);
//! - `progpu`: the ProGPU backend with its font manager and the shaper of the engine;
//! - `progpu-harfbuzz`: the ProGPU backend drawing the glyphs the HarfBuzz shaper placed, which
//!   leaves the rasterization as the only difference to the reference.
//!
//! The images are encoded by the Skia backend, which has the codec.

#[cfg(target_os = "macos")]
mod example {
    use ferroui_base::media::imaging::PngBitmapEncoderOptions;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::text_formatting::{GlyphInfo, TextShaperOptions};
    use ferroui_base::media::{BoxShadows, Color, FontSimulations, GlyphTypeface, IBrush};
    use ferroui_base::platform::{AlphaFormat, IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl, PixelFormat};
    use ferroui_base::utilities::ReadOnlyMemory;
    use ferroui_base::{FerroLocator, PixelSize, Point, Rect, RoundedRect, Vector};
    use std::collections::HashMap;
    use std::fmt::Write as _;
    use std::path::Path;
    use std::rc::Rc;

    macro_rules! font {
        ($folder:literal, $file:literal) => {
            include_bytes!(concat!("../../../Skia/FerroUI.Skia/test_assets/", $folder, "/", $file)).as_slice()
        };
    }

    /// The fonts of the scene, by index.
    const FONTS: [(&str, &[u8]); 5] = [
        ("Inter Regular", font!("assets", "Inter-Regular.ttf")),
        ("Inter Bold", font!("assets", "Inter-Bold.ttf")),
        ("Noto Sans Arabic", font!("assets", "NotoSansArabic-Regular.ttf")),
        ("Noto Sans Hebrew", font!("assets", "NotoSansHebrew-Regular.ttf")),
        ("MiSans", font!("assets", "MiSans-Normal.ttf")),
    ];

    const LATIN: &str = "Sphinx of black quartz, judge my vow. 0123456789 fi fl (@#&) Waffle Type";
    const ARABIC: &str = "مرحبا بالعالم، هذا نص تجريبي للعرض";
    const HEBREW: &str = "שלום עולם, זה טקסט לדוגמה";
    const JAPANESE: &str = "日本語のテキスト表示";

    const SIZES: [f64; 12] = [9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0];

    const PANEL_WIDTH: i32 = 560;
    const MARGIN: f64 = 12.0;

    /// The two panels: the background and the colour of the text.
    const PANELS: [(&str, [u8; 3], [u8; 3]); 2] = [("dark on light", [255, 255, 255], [0, 0, 0]), ("light on dark", [30, 30, 30], [255, 255, 255])];

    /// The colour of the lines in a colour, on both panels.
    const ACCENT: [u8; 3] = [0, 120, 212];

    /// A line of the scene, in the coordinates of its panel.
    struct Line {
        label: String,
        font: usize,
        simulations: FontSimulations,
        text: String,
        size: f64,
        bidi_level: i8,
        /// The start of the baseline; both parts have a fraction.
        origin: Point,
        /// The rows of the line.
        rows: std::ops::Range<i32>,
        accent: bool,
    }

    struct Scene {
        lines: Vec<Line>,
        /// The first row of the lines of simulated styles.
        simulated_from: i32,
        height: i32,
    }

    /// The lines of a panel as they are added, top to bottom.
    struct Layout {
        lines: Vec<Line>,
        top: f64,
    }

    impl Layout {
        #[allow(clippy::too_many_arguments)]
        fn add(&mut self, label: String, font: usize, simulations: FontSimulations, text: &str, size: f64, bidi_level: i8, leading: f64, accent: bool) {
            const X_FRACTIONS: [f64; 8] = [0.0, 0.25, 0.5, 0.75, 0.37, 0.61, 0.18, 0.88];
            const Y_FRACTIONS: [f64; 4] = [0.0, 0.3, 0.6, 0.8];
            let index = self.lines.len();
            let height = (size * leading).ceil();
            let baseline = (self.top + size * (leading - 0.35)).floor() + Y_FRACTIONS[index % Y_FRACTIONS.len()];
            // As many characters as fit the panel.
            let budget = ((f64::from(PANEL_WIDTH) - 2.0 * MARGIN) / (size * 0.62)).floor() as usize;
            self.lines.push(Line {
                label,
                font,
                simulations,
                text: text.chars().take(budget).collect(),
                size,
                bidi_level,
                origin: Point::new(MARGIN + X_FRACTIONS[index % X_FRACTIONS.len()], baseline),
                rows: self.top as i32..(self.top + height) as i32,
                accent,
            });
            self.top += height;
        }
    }

    fn scene() -> Scene {
        let none = FontSimulations::None;
        let mut layout = Layout { lines: Vec::new(), top: 8.0 };
        for size in SIZES {
            layout.add(format!("latin regular {size}"), 0, none, LATIN, size, 0, 1.35, false);
            layout.add(format!("latin bold {size}"), 1, none, LATIN, size, 0, 1.35, false);
        }
        for size in [11.0, 14.0, 20.0, 28.0] {
            layout.add(format!("arabic {size}"), 2, none, ARABIC, size, 1, 1.8, false);
        }
        for size in [11.0, 14.0, 20.0] {
            layout.add(format!("hebrew {size}"), 3, none, HEBREW, size, 1, 1.5, false);
        }
        for size in [12.0, 16.0, 24.0] {
            layout.add(format!("japanese {size}"), 4, none, JAPANESE, size, 0, 1.5, false);
        }
        for size in [12.0, 16.0] {
            layout.add(format!("latin colour {size}"), 0, none, LATIN, size, 0, 1.35, true);
        }
        layout.top += 8.0;
        let simulated_from = layout.top as i32;
        for size in [12.0, 16.0, 24.0] {
            layout.add(format!("simulated bold {size}"), 0, FontSimulations::Bold, LATIN, size, 0, 1.35, false);
            layout.add(format!("simulated oblique {size}"), 0, FontSimulations::Oblique, LATIN, size, 0, 1.35, false);
        }
        Scene { lines: layout.lines, simulated_from, height: layout.top as i32 + 8 }
    }

    /// An opaque image: rows of RGBA bytes.
    #[derive(Clone)]
    struct Image {
        width: i32,
        height: i32,
        rgba: Vec<u8>,
    }

    impl Image {
        fn new(width: i32, height: i32) -> Self {
            Image { width, height, rgba: vec![255; (width * height * 4) as usize] }
        }

        fn rgb(&self, x: i32, y: i32) -> [u8; 3] {
            let index = ((y * self.width + x) * 4) as usize;
            [self.rgba[index], self.rgba[index + 1], self.rgba[index + 2]]
        }

        fn set(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
            let index = ((y * self.width + x) * 4) as usize;
            self.rgba[index..index + 3].copy_from_slice(&rgb);
        }
    }

    /// A renderer of the scene: a render interface, a font manager and a shaper.
    struct Stack {
        name: &'static str,
        render: Rc<dyn IPlatformRenderInterface>,
        fonts: Rc<dyn IFontManagerImpl>,
        shaper: Rc<dyn ITextShaperImpl>,
    }

    fn brush(rgb: [u8; 3]) -> ImmutableSolidColorBrush {
        ImmutableSolidColorBrush::new(Color::from_rgb(rgb[0], rgb[1], rgb[2]))
    }

    /// The scene drawn by a stack, and the count of glyphs the fonts do not have.
    fn draw(stack: &Stack, scene: &Scene) -> (Image, usize) {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(stack.render.clone())
            .bind::<dyn IFontManagerImpl>()
            .to_constant(stack.fonts.clone())
            .bind::<dyn ITextShaperImpl>()
            .to_constant(stack.shaper.clone());

        let (width, height) = (PANEL_WIDTH * PANELS.len() as i32, scene.height);
        let target = stack.render.create_render_target_bitmap(PixelSize::new(width, height), Vector::new(96.0, 96.0));
        let mut typefaces: HashMap<(usize, FontSimulations), Rc<GlyphTypeface>> = HashMap::new();
        let mut missing = 0;
        let mut context = target.create_drawing_context();
        context.clear(Color::from_rgb(255, 255, 255));
        for (panel, (_, background, foreground)) in PANELS.iter().enumerate() {
            let left = f64::from(PANEL_WIDTH) * panel as f64;
            let background = brush(*background);
            context.draw_rectangle(
                Some(&background as &dyn IBrush),
                None,
                RoundedRect::from(Rect::new(left, 0.0, f64::from(PANEL_WIDTH), f64::from(height))),
                &BoxShadows::default(),
            );
            let (foreground, accent) = (brush(*foreground), brush(ACCENT));
            for line in &scene.lines {
                let simulations = line.simulations;
                let glyph_typeface = typefaces
                    .entry((line.font, simulations))
                    .or_insert_with(|| {
                        let typeface = stack.fonts.try_create_glyph_typeface_from_stream(&mut &FONTS[line.font].1[..], simulations).expect("the font is read");
                        GlyphTypeface::new(typeface, simulations).expect("the tables of the font are read")
                    })
                    .clone();
                let units: Vec<u16> = line.text.encode_utf16().collect();
                let options = TextShaperOptions::with_all(glyph_typeface.clone(), line.size, line.bidi_level, None, 0.0, 0.0, None);
                let glyphs: Vec<GlyphInfo> = stack.shaper.shape_text(&ReadOnlyMemory::from_vec(units), &options).glyph_infos().to_vec();
                if panel == 0 {
                    missing += glyphs.iter().filter(|glyph| glyph.glyph_index == 0).count();
                }
                let origin = Point::new(left + line.origin.x, line.origin.y);
                let run = stack.render.create_glyph_run(&glyph_typeface, line.size, &glyphs, origin);
                let brush: &dyn IBrush = if line.accent { &accent } else { &foreground };
                context.draw_glyph_run(Some(brush), &*run);
            }
        }
        context.dispose();

        let mut image = Image::new(width, height);
        let framebuffer = target.lock();
        let row_bytes = framebuffer.row_bytes() as usize;
        let bgra = framebuffer.format() == PixelFormat::BGRA8888;
        framebuffer.with_data(&mut |data| {
            for y in 0..height as usize {
                let row = &data[y * row_bytes..y * row_bytes + width as usize * 4];
                for (x, pixel) in row.chunks_exact(4).enumerate() {
                    let rgb = if bgra { [pixel[2], pixel[1], pixel[0]] } else { [pixel[0], pixel[1], pixel[2]] };
                    image.set(x as i32, y as i32, rgb);
                }
            }
        });
        framebuffer.dispose();
        for glyph_typeface in typefaces.into_values() {
            glyph_typeface.dispose();
        }
        scope.dispose();
        println!("{}: drawn, {missing} glyphs missing in the fonts", stack.name);
        (image, missing)
    }

    /// The difference of two images in a box of pixels.
    #[derive(Default)]
    struct Difference {
        pixels: u64,
        max: [u8; 3],
        sum: [u64; 3],
        above_8: u64,
        above_32: u64,
        /// The pixels that are not the background in one of the images, and their part of the sums.
        inked: u64,
        inked_sum: [u64; 3],
        inked_above_8: u64,
        /// The ink of the two images: the distance of the green channel to the background.
        ink: [u64; 2],
    }

    impl Difference {
        fn of(reference: &Image, other: &Image, columns: std::ops::Range<i32>, rows: std::ops::Range<i32>, background: [u8; 3]) -> Self {
            let mut difference = Difference::default();
            for y in rows {
                for x in columns.clone() {
                    let (a, b) = (reference.rgb(x, y), other.rgb(x, y));
                    let distance = [a[0].abs_diff(b[0]), a[1].abs_diff(b[1]), a[2].abs_diff(b[2])];
                    let largest = distance.into_iter().max().unwrap_or(0);
                    let inked = a != background || b != background;
                    difference.pixels += 1;
                    difference.above_8 += u64::from(largest > 8);
                    difference.above_32 += u64::from(largest > 32);
                    difference.inked += u64::from(inked);
                    difference.inked_above_8 += u64::from(inked && largest > 8);
                    for channel in 0..3 {
                        difference.max[channel] = difference.max[channel].max(distance[channel]);
                        difference.sum[channel] += u64::from(distance[channel]);
                        if inked {
                            difference.inked_sum[channel] += u64::from(distance[channel]);
                        }
                    }
                    difference.ink[0] += u64::from(a[1].abs_diff(background[1]));
                    difference.ink[1] += u64::from(b[1].abs_diff(background[1]));
                }
            }
            difference
        }

        fn add(&mut self, other: &Difference) {
            self.pixels += other.pixels;
            self.above_8 += other.above_8;
            self.above_32 += other.above_32;
            self.inked += other.inked;
            self.inked_above_8 += other.inked_above_8;
            for channel in 0..3 {
                self.max[channel] = self.max[channel].max(other.max[channel]);
                self.sum[channel] += other.sum[channel];
                self.inked_sum[channel] += other.inked_sum[channel];
            }
            self.ink[0] += other.ink[0];
            self.ink[1] += other.ink[1];
        }

        fn ink_ratio(&self) -> f64 {
            self.ink[1] as f64 / self.ink[0].max(1) as f64
        }

        fn describe(&self) -> String {
            let mean = |sum: [u64; 3], count: u64| sum.map(|value| value as f64 / count.max(1) as f64);
            let (all, inked) = (mean(self.sum, self.pixels), mean(self.inked_sum, self.inked));
            format!(
                "max {:>3} {:>3} {:>3}  mean {:.3} {:.3} {:.3}  above 8: {:.3}%  above 32: {:.3}%  |  text pixels {:.2}%: mean {:.2} {:.2} {:.2}, above 8: {:.2}%  |  ink ratio {:.4}",
                self.max[0],
                self.max[1],
                self.max[2],
                all[0],
                all[1],
                all[2],
                100.0 * self.above_8 as f64 / self.pixels.max(1) as f64,
                100.0 * self.above_32 as f64 / self.pixels.max(1) as f64,
                100.0 * self.inked as f64 / self.pixels.max(1) as f64,
                inked[0],
                inked[1],
                inked[2],
                100.0 * self.inked_above_8 as f64 / self.inked.max(1) as f64,
                self.ink_ratio(),
            )
        }
    }

    /// What differs between the reference and another rendering, as text.
    fn report(scene: &Scene, name: &str, reference: &Image, other: &Image) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "== skia against {name} ==");
        let _ = writeln!(text, "(channels are red green blue, of 255; a text pixel is one that is not the background in one of the two images;");
        let _ = writeln!(text, " the ink ratio is the summed coverage of {name} over that of skia)");
        let mut whole = Difference::default();
        for (panel, (panel_name, background, _)) in PANELS.iter().enumerate() {
            let columns = PANEL_WIDTH * panel as i32..PANEL_WIDTH * (panel as i32 + 1);
            let difference = Difference::of(reference, other, columns.clone(), 0..scene.simulated_from, *background);
            let _ = writeln!(text, "{panel_name:<28} {}", difference.describe());
            whole.add(&difference);
        }
        let _ = writeln!(text, "{:<28} {}", "both panels", whole.describe());
        for (panel, (panel_name, background, _)) in PANELS.iter().enumerate() {
            let columns = PANEL_WIDTH * panel as i32..PANEL_WIDTH * (panel as i32 + 1);
            let difference = Difference::of(reference, other, columns, scene.simulated_from..scene.height, *background);
            let _ = writeln!(text, "{:<28} {}", format!("simulated, {panel_name}"), difference.describe());
        }
        let _ = writeln!(text, "-- by line --");
        for line in &scene.lines {
            let mut parts = String::new();
            for (panel, (panel_name, background, _)) in PANELS.iter().enumerate() {
                let columns = PANEL_WIDTH * panel as i32..PANEL_WIDTH * (panel as i32 + 1);
                let difference = Difference::of(reference, other, columns, line.rows.clone(), *background);
                let mean = difference.inked_sum[1] as f64 / difference.inked.max(1) as f64;
                let _ = write!(
                    parts,
                    "  {panel_name}: ink {:.4} max {:>3} text mean {:>5.2} above 8 {:>5.2}%",
                    difference.ink_ratio(),
                    difference.max[1],
                    mean,
                    100.0 * difference.inked_above_8 as f64 / difference.inked.max(1) as f64
                );
            }
            let _ = writeln!(text, "{:<24}{parts}", line.label);
        }
        text
    }

    /// The difference of two images, four times as strong.
    fn difference_image(reference: &Image, other: &Image) -> Image {
        let mut image = Image::new(reference.width, reference.height);
        for y in 0..reference.height {
            for x in 0..reference.width {
                let (a, b) = (reference.rgb(x, y), other.rgb(x, y));
                image.set(x, y, [0, 1, 2].map(|channel: usize| a[channel].abs_diff(b[channel]).saturating_mul(4)));
            }
        }
        image
    }

    /// A box of the images enlarged without smoothing: the first image over the second.
    fn enlarged(first: &Image, second: &Image, columns: std::ops::Range<i32>, rows: std::ops::Range<i32>, factor: i32) -> Image {
        let (width, height) = (columns.len() as i32, rows.len() as i32);
        let mut image = Image::new(width * factor, height * factor * 2);
        for (part, source) in [first, second].into_iter().enumerate() {
            for y in 0..height * factor {
                for x in 0..width * factor {
                    image.set(x, y + part as i32 * height * factor, source.rgb(columns.start + x / factor, rows.start + y / factor));
                }
            }
        }
        image
    }

    fn save(encoder: &dyn IPlatformRenderInterface, image: &Image, path: &Path) {
        let bitmap = encoder.load_bitmap_from_pixels(
            PixelFormat::RGBA8888,
            AlphaFormat::Opaque,
            &image.rgba,
            PixelSize::new(image.width, image.height),
            Vector::new(96.0, 96.0),
            image.width * 4,
        );
        let mut file = std::fs::File::create(path).expect("the image file is created");
        bitmap.save(&mut file, &PngBitmapEncoderOptions::DEFAULT.into()).expect("the image is written");
        bitmap.dispose();
        println!("wrote {}", path.display());
    }

    pub fn main() {
        let folder = std::env::args().nth(1).unwrap_or_else(|| String::from("."));
        let folder = Path::new(&folder);
        std::fs::create_dir_all(folder).expect("the output folder");

        let skia_render: Rc<dyn IPlatformRenderInterface> = Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None));
        let progpu_render: Rc<dyn IPlatformRenderInterface> = Rc::new(ferroui_progpu::ProGpuPlatformRenderInterface::new());
        let progpu_fonts: Rc<dyn IFontManagerImpl> = Rc::new(ferroui_progpu::ProGpuFontManager::new());
        let harfbuzz: Rc<dyn ITextShaperImpl> = Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new());
        let stacks = [
            Stack { name: "skia", render: skia_render.clone(), fonts: Rc::new(ferroui_skia::FontManagerImpl::new()), shaper: harfbuzz.clone() },
            Stack { name: "progpu", render: progpu_render.clone(), fonts: progpu_fonts.clone(), shaper: Rc::new(ferroui_progpu::ProGpuTextShaper::new()) },
            Stack { name: "progpu-harfbuzz", render: progpu_render, fonts: progpu_fonts, shaper: harfbuzz },
        ];

        let scene = scene();
        let images: Vec<Image> = stacks.iter().map(|stack| draw(stack, &scene).0).collect();
        let mut text = String::new();
        for (stack, image) in stacks.iter().zip(&images) {
            save(&*skia_render, image, &folder.join(format!("text-quality-{}.png", stack.name)));
            if stack.name != "skia" {
                save(&*skia_render, &difference_image(&images[0], image), &folder.join(format!("text-quality-difference-{}.png", stack.name)));
                text.push_str(&report(&scene, stack.name, &images[0], image));
                text.push('\n');
            }
        }
        // The small sizes, where a difference is seen first: Skia over ProGPU, four times as large.
        let small = scene.lines.iter().filter(|line| line.size <= 14.0 && line.label.starts_with("latin") && !line.accent);
        let rows = small.clone().map(|line| line.rows.start).min().unwrap_or(0)..small.map(|line| line.rows.end).max().unwrap_or(0);
        for (panel, name) in ["dark-on-light", "light-on-dark"].into_iter().enumerate() {
            let columns = PANEL_WIDTH * panel as i32..PANEL_WIDTH * panel as i32 + 300;
            save(&*skia_render, &enlarged(&images[0], &images[1], columns, rows.clone(), 4), &folder.join(format!("text-quality-small-{name}-skia-over-progpu.png")));
        }
        print!("{text}");
        let numbers = folder.join("text-quality.txt");
        std::fs::write(&numbers, text).expect("the numbers are written");
        println!("wrote {}", numbers.display());
    }
}

#[cfg(target_os = "macos")]
fn main() {
    example::main();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the comparison with the Skia backend is made on macOS");
}
