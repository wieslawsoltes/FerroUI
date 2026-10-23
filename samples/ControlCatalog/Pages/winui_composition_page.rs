//! The page of the WinUI composition API (not from upstream; `docs/porting/winui-composition.md`):
//! a connected animation between a list and a detail view, the implicit show and hide
//! animations of `ElementCompositionPreview`, a light that follows the pointer through the
//! pointer position property set, and a parallax header driven by the manipulation property set
//! of a scroll viewer.

use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontWeight, IBrush, TextWrapping};
use ferroui_base::rendering::composition::animations::{ICompositionAnimation, ICompositionAnimationBase};
use ferroui_base::rendering::composition::{Compositor, ICompositionObjectAnimations};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{BoxedValue, CornerRadius, Ref, Thickness, Vector, Vector3D, Visual};
use ferroui_controls::animation::{
    BasicConnectedAnimationConfiguration, ConnectedAnimationConfiguration, ConnectedAnimationService,
    DirectConnectedAnimationConfiguration, GravityConnectedAnimationConfiguration,
};
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::{Border, Button, ContentPage, Control, ScrollViewer, StackPanel, TextBlock, TopLevel, WrapPanel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The cards of the list: a title, a colour and a description.
const CARDS: &[(&str, (u8, u8, u8), &str)] = &[
    ("Aurora", (0x2E, 0x86, 0xDE), "A sweep of cold light over the northern sky."),
    ("Ember", (0xE8, 0x5D, 0x3A), "The last glow of a fire, still warm to the touch."),
    ("Moss", (0x3F, 0x9E, 0x5A), "A soft green carpet on stones in the shade."),
    ("Dune", (0xD9, 0xA4, 0x41), "Sand shaped by the wind into long ridges."),
    ("Lagoon", (0x1A, 0xA6, 0xA6), "Still water behind a reef, clear to the bottom."),
    ("Orchid", (0x9B, 0x59, 0xB6), "A flower that grows on the branches of trees."),
];

fn brush(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::from_rgb(r, g, b)
}

fn text(value: &str, size: f64, weight: FontWeight) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_text(Some(value));
    block.set_font_size(size);
    block.set_font_weight(weight);
    block.set_text_wrapping(TextWrapping::Wrap);
    block
}

fn label(value: &str) -> BoxedValue {
    Rc::new(value.to_owned()) as BoxedValue
}

fn section(title: &str, description: &str, content: &Control) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.children().add(text(title, 18.0, FontWeight::SemiBold));
    let about = text(description, 13.0, FontWeight::Normal);
    about.set_opacity(0.7);
    panel.children().add(about);
    panel.children().add(content.to_ref());
    panel
}

/// The compositor of the window of `visual`.
fn compositor_of(visual: &Visual) -> Option<Rc<Compositor>> {
    ElementCompositionPreview::get_element_visual(visual)
        .map(|composition_visual| composition_visual.compositor().clone())
        .or_else(Compositor::try_get_default_compositor)
}

/// A scalar key frame animation of `target` from `from` to `to`.
fn scalar(compositor: &Rc<Compositor>, target: &str, from: f32, to: f32, ms: u64) -> Rc<dyn ICompositionAnimation> {
    let animation = compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame(0.0, from);
    animation.insert_key_frame(1.0, to);
    animation.set_duration(Duration::from_millis(ms));
    animation.set_target(Some(target.to_owned()));
    animation
}

/// A vector key frame animation of `target` from `from` to `to`.
fn vector(compositor: &Rc<Compositor>, target: &str, from: Vector3D, to: Vector3D, ms: u64) -> Rc<dyn ICompositionAnimation> {
    let animation = compositor.create_vector3d_key_frame_animation();
    animation.insert_key_frame(0.0, from);
    animation.insert_key_frame(1.0, to);
    animation.set_duration(Duration::from_millis(ms));
    animation.set_target(Some(target.to_owned()));
    animation
}

/// The implicit show animation of the page: a fade in while sliding down into place.
fn show_animation(compositor: &Rc<Compositor>) -> Rc<dyn ICompositionAnimationBase> {
    let group = compositor.create_animation_group();
    group.add(scalar(compositor, "Opacity", 0.0, 1.0, 300));
    group.add(vector(compositor, "Translation", Vector3D::new(0.0, -24.0, 0.0), Vector3D::new(0.0, 0.0, 0.0), 300));
    group
}

/// The implicit hide animation of the page: a fade out while shrinking.
fn hide_animation(compositor: &Rc<Compositor>) -> Rc<dyn ICompositionAnimationBase> {
    let group = compositor.create_animation_group();
    group.add(scalar(compositor, "Opacity", 1.0, 0.0, 250));
    group.add(vector(compositor, "Translation", Vector3D::new(0.0, 0.0, 0.0), Vector3D::new(0.0, 16.0, 0.0), 250));
    group
}

ferroui_base::ferro_thread_local! {
    /// The gallery of the page last created, for the smoke run of the desktop entry point.
    static GALLERY: RefCell<std::rc::Weak<Gallery>> = RefCell::new(std::rc::Weak::new());

    /// The box of the implicit animations and the parallax scroller of the page last created,
    /// for the smoke run.
    static SMOKE_TARGETS: RefCell<(Option<ferroui_base::WeakRef<Border>>, Option<ferroui_base::WeakRef<ScrollViewer>>)> =
        const { RefCell::new((None, None)) };
}

/// Toggles `IsVisible` of the box of the implicit animations of the page last created (for the
/// smoke run of the desktop entry point).
pub fn smoke_toggle_box() -> bool {
    let Some(box_) = SMOKE_TARGETS.with(|targets| targets.borrow().0.as_ref().and_then(|weak| weak.upgrade())) else {
        return false;
    };
    box_.set_is_visible(!box_.is_visible());
    true
}

/// Scrolls the parallax scroller of the page last created, and brings it into view (for the
/// smoke run of the desktop entry point).
pub fn smoke_scroll_parallax(offset: f64) -> bool {
    let Some(scroller) = SMOKE_TARGETS.with(|targets| targets.borrow().1.as_ref().and_then(|weak| weak.upgrade()))
    else {
        return false;
    };
    scroller.bring_into_view();
    scroller.set_offset(Vector::new(0.0, offset));
    true
}

/// Opens the detail view of card `index` of the page last created, as a click on it does (for
/// the smoke run of the desktop entry point). Returns whether there was such a page.
pub fn smoke_open_card(index: usize) -> bool {
    let Some(gallery) = GALLERY.with(|gallery| gallery.borrow().upgrade()) else { return false };
    let Some(image) = gallery.images.get(index).cloned() else { return false };
    gallery.open(&image, index);
    true
}

/// Chooses the configuration of the page last created (0 gravity, 1 direct, 2 basic), as its
/// buttons do.
pub fn smoke_set_configuration(index: usize) -> bool {
    let Some(gallery) = GALLERY.with(|gallery| gallery.borrow().upgrade()) else { return false };
    gallery.configuration.set(index);
    true
}

/// Returns to the list of the page last created, as the back button does.
pub fn smoke_close_card() -> bool {
    let Some(gallery) = GALLERY.with(|gallery| gallery.borrow().upgrade()) else { return false };
    gallery.close();
    true
}

/// The connected animation between the cards and the detail view.
struct Gallery {
    images: Vec<Ref<Border>>,
    list: Ref<WrapPanel>,
    detail: Ref<StackPanel>,
    detail_image: Ref<Border>,
    detail_title: Ref<TextBlock>,
    detail_description: Ref<TextBlock>,
    selected: RefCell<Option<Ref<Border>>>,
    configuration: Cell<usize>,
    scale: Cell<bool>,
    status: Ref<TextBlock>,
}

impl Gallery {
    fn configuration(&self) -> Rc<dyn ConnectedAnimationConfiguration> {
        match self.configuration.get() {
            1 => DirectConnectedAnimationConfiguration::new(),
            2 => BasicConnectedAnimationConfiguration::new(),
            _ => GravityConnectedAnimationConfiguration::new(),
        }
    }

    fn service(&self) -> Option<Ref<ConnectedAnimationService>> {
        TopLevel::get_top_level(Some(&self.list)).map(|top_level| ConnectedAnimationService::get_for_top_level(&top_level))
    }

    /// Opens the detail view of a card: the image of the card flies to the image of the view,
    /// the title and the description enter with it.
    fn open(&self, image: &Ref<Border>, index: usize) {
        let Some(service) = self.service() else { return };
        let animation = service.prepare_to_animate("card", image);
        animation.set_configuration(Some(self.configuration()));
        animation.set_is_scale_animation_enabled(self.scale.get());

        let (title, color, description) = CARDS[index];
        self.detail_image.set_background(Some(brush(rgb(color))));
        self.detail_title.set_text(Some(title));
        self.detail_description.set_text(Some(description));
        *self.selected.borrow_mut() = Some(image.clone());
        self.list.set_is_visible(false);
        self.detail.set_is_visible(true);

        let coordinated: [Ref<Visual>; 2] = [self.detail_title.clone().upcast(), self.detail_description.clone().upcast()];
        let started = animation.try_start_with_coordinated_elements(&self.detail_image, &coordinated);
        self.status.set_text(Some(if started { "Flying to the detail view" } else { "The animation did not start" }));
        let status = self.status.clone();
        let _subscription = animation.completed(move |e| {
            status.set_text(Some(if e.cancelled() { "Cancelled" } else { "Arrived" }));
        });
    }

    /// Returns to the list: the image flies back to its card, with the direct configuration as
    /// WinUI recommends for back navigation.
    fn close(&self) {
        let Some(service) = self.service() else { return };
        let Some(card) = self.selected.borrow_mut().take() else { return };
        let animation = service.prepare_to_animate("card-back", &self.detail_image);
        animation.set_configuration(Some(DirectConnectedAnimationConfiguration::new()));
        self.detail.set_is_visible(false);
        self.list.set_is_visible(true);
        animation.try_start(&card);
        self.status.set_text(Some("Flying back"));
    }
}

fn connected_animation_section() -> Ref<StackPanel> {
    let status = text("Pick a card", 12.0, FontWeight::Normal);
    status.set_opacity(0.7);

    let list = WrapPanel::new();
    list.set_orientation(Orientation::Horizontal);

    let detail = StackPanel::new();
    detail.set_spacing(10.0);
    detail.set_is_visible(false);
    let detail_image = Border::new();
    detail_image.set_width(360.0);
    detail_image.set_height(200.0);
    detail_image.set_corner_radius(CornerRadius::uniform(12.0));
    detail_image.set_horizontal_alignment(HorizontalAlignment::Left);
    let detail_title = text("", 24.0, FontWeight::Bold);
    let detail_description = text("", 14.0, FontWeight::Normal);
    detail_description.set_max_width(360.0);
    detail_description.set_horizontal_alignment(HorizontalAlignment::Left);
    let back = Button::new();
    back.set_content(Some(label("Back")));
    detail.children().add(detail_image.clone());
    detail.children().add(detail_title.clone());
    detail.children().add(detail_description.clone());
    detail.children().add(back.clone());

    let images: Vec<Ref<Border>> = CARDS
        .iter()
        .map(|(_, color, _)| {
            let image = Border::new();
            image.set_width(140.0);
            image.set_height(84.0);
            image.set_corner_radius(CornerRadius::uniform(8.0));
            image.set_background(Some(brush(rgb(*color))));
            image
        })
        .collect();
    let gallery = Rc::new(Gallery {
        images: images.clone(),
        list: list.clone(),
        detail: detail.clone(),
        detail_image,
        detail_title,
        detail_description,
        selected: RefCell::new(None),
        configuration: Cell::new(0),
        scale: Cell::new(true),
        status: status.clone(),
    });

    GALLERY.with(|current| *current.borrow_mut() = Rc::downgrade(&gallery));
    for (index, ((title, _, _), image)) in CARDS.iter().zip(images).enumerate() {
        let caption = text(title, 13.0, FontWeight::SemiBold);
        let content = StackPanel::new();
        content.set_spacing(6.0);
        content.children().add(image.clone());
        content.children().add(caption);
        let card = Button::new();
        card.set_margin(Thickness::new(0.0, 0.0, 10.0, 10.0));
        card.set_padding(Thickness::uniform(8.0));
        card.set_content(Some(Control::boxed(content)));
        let gallery = Rc::downgrade(&gallery);
        card.click(move |_, _| {
            if let Some(gallery) = gallery.upgrade() {
                gallery.open(&image, index);
            }
        });
        list.children().add(card);
    }
    {
        let gallery = Rc::downgrade(&gallery);
        back.click(move |_, _| {
            if let Some(gallery) = gallery.upgrade() {
                gallery.close();
            }
        });
    }

    // The configuration and the scale animation.
    let options = StackPanel::new();
    options.set_orientation(Orientation::Horizontal);
    options.set_spacing(8.0);
    for (index, name) in ["Gravity", "Direct", "Basic"].into_iter().enumerate() {
        let button = Button::new();
        button.set_content(Some(label(name)));
        let gallery = Rc::downgrade(&gallery);
        let status = status.clone();
        button.click(move |_, _| {
            if let Some(gallery) = gallery.upgrade() {
                gallery.configuration.set(index);
                status.set_text(Some(&format!("Configuration: {name}")));
            }
        });
        options.children().add(button);
    }
    let scale = Button::new();
    scale.set_content(Some(label("Scale: on")));
    {
        let gallery = Rc::downgrade(&gallery);
        let weak_scale = scale.downgrade();
        scale.click(move |_, _| {
            let (Some(gallery), Some(scale)) = (gallery.upgrade(), weak_scale.upgrade()) else { return };
            gallery.scale.set(!gallery.scale.get());
            scale.set_content(Some(label(if gallery.scale.get() { "Scale: on" } else { "Scale: off" })));
        });
    }
    options.children().add(scale);
    options.children().add(status);

    let content = StackPanel::new();
    content.set_spacing(12.0);
    content.children().add(options);
    content.children().add(list.clone());
    content.children().add(detail.clone());
    // The gallery lives as long as its panel: the handlers of the cards hold it weakly.
    let keep = gallery.clone();
    let _ = content.detached_from_visual_tree(move |_| {
        let _ = &keep;
    });

    section(
        "Connected animation",
        "ConnectedAnimationService: prepare_to_animate on a card, try_start on the image of the detail view, with the title and the description as coordinated elements.",
        &content.upcast(),
    )
}

fn implicit_animations_section() -> Ref<StackPanel> {
    let box_ = Border::new();
    SMOKE_TARGETS.with(|targets| targets.borrow_mut().0 = Some(box_.downgrade()));
    box_.set_width(220.0);
    box_.set_height(60.0);
    box_.set_corner_radius(CornerRadius::uniform(8.0));
    box_.set_background(Some(brush(Color::from_rgb(0x58, 0x6C, 0xE0))));
    box_.set_horizontal_alignment(HorizontalAlignment::Left);
    let caption = text("Shown and hidden by composition", 12.0, FontWeight::SemiBold);
    caption.set_margin(Thickness::uniform(10.0));
    box_.set_child(caption);

    let chips = WrapPanel::new();
    chips.set_orientation(Orientation::Horizontal);

    let toggle = Button::new();
    toggle.set_content(Some(label("Toggle IsVisible")));
    {
        let weak = box_.downgrade();
        toggle.click(move |_, _| {
            if let Some(box_) = weak.upgrade() {
                box_.set_is_visible(!box_.is_visible());
            }
        });
    }
    let add = Button::new();
    add.set_content(Some(label("Add a chip (click a chip to remove it)")));
    let count = Rc::new(Cell::new(0usize));
    {
        let weak = chips.downgrade();
        add.click(move |_, _| {
            let Some(chips) = weak.upgrade() else { return };
            let Some(compositor) = compositor_of(&chips) else { return };
            count.set(count.get() + 1);
            let chip = Button::new();
            chip.set_margin(Thickness::new(0.0, 0.0, 8.0, 8.0));
            chip.set_content(Some(label(&format!("Chip {}", count.get()))));
            ElementCompositionPreview::set_implicit_show_animation(&chip, Some(show_animation(&compositor)));
            ElementCompositionPreview::set_implicit_hide_animation(&chip, Some(hide_animation(&compositor)));
            let weak_chips = chips.downgrade();
            let weak_chip = chip.downgrade();
            chip.click(move |_, _| {
                if let (Some(chips), Some(chip)) = (weak_chips.upgrade(), weak_chip.upgrade()) {
                    chips.children().remove(&chip);
                }
            });
            chips.children().add(chip);
        });
    }

    let buttons = StackPanel::new();
    buttons.set_orientation(Orientation::Horizontal);
    buttons.set_spacing(8.0);
    buttons.children().add(toggle);
    buttons.children().add(add);

    let content = StackPanel::new();
    content.set_spacing(10.0);
    content.children().add(buttons);
    content.children().add(box_.clone());
    content.children().add(chips);
    let weak = box_.downgrade();
    content.attached_to_visual_tree(move |_| {
        let Some(box_) = weak.upgrade() else { return };
        let Some(compositor) = compositor_of(&box_) else { return };
        ElementCompositionPreview::set_implicit_show_animation(&box_, Some(show_animation(&compositor)));
        ElementCompositionPreview::set_implicit_hide_animation(&box_, Some(hide_animation(&compositor)));
    });

    section(
        "Implicit show and hide animations",
        "ElementCompositionPreview.SetImplicitShowAnimation and SetImplicitHideAnimation: a fade and a slide when IsVisible changes, and when an element is added to or removed from the tree.",
        &content.upcast(),
    )
}

fn pointer_section() -> Ref<StackPanel> {
    let surface = Border::new();
    surface.set_width(420.0);
    surface.set_height(160.0);
    surface.set_corner_radius(CornerRadius::uniform(8.0));
    surface.set_clip_to_bounds(true);
    surface.set_horizontal_alignment(HorizontalAlignment::Left);
    surface.set_background(Some(brush(Color::from_rgb(0x20, 0x24, 0x30))));
    let caption = text("Move the pointer over the window", 13.0, FontWeight::SemiBold);
    caption.set_foreground(Some(brush(Color::from_rgb(0xE0, 0xE4, 0xF0))));
    caption.set_margin(Thickness::uniform(12.0));
    surface.set_child(caption);

    let weak = surface.downgrade();
    surface.attached_to_visual_tree(move |_| {
        let Some(surface) = weak.upgrade() else { return };
        // The element's visual exists once the renderer has seen it.
        let weak = surface.downgrade();
        let _ = Dispatcher::ui_thread().invoke_async_local_with_priority(
            move || {
                let Some(surface) = weak.upgrade() else { return };
                let Some(compositor) = compositor_of(&surface) else { return };
                let Some(pointer) = ElementCompositionPreview::get_pointer_position_property_set(&surface) else { return };
                // A light: a round sprite whose offset follows the pointer.
                let light = compositor.create_sprite_visual();
                light.set_size(Vector::new(90.0, 90.0));
                let brush = compositor.create_color_brush_with_color(Color::from_argb(0x90, 0xFF, 0xE8, 0x9A));
                light.set_brush(Some((*brush).clone()));
                let clip = compositor.create_rectangle_clip_with_radii(
                    0.0,
                    0.0,
                    90.0,
                    90.0,
                    Vector::new(45.0, 45.0),
                    Vector::new(45.0, 45.0),
                    Vector::new(45.0, 45.0),
                    Vector::new(45.0, 45.0),
                );
                light.set_composition_clip(Some((*clip).clone()));
                let shadow = compositor.create_drop_shadow();
                shadow.set_blur_radius(30.0);
                shadow.set_color(Color::from_rgb(0xFF, 0xD0, 0x60));
                light.set_shadow(Some(shadow));
                let follow = compositor.create_expression_animation_with("Vector3(pointer.Position.X - 45, pointer.Position.Y - 45, 0)");
                follow.set_reference_parameter("pointer", pointer);
                light.start_animation("Offset", &*follow);
                ElementCompositionPreview::set_element_child_visual(&surface, Some(light.visual().clone()));
            },
            DispatcherPriority::BACKGROUND,
        );
    });

    section(
        "Pointer position property set",
        "ElementCompositionPreview.GetPointerPositionPropertySet drives an expression animation of a sprite visual (a colour brush, a rounded rectangle clip and a drop shadow) set with SetElementChildVisual.",
        &surface.upcast(),
    )
}

fn parallax_section() -> Ref<StackPanel> {
    let header = Border::new();
    header.set_height(140.0);
    header.set_corner_radius(CornerRadius::uniform(8.0));
    header.set_background(Some(brush(Color::from_rgb(0xC0, 0x4A, 0x8C))));
    let caption = text("Parallax header", 18.0, FontWeight::Bold);
    caption.set_margin(Thickness::uniform(14.0));
    caption.set_foreground(Some(brush(Color::from_rgb(0xFF, 0xFF, 0xFF))));
    header.set_child(caption);

    let lines = StackPanel::new();
    lines.set_spacing(6.0);
    lines.children().add(header.clone());
    for line in 1..=30 {
        lines.children().add(text(&format!("Line {line}: the header moves at half the speed of the content."), 13.0, FontWeight::Normal));
    }
    let scroller = ScrollViewer::new();
    SMOKE_TARGETS.with(|targets| targets.borrow_mut().1 = Some(scroller.downgrade()));
    scroller.set_height(260.0);
    scroller.set_width(420.0);
    scroller.set_horizontal_alignment(HorizontalAlignment::Left);
    scroller.set_content(Some(Control::boxed(lines)));

    let weak_scroller = scroller.downgrade();
    let weak_header = header.downgrade();
    scroller.attached_to_visual_tree(move |_| {
        let (weak_scroller, weak_header) = (weak_scroller.clone(), weak_header.clone());
        let _ = Dispatcher::ui_thread().invoke_async_local_with_priority(
            move || {
                let (Some(scroller), Some(header)) = (weak_scroller.upgrade(), weak_header.upgrade()) else { return };
                let Some(set) = ElementCompositionPreview::get_scroll_viewer_manipulation_property_set(&scroller) else {
                    return;
                };
                let Some(visual) = ElementCompositionPreview::get_element_visual(&header) else { return };
                let compositor = visual.compositor().clone();
                let parallax = compositor.create_expression_animation_with("Vector3(0, -scroller.Translation.Y * 0.5, 0)");
                parallax.set_reference_parameter("scroller", set);
                visual.start_animation("Translation", &*parallax);
            },
            DispatcherPriority::BACKGROUND,
        );
    });

    section(
        "Scroll viewer manipulation property set",
        "ElementCompositionPreview.GetScrollViewerManipulationPropertySet: an expression animation of the Translation of the header reads the scrolling of its scroll viewer.",
        &scroller.upcast(),
    )
}

/// Creates the page.
pub fn winui_composition_page() -> Ref<ContentPage> {
    let page = ContentPage::new();
    page.set_header(Some(label("Composition (WinUI)")));
    let panel = StackPanel::new();
    panel.set_spacing(28.0);
    panel.set_margin(Thickness::uniform(16.0));
    panel.set_vertical_alignment(VerticalAlignment::Top);
    panel.children().add(connected_animation_section());
    panel.children().add(implicit_animations_section());
    panel.children().add(pointer_section());
    panel.children().add(parallax_section());
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(panel)));
    page.set_content(Some(Control::boxed(scroller)));
    page
}
