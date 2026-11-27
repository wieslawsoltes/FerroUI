//! The four original ConnectedAnimation compositions, using the native service.
//!
//! The containers are the source's: a ListView, a GridView and an ItemsRepeater with a uniform
//! grid. The sample calls ConnectedAnimationService directly. Frame navigation, cached source
//! pages, selected data, coordinated details, return configuration, scroll and focus are real.

use ferroui_base::input::{InputElement, InputElementImpl, Key, KeyEventArgs, TappedEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation};
use ferroui_base::media::{Stretch, TextTrimming, TextWrapping};
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
    Grid, GridView, Image, ItemsSource, ListView, ListViewBase, RadioButtons, ScrollIntoViewAlignment,
    ScrollViewer, StackPanel, TextBlock, ToolTip, TopLevel,
};
use ferroui_controls_repeater::ItemsRepeater;
use ferroui_markup_xaml::XamlLoadError;
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
            /// The page type of the page, created by the page factory the sample registers.
            pub(crate) const PAGE_TYPE: ferroui_controls::PageType = ferroui_controls::PageType::for_factory::<$name>();

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
pub(crate) const DESCRIPTIONS: [&str; 8] = [
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

/// An item of a composition by name: the items of a ListView and a GridView are logical children,
/// and those of an ItemsRepeater are found in its source, realized or not.
pub(crate) fn item_control<T: ferroui_base::ObjectType>(
    root: &Control,
    name: &str,
) -> Option<Ref<T>> {
    let named = |candidate: &Ref<Control>| {
        std::iter::once(candidate.clone().upcast::<ferroui_base::StyledElement>())
            .chain(candidate.get_logical_descendants())
            .find(|child| child.name().as_deref() == Some(name))
            .and_then(|child| child.cast::<T>())
    };
    root.get_logical_descendants()
        .find(|child| child.name().as_deref() == Some(name))
        .and_then(|child| child.cast::<T>())
        .or_else(|| {
            let repeater = root.find_control::<Control>("CollectionItems")?.cast::<ItemsRepeater>()?;
            let source = repeater.items_source()?;
            (0..source.count())
                .filter_map(|index| source.get_at(index))
                .filter_map(|item| Control::from_boxed(&item))
                .find_map(|item| named(&item))
        })
}

/// What activating an item of a composition does, kept on the root of the composition: a click of
/// the ListView or GridView, or a tap or Enter on an item of the ItemsRepeater.
struct ItemActivation(Rc<dyn Fn(usize)>);

impl PartialEq for ItemActivation {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Activates an item as its container would, without needing a realized container.
#[cfg(test)]
pub(crate) fn activate_item(form: &Control, index: usize) {
    let activation = form.tag().expect("the activation of the composition");
    let activation = activation.downcast_ref::<ItemActivation>().expect("an item activation");
    (activation.0)(index);
}

/// The index an item keeps in its tag.
fn item_index(item: &Control) -> Option<usize> {
    item.tag()?.downcast_ref::<usize>().copied()
}

/// `ItemClick` of a ListView or GridView.
fn wire_item_click(list: &ListViewBase, activate: &Rc<dyn Fn(usize)>) {
    let activate = activate.clone();
    list.item_click(move |_, args| {
        let index = args.clicked_item().as_ref().and_then(Control::from_boxed).and_then(|item| item_index(&item));
        if let Some(index) = index {
            activate(index);
        }
    })
    .detach();
}

/// `Item_Tapped` and `Item_KeyDown` of the ItemsRepeater page: its items take the focus themselves.
fn wire_tile_activation(tile: &Ref<Control>, index: usize, activate: &Rc<dyn Fn(usize)>) {
    tile.set_focusable(true);
    tile.set_is_tab_stop(true);
    let on_tap = activate.clone();
    tile.add_handler(InputElement::tapped_event(), move |_, e: &TappedEventArgs| {
        on_tap(index);
        e.set_handled(true);
    });
    // As a button: Enter activates on the way down, Space on the way up.
    let on_key = activate.clone();
    tile.add_handler(InputElement::key_down_event(), move |_, e: &KeyEventArgs| {
        if e.key == Key::Enter {
            on_key(index);
            e.set_handled(true);
        } else if e.key == Key::Space {
            e.set_handled(true);
        }
    });
    let on_key = activate.clone();
    tile.add_handler(InputElement::key_up_event(), move |_, e: &KeyEventArgs| {
        if e.key == Key::Space {
            on_key(index);
            e.set_handled(true);
        }
    });
}

/// Scrolls a ListView or GridView to its item, and focuses the container once it is realized.
fn return_to_item(list: &ListViewBase, index: usize) {
    let item = list.items().get_at(index);
    list.scroll_into_view_with_alignment(&item, ScrollIntoViewAlignment::Default);
    list.update_layout();
    if let Some(container) = list.container_from_index(index as i32) {
        container.focus();
    }
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

/// The source's IsTextTrimmedChanged handler exposes the whole description only
/// while it is truncated. Native layout already records collapsed text lines;
/// no additional control property or compatibility event is needed.
fn wire_trimmed_description(description: &Ref<TextBlock>) {
    let weak_description = description.downgrade();
    description.layout_updated(move || {
        let Some(description) = weak_description.upgrade() else {
            return;
        };
        if !description.is_measure_valid() || !description.is_arrange_valid() {
            return;
        }
        let collapsed = description
            .text_layout()
            .text_lines()
            .iter()
            .any(|line| line.has_collapsed());
        let tooltip = if collapsed { description.text() } else { None };
        let current = ToolTip::get_tip(&description)
            .and_then(|value| value.downcast_ref::<String>().cloned());
        if current != tooltip {
            ToolTip::set_tip(
                &description,
                tooltip.map(|text| Rc::new(text) as ferroui_base::BoxedValue),
            );
        }
    }).detach();
}

/// The `DataTemplate` of the source's ListView.
fn list_item(item: &GalleryItem, index: usize) -> (Ref<Control>, Ref<Image>, Option<String>) {
    let image = Image::new();
    image.set_name(Some(format!("ConnectedImage{}", index + 1)));
    image.set_width(150.0);
    image.set_height(100.0);
    image.set_stretch(Stretch::Fill);
    // A decoder failure stays visible in the composition, without disabling
    // its navigation or replacing the original asset with a fabricated bitmap.
    let image_error = load_image(&image, item).err();
    let body = StackPanel::new();
    body.set_name(Some(format!("ConnectedItem{}", index + 1)));
    body.set_tag(Some(Rc::new(index)));
    body.set_orientation(Orientation::Horizontal);
    body.set_spacing(12.0);
    body.set_margin(Thickness::new(0.0, 12.0, 0.0, 12.0));
    body.set_horizontal_alignment(HorizontalAlignment::Stretch);
    AutomationProperties::set_name(&body, Some(&item.title));
    body.children().add(image.clone());
    let labels = StackPanel::new();
    labels.set_spacing(4.0);
    let title = text(&item.title);
    title.set_font_size(20.0);
    labels.children().add(title);
    labels.children().add(text(&format!("Views: {}   Likes: {}", item.views, item.likes)));
    let description = text(item.description);
    description.set_max_width(500.0);
    description.set_max_lines(2);
    description.set_text_wrapping(TextWrapping::Wrap);
    description.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    wire_trimmed_description(&description);
    labels.children().add(description);
    body.children().add(labels);
    (body.upcast(), image, image_error)
}

/// The `DataTemplate` of the source's GridView and ItemsRepeater.
fn tile_item(
    item: &GalleryItem,
    index: usize,
    height: f64,
) -> Result<(Ref<Control>, Ref<Image>, Option<String>), NavigationError> {
    let tile = parse(include_str!("Pages/ConnectedTile.xaml"))?;
    tile.set_name(Some(format!("ConnectedItem{}", index + 1)));
    tile.set_tag(Some(Rc::new(index)));
    tile.set_height(height);
    AutomationProperties::set_name(&tile, Some(&item.title));
    let image = tile.get_control::<Image>("TileImage");
    image.set_name(Some(format!("ConnectedImage{}", index + 1)));
    let image_error = load_image(&image, item).err();
    tile.get_control::<TextBlock>("TileTitle").set_text(Some(&item.title));
    Ok((tile, image, image_error))
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
            if let Some(scroll) = current.find_control::<ScrollViewer>("CollectionScroll") {
                self.scroll_position.set(scroll.offset());
            }
        }
        let animation = service(source).prepare_to_animate("ForwardConnectedAnimation", source);
        self.animation.replace(animation);
        if let Err(error) = frame.navigate(ConnectedDetailPage::PAGE_TYPE, Some(Rc::new(index))) {
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
    let weak_state = Rc::downgrade(state);
    let weak_root = root.downgrade();
    let activate: Rc<dyn Fn(usize)> = Rc::new(move |index| {
        let (Some(state), Some(root)) = (weak_state.upgrade(), weak_root.upgrade()) else {
            return;
        };
        if let Some(image) = item_control::<Image>(&root, &format!("ConnectedImage{}", index + 1)) {
            state.open(index, &image);
        }
    });
    root.set_tag(Some(Rc::new(ItemActivation(activate.clone())) as ferroui_base::BoxedValue));
    let mut image_errors = Vec::new();
    if state.tiles {
        let mut tiles = Vec::with_capacity(state.items.len());
        for (index, item) in state.items.iter().enumerate() {
            let (tile, _, error) = tile_item(item, index, 120.0)?;
            image_errors.extend(error);
            wire_tile_activation(&tile, index, &activate);
            tiles.push(Some(Control::boxed(tile)));
        }
        root.get_control::<ItemsRepeater>("CollectionItems").set_items_source(Some(ItemsSource::from(tiles)));
    } else {
        let list = root.get_control::<ListView>("CollectionItems");
        for (index, item) in state.items.iter().enumerate() {
            let (body, _, error) = list_item(item, index);
            image_errors.extend(error);
            list.items().add(Some(Control::boxed(body)));
        }
        wire_item_click(&list, &activate);
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
        ConnectedRepeaterPage::PAGE_TYPE
    } else {
        ConnectedListPage::PAGE_TYPE
    };
    let factory_state = state.clone();
    frame.register_page_factory(
        source_type,
        Rc::new(move || collection_page(&factory_state)),
    )?;
    let factory_state = state.clone();
    frame.register_page_factory(
        ConnectedDetailPage::PAGE_TYPE,
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
        if event.source_page_type() == ConnectedDetailPage::PAGE_TYPE {
            if let Some(animation) = state.animation.current() {
                let image = current.get_control::<Image>("DetailedImage");
                let panel = current.get_control::<StackPanel>("CoordinatedPanel");
                animation.try_start_with_coordinated_elements(&image, &[panel.upcast()]);
            }
            state.report(&format!("{} · details", state.items[index].title));
        } else if event.navigation_mode() == NavigationMode::Back {
            if state.tiles {
                current
                    .get_control::<ScrollViewer>("CollectionScroll")
                    .set_offset(state.scroll_position.get());
            } else {
                return_to_item(&current.get_control::<ListView>("CollectionItems"), index);
            }
            current.update_layout();
            if let (Some(animation), Some(target)) = (
                state.animation.current(),
                item_control::<Image>(&current, &format!("ConnectedImage{}", index + 1)),
            ) {
                animation.try_start(&target);
            }
            if state.tiles {
                if let Some(item) = item_control::<Control>(&current, &format!("ConnectedItem{}", index + 1)) {
                    item.focus();
                }
            }
            state.report(&format!("Returned to {}", state.items[index].title));
        }
    }).detach();
    let weak_state = Rc::downgrade(&state);
    root.detached_from_visual_tree(move |_| {
        if let Some(state) = weak_state.upgrade() {
            state.animation.clear();
        }
    }).detach();
    frame.navigate(source_type, None)?;
    Ok(())
}

fn wire_simple(root: &Ref<Control>) -> Result<(), NavigationError> {
    let frame = root.get_control::<Frame>("ConnectedContentFrame");
    frame.register_page_factory(
        ConnectedFirstPage::PAGE_TYPE,
        Rc::new(|| {
            Ok(ConnectedFirstPage::with_content(parse(include_str!(
                "Pages/NavigationSample1.xaml"
            ))?))
        }),
    )?;
    frame.register_page_factory(
        ConnectedSecondPage::PAGE_TYPE,
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
            let forward = frame.current_source_page_type() == Some(ConnectedFirstPage::PAGE_TYPE);
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
                ConnectedSecondPage::PAGE_TYPE
            } else {
                ConnectedFirstPage::PAGE_TYPE
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
    }).detach();
    frame.navigate(ConnectedFirstPage::PAGE_TYPE, None)?;
    Ok(())
}

fn wire_cards(root: &Ref<Control>) -> Result<(), NavigationError> {
    let frame = root.get_control::<Frame>("ConnectedCardFrame");
    frame.register_page_factory(
        ConnectedCardPage::PAGE_TYPE,
        Rc::new(|| {
            let cards = parse(include_str!("Pages/ConnectedCards.xaml"))?;
            let items = Rc::new(data(13));
            let selected = Rc::new(Cell::new(None::<usize>));
            let active = Rc::new(ActiveAnimation::default());
            let grid = cards.get_control::<GridView>("CardItems");
            let mut image_errors: Vec<String> = Vec::new();
            let mut item_errors: Vec<Option<String>> = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let (tile, _, image_error) = tile_item(item, index, 110.0)?;
                tile.set_margin(Thickness::uniform(4.0));
                image_errors.extend(image_error.clone());
                item_errors.push(image_error);
                grid.items().add(Some(Control::boxed(tile)));
            }
            let weak_cards = cards.downgrade();
            let open_selected = selected.clone();
            let open_active = active.clone();
            let open_items = items.clone();
            // `TipsGrid_ItemClick`.
            let activate: Rc<dyn Fn(usize)> = Rc::new(move |index| {
                let Some(cards) = weak_cards.upgrade() else {
                    return;
                };
                let Some(tile) = item_control::<Control>(&cards, &format!("ConnectedItem{}", index + 1)) else {
                    return;
                };
                open_active.clear();
                open_selected.set(Some(index));
                let animation = service(&tile).prepare_to_animate("forwardAnimation", &tile);
                open_active.replace(animation.clone());
                let item = &open_items[index];
                let image = cards.get_control::<Image>("DetailImage");
                // The identical already-decoded item image is shared with the detail view.
                if let Some(source) = item_control::<Image>(&tile, &format!("ConnectedImage{}", index + 1)) {
                    image.set_source(source.source());
                }
                AutomationProperties::set_name(&image, Some(&item.title));
                let errors: Vec<_> = item_errors[index].iter().cloned().collect();
                show_image_errors(&cards.get_control::<TextBlock>("DetailImageError"), &errors);
                cards.get_control::<TextBlock>("CardTitle").set_text(Some(&item.title));
                cards.get_control::<TextBlock>("CardDescription").set_text(Some(item.description));
                cards.get_control::<Grid>("SmokeGrid").set_is_visible(true);
                animation.try_start(&cards.get_control::<Border>("DestinationElement"));
            });
            cards.set_tag(Some(Rc::new(ItemActivation(activate.clone())) as ferroui_base::BoxedValue));
            wire_item_click(&grid, &activate);
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
                    item_control::<Control>(&cards, &format!("ConnectedItem{}", index + 1))
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
                }).detach();
                let grid = cards.get_control::<GridView>("CardItems");
                return_to_item(&grid, index);
                cards.update_layout();
                if !animation.try_start(&target) {
                    smoke.set_is_visible(false);
                    smoke.children().add(destination);
                }
            });
            cards.detached_from_visual_tree(move |_| {
                active.clear();
            }).detach();
            Ok(ConnectedCardPage::with_content(cards))
        }),
    )?;
    frame.navigate(ConnectedCardPage::PAGE_TYPE, None)?;
    Ok(())
}

pub(crate) fn wire(root: &Ref<Control>) -> Result<(), XamlLoadError> {
    if root
        .find_control::<Frame>("ConnectedCollectionFrame")
        .is_none()
    {
        return Ok(());
    }
    let load_error = |error| {
        XamlLoadError::with_inner("Could not initialize connected animation examples.", error)
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
