//! The four original ConnectedAnimation compositions, using the native service.
//!
//! ListView/GridView/ItemsRepeater containers are explicitly composed from native
//! panels and buttons. The sample calls ConnectedAnimationService directly; it
//! does not add container shims. Frame navigation, cached source pages, selected
//! data, coordinated details, return configuration, scroll and focus are real.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation};
use ferroui_base::media::{Stretch, TextWrapping};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, Thickness, Vector, Visual, VisualImpl, WeakRef,
};
use ferroui_controls::animation::{
    BasicConnectedAnimationConfiguration, ConnectedAnimation, ConnectedAnimationConfiguration,
    ConnectedAnimationService, DirectConnectedAnimationConfiguration,
    GravityConnectedAnimationConfiguration,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::navigation::{NavigationCacheMode, NavigationError, NavigationMode};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Border, Button, ContentControlImpl, Control, ControlImpl, Frame, FramePage, FramePageImpl,
    Grid, Image, Panel, RadioButtons, ScrollViewer, StackPanel, TextBlock, TopLevel,
};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

macro_rules! connected_page {
    ($name:ident) => {
        #[repr(C)]
        pub(crate) struct $name { base: FramePage }
        ferro_class!($name: FramePage);
        ferro_class_info!($name {});
        ferro_impl_classes!($name: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
            InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl,
            FramePageImpl);
        impl $name {
            fn with_content(content: Ref<Control>) -> Ref<FramePage> {
                let page = instantiate(Self { base: FramePage::construct() });
                page.set_content(Some(Control::boxed(content)));
                page.upcast()
            }
        }
    };
}

connected_page!(ConnectedListPage);
connected_page!(ConnectedRepeaterPage);
connected_page!(ConnectedDetailPage);
connected_page!(ConnectedCardPage);
connected_page!(ConnectedFirstPage);
connected_page!(ConnectedSecondPage);

#[derive(Clone)]
struct GalleryItem {
    title: String,
    image_uri: String,
    views: u32,
    likes: u32,
    description: &'static str,
}

// The original CustomDataObject supplies these texts and cycles them for 13 items.
const DESCRIPTIONS: [&str; 8] = [
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer id facilisis lectus. Cras nec convallis ante, quis pulvinar tellus. Integer dictum accumsan pulvinar. Pellentesque eget enim sodales sapien vestibulum consequat.",
    "Nullam eget mattis metus. Donec pharetra, tellus in mattis tincidunt, magna ipsum gravida nibh, vitae lobortis ante odio vel quam.",
    "Quisque accumsan pretium ligula in faucibus. Mauris sollicitudin augue vitae lorem cursus condimentum quis ac mauris. Pellentesque quis turpis non nunc pretium sagittis. Nulla facilisi. Maecenas eu lectus ante. Proin eleifend vel lectus non tincidunt. Fusce condimentum luctus nisi, in elementum ante tincidunt nec.",
    "Aenean in nisl at elit venenatis blandit ut vitae lectus. Praesent in sollicitudin nunc. Pellentesque justo augue, pretium at sem lacinia, scelerisque semper erat. Ut cursus tortor at metus lacinia dapibus.",
    "Ut consequat magna luctus justo egestas vehicula. Integer pharetra risus libero, et posuere justo mattis et.",
    "Proin malesuada, libero vitae aliquam venenatis, diam est faucibus felis, vitae efficitur erat nunc non mauris. Suspendisse at sodales erat.",
    "Aenean vulputate, turpis non tincidunt ornare, metus est sagittis erat, id lobortis orci odio eget quam. Suspendisse ex purus, lobortis quis suscipit a, volutpat vitae turpis.",
    "Duis facilisis, quam ut laoreet commodo, elit ex aliquet massa, non varius tellus lectus et nunc. Donec vitae risus ut ante pretium semper. Phasellus consectetur volutpat orci, eu dapibus turpis. Fusce varius sapien eu mattis pharetra.",
];

fn data(count: usize) -> Vec<GalleryItem> {
    // Preserve the source's randomized display counts and exclusive upper bounds.
    // This generator is only sample data; the exact System.Random sequence is not a contract.
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut next = |range: u32| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 32) as u32) % range
    };
    (0..count)
        .map(|index| GalleryItem {
            title: format!("Item {}", index + 1),
            image_uri: format!(
                "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage{}.jpg",
                index + 1
            ),
            views: 100 + next(899),
            likes: 10 + next(89),
            description: DESCRIPTIONS[index % DESCRIPTIONS.len()],
        })
        .collect()
}

#[derive(Default)]
struct ActiveAnimation(RefCell<Option<Rc<ConnectedAnimation>>>);

impl ActiveAnimation {
    fn replace(&self, value: Rc<ConnectedAnimation>) {
        let old = self.0.replace(Some(value));
        if let Some(old) = old {
            old.dispose();
        }
    }

    fn current(&self) -> Option<Rc<ConnectedAnimation>> {
        self.0.borrow().clone()
    }

    fn clear(&self) {
        let old = self.0.take();
        if let Some(old) = old {
            old.dispose();
        }
    }
}

impl Drop for ActiveAnimation {
    fn drop(&mut self) {
        if let Some(animation) = self.0.get_mut().take() {
            animation.dispose();
        }
    }
}

fn parse(markup: &str) -> Result<Ref<Control>, NavigationError> {
    FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(markup, None)
        .map_err(|error| NavigationError::FactoryFailed(error.to_string()))
}

pub(crate) fn content(frame: &Frame) -> Option<Ref<Control>> {
    frame
        .current_page()?
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
}

/// Programmatically composed item children keep ordinary names and logical ancestry.
pub(crate) fn item_control<T: ferroui_base::ObjectType>(
    root: &Control,
    name: &str,
) -> Option<Ref<T>> {
    root.get_logical_descendants()
        .find(|child| child.name().as_deref() == Some(name))
        .and_then(|child| child.cast::<T>())
}

fn service(source: &Visual) -> Ref<ConnectedAnimationService> {
    source
        .find_ancestor_of_type::<TopLevel>(false)
        .map(|window| ConnectedAnimationService::get_for_top_level(&window))
        .unwrap_or_else(ConnectedAnimationService::get_for_current_view)
}

fn load_image(image: &Image, item: &GalleryItem) -> Result<(), String> {
    AutomationProperties::set_name(image, Some(&item.title));
    image.set_source(None);
    let bitmap = crate::assets::load_bitmap(&item.image_uri)
        .map_err(|error| format!("Could not load {}: {error}", item.image_uri))?;
    image.set_source(Some(Rc::new(bitmap)));
    Ok(())
}

fn show_image_errors(output: &TextBlock, errors: &[String]) {
    output.set_text(Some(&errors.join("\n")));
    output.set_is_visible(!errors.is_empty());
}

fn text(value: &str) -> Ref<TextBlock> {
    let result = TextBlock::new();
    result.set_text(Some(value));
    result
}

fn item_button(
    item: &GalleryItem,
    index: usize,
    tile: bool,
) -> (Ref<Button>, Ref<Image>, Option<String>) {
    let button = Button::new();
    button.set_name(Some(format!("ConnectedItem{}", index + 1)));
    button.set_tag(Some(Rc::new(index)));
    button.set_padding(Thickness::uniform(0.0));
    button.set_horizontal_alignment(HorizontalAlignment::Stretch);
    AutomationProperties::set_name(&button, Some(&item.title));
    let image = Image::new();
    image.set_name(Some(format!("ConnectedImage{}", index + 1)));
    image.set_width(150.0);
    image.set_height(if tile { 90.0 } else { 100.0 });
    image.set_stretch(if tile {
        Stretch::UniformToFill
    } else {
        Stretch::Fill
    });
    // A decoder failure stays visible in the composition, without disabling
    // its navigation or replacing the original asset with a fabricated bitmap.
    let image_error = load_image(&image, item).err();
    let body = StackPanel::new();
    body.set_orientation(if tile {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    });
    body.set_spacing(if tile { 4.0 } else { 12.0 });
    body.children().add(image.clone());
    let labels = StackPanel::new();
    labels.set_spacing(4.0);
    let title = text(&item.title);
    title.set_font_size(if tile { 14.0 } else { 20.0 });
    labels.children().add(title);
    if !tile {
        labels.children().add(text(&format!(
            "Views: {}   Likes: {}",
            item.views, item.likes
        )));
        let description = text(item.description);
        description.set_max_width(500.0);
        description.set_max_lines(2);
        description.set_text_wrapping(TextWrapping::Wrap);
        labels.children().add(description);
    } else {
        button.set_width(150.0);
        button.set_height(120.0);
        button.set_margin(Thickness::uniform(4.0));
    }
    body.children().add(labels);
    button.set_content(Some(Control::boxed(body)));
    (button, image, image_error)
}

struct CollectionState {
    frame: WeakRef<Frame>,
    status: WeakRef<TextBlock>,
    items: Vec<GalleryItem>,
    selected: Cell<Option<usize>>,
    scroll_position: Cell<Vector>,
    tiles: bool,
    animation: ActiveAnimation,
}

impl CollectionState {
    fn open(&self, index: usize, source: &Image) {
        let Some(frame) = self.frame.upgrade() else {
            return;
        };
        self.animation.clear();
        self.selected.set(Some(index));
        if let Some(current) = content(&frame) {
            self.scroll_position.set(
                current
                    .get_control::<ScrollViewer>("CollectionScroll")
                    .offset(),
            );
        }
        let animation = service(source).prepare_to_animate("ForwardConnectedAnimation", source);
        self.animation.replace(animation);
        if let Err(error) = frame.navigate(ConnectedDetailPage::TYPE, Some(Rc::new(index))) {
            self.animation.clear();
            self.report(&format!("Could not open item: {error}"));
        }
    }

    fn back(&self, source: &Image) {
        let Some(frame) = self.frame.upgrade() else {
            return;
        };
        self.animation.clear();
        let animation = service(source).prepare_to_animate("BackConnectedAnimation", source);
        animation.set_configuration(Some(DirectConnectedAnimationConfiguration::new()));
        self.animation.replace(animation);
        if let Err(error) = frame.go_back() {
            self.animation.clear();
            self.report(&format!("Could not return: {error}"));
        }
    }

    fn report(&self, value: &str) {
        if let Some(status) = self.status.upgrade() {
            status.set_text(Some(value));
        }
    }
}

fn collection_page(state: &Rc<CollectionState>) -> Result<Ref<FramePage>, NavigationError> {
    let root = parse(if state.tiles {
        include_str!("Pages/ConnectedTiles.xaml")
    } else {
        include_str!("Pages/ConnectedCollection.xaml")
    })?;
    let panel = root.get_control::<Panel>("CollectionItems");
    let mut image_errors = Vec::new();
    for (index, item) in state.items.iter().enumerate() {
        let (button, image, error) = item_button(item, index, state.tiles);
        image_errors.extend(error);
        let weak_state = Rc::downgrade(state);
        let weak_image = image.downgrade();
        button.click(move |_, _| {
            if let (Some(state), Some(image)) = (weak_state.upgrade(), weak_image.upgrade()) {
                state.open(index, &image);
            }
        });
        panel.children().add(button);
    }
    show_image_errors(
        &root.get_control::<TextBlock>("CollectionImageErrors"),
        &image_errors,
    );
    let page = if state.tiles {
        ConnectedRepeaterPage::with_content(root)
    } else {
        ConnectedListPage::with_content(root)
    };
    page.set_navigation_cache_mode(NavigationCacheMode::Enabled);
    Ok(page)
}

fn detail_page(state: &Rc<CollectionState>) -> Result<Ref<FramePage>, NavigationError> {
    let root = parse(include_str!("Pages/ConnectedDetail.xaml"))?;
    let item = &state.items[state.selected.get().unwrap_or(0)];
    let image = root.get_control::<Image>("DetailedImage");
    let errors: Vec<_> = load_image(&image, item).err().into_iter().collect();
    show_image_errors(&root.get_control::<TextBlock>("DetailImageError"), &errors);
    root.get_control::<TextBlock>("DetailTitle")
        .set_text(Some(&item.title));
    root.get_control::<TextBlock>("DetailViews")
        .set_text(Some(&format!("Views: {}", item.views)));
    root.get_control::<TextBlock>("DetailLikes")
        .set_text(Some(&format!("Likes: {}", item.likes)));
    root.get_control::<TextBlock>("DetailDescription")
        .set_text(Some(item.description));
    let back = root.get_control::<Button>("GoBackButton");
    let weak_state = Rc::downgrade(state);
    let weak_image = image.downgrade();
    back.click(move |_, _| {
        if let (Some(state), Some(image)) = (weak_state.upgrade(), weak_image.upgrade()) {
            state.back(&image);
        }
    });
    let weak_back = back.downgrade();
    back.loaded(move |_, _| {
        if let Some(back) = weak_back.upgrade() {
            back.focus();
            back.bring_into_view();
        }
    });
    Ok(ConnectedDetailPage::with_content(root))
}

fn wire_collection(
    root: &Ref<Control>,
    frame_name: &str,
    status_name: &str,
    tiles: bool,
) -> Result<(), NavigationError> {
    let frame = root.get_control::<Frame>(frame_name);
    let state = Rc::new(CollectionState {
        frame: frame.downgrade(),
        status: root.get_control::<TextBlock>(status_name).downgrade(),
        items: data(if tiles { 13 } else { 8 }),
        selected: Cell::new(None),
        scroll_position: Cell::new(Vector::default()),
        tiles,
        animation: ActiveAnimation::default(),
    });
    frame.set_cache_size(1)?;
    let source_type = if tiles {
        ConnectedRepeaterPage::TYPE
    } else {
        ConnectedListPage::TYPE
    };
    let factory_state = state.clone();
    frame.register_page_factory(
        source_type,
        Rc::new(move || collection_page(&factory_state)),
    )?;
    let factory_state = state.clone();
    frame.register_page_factory(
        ConnectedDetailPage::TYPE,
        Rc::new(move || detail_page(&factory_state)),
    )?;
    let weak_state = Rc::downgrade(&state);
    frame.navigated(move |event| {
        let Some(state) = weak_state.upgrade() else {
            return;
        };
        let Some(frame) = state.frame.upgrade() else {
            return;
        };
        let Some(current) = content(&frame) else {
            return;
        };
        let Some(index) = state.selected.get() else {
            return;
        };
        if event.source_page_type() == ConnectedDetailPage::TYPE {
            if let Some(animation) = state.animation.current() {
                let image = current.get_control::<Image>("DetailedImage");
                let panel = current.get_control::<StackPanel>("CoordinatedPanel");
                animation.try_start_with_coordinated_elements(&image, &[panel.upcast()]);
            }
            state.report(&format!("{} · details", state.items[index].title));
        } else if event.navigation_mode() == NavigationMode::Back {
            let scroll = current.get_control::<ScrollViewer>("CollectionScroll");
            if state.tiles {
                scroll.set_offset(state.scroll_position.get());
            }
            if let Some(item) =
                item_control::<Button>(&current, &format!("ConnectedItem{}", index + 1))
            {
                if !state.tiles {
                    item.bring_into_view();
                }
                current.update_layout();
                if let (Some(animation), Some(target)) = (
                    state.animation.current(),
                    item_control::<Image>(&current, &format!("ConnectedImage{}", index + 1)),
                ) {
                    animation.try_start(&target);
                }
                item.focus();
            }
            state.report(&format!("Returned to {}", state.items[index].title));
        }
    });
    let weak_state = Rc::downgrade(&state);
    root.detached_from_visual_tree(move |_| {
        if let Some(state) = weak_state.upgrade() {
            state.animation.clear();
        }
    });
    frame.navigate(source_type, None)?;
    Ok(())
}

fn wire_simple(root: &Ref<Control>) -> Result<(), NavigationError> {
    let frame = root.get_control::<Frame>("ConnectedContentFrame");
    frame.register_page_factory(
        ConnectedFirstPage::TYPE,
        Rc::new(|| {
            Ok(ConnectedFirstPage::with_content(parse(include_str!(
                "Pages/NavigationSample1.xaml"
            ))?))
        }),
    )?;
    frame.register_page_factory(
        ConnectedSecondPage::TYPE,
        Rc::new(|| {
            Ok(ConnectedSecondPage::with_content(parse(include_str!(
                "Pages/NavigationSample2.xaml"
            ))?))
        }),
    )?;
    let animation = Rc::new(ActiveAnimation::default());
    let weak_frame = frame.downgrade();
    let weak_options = root
        .get_control::<RadioButtons>("ConfigurationPanel")
        .downgrade();
    let weak_status = root
        .get_control::<TextBlock>("ConnectedSimpleStatus")
        .downgrade();
    let current_animation = animation.clone();
    root.get_control::<Button>("ConnectedNavigate")
        .click(move |_, _| {
            let (Some(frame), Some(options), Some(status)) = (
                weak_frame.upgrade(),
                weak_options.upgrade(),
                weak_status.upgrade(),
            ) else {
                return;
            };
            let Some(current) = content(&frame) else {
                return;
            };
            current_animation.clear();
            let forward = frame.current_source_page_type() == Some(ConnectedFirstPage::TYPE);
            let source = current.get_control::<Grid>(if forward {
                "SourceElement"
            } else {
                "DestinationElement"
            });
            let next = service(&source).prepare_to_animate(
                if forward {
                    "ForwardConnectedAnimation"
                } else {
                    "BackwardConnectedAnimation"
                },
                &source,
            );
            let configuration: Option<Rc<dyn ConnectedAnimationConfiguration>> =
                match options.selected_index() {
                    1 => Some(GravityConnectedAnimationConfiguration::new()),
                    2 => Some(DirectConnectedAnimationConfiguration::new()),
                    3 => Some(BasicConnectedAnimationConfiguration::new()),
                    _ => None,
                };
            next.set_configuration(configuration);
            current_animation.replace(next.clone());
            let target = if forward {
                ConnectedSecondPage::TYPE
            } else {
                ConnectedFirstPage::TYPE
            };
            match frame.navigate(target, None) {
                Ok(_) => {
                    if let Some(destination) = content(&frame) {
                        let target = destination.get_control::<Grid>(if forward {
                            "DestinationElement"
                        } else {
                            "SourceElement"
                        });
                        next.try_start(&target);
                        status.set_text(Some(if forward { "Second page" } else { "First page" }));
                    }
                }
                Err(error) => {
                    current_animation.clear();
                    status.set_text(Some(&format!("Navigation failed: {error}")));
                }
            }
        });
    root.detached_from_visual_tree(move |_| {
        animation.clear();
    });
    frame.navigate(ConnectedFirstPage::TYPE, None)?;
    Ok(())
}

fn wire_cards(root: &Ref<Control>) -> Result<(), NavigationError> {
    let frame = root.get_control::<Frame>("ConnectedCardFrame");
    frame.register_page_factory(
        ConnectedCardPage::TYPE,
        Rc::new(|| {
            let cards = parse(include_str!("Pages/ConnectedCards.xaml"))?;
            let items = Rc::new(data(13));
            let selected = Rc::new(Cell::new(None::<usize>));
            let active = Rc::new(ActiveAnimation::default());
            let panel = cards.get_control::<Panel>("CardItems");
            let mut image_errors = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let (button, _, image_error) = item_button(item, index, true);
                image_errors.extend(image_error.clone());
                let weak_cards = cards.downgrade();
                let weak_button = button.downgrade();
                let selected = selected.clone();
                let active = active.clone();
                let items = items.clone();
                button.click(move |_, _| {
                    let (Some(cards), Some(button)) = (weak_cards.upgrade(), weak_button.upgrade())
                    else {
                        return;
                    };
                    active.clear();
                    selected.set(Some(index));
                    let animation =
                        service(&button).prepare_to_animate("forwardAnimation", &button);
                    active.replace(animation.clone());
                    let item = &items[index];
                    let image = cards.get_control::<Image>("DetailImage");
                    // The identical already-decoded item image is shared with the detail view.
                    if let Some(source) =
                        item_control::<Image>(&button, &format!("ConnectedImage{}", index + 1))
                    {
                        image.set_source(source.source());
                    }
                    AutomationProperties::set_name(&image, Some(&item.title));
                    let errors: Vec<_> = image_error.iter().cloned().collect();
                    show_image_errors(&cards.get_control::<TextBlock>("DetailImageError"), &errors);
                    cards
                        .get_control::<TextBlock>("CardTitle")
                        .set_text(Some(&item.title));
                    cards
                        .get_control::<TextBlock>("CardDescription")
                        .set_text(Some(item.description));
                    cards.get_control::<Grid>("SmokeGrid").set_is_visible(true);
                    animation.try_start(&cards.get_control::<Border>("DestinationElement"));
                });
                panel.children().add(button);
            }
            show_image_errors(
                &cards.get_control::<TextBlock>("CardImageErrors"),
                &image_errors,
            );
            let weak_cards = cards.downgrade();
            let active_back = active.clone();
            cards.get_control::<Button>("CloseCard").click(move |_, _| {
                let (Some(cards), Some(index)) = (weak_cards.upgrade(), selected.get()) else {
                    return;
                };
                active_back.clear();
                let smoke = cards.get_control::<Grid>("SmokeGrid");
                let destination = cards.get_control::<Border>("DestinationElement");
                let Some(target) =
                    item_control::<Button>(&cards, &format!("ConnectedItem{}", index + 1))
                else {
                    return;
                };
                let animation =
                    service(&destination).prepare_to_animate("backwardsAnimation", &destination);
                animation.set_configuration(Some(DirectConnectedAnimationConfiguration::new()));
                active_back.replace(animation.clone());
                smoke.children().remove(&destination);
                let weak_smoke = smoke.downgrade();
                let restored_destination = destination.clone();
                animation.completed(move |_| {
                    if let Some(smoke) = weak_smoke.upgrade() {
                        smoke.set_is_visible(false);
                        if !smoke
                            .children()
                            .contains(&restored_destination.clone().upcast())
                        {
                            smoke.children().add(restored_destination.clone());
                        }
                    }
                });
                target.bring_into_view();
                cards.update_layout();
                if !animation.try_start(&target) {
                    smoke.set_is_visible(false);
                    smoke.children().add(destination);
                }
                target.focus();
            });
            cards.detached_from_visual_tree(move |_| {
                active.clear();
            });
            Ok(ConnectedCardPage::with_content(cards))
        }),
    )?;
    frame.navigate(ConnectedCardPage::TYPE, None)?;
    Ok(())
}

pub(crate) fn wire(root: &Ref<Control>) -> Result<(), XamlLoadException> {
    if root
        .find_control::<Frame>("ConnectedCollectionFrame")
        .is_none()
    {
        return Ok(());
    }
    let load_error = |error| {
        XamlLoadException::with_inner("Could not initialize connected animation examples.", error)
    };
    wire_collection(
        root,
        "ConnectedCollectionFrame",
        "ConnectedCollectionStatus",
        false,
    )
    .map_err(load_error)?;
    wire_cards(root).map_err(load_error)?;
    wire_simple(root).map_err(load_error)?;
    wire_collection(
        root,
        "ConnectedRepeaterFrame",
        "ConnectedRepeaterStatus",
        true,
    )
    .map_err(load_error)?;
    Ok(())
}
