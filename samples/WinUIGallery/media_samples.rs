//! Pinned Gallery Image and FlipView examples using existing native controls.
//!
//! FlipView maps to Carousel; PageSlide selects the navigation/gesture axis and
//! PipsPager exposes navigation. Native Carousel also accepts mouse swipes. Its
//! transitions, focus transfer, RTL behavior and chrome are not asserted to match
//! WinUI FlipView. The bound example retains the actual page's 36px centered
//! image and 60px caption row, rather than its different instructional snippet.
//!
//! The fixed-size NineGrid examples compose native CroppedBitmap, ImageDrawing,
//! DrawingGroup and DrawingImage objects. No compatibility property is added to
//! Image. SVG decoding and GIF playback remain visible pending features.

use ferroui_base::animation::{PageSlide, SlideAxis, TimeSpan};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::imaging::{Bitmap, BitmapInterpolationMode, CroppedBitmap};
use ferroui_base::media::{
    Brushes, Color, DrawingGroup, DrawingImage, IImage, ImageDrawing, SolidColorBrush, Stretch,
};
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{BoxedValue, PixelRect, Rect, Ref, Thickness};
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Border, Carousel, Control, Grid, Image, ItemsSource, PipsPager, RadioButtons, RowDefinitions,
    TextBlock,
};
use std::rc::Rc;

const MEDIA: &str = "ferres://WinUIGallery/Assets/SampleMedia/";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FlipItem {
    pub id: &'static str,
    pub title: &'static str,
    pub image: &'static str,
}

// ControlInfoDataSource.Groups.Take(3).SelectMany(g => g.Items), including the
// Scratch Pad entry even though it has no ordinary executable catalog page.
pub(crate) const BOUND_ITEMS: &[FlipItem] = &[
    FlipItem {
        id: "XamlResources",
        title: "Resources",
        image: "CodeTagIcon.png",
    },
    FlipItem {
        id: "XamlStyles",
        title: "Style",
        image: "CodeTagIcon.png",
    },
    FlipItem {
        id: "Binding",
        title: "Binding",
        image: "CodeTagIcon.png",
    },
    FlipItem {
        id: "Templates",
        title: "Templates",
        image: "CodeTagIcon.png",
    },
    FlipItem {
        id: "CustomUserControls",
        title: "Custom & User Controls",
        image: "CustomControls.png",
    },
    FlipItem {
        id: "CustomXamlConditionals",
        title: "XAML Conditions",
        image: "CodeTagIcon.png",
    },
    FlipItem {
        id: "ScratchPad",
        title: "Scratch Pad",
        image: "ScratchPad.png",
    },
    FlipItem {
        id: "Color",
        title: "Color",
        image: "ColorPaletteResources.png",
    },
    FlipItem {
        id: "Geometry",
        title: "Geometry",
        image: "Shape.png",
    },
    FlipItem {
        id: "Iconography",
        title: "Iconography",
        image: "IconElement.png",
    },
    FlipItem {
        id: "Spacing",
        title: "Spacing",
        image: "CompactSizing.png",
    },
    FlipItem {
        id: "Typography",
        title: "Typography",
        image: "TextBlock.png",
    },
    FlipItem {
        id: "AccessibilityColorContrast",
        title: "Color Contrast",
        image: "Accessibility.png",
    },
    FlipItem {
        id: "AccessibilityKeyboard",
        title: "Keyboard Navigation",
        image: "Accessibility.png",
    },
    FlipItem {
        id: "AccessibilityScreenReader",
        title: "Screen Reader",
        image: "Accessibility.png",
    },
];

pub(crate) const LANDSCAPES: &[(&str, &str)] = &[
    ("Cliff", "cliff.jpg"),
    ("Grapes", "grapes.jpg"),
    ("Rainier", "rainier.jpg"),
    ("Sunset", "sunset.jpg"),
    ("Valley", "valley.jpg"),
];

pub(crate) fn wire(root: &Ref<Control>) {
    if root.find_control::<Image>("BasicImage").is_some() {
        wire_images(root);
    }
    if root.find_control::<Carousel>("DeclaredFlipView").is_some() {
        wire_flips(root);
    }
}

fn image_result(
    image: &Image,
    result: std::io::Result<Bitmap>,
    label: &str,
    errors: &mut Vec<String>,
) {
    match result {
        Ok(bitmap) => image.set_source(Some(Rc::new(bitmap))),
        Err(error) => errors.push(format!("{label}: {error}")),
    }
}

fn show_errors(root: &Control, name: &str, errors: &[String]) {
    let output = root.get_control::<TextBlock>(name);
    output.set_text(Some(&errors.join("\n")));
    output.set_is_visible(!errors.is_empty());
}

pub(crate) fn decode_to_height(uri: &str, height: i32) -> std::io::Result<Bitmap> {
    crate::assets::register();
    let uri = Uri::new(uri, UriKind::Absolute).map_err(|error| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string())
    })?;
    let mut stream = AssetLoader::open(&uri, None)?;
    Bitmap::decode_to_height(&mut *stream, height, BitmapInterpolationMode::HighQuality)
}

fn wire_images(root: &Ref<Control>) {
    let mut errors = Vec::new();
    for (name, source) in [
        ("BasicImage", "treetops.jpg"),
        ("StretchImage", "valley.jpg"),
        ("NineGridNormal", "ninegrid.gif"),
        ("GifAutoPreview", "animated.gif"),
        ("GifPausedPreview", "animated.gif"),
        ("GifManualPreview", "animated.gif"),
    ] {
        image_result(
            &root.get_control::<Image>(name),
            crate::assets::load_bitmap(&format!("{MEDIA}{source}")),
            source,
            &mut errors,
        );
    }
    match decode_to_height(&format!("{MEDIA}treetops.jpg"), 100) {
        Ok(bitmap) => {
            root.get_control::<TextBlock>("DecodedImageDimensions")
                .set_text(Some(&format!(
                    "Decoded image height: {} pixels",
                    bitmap.pixel_size().height
                )));
            root.get_control::<Image>("DecodedImage")
                .set_source(Some(Rc::new(bitmap)));
        }
        Err(error) => errors.push(format!("Decode to 100 pixels: {error}")),
    }

    match crate::assets::load_bitmap(&format!("{MEDIA}ninegrid.gif")) {
        Ok(bitmap) => {
            let bitmap = Rc::new(bitmap);
            for (name, insets) in [
                ("NineGridSmallInsets", [3, 3, 3, 3]),
                ("NineGridLargeInsets", [30, 20, 30, 20]),
            ] {
                root.get_control::<Image>(name)
                    .set_source(Some(nine_grid_image(bitmap.clone(), insets)));
            }
        }
        Err(error) => errors.push(format!("Nine-grid image: {error}")),
    }

    let modes = root.get_control::<RadioButtons>("ImageStretchModes");
    let weak_modes = modes.downgrade();
    let weak_image = root.get_control::<Image>("StretchImage").downgrade();
    modes.selection_changed(move |_, _| {
        if let (Some(modes), Some(image)) = (weak_modes.upgrade(), weak_image.upgrade()) {
            image.set_stretch(match modes.selected_index() {
                1 => Stretch::Fill,
                2 => Stretch::Uniform,
                3 => Stretch::UniformToFill,
                _ => Stretch::None,
            });
        }
    });
    show_errors(root, "ImageLoadError", &errors);
}

/// The source page's output is fixed at 164x164. Corners retain their original
/// pixel size, edges stretch on one axis, and the center stretches on both.
/// This composition is intentionally limited to that example, not an Image API.
pub(crate) fn nine_grid_image(bitmap: Rc<Bitmap>, insets: [i32; 4]) -> Rc<dyn IImage> {
    let size = bitmap.pixel_size();
    let [left, top, right, bottom] = insets;
    let source_x = [0, left, size.width - right, size.width];
    let source_y = [0, top, size.height - bottom, size.height];
    let target_x = [0, left, 164 - right, 164];
    let target_y = [0, top, 164 - bottom, 164];
    let drawing = DrawingGroup::new();
    for y in 0..3 {
        for x in 0..3 {
            let crop = CroppedBitmap::with_source(
                bitmap.clone(),
                PixelRect::new(
                    source_x[x],
                    source_y[y],
                    source_x[x + 1] - source_x[x],
                    source_y[y + 1] - source_y[y],
                ),
            );
            let patch = ImageDrawing::new();
            patch.set_image_source(Some(crop.into()));
            patch.set_rect(Rect::new(
                f64::from(target_x[x]),
                f64::from(target_y[y]),
                f64::from(target_x[x + 1] - target_x[x]),
                f64::from(target_y[y + 1] - target_y[y]),
            ));
            drawing.children().add(patch.upcast());
        }
    }
    DrawingImage::with_drawing(drawing).into()
}

fn wire_flips(root: &Ref<Control>) {
    let mut errors = Vec::new();
    for name in ["DeclaredFlipView", "VerticalFlipView"] {
        let carousel = root.get_control::<Carousel>(name);
        for (title, file) in LANDSCAPES {
            let image = Image::new();
            image.set_stretch(Stretch::Uniform);
            AutomationProperties::set_name(&image, Some(title));
            image_result(
                &image,
                crate::assets::load_bitmap(&format!("{MEDIA}{file}")),
                file,
                &mut errors,
            );
            carousel.items().add(Some(Control::boxed(image)));
        }
    }
    let bound = root.get_control::<Carousel>("BoundFlipView");
    bound.set_item_template(Some(FuncDataTemplate::for_type::<FlipItem>(
        |item, _| Some(bound_item(item)),
        false,
    )));
    bound.set_items_source(Some(ItemsSource::from_items(
        BOUND_ITEMS
            .iter()
            .map(|item| Some(Rc::new(*item) as BoxedValue)),
    )));
    for (name, axis) in [
        ("DeclaredFlipView", SlideAxis::Horizontal),
        ("BoundFlipView", SlideAxis::Horizontal),
        ("VerticalFlipView", SlideAxis::Vertical),
    ] {
        wire_pager(root, name, axis);
    }
    show_errors(root, "FlipViewLoadError", &errors);
}

fn wire_pager(root: &Control, name: &str, axis: SlideAxis) {
    let carousel = root.get_control::<Carousel>(name);
    carousel.set_is_swipe_enabled(true);
    carousel.set_wrap_selection(false);
    carousel.set_page_transition(Some(Rc::new(PageSlide::with_duration(
        TimeSpan::from_milliseconds(250.0),
        axis,
    ))));
    AutomationProperties::set_control_type_override(&carousel, Some(AutomationControlType::List));
    let pager = root.get_control::<PipsPager>(&format!("{name}Pager"));
    pager.set_orientation(if axis == SlideAxis::Vertical {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    });
    pager.set_number_of_pages(carousel.item_count());
    let output = root.get_control::<TextBlock>(&format!("{name}Selection"));
    carousel.set_selected_index(0);
    pager.set_selected_page_index(0);
    update_selection(&carousel, &output);
    let weak_carousel = carousel.downgrade();
    let weak_pager = pager.downgrade();
    let weak_output = output.downgrade();
    carousel.selection_changed(move |_, _| {
        if let (Some(carousel), Some(pager), Some(output)) = (
            weak_carousel.upgrade(),
            weak_pager.upgrade(),
            weak_output.upgrade(),
        ) {
            pager.set_selected_page_index(carousel.selected_index().max(0));
            update_selection(&carousel, &output);
        }
    });
    let weak_carousel = carousel.downgrade();
    let weak_pager = pager.downgrade();
    pager.selected_index_changed(move |_, _| {
        if let (Some(carousel), Some(pager)) = (weak_carousel.upgrade(), weak_pager.upgrade()) {
            carousel.set_selected_index(pager.selected_page_index());
        }
    });
}

fn update_selection(carousel: &Carousel, output: &TextBlock) {
    output.set_text(Some(&format!(
        "{} of {}",
        carousel.selected_index() + 1,
        carousel.item_count()
    )));
}

fn bound_item(item: &FlipItem) -> Ref<Control> {
    let grid = Grid::new();
    grid.set_row_definitions(RowDefinitions::parse("*,Auto").expect("source row definitions"));
    AutomationProperties::set_automation_id(&grid, Some(item.id));
    let image = Image::new();
    image.set_width(36.0);
    image.set_vertical_alignment(VerticalAlignment::Center);
    image.set_stretch(Stretch::Uniform);
    AutomationProperties::set_name(&image, Some(item.title));
    let uri = format!("ferres://WinUIGallery/Assets/ControlImages/{}", item.image);
    match crate::assets::load_bitmap(&uri) {
        Ok(bitmap) => image.set_source(Some(Rc::new(bitmap))),
        Err(error) => {
            let diagnostic = TextBlock::new();
            diagnostic.set_text(Some(&format!("{}: {error}", item.image)));
            diagnostic.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
            grid.children().add(diagnostic);
        }
    }
    grid.children().add(image);
    let caption = Border::new();
    caption.set_height(60.0);
    caption.set_background(Some(
        SolidColorBrush::with_color(Color::new(0xA5, 255, 255, 255)).into(),
    ));
    Grid::set_row(&caption, 1);
    let text = TextBlock::new();
    text.set_name(Some("Control2Text".to_string()));
    text.set_text(Some(item.title));
    text.set_padding(Thickness::uniform(12.0));
    text.set_horizontal_alignment(HorizontalAlignment::Center);
    text.set_foreground(Some(Brushes::black()));
    text.set_font_size(28.0);
    caption.set_child(Some(text.upcast()));
    grid.children().add(caption);
    grid.upcast()
}
