//! Layout-sized SVG rendering through the same native bitmap path as other icons.
use ferroui_base::{
    PixelSize, Rect, Size, Vector, WeakRef,
    media::{DrawingContext, IImage, imaging::Bitmap},
    platform::{AlphaFormat, PixelFormat},
};
use ferroui_controls::{Image, TopLevel};
use resvg::{tiny_skia, usvg};
use std::{
    any::Any,
    cell::RefCell,
    io,
    rc::Rc,
    sync::{Arc, LazyLock},
};

static FONTS: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut fonts = usvg::fontdb::Database::new();
    fonts.load_system_fonts();
    Arc::new(fonts)
});

struct SvgImage {
    tree: usvg::Tree,
    target: WeakRef<Image>,
    raster: RefCell<Option<Rc<Bitmap>>>,
}

pub(super) fn decode_svg(bytes: &[u8], target: WeakRef<Image>) -> io::Result<Rc<dyn IImage>> {
    let options = usvg::Options {
        fontdb: FONTS.clone(),
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            ..Default::default()
        },
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Rc::new(SvgImage {
        tree,
        target,
        raster: RefCell::default(),
    }))
}

impl IImage for SvgImage {
    fn size(&self) -> Size {
        Size::new(
            f64::from(self.tree.size().width()),
            f64::from(self.tree.size().height()),
        )
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source: Rect, destination: Rect) {
        let natural = self.size();
        let viewport = self.target.upgrade().map_or(destination.size(), |image| {
            let scale =
                TopLevel::get_top_level(Some(&image)).map_or(1., |root| root.render_scaling());
            image.bounds().size() * scale
        });
        let scale = (viewport.width / natural.width).min(viewport.height / natural.height);
        if !scale.is_finite() || scale <= 0. {
            return;
        }
        let scale = scale.min(4_096. / natural.width.max(natural.height));
        let size = PixelSize::new(
            (natural.width * scale).round().max(1.) as i32,
            (natural.height * scale).round().max(1.) as i32,
        );
        let scale_x = f64::from(size.width) / natural.width;
        let scale_y = f64::from(size.height) / natural.height;
        let mut cache = self.raster.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|bitmap| bitmap.pixel_size() != size)
        {
            let Some(mut pixels) = tiny_skia::Pixmap::new(size.width as u32, size.height as u32)
            else {
                return;
            };
            resvg::render(
                &self.tree,
                tiny_skia::Transform::from_scale(scale_x as f32, scale_y as f32),
                &mut pixels.as_mut(),
            );
            *cache = Some(Rc::new(Bitmap::from_pixels(
                PixelFormat::RGBA8888,
                AlphaFormat::Premul,
                pixels.data(),
                size,
                Vector::new(96., 96.),
                size.width * 4,
            )));
        }
        let bitmap = cache.as_ref().unwrap().clone();
        drop(cache);
        // Native bitmap source rectangles are in raster pixels. The image
        // wrapper passes a rectangle in the SVG's own coordinate space.
        let source_pixels = Rect::new(
            source.x * scale_x,
            source.y * scale_y,
            source.width * scale_x,
            source.height * scale_y,
        );
        bitmap.draw(context, source_pixels, destination);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
