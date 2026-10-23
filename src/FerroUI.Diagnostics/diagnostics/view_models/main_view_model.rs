//! Port of `Diagnostics/ViewModels/MainViewModel.cs`, with the pages that
//! other crates add to the developer tools (an addition of the port, see
//! [`DevToolsPage`]).

use super::{
    CombinedTreeNode, EventsPageViewModel, HotKeyPageViewModel, LogicalTreeNode, TreePageViewModel, ViewModelBase, VisualTreeNode,
};
use crate::diagnostics::controls::{Application, ControlHighlightAdorner};
use crate::diagnostics::{DevTools, DevToolsOptions, DevToolsViewKind, IScreenshotHandler};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::input::raw::RawPointerEventArgs;
use ferroui_base::input::{IInputRoot, InputElement, InputManager, KeyboardDevice};
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{from_markup_value, into_markup_value};
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::rendering::{IRenderer, RendererDebugOverlays};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, Ref};
use ferroui_controls::{Control, TopLevel};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Addition of the port (not in the original): a page that another crate
/// adds to the developer tools, shown as a tab after the tabs of the
/// original (the logical tree, the visual tree, the events and the hidden
/// hot keys tab).
///
/// A page is a title and a factory of its view. The factory is called with
/// the view model of the developer tools window the first time the tab of
/// the page is selected in that window; the view is kept for the lifetime of
/// the window. [`MainViewModel::root`] is the object the window inspects.
///
/// ```ignore
/// let registration = MainViewModel::register_page(DevToolsPage::new("Components", |main| {
///     ComponentsPageView::new(main.root()).upcast()
/// }));
/// // ...
/// registration.dispose(); // removes the page from the windows opened afterwards
/// ```
///
/// Pages are registered per thread (the UI thread), as everything the
/// developer tools show. A registration applies to the developer tools
/// windows opened after it.
#[derive(Clone)]
pub struct DevToolsPage {
    title: String,
    create_view: Rc<dyn Fn(&Rc<MainViewModel>) -> Ref<Control>>,
}

impl DevToolsPage {
    /// A page with the title of its tab and the factory of its view.
    pub fn new(title: impl Into<String>, create_view: impl Fn(&Rc<MainViewModel>) -> Ref<Control> + 'static) -> Self {
        Self { title: title.into(), create_view: Rc::new(create_view) }
    }

    /// The title of the tab of the page.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Creates the view of the page for the view model of a developer tools
    /// window.
    pub fn create_view(&self, main_view_model: &Rc<MainViewModel>) -> Ref<Control> {
        (self.create_view)(main_view_model)
    }
}

ferroui_base::ferro_thread_local! {
    /// The pages registered on this thread, with the identifiers of their registrations.
    static REGISTERED_PAGES: RefCell<Vec<(u64, DevToolsPage)>> = const { RefCell::new(Vec::new()) };
    /// The identifier of the next registration.
    static NEXT_PAGE_ID: Cell<u64> = const { Cell::new(0) };
}

/// A page of a window: the registered page and its view, once created.
struct ExtensionPage {
    page: DevToolsPage,
    view: RefCell<Option<Ref<Control>>>,
}

/// The view model that the main view shows as the content of the selected
/// tab (`ViewModelBase? Content` of the original), or the view of an
/// extension page.
#[derive(Clone)]
enum MainContent {
    Tree(Rc<TreePageViewModel>),
    Events(Rc<EventsPageViewModel>),
    HotKeys(Rc<HotKeyPageViewModel>),
    /// Addition of the port: the view of the extension page at an index.
    Page(usize, Ref<Control>),
}

impl PartialEq for MainContent {
    /// Reference equality of the view models, as the original compares them.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MainContent::Tree(x), MainContent::Tree(y)) => Rc::ptr_eq(x, y),
            (MainContent::Events(x), MainContent::Events(y)) => Rc::ptr_eq(x, y),
            (MainContent::HotKeys(x), MainContent::HotKeys(y)) => Rc::ptr_eq(x, y),
            (MainContent::Page(x, _), MainContent::Page(y, _)) => x == y,
            _ => false,
        }
    }
}

impl MainContent {
    /// The content as an untyped value: a view model as markup holds it
    /// (`into_markup_value`), the view of a page as a control.
    fn boxed(&self) -> Option<BoxedValue> {
        match self {
            MainContent::Tree(tree) => into_markup_value(tree.clone()),
            MainContent::Events(events) => into_markup_value(events.clone()),
            MainContent::HotKeys(hot_keys) => into_markup_value(hot_keys.clone()),
            MainContent::Page(_, view) => Some(Control::boxed(view.clone())),
        }
    }
}

/// `a == b` of two input roots that may be null: reference equality.
fn same_root(a: &Option<Rc<dyn IInputRoot>>, b: &Option<Rc<dyn IInputRoot>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
}

/// The view model of the developer tools window: the pages (the logical
/// and visual trees, the events and the hot keys), the selected tab, the
/// focused element and the element under the pointer, the overlays of the
/// renderer, the screenshot of the selected control and the options.
pub struct MainViewModel {
    base: ViewModelBase,
    this: Weak<MainViewModel>,
    root: Ref<FerroObject>,
    combined_tree: OnceCell<Rc<TreePageViewModel>>,
    logical_tree: OnceCell<Rc<TreePageViewModel>>,
    visual_tree: OnceCell<Rc<TreePageViewModel>>,
    events: Rc<EventsPageViewModel>,
    hot_keys: Rc<HotKeyPageViewModel>,
    pointer_over_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    keyboard_property_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    content: RefCell<Option<MainContent>>,
    selected_tab: Cell<i32>,
    focused_control: RefCell<Option<String>>,
    pointer_over_element: RefCell<Option<Ref<InputElement>>>,
    should_visualize_margin_padding: Cell<bool>,
    freeze_popups: Cell<bool>,
    /// Addition of the port: whether the left pane of the pages (the tree, the routed events) is
    /// shown.
    is_tree_pane_open: Cell<bool>,
    /// Addition of the port: whether the side pane of the details of a control (the layout, the
    /// value frames, the pseudo-classes) is shown.
    is_side_pane_open: Cell<bool>,
    pointer_over_element_name: RefCell<Option<String>>,
    pointer_over_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    screenshot_handler: RefCell<Option<Rc<dyn IScreenshotHandler>>>,
    show_property_type: Cell<bool>,
    show_implemented_interfaces: Cell<bool>,
    pinned_properties: Rc<RefCell<HashSet<String>>>,
    focus_highlighter: RefCell<Option<Rc<dyn IBrush>>>,
    current_focus_highlight_adorner: RefCell<Option<Rc<dyn IDisposable>>>,
    startup_screen_index: Cell<Option<i32>>,
    /// Addition of the port: the pages registered when the window was
    /// created.
    extension_pages: Vec<ExtensionPage>,
}

/// Reference equality, as the class of the original.
impl PartialEq for MainViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MainViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl MainViewModel {
    /// Addition of the port: the tab of the combined tree, the first one.
    pub const COMBINED_TREE_TAB: i32 = 0;
    /// The tab of the logical tree (upstream's first).
    pub const LOGICAL_TREE_TAB: i32 = 1;
    /// The tab of the visual tree.
    pub const VISUAL_TREE_TAB: i32 = 2;
    /// The tab of the routed events.
    pub const EVENTS_TAB: i32 = 3;
    /// The tab of the hot keys (hidden; the View menu selects it).
    pub const HOT_KEYS_TAB: i32 = 4;
    /// Addition of the port: the index of the tab of the first extension
    /// page ([`DevToolsPage`]), after the five tabs of the tools.
    pub const FIRST_EXTENSION_TAB: i32 = 5;

    /// C# `new MainViewModel(FerroObject root)`: the developer tools of
    /// `root`, a top level or the application the tools are attached to.
    pub fn new(root: Ref<FerroObject>) -> Rc<Self> {
        let extension_pages = Self::registered_pages()
            .into_iter()
            .map(|page| ExtensionPage { page, view: RefCell::new(None) })
            .collect();

        let this = Rc::new_cyclic(|this: &Weak<MainViewModel>| Self {
            base: ViewModelBase::new(),
            this: this.clone(),
            root: root.clone(),
            combined_tree: OnceCell::new(),
            logical_tree: OnceCell::new(),
            visual_tree: OnceCell::new(),
            events: EventsPageViewModel::new(this.clone()),
            hot_keys: HotKeyPageViewModel::new(),
            pointer_over_subscription: RefCell::new(None),
            keyboard_property_changed_subscription: RefCell::new(None),
            content: RefCell::new(None),
            selected_tab: Cell::new(0),
            focused_control: RefCell::new(None),
            pointer_over_element: RefCell::new(None),
            should_visualize_margin_padding: Cell::new(true),
            freeze_popups: Cell::new(false),
            is_tree_pane_open: Cell::new(true),
            is_side_pane_open: Cell::new(true),
            pointer_over_element_name: RefCell::new(None),
            pointer_over_root: RefCell::new(None),
            screenshot_handler: RefCell::new(None),
            show_property_type: Cell::new(false),
            show_implemented_interfaces: Cell::new(false),
            pinned_properties: Rc::new(RefCell::new(HashSet::new())),
            focus_highlighter: RefCell::new(None),
            current_focus_highlight_adorner: RefCell::new(None),
            startup_screen_index: Cell::new(None),
            extension_pages,
        });

        let combined_tree =
            TreePageViewModel::new(&this, CombinedTreeNode::create(root.clone()), this.pinned_properties.clone());
        let _ = this.combined_tree.set(combined_tree);
        let logical_tree =
            TreePageViewModel::new(&this, LogicalTreeNode::create(root.clone()), this.pinned_properties.clone());
        let _ = this.logical_tree.set(logical_tree);
        let visual_tree =
            TreePageViewModel::new(&this, VisualTreeNode::create(root.clone()), this.pinned_properties.clone());
        let _ = this.visual_tree.set(visual_tree);

        this.update_focused_control();

        if let Some(keyboard_device) = KeyboardDevice::instance() {
            let weak = Rc::downgrade(&this);
            let subscription = keyboard_device.property_changed(move |property_name| {
                if let Some(this) = weak.upgrade() {
                    this.keyboard_property_changed(property_name);
                }
            });
            *this.keyboard_property_changed_subscription.borrow_mut() = Some(subscription);
        }
        this.set_selected_tab(0);

        let pointer_over_subscription = if let Some(top_level) = root.cast::<TopLevel>() {
            let input_root = top_level.input_root();
            *this.pointer_over_root.borrow_mut() = Some(input_root.clone());
            // Deviation (DEVIATIONS.md, Developer tools): TopLevel.PointerOverElementProperty (the
            // observable property of the element under the pointer) does not exist in the port; the
            // element under the pointer is the `pointer_over_element` of the input root of the top
            // level, which the input pipeline updates without a notification. It is read now and
            // after every pointer event of the root has been processed.
            this.set_pointer_over_element(input_root.pointer_over_element());
            match InputManager::instance() {
                Some(input_manager) => {
                    let weak = Rc::downgrade(&this);
                    input_manager.post_process().subscribe_fn(move |e| {
                        if e.downcast_ref::<RawPointerEventArgs>().is_some()
                            && std::ptr::addr_eq(Rc::as_ptr(e.root()), Rc::as_ptr(&input_root))
                        {
                            if let Some(this) = weak.upgrade() {
                                this.set_pointer_over_element(input_root.pointer_over_element());
                            }
                        }
                    })
                }
                None => Disposable::empty(),
            }
        } else {
            // `InputManager.Instance!` upstream: without an input manager there is no input to
            // observe.
            match InputManager::instance() {
                Some(input_manager) => {
                    let weak = Rc::downgrade(&this);
                    input_manager.pre_process().subscribe_fn(move |e| {
                        if let Some(pointer_event_args) = e.downcast_ref::<RawPointerEventArgs>() {
                            if let Some(this) = weak.upgrade() {
                                let root = pointer_event_args.root().clone();
                                let element = root
                                    .try_root_element()
                                    .and_then(|root_element| root_element.input_hit_test(pointer_event_args.position()));
                                this.set_pointer_over_root(Some(root));
                                this.set_pointer_over_element(element);
                            }
                        }
                    })
                }
                None => Disposable::empty(),
            }
        };
        *this.pointer_over_subscription.borrow_mut() = Some(pointer_over_subscription);

        this
    }

    /// Addition of the port: the object the developer tools inspect (the
    /// root the view model was created with).
    pub fn root(&self) -> Ref<FerroObject> {
        self.root.clone()
    }

    /// The page of the logical tree.
    /// The page of the combined tree (an addition of the port).
    pub fn combined_tree(&self) -> Rc<TreePageViewModel> {
        self.combined_tree.get().expect("the trees are created with the view model").clone()
    }

    pub fn logical_tree(&self) -> Rc<TreePageViewModel> {
        self.logical_tree.get().expect("the trees are created with the view model").clone()
    }

    /// The page of the visual tree.
    pub fn visual_tree(&self) -> Rc<TreePageViewModel> {
        self.visual_tree.get().expect("the trees are created with the view model").clone()
    }

    /// The events page.
    pub fn events(&self) -> Rc<EventsPageViewModel> {
        self.events.clone()
    }

    /// The hot keys page.
    pub fn hot_keys(&self) -> Rc<HotKeyPageViewModel> {
        self.hot_keys.clone()
    }

    pub fn freeze_popups(&self) -> bool {
        self.freeze_popups.get()
    }

    pub fn set_freeze_popups(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.freeze_popups, value, "FreezePopups");
    }

    /// Addition of the port: whether the left pane of the pages (the tree of a tree page, the
    /// routed events of the events page) is shown; the button at the left of the header of the
    /// tools toggles it.
    pub fn is_tree_pane_open(&self) -> bool {
        self.is_tree_pane_open.get()
    }

    pub fn set_is_tree_pane_open(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_tree_pane_open, value, "IsTreePaneOpen");
    }

    /// Shows or hides the left pane of the pages.
    pub fn toggle_tree_pane(&self) {
        self.set_is_tree_pane_open(!self.is_tree_pane_open());
    }

    /// Addition of the port: whether the side pane of the details of a control (the layout, the
    /// value frames, the pseudo-classes, at the right of the properties) is shown; a toggle of the
    /// command bar of the header shows or hides it.
    pub fn is_side_pane_open(&self) -> bool {
        self.is_side_pane_open.get()
    }

    pub fn set_is_side_pane_open(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_side_pane_open, value, "IsSidePaneOpen");
    }

    /// Shows or hides the side pane of the details of a control.
    pub fn toggle_side_pane(&self) {
        self.set_is_side_pane_open(!self.is_side_pane_open());
    }

    pub fn should_visualize_margin_padding(&self) -> bool {
        self.should_visualize_margin_padding.get()
    }

    pub fn set_should_visualize_margin_padding(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(
            &self.should_visualize_margin_padding,
            value,
            "ShouldVisualizeMarginPadding",
        );
    }

    pub fn toggle_visualize_margin_padding(&self) {
        self.set_should_visualize_margin_padding(!self.should_visualize_margin_padding());
    }

    /// The renderer of the root: the renderer of a top level, the renderer
    /// of the main window or view of an application.
    fn try_get_renderer(&self) -> Option<Rc<dyn IRenderer>> {
        if let Some(top_level) = self.root.cast::<TopLevel>() {
            let renderer: Rc<dyn IRenderer> = top_level.renderer();
            return Some(renderer);
        }
        if let Some(app) = self.root.cast::<Application>() {
            return app.renderer_root().map(|renderer| renderer as Rc<dyn IRenderer>);
        }
        None
    }

    fn get_debug_overlay(&self, overlay: RendererDebugOverlays) -> bool {
        self.try_get_renderer()
            .map_or(RendererDebugOverlays::NONE, |renderer| renderer.diagnostics().debug_overlays())
            .intersects(overlay)
    }

    fn set_debug_overlay(&self, overlay: RendererDebugOverlays, enable: bool, property_name: &str) {
        let Some(renderer) = self.try_get_renderer() else {
            return;
        };

        let diagnostics = renderer.diagnostics();
        let old_value = diagnostics.debug_overlays();
        let new_value = if enable { old_value | overlay } else { old_value & !overlay };

        if old_value == new_value {
            return;
        }

        diagnostics.set_debug_overlays(new_value);
        self.base.raise_property_changed(property_name);
    }

    pub fn show_dirty_rects_overlay(&self) -> bool {
        self.get_debug_overlay(RendererDebugOverlays::DIRTY_RECTS)
    }

    pub fn set_show_dirty_rects_overlay(&self, value: bool) {
        self.set_debug_overlay(RendererDebugOverlays::DIRTY_RECTS, value, "ShowDirtyRectsOverlay");
    }

    pub fn toggle_dirty_rects_overlay(&self) {
        self.set_show_dirty_rects_overlay(!self.show_dirty_rects_overlay());
    }

    pub fn show_fps_overlay(&self) -> bool {
        self.get_debug_overlay(RendererDebugOverlays::FPS)
    }

    pub fn set_show_fps_overlay(&self, value: bool) {
        self.set_debug_overlay(RendererDebugOverlays::FPS, value, "ShowFpsOverlay");
    }

    pub fn toggle_fps_overlay(&self) {
        self.set_show_fps_overlay(!self.show_fps_overlay());
    }

    pub fn show_layout_time_graph_overlay(&self) -> bool {
        self.get_debug_overlay(RendererDebugOverlays::LAYOUT_TIME_GRAPH)
    }

    pub fn set_show_layout_time_graph_overlay(&self, value: bool) {
        self.set_debug_overlay(RendererDebugOverlays::LAYOUT_TIME_GRAPH, value, "ShowLayoutTimeGraphOverlay");
    }

    pub fn toggle_layout_time_graph_overlay(&self) {
        self.set_show_layout_time_graph_overlay(!self.show_layout_time_graph_overlay());
    }

    pub fn show_render_time_graph_overlay(&self) -> bool {
        self.get_debug_overlay(RendererDebugOverlays::RENDER_TIME_GRAPH)
    }

    pub fn set_show_render_time_graph_overlay(&self, value: bool) {
        self.set_debug_overlay(RendererDebugOverlays::RENDER_TIME_GRAPH, value, "ShowRenderTimeGraphOverlay");
    }

    pub fn toggle_render_time_graph_overlay(&self) {
        self.set_show_render_time_graph_overlay(!self.show_render_time_graph_overlay());
    }

    /// The content of the selected tab: the view model of the page, or the
    /// view of an extension page.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().as_ref().and_then(MainContent::boxed)
    }

    /// The tree page that is the content, if the content is a tree page
    /// (C# `Content as TreePageViewModel`).
    fn content_tree(&self) -> Option<Rc<TreePageViewModel>> {
        match &*self.content.borrow() {
            Some(MainContent::Tree(tree)) => Some(tree.clone()),
            _ => None,
        }
    }

    fn set_content(&self, value: MainContent) {
        if let (Some(old_tree), MainContent::Tree(new_tree)) = (self.content_tree(), &value) {
            if let Some(control) = old_tree.selected_node().and_then(|node| node.visual().cast::<Control>()) {
                // HACK: We want to select the currently selected control in the new tree, but
                // to select nested nodes in TreeView, currently the TreeView has to be able to
                // expand the parent nodes. Because at this point the TreeView isn't visible,
                // this will fail unless we schedule the selection to run after layout.
                let new_tree = new_tree.clone();
                let _ = DispatcherTimer::run_once(
                    move || {
                        new_tree.select_control(&control);
                    },
                    Duration::ZERO,
                    DispatcherPriority::default(),
                );
            }
        }

        self.base.raise_and_set_if_changed(&self.content, Some(value), "Content");
    }

    /// The index of the selected tab: 0 the combined tree (an addition of the
    /// port), 1 the logical tree, 2 the visual tree, 3 the events, 4 the hot
    /// keys (upstream: 0 to 3, from the logical tree); from
    /// [`FIRST_EXTENSION_TAB`](Self::FIRST_EXTENSION_TAB) on, the extension
    /// pages (an addition of the port). Any other index selects the combined
    /// tree.
    pub fn selected_tab(&self) -> i32 {
        self.selected_tab.get()
    }

    pub fn set_selected_tab(&self, value: i32) {
        self.selected_tab.set(value);

        match value {
            Self::LOGICAL_TREE_TAB => self.set_content(MainContent::Tree(self.logical_tree())),
            Self::VISUAL_TREE_TAB => self.set_content(MainContent::Tree(self.visual_tree())),
            Self::EVENTS_TAB => self.set_content(MainContent::Events(self.events.clone())),
            Self::HOT_KEYS_TAB => self.set_content(MainContent::HotKeys(self.hot_keys.clone())),
            _ => match self.extension_page_view(value) {
                Some((index, view)) => self.set_content(MainContent::Page(index, view)),
                None => self.set_content(MainContent::Tree(self.combined_tree())),
            },
        }

        self.base.raise_property_changed("SelectedTab");
    }

    pub fn focused_control(&self) -> Option<String> {
        self.focused_control.borrow().clone()
    }

    fn set_focused_control(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.focused_control, value, "FocusedControl");
    }

    /// The input root the pointer is over.
    pub fn pointer_over_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.pointer_over_root.borrow().clone()
    }

    fn set_pointer_over_root(&self, value: Option<Rc<dyn IInputRoot>>) {
        let changed = !same_root(&self.pointer_over_root.borrow(), &value);
        if changed {
            *self.pointer_over_root.borrow_mut() = value;
            self.base.raise_property_changed("PointerOverRoot");
        }
    }

    /// The element under the pointer.
    pub fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        self.pointer_over_element.borrow().clone()
    }

    fn set_pointer_over_element(&self, value: Option<Ref<InputElement>>) {
        let name = value.as_ref().map(|element| element.get_type().name().to_string());
        self.base.raise_and_set_if_changed(&self.pointer_over_element, value, "PointerOverElement");
        self.set_pointer_over_element_name(name);
    }

    /// The name of the type of the element under the pointer.
    pub fn pointer_over_element_name(&self) -> Option<String> {
        self.pointer_over_element_name.borrow().clone()
    }

    fn set_pointer_over_element_name(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.pointer_over_element_name, value, "PointerOverElementName");
    }

    /// Selects the hot keys tab.
    pub fn show_hot_keys(&self) {
        self.set_selected_tab(Self::HOT_KEYS_TAB);
    }

    /// Selects `control` in the tree page that is the content.
    pub fn select_control(&self, control: &Ref<Control>) {
        let tree = self.content_tree();

        if let Some(tree) = tree {
            tree.select_control(control);
        }
    }

    /// Freezes (`true`) or resumes the value frames of the control details
    /// of the tree page that is the content.
    pub fn enable_snapshot_styles(&self, enable: bool) {
        if let Some(tree_vm) = self.content_tree() {
            if let Some(details) = tree_vm.details() {
                details.set_snapshot_frames(enable);
            }
        }
    }

    /// C# `Dispose()`: ends the subscriptions, disposes the trees and the
    /// highlight of the focused element, and turns the overlays of the
    /// renderer off.
    pub fn dispose(&self) {
        if let Some(subscription) = self.keyboard_property_changed_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        let pointer_over_subscription = self.pointer_over_subscription.borrow().clone();
        if let Some(pointer_over_subscription) = pointer_over_subscription {
            pointer_over_subscription.dispose();
        }
        self.combined_tree().dispose();
        self.logical_tree().dispose();
        self.visual_tree().dispose();
        let current_focus_highlight_adorner = self.current_focus_highlight_adorner.borrow().clone();
        if let Some(current_focus_highlight_adorner) = current_focus_highlight_adorner {
            current_focus_highlight_adorner.dispose();
        }
        if let Some(renderer) = self.try_get_renderer() {
            renderer.diagnostics().set_debug_overlays(RendererDebugOverlays::NONE);
        }
    }

    fn update_focused_control(&self) {
        let element = KeyboardDevice::instance().and_then(|keyboard_device| keyboard_device.focused_element());
        self.set_focused_control(element.as_ref().map(|element| element.get_type().name().to_string()));
        let current_focus_highlight_adorner = self.current_focus_highlight_adorner.borrow_mut().take();
        if let Some(current_focus_highlight_adorner) = current_focus_highlight_adorner {
            current_focus_highlight_adorner.dispose();
        }
        if let (Some(brush), Some(input)) = (self.focus_highlighter(), element) {
            if !DevTools::does_belong_to_dev_tool(&input) {
                let adorner = ControlHighlightAdorner::add(&input, brush);
                *self.current_focus_highlight_adorner.borrow_mut() = adorner;
            }
        }
    }

    fn keyboard_property_changed(&self, property_name: &str) {
        if property_name == "FocusedElement" {
            self.update_focused_control();
        }
    }

    /// Selects `control` in the logical or the visual tree page, and that
    /// page, if the tree has a node of it.
    pub fn request_tree_navigate_to(&self, control: &Ref<Control>, is_visual_tree: bool) {
        let tree = if is_visual_tree { self.visual_tree() } else { self.logical_tree() };

        let node = tree.find_node(control);

        if node.is_some() {
            self.set_selected_tab(if is_visual_tree { Self::VISUAL_TREE_TAB } else { Self::LOGICAL_TREE_TAB });

            tree.select_control(control);
        }
    }

    /// The index of the screen the window opens maximized on.
    pub fn startup_screen_index(&self) -> Option<i32> {
        self.startup_screen_index.get()
    }

    /// Whether a screenshot of the selected control can be taken: the
    /// content is a tree page whose selected node is a visual in a visual
    /// tree.
    pub fn can_shot(&self, _parameter: Option<BoxedValue>) -> bool {
        self.content_tree()
            .and_then(|tree| tree.selected_node())
            .and_then(|selected_node| selected_node.visual().cast::<ferroui_base::Visual>())
            .is_some_and(|visual| visual.visual_root().is_some())
    }

    /// Takes a screenshot of the control selected in the tree page that is
    /// the content, with the screenshot handler of the options.
    pub fn shot(&self, _parameter: Option<BoxedValue>) {
        let control = self
            .content_tree()
            .and_then(|tree| tree.selected_node())
            .and_then(|selected_node| selected_node.visual().cast::<Control>());
        let screenshot_handler = self.screenshot_handler.borrow().clone();
        if let (Some(control), Some(screenshot_handler)) = (control, screenshot_handler) {
            // `async void`: the screenshot is taken on the dispatcher; a failure is written to
            // the debug output.
            let take = screenshot_handler.take(&control);
            let _ = Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                if let Err(ex) = take.await {
                    if cfg!(debug_assertions) {
                        // `Debug.WriteLine` of upstream: written in debug builds only.
                        eprintln!("{ex}");
                    }
                    //TODO: Notify error
                }
            });
        }
    }

    /// Applies the options the developer tools were opened with.
    pub fn set_options(&self, options: &DevToolsOptions) {
        *self.screenshot_handler.borrow_mut() = Some(options.screenshot_handler());
        self.startup_screen_index.set(options.startup_screen_index());
        self.set_show_implemented_interfaces(options.show_implemented_interfaces());
        self.set_focus_highlighter(options.focus_highlighter_brush());
        self.set_selected_tab(match options.launch_view() {
            DevToolsViewKind::CombinedTree => Self::COMBINED_TREE_TAB,
            DevToolsViewKind::LogicalTree => Self::LOGICAL_TREE_TAB,
            DevToolsViewKind::VisualTree => Self::VISUAL_TREE_TAB,
            DevToolsViewKind::Events => Self::EVENTS_TAB,
        });

        self.hot_keys.set_options(options);
    }

    pub fn show_implemented_interfaces(&self) -> bool {
        self.show_implemented_interfaces.get()
    }

    fn set_show_implemented_interfaces(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.show_implemented_interfaces, value, "ShowImplementedInterfaces");
    }

    pub fn toggle_show_implemented_interfaces(&self, _parameter: Option<BoxedValue>) {
        self.set_show_implemented_interfaces(!self.show_implemented_interfaces());
        if let Some(view_model) = self.content_tree() {
            view_model.update_properties_view();
        }
    }

    pub fn show_details_property_type(&self) -> bool {
        self.show_property_type.get()
    }

    fn set_show_details_property_type(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.show_property_type, value, "ShowDetailsPropertyType");
    }

    pub fn toggle_show_details_property_type(&self, _parameter: Option<BoxedValue>) {
        self.set_show_details_property_type(!self.show_details_property_type());
    }

    /// The brush that highlights the focused element; `None` for no
    /// highlight.
    pub fn focus_highlighter(&self) -> Option<Rc<dyn IBrush>> {
        self.focus_highlighter.borrow().clone()
    }

    fn set_focus_highlighter(&self, value: Option<Rc<dyn IBrush>>) {
        self.base.raise_and_set_if_changed(&self.focus_highlighter, value, "FocusHighlighter");
    }

    /// Selects the brush that highlights the focused element (C#
    /// `parameter as IBrush`).
    pub fn select_focus_highlighter(&self, parameter: Option<BoxedValue>) {
        self.set_focus_highlighter(from_markup_value::<Rc<dyn IBrush>>(&parameter));
    }

    /// The pinned properties of the control details, shared by the tree
    /// pages.
    pub fn pinned_properties(&self) -> Rc<RefCell<HashSet<String>>> {
        self.pinned_properties.clone()
    }

    // --- Addition of the port: the extension pages ------------------------------------------
    // Deviation (DEVIATIONS.md, Developer tools): upstream has no pages besides its own.

    /// Registers a page for the developer tools windows opened afterwards on
    /// this thread. Disposing the returned handle removes the registration.
    pub fn register_page(page: DevToolsPage) -> Rc<dyn IDisposable> {
        let id = NEXT_PAGE_ID.with(|next| {
            let id = next.get();
            next.set(id + 1);
            id
        });
        REGISTERED_PAGES.with(|pages| pages.borrow_mut().push((id, page)));
        Disposable::create(move || {
            REGISTERED_PAGES.with(|pages| pages.borrow_mut().retain(|(page_id, _)| *page_id != id));
        })
    }

    /// The pages registered on this thread, in the order of their
    /// registrations.
    pub fn registered_pages() -> Vec<DevToolsPage> {
        REGISTERED_PAGES.with(|pages| pages.borrow().iter().map(|(_, page)| page.clone()).collect())
    }

    /// The extension pages of this window, in the order of their tabs: the
    /// page at index `i` has the tab `FIRST_EXTENSION_TAB + i`.
    pub fn extension_pages(&self) -> Vec<DevToolsPage> {
        self.extension_pages.iter().map(|page| page.page.clone()).collect()
    }

    /// The view of the extension page of a tab, created on first use, with
    /// the index of the page; `None` when no extension page has the tab.
    fn extension_page_view(&self, tab: i32) -> Option<(usize, Ref<Control>)> {
        let index = usize::try_from(tab.checked_sub(Self::FIRST_EXTENSION_TAB)?).ok()?;
        let page = self.extension_pages.get(index)?;
        let existing = page.view.borrow().clone();
        let view = match existing {
            Some(view) => view,
            None => {
                let this = self.this.upgrade()?;
                let view = page.page.create_view(&this);
                *page.view.borrow_mut() = Some(view.clone());
                view
            }
        };
        Some((index, view))
    }
}

ferro_markup_type!(class MainViewModel {
    this: Rc<MainViewModel>,
    handles: [MainViewModel, Rc<MainViewModel>, Option<Rc<MainViewModel>>],
    base: ViewModelBase,
    properties: [
        FreezePopups: bool { get: MainViewModel::freeze_popups, set: MainViewModel::set_freeze_popups },
        IsTreePaneOpen: bool { get: MainViewModel::is_tree_pane_open, set: MainViewModel::set_is_tree_pane_open },
        IsSidePaneOpen: bool { get: MainViewModel::is_side_pane_open, set: MainViewModel::set_is_side_pane_open },
        ShouldVisualizeMarginPadding: bool {
            get: MainViewModel::should_visualize_margin_padding,
            set: MainViewModel::set_should_visualize_margin_padding
        },
        ShowDirtyRectsOverlay: bool {
            get: MainViewModel::show_dirty_rects_overlay,
            set: MainViewModel::set_show_dirty_rects_overlay
        },
        ShowFpsOverlay: bool { get: MainViewModel::show_fps_overlay, set: MainViewModel::set_show_fps_overlay },
        ShowLayoutTimeGraphOverlay: bool {
            get: MainViewModel::show_layout_time_graph_overlay,
            set: MainViewModel::set_show_layout_time_graph_overlay
        },
        ShowRenderTimeGraphOverlay: bool {
            get: MainViewModel::show_render_time_graph_overlay,
            set: MainViewModel::set_show_render_time_graph_overlay
        },
        Content: Option<BoxedValue> { get: MainViewModel::content },
        SelectedTab: i32 { get: MainViewModel::selected_tab, set: MainViewModel::set_selected_tab },
        FocusedControl: Option<String> { get: MainViewModel::focused_control },
        PointerOverElement: Option<Ref<InputElement>> { get: MainViewModel::pointer_over_element },
        PointerOverElementName: Option<String> { get: MainViewModel::pointer_over_element_name },
        StartupScreenIndex: Option<i32> { get: MainViewModel::startup_screen_index },
        ShowImplementedInterfaces: bool { get: MainViewModel::show_implemented_interfaces },
        ShowDetailsPropertyType: bool { get: MainViewModel::show_details_property_type },
        FocusHighlighter: Option<Rc<dyn IBrush>> { get: MainViewModel::focus_highlighter },
    ],
    methods: [
        fn ToggleVisualizeMarginPadding() => MainViewModel::toggle_visualize_margin_padding,
        fn ToggleDirtyRectsOverlay() => MainViewModel::toggle_dirty_rects_overlay,
        fn ToggleFpsOverlay() => MainViewModel::toggle_fps_overlay,
        fn ToggleLayoutTimeGraphOverlay() => MainViewModel::toggle_layout_time_graph_overlay,
        fn ToggleRenderTimeGraphOverlay() => MainViewModel::toggle_render_time_graph_overlay,
        fn ShowHotKeys() => MainViewModel::show_hot_keys,
        fn ToggleTreePane() => MainViewModel::toggle_tree_pane,
        fn ToggleSidePane() => MainViewModel::toggle_side_pane,
        fn SelectControl(Ref<Control>) =>
            (|this: &Rc<MainViewModel>, control: Ref<Control>| this.select_control(&control)),
        fn EnableSnapshotStyles(bool) => MainViewModel::enable_snapshot_styles,
        fn RequestTreeNavigateTo(Ref<Control>, bool) =>
            (|this: &Rc<MainViewModel>, control: Ref<Control>, is_visual_tree: bool| {
                this.request_tree_navigate_to(&control, is_visual_tree)
            }),
        fn CanShot(Option<BoxedValue>) -> bool => MainViewModel::can_shot [DependsOn("SelectedNode"), DependsOn("Content")],
        fn Shot(Option<BoxedValue>) => MainViewModel::shot,
        fn ToggleShowImplementedInterfaces(Option<BoxedValue>) => MainViewModel::toggle_show_implemented_interfaces,
        fn ToggleShowDetailsPropertyType(Option<BoxedValue>) => MainViewModel::toggle_show_details_property_type,
        fn SelectFocusHighlighter(Option<BoxedValue>) => MainViewModel::select_focus_highlighter,
    ],
    notify_property_changed: MainViewModel,
});

/// What the untyped value conversions must know about the view models of
/// the main window, the events page and the hot keys page: they are
/// reference types. Registered once with `ValueTypes::register_global` from
/// the `register_types()` of the crate, which also registers their markup
/// types and their lists (`EventTreeNodeList`, `EventChainLinkList`,
/// `FiredEventList`, `HotKeyDescriptionList`).
pub fn register_main_value_types() {
    use ferroui_base::data::core::ValueTypes;

    ValueTypes::register_reference::<MainViewModel>();
    ValueTypes::register_reference::<EventsPageViewModel>();
    ValueTypes::register_reference::<super::EventTreeNodeBase>();
    ValueTypes::register_reference::<super::FiredEvent>();
    ValueTypes::register_reference::<HotKeyPageViewModel>();
    ValueTypes::register_reference::<super::HotKeyDescription>();
}
