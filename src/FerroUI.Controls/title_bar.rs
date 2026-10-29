//! WinUI's `TitleBar` (`controls/dev/TitleBar/TitleBar.cpp`, `TitleBar.h` and `TitleBar.idl` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use crate::automation::peers::{AutomationPeer, TitleBarAutomationPeer};
use crate::chrome::WindowDecorationProperties;
use crate::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt, TitleBarTemplateSettings,
};
use crate::{Button, ColumnDefinition, Control, ControlImpl, GridLength, IconSource, TopLevel, Window, WindowBase};
use ferroui_base::input::{InputElementImpl, WindowDecorationsElementRole};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::FlowDirection;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, AttachedProperty,
    BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_BACK_BUTTON_VISIBLE: &str = ":back-button-visible";
const PC_PANE_TOGGLE_BUTTON_VISIBLE: &str = ":pane-toggle-button-visible";
const PC_LEFT_HEADER_VISIBLE: &str = ":left-header-visible";
const PC_ICON_VISIBLE: &str = ":icon-visible";
const PC_TITLE_TEXT_VISIBLE: &str = ":title-text-visible";
const PC_SUBTITLE_TEXT_VISIBLE: &str = ":subtitle-text-visible";
const PC_CONTENT_VISIBLE: &str = ":content-visible";
const PC_RIGHT_HEADER_VISIBLE: &str = ":right-header-visible";
const PC_DEACTIVATED: &str = ":deactivated";
const PC_EXPANDED_HEIGHT: &str = ":expanded-height";
const PC_COMPACT: &str = ":compact";
const PC_NEGATIVE_INSET_SPACING: &str = ":negative-inset-spacing";

/// The name and the tool tip of the back button and of the pane toggle button (WinUI's
/// `ToolTipService.ToolTip` of the template).
const BACK_BUTTON_NAME: &str = "Back";
const PANE_TOGGLE_BUTTON_NAME: &str = "Navigation";

/// The template parts the title bar looks up.
#[derive(Default)]
struct Parts {
    left_padding: Option<Ref<ColumnDefinition>>,
    right_padding: Option<Ref<ColumnDefinition>>,
    back: Option<(Ref<Button>, RoutedEventHandlerToken)>,
    toggle: Option<(Ref<Button>, RoutedEventHandlerToken)>,
    icon: Option<Ref<Control>>,
    left_header: Option<Ref<Control>>,
    content_grid: Option<Ref<Control>>,
    content: Option<Ref<Control>>,
    right_header: Option<Ref<Control>>,
}

/// The title bar of a window (WinUI's `TitleBar`): a back button, a pane toggle button, a left
/// header, an icon, a title and a subtitle, content (a search box, in WinUI Gallery) and a right
/// header, drawn in the title bar of the window when the client area extends into it.
///
/// The control sets the states of WinUI as pseudo-classes, which the control theme of the WinUI
/// theme (`src/FerroUI.Themes.WinUI/Controls/TitleBar.xaml`) styles: `:back-button-visible`,
/// `:pane-toggle-button-visible`, `:left-header-visible`, `:icon-visible`,
/// `:title-text-visible`, `:subtitle-text-visible`, `:content-visible`,
/// `:right-header-visible` (the Visible states of their groups; their absence is Collapsed),
/// `:deactivated` (the Deactivated states: the window is not active), `:expanded-height`
/// (HeightGroup: content or a header), `:compact` (DisplayModeGroup: the content no longer fits
/// beside the title) and `:negative-inset-spacing` (LeftHeaderSpacingGroup). The template parts
/// are WinUI's: `LeftPaddingColumn`, `RightPaddingColumn`, `PART_BackButton`,
/// `PART_PaneToggleButton`, `PART_Icon`, `PART_LeftHeaderPresenter`, `PART_ContentPresenterGrid`,
/// `PART_ContentPresenter` and `PART_RightHeaderPresenter`. The Fluent and Simple themes have no
/// template for it (docs/porting/winui-controls.md, section 5.1).
///
/// The window: WinUI's `AppWindow` gives the insets of the caption buttons, the drag regions and
/// the activation. The port takes the insets from [`Window::caption_button_insets`] (the traffic
/// lights of macOS at the left, the caption buttons the toolkit draws at the right), marks the
/// title bar as the drag region of the window and its back button, pane toggle button, headers and
/// the interactive controls of its content as elements that take the input
/// ([`WindowDecorationProperties`] roles, which take effect while the client area extends into the
/// decorations), and follows [`WindowBase::is_active`]. A title is the title of the window while
/// the bar shows it, as in WinUI.
#[repr(C)]
pub struct TitleBar {
    base: TemplatedControl,
    settings: Ref<TitleBarTemplateSettings>,
    parts: RefCell<Parts>,
    is_compact: Cell<bool>,
    compact_mode_threshold_width: Cell<f64>,
    window: RefCell<Option<WeakRef<Window>>>,
    window_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// The elements the bar marked as taking the input (their role is reset when the list changes).
    interactable: RefCell<Vec<WeakRef<Visual>>>,
    /// The title of the window before the bar set its own (WinUI's `m_defaultAppWindowTitle`).
    default_window_title: RefCell<Option<Option<String>>>,
    back_requested: HandlerList<dyn Fn(&TitleBar)>,
    pane_toggle_requested: HandlerList<dyn Fn(&TitleBar)>,
}

ferro_class!(TitleBar: TemplatedControl);
ferro_class_info!(TitleBar {
    new: TitleBar::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        events: [
            BackRequested(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<TitleBar>, handler: MarkupDelegate| {
                this.back_requested(move |sender: &TitleBar| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(None::<BoxedValue>)]);
                })
            },
            PaneToggleRequested(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<TitleBar>, handler: MarkupDelegate| {
                this.pane_toggle_requested(move |sender: &TitleBar| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(None::<BoxedValue>)]);
                })
            },
        ],
        attributes: [
            TemplatePart("PART_BackButton", type(Ref<Button>)),
            TemplatePart("PART_PaneToggleButton", type(Ref<Button>)),
            TemplatePart("PART_Icon", type(Ref<Control>)),
            TemplatePart("PART_LeftHeaderPresenter", type(Ref<Control>)),
            TemplatePart("PART_ContentPresenterGrid", type(Ref<Control>)),
            TemplatePart("PART_ContentPresenter", type(Ref<Control>)),
            TemplatePart("PART_RightHeaderPresenter", type(Ref<Control>)),
            PseudoClasses(
                ":back-button-visible",
                ":pane-toggle-button-visible",
                ":left-header-visible",
                ":icon-visible",
                ":title-text-visible",
                ":subtitle-text-visible",
                ":content-visible",
                ":right-header-visible",
                ":deactivated",
                ":expanded-height",
                ":compact",
                ":negative-inset-spacing"
            ),
        ],
    },
});
ferro_impl_classes!(TitleBar: StyledElementImpl, InteractiveImpl, InputElementImpl);

impl FerroObjectImpl for TitleBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_states();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::title_property().as_property() {
            this.update_title();
        } else if property == Self::icon_source_property().as_property() {
            this.update_icon();
        } else if property == Visual::bounds_property().as_property() {
            // TitleBar::OnSizeChanged.
            this.on_size_changed();
            return;
        } else if property == Visual::flow_direction_property().as_property() {
            this.update_padding();
            return;
        } else if property == Self::content_property().as_property()
            || property == Self::left_header_property().as_property()
            || property == Self::right_header_property().as_property()
            || property == Self::is_back_button_visible_property().as_property()
            || property == Self::is_back_button_enabled_property().as_property()
            || property == Self::is_pane_toggle_button_visible_property().as_property()
            || property == Self::subtitle_property().as_property()
        {
        } else {
            return;
        }
        this.update_states();
        this.update_interactable_elements();
    }
}

impl VisualImpl for TitleBar {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attach_to_window();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.detach_from_window();
    }
}

impl LayoutableImpl for TitleBar {}

impl ControlImpl for TitleBar {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        TitleBarAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for TitleBar {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        this.detach_buttons();
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        let mut parts = Parts {
            left_padding: scope.find_as::<ColumnDefinition>("LeftPaddingColumn"),
            right_padding: scope.find_as::<ColumnDefinition>("RightPaddingColumn"),
            icon: scope.find_as::<Control>("PART_Icon"),
            left_header: scope.find_as::<Control>("PART_LeftHeaderPresenter"),
            content_grid: scope.find_as::<Control>("PART_ContentPresenterGrid"),
            content: scope.find_as::<Control>("PART_ContentPresenter"),
            right_header: scope.find_as::<Control>("PART_RightHeaderPresenter"),
            ..Parts::default()
        };
        if let Some(back) = scope.find_as::<Button>("PART_BackButton") {
            let weak = this.to_ref().downgrade();
            let token = back.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.raise_back_requested();
                }
            });
            name_button(&back, BACK_BUTTON_NAME);
            parts.back = Some((back, token));
        }
        if let Some(toggle) = scope.find_as::<Button>("PART_PaneToggleButton") {
            let weak = this.to_ref().downgrade();
            let token = toggle.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.raise_pane_toggle_requested();
                }
            });
            name_button(&toggle, PANE_TOGGLE_BUTTON_NAME);
            parts.toggle = Some((toggle, token));
        }
        *this.parts.borrow_mut() = parts;
        this.update_padding();
        this.update_icon();
        this.update_title();
        this.update_states();
        this.update_interactable_elements();
    }
}

fn name_button(button: &Button, name: &str) {
    use crate::automation::AutomationProperties;
    if AutomationProperties::get_name(button).is_none_or(|current| current.is_empty()) {
        AutomationProperties::set_name(button, Some(name));
    }
    crate::ToolTip::set_tip(button, Some(Rc::new(name.to_string()) as BoxedValue));
}

ferro_properties! { impl TitleBar {
    ferro_property!(
        /// Defines the `Title` property: the title the bar shows, which is also the title of its
        /// window.
        pub fn title_property() -> StyledProperty<String> {
            FerroProperty::register::<TitleBar, _>("Title", String::new())
        }
    );

    ferro_property!(
        /// Defines the `Subtitle` property: the text beside the title.
        pub fn subtitle_property() -> StyledProperty<String> {
            FerroProperty::register::<TitleBar, _>("Subtitle", String::new())
        }
    );

    ferro_property!(
        /// Defines the `IconSource` property: the icon before the title.
        pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
            FerroProperty::register::<TitleBar, _>("IconSource", None)
        }
    );

    ferro_property!(
        /// Defines the `LeftHeader` property: an element after the buttons, before the icon.
        pub fn left_header_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<TitleBar, _>("LeftHeader", None)
        }
    );

    ferro_property!(
        /// Defines the `Content` property: the element in the middle of the bar (a search box).
        pub fn content_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<TitleBar, _>("Content", None)
        }
    );

    ferro_property!(
        /// Defines the `RightHeader` property: an element at the right of the bar, before the
        /// caption buttons.
        pub fn right_header_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<TitleBar, _>("RightHeader", None)
        }
    );

    ferro_property!(
        /// Defines the `IsBackButtonVisible` property (false by default).
        pub fn is_back_button_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<TitleBar, _>("IsBackButtonVisible", false)
        }
    );

    ferro_property!(
        /// Defines the `IsBackButtonEnabled` property (true by default).
        pub fn is_back_button_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<TitleBar, _>("IsBackButtonEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `IsPaneToggleButtonVisible` property (false by default).
        pub fn is_pane_toggle_button_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<TitleBar, _>("IsPaneToggleButtonVisible", false)
        }
    );

    ferro_property!(
        /// Defines the `AutoRefreshDragRegions` property: whether the bar finds the interactive
        /// elements of its content again when the content changes its layout (false by default:
        /// once, after the content is first laid out, as in WinUI).
        pub fn auto_refresh_drag_regions_property() -> StyledProperty<bool> {
            FerroProperty::register::<TitleBar, _>("AutoRefreshDragRegions", false)
        }
    );

    ferro_property!(
        /// Defines the `TemplateSettings` property: the values the template binds to (read-only).
        pub fn template_settings_property() -> DirectProperty<TitleBar, Ref<TitleBarTemplateSettings>> {
            FerroProperty::register_direct::<TitleBar, _>(
                "TemplateSettings",
                |owner| owner.template_settings(),
                None,
                TitleBarTemplateSettings::new(),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsDragRegion` attached property of an element of the content: `Some(true)`
        /// drags the window (and, on a control, its whole area), `Some(false)` takes the input, and
        /// `None` (the default) leaves it to the bar, which makes the enabled controls take the
        /// input.
        pub fn is_drag_region_property() -> AttachedProperty<Option<bool>> {
            FerroProperty::register_attached::<TitleBar, Visual, _>("IsDragRegion", None)
        }
    );
} }

impl TitleBar {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            settings: TitleBarTemplateSettings::new(),
            parts: RefCell::new(Parts::default()),
            is_compact: Cell::new(false),
            compact_mode_threshold_width: Cell::new(0.0),
            window: RefCell::new(None),
            window_subscriptions: RefCell::new(Vec::new()),
            interactable: RefCell::new(Vec::new()),
            default_window_title: RefCell::new(None),
            back_requested: HandlerList::new(),
            pane_toggle_requested: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn title(&self) -> String {
        self.get_value(Self::title_property())
    }
    pub fn set_title(&self, value: &str) {
        self.set_value(Self::title_property(), value.to_string());
    }
    pub fn subtitle(&self) -> String {
        self.get_value(Self::subtitle_property())
    }
    pub fn set_subtitle(&self, value: &str) {
        self.set_value(Self::subtitle_property(), value.to_string());
    }
    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }
    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        self.set_value(Self::icon_source_property(), value);
    }
    pub fn left_header(&self) -> Option<Ref<Control>> {
        self.get_value(Self::left_header_property())
    }
    pub fn set_left_header(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::left_header_property(), value);
    }
    pub fn content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::content_property())
    }
    pub fn set_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::content_property(), value);
    }
    pub fn right_header(&self) -> Option<Ref<Control>> {
        self.get_value(Self::right_header_property())
    }
    pub fn set_right_header(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::right_header_property(), value);
    }
    pub fn is_back_button_visible(&self) -> bool {
        self.get_value(Self::is_back_button_visible_property())
    }
    pub fn set_is_back_button_visible(&self, value: bool) {
        self.set_value(Self::is_back_button_visible_property(), value);
    }
    pub fn is_back_button_enabled(&self) -> bool {
        self.get_value(Self::is_back_button_enabled_property())
    }
    pub fn set_is_back_button_enabled(&self, value: bool) {
        self.set_value(Self::is_back_button_enabled_property(), value);
    }
    pub fn is_pane_toggle_button_visible(&self) -> bool {
        self.get_value(Self::is_pane_toggle_button_visible_property())
    }
    pub fn set_is_pane_toggle_button_visible(&self, value: bool) {
        self.set_value(Self::is_pane_toggle_button_visible_property(), value);
    }
    pub fn auto_refresh_drag_regions(&self) -> bool {
        self.get_value(Self::auto_refresh_drag_regions_property())
    }
    pub fn set_auto_refresh_drag_regions(&self, value: bool) {
        self.set_value(Self::auto_refresh_drag_regions_property(), value);
    }
    pub fn template_settings(&self) -> Ref<TitleBarTemplateSettings> {
        self.settings.clone()
    }

    /// The `IsDragRegion` of `element`.
    pub fn get_is_drag_region(element: &Visual) -> Option<bool> {
        element.get_value(Self::is_drag_region_property())
    }

    /// Sets the `IsDragRegion` of `element`; the title bar it is in finds its interactive
    /// elements again.
    pub fn set_is_drag_region(element: &Visual, value: Option<bool>) {
        element.set_value(Self::is_drag_region_property(), value);
        let mut current = Some(element.to_ref());
        while let Some(visual) = current {
            if let Some(bar) = visual.cast::<TitleBar>() {
                bar.update_interactable_elements();
                return;
            }
            current = visual.visual_parent();
        }
    }

    /// Finds the interactive elements of the bar again after the application changed its
    /// content (WinUI's `RecomputeDragRegions`).
    pub fn recompute_drag_regions(&self) {
        self.update_layout();
        self.update_interactable_elements();
    }

    /// Adds a handler of `BackRequested`: the back button was clicked.
    pub fn back_requested(&self, handler: impl Fn(&TitleBar) + 'static) -> Rc<dyn IDisposable> {
        let token = self.back_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        ferroui_base::reactive::Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.back_requested.remove(token);
            }
        })
    }

    /// Adds a handler of `PaneToggleRequested`: the pane toggle button was clicked.
    pub fn pane_toggle_requested(&self, handler: impl Fn(&TitleBar) + 'static) -> Rc<dyn IDisposable> {
        let token = self.pane_toggle_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        ferroui_base::reactive::Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.pane_toggle_requested.remove(token);
            }
        })
    }

    fn raise_back_requested(&self) {
        for (_, handler) in self.back_requested.snapshot().iter() {
            handler(self);
        }
    }

    fn raise_pane_toggle_requested(&self) {
        for (_, handler) in self.pane_toggle_requested.snapshot().iter() {
            handler(self);
        }
    }

    #[cfg(test)]
    pub(crate) fn raise_back_requested_for_tests(&self) {
        self.raise_back_requested();
    }

    #[cfg(test)]
    pub(crate) fn raise_pane_toggle_requested_for_tests(&self) {
        self.raise_pane_toggle_requested();
    }

    fn detach_buttons(&self) {
        let mut parts = self.parts.borrow_mut();
        if let Some((back, token)) = parts.back.take() {
            back.remove_handler(Button::click_event(), token);
        }
        if let Some((toggle, token)) = parts.toggle.take() {
            toggle.remove_handler(Button::click_event(), token);
        }
    }

    /// The window of the bar, while it is in one.
    pub fn window(&self) -> Option<Ref<Window>> {
        self.window.borrow().as_ref().and_then(|window| window.upgrade())
    }

    fn attach_to_window(&self) {
        self.detach_from_window();
        let Some(window) = TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.cast::<Window>()) else {
            self.update_states();
            return;
        };
        *self.window.borrow_mut() = Some(window.downgrade());
        let weak = self.to_ref().downgrade();
        let subscription = window.property_changed(move |change| {
            let Some(bar) = weak.upgrade() else { return };
            let property = change.property();
            if property == WindowBase::is_active_property().as_property() {
                bar.update_states();
            } else if property == Window::is_extended_into_window_decorations_property().as_property()
                || property == Window::window_decoration_margin_property().as_property()
                || property == Window::window_state_property().as_property()
                || property == Visual::bounds_property().as_property()
            {
                bar.update_padding();
            }
        });
        self.window_subscriptions.borrow_mut().push(subscription);
        self.update_padding();
        self.update_title();
        self.update_states();
        self.update_interactable_elements();
    }

    fn detach_from_window(&self) {
        for subscription in self.window_subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        if self.window().is_some() {
            self.reset_window_title(&self.title());
        }
        *self.window.borrow_mut() = None;
    }

    fn is_window_active(&self) -> bool {
        self.window().is_none_or(|window| window.is_active())
    }

    /// TitleBar::UpdatePadding: the padding columns leave the caption buttons free, swapped in a
    /// right-to-left bar.
    fn update_padding(&self) {
        let insets = self.window().map(|window| window.caption_button_insets()).unwrap_or_default();
        let (left, right) = if self.flow_direction() == FlowDirection::RightToLeft {
            (insets.right, insets.left)
        } else {
            (insets.left, insets.right)
        };
        let parts = self.parts.borrow();
        if let Some(column) = &parts.left_padding {
            column.set_width(GridLength::from_pixels(left));
        }
        if let Some(column) = &parts.right_padding {
            column.set_width(GridLength::from_pixels(right));
        }
    }

    /// TitleBar::UpdateIcon.
    fn update_icon(&self) {
        let element = self.icon_source().and_then(|source| source.create_icon_element());
        self.settings.set_icon_element(element);
        self.update_states();
    }

    /// TitleBar::UpdateTitle: a title is the title of the window too; an empty one gives the
    /// window its own title back (ResetTitle).
    fn update_title(&self) {
        let title = self.title();
        let Some(window) = self.window() else { return };
        if self.default_window_title.borrow().is_none() {
            *self.default_window_title.borrow_mut() = Some(window.title());
        }
        if title.is_empty() {
            return;
        }
        if window.title().as_deref() != Some(title.as_str()) {
            window.set_title(Some(title));
        }
    }

    /// TitleBar::ResetTitle: the window gets its own title back when it still has the one the bar
    /// gave it.
    fn reset_window_title(&self, last_applied: &str) {
        let Some(window) = self.window() else { return };
        let Some(default_title) = self.default_window_title.borrow_mut().take() else { return };
        if window.title().as_deref() == Some(last_applied) && window.title() != default_title {
            window.set_title(default_title);
        }
    }

    /// TitleBar::OnSizeChanged: the bar is compact (no title and subtitle, the content at the
    /// left) once its content no longer fits, and expanded again when it is as wide as it was
    /// then.
    fn on_size_changed(&self) {
        if self.content().is_some() {
            let (content, grid) = {
                let parts = self.parts.borrow();
                (parts.content.clone(), parts.content_grid.clone())
            };
            if let (Some(content), Some(grid)) = (content, grid) {
                let width = self.bounds().width;
                if self.compact_mode_threshold_width.get() == 0.0 && content.desired_size().width >= grid.bounds().width {
                    self.compact_mode_threshold_width.set(width);
                    self.is_compact.set(true);
                    self.update_states();
                } else if self.is_compact.get() && width >= self.compact_mode_threshold_width.get() {
                    self.compact_mode_threshold_width.set(0.0);
                    self.is_compact.set(false);
                    self.update_states();
                }
            }
        }
    }

    /// The visual states of WinUI as pseudo-classes.
    fn update_states(&self) {
        let deactivated = !self.is_window_active();
        let content = self.content().is_some();
        let left_header = self.left_header().is_some();
        let right_header = self.right_header().is_some();
        let back = self.is_back_button_visible();
        let toggle = self.is_pane_toggle_button_visible();
        let compact = self.is_compact.get();
        let classes = self.pseudo_classes();
        classes.set(PC_BACK_BUTTON_VISIBLE, back);
        classes.set(PC_PANE_TOGGLE_BUTTON_VISIBLE, toggle);
        classes.set(PC_LEFT_HEADER_VISIBLE, left_header);
        classes.set(PC_ICON_VISIBLE, self.settings.icon_element().is_some());
        classes.set(PC_TITLE_TEXT_VISIBLE, !self.title().is_empty());
        classes.set(PC_SUBTITLE_TEXT_VISIBLE, !self.subtitle().is_empty());
        classes.set(PC_CONTENT_VISIBLE, content);
        classes.set(PC_RIGHT_HEADER_VISIBLE, right_header);
        classes.set(PC_DEACTIVATED, deactivated);
        classes.set(PC_EXPANDED_HEIGHT, content || left_header || right_header);
        classes.set(PC_COMPACT, compact);
        // UpdateLeftHeaderSpacing.
        classes.set(PC_NEGATIVE_INSET_SPACING, back != toggle);
    }

    /// TitleBar::UpdateInteractableElementsList and UpdateDragRegion: the bar drags the window,
    /// and its buttons, headers and the interactive elements of its content take the input.
    fn update_interactable_elements(&self) {
        for element in self.interactable.borrow_mut().drain(..) {
            if let Some(element) = element.upgrade() {
                WindowDecorationProperties::set_element_role(&element, WindowDecorationsElementRole::None);
            }
        }
        WindowDecorationProperties::set_element_role(self, WindowDecorationsElementRole::TitleBar);
        let mut list: Vec<Ref<Visual>> = Vec::new();
        {
            let parts = self.parts.borrow();
            if self.is_back_button_visible() && self.is_back_button_enabled() {
                if let Some((back, _)) = &parts.back {
                    list.push(back.clone().upcast());
                }
            }
            if self.is_pane_toggle_button_visible() {
                if let Some((toggle, _)) = &parts.toggle {
                    list.push(toggle.clone().upcast());
                }
            }
            if self.left_header().is_some() {
                if let Some(area) = &parts.left_header {
                    list.push(area.clone().upcast());
                }
            }
            if self.right_header().is_some() {
                if let Some(area) = &parts.right_header {
                    list.push(area.clone().upcast());
                }
            }
        }
        if let Some(content) = self.content() {
            Self::find_interactable_elements(&content.upcast(), false, &mut list);
        }
        let mut marked = self.interactable.borrow_mut();
        for element in list {
            WindowDecorationProperties::set_element_role(&element, WindowDecorationsElementRole::User);
            marked.push(element.downgrade());
        }
    }

    /// TitleBar::FindInteractableElements: the enabled templated controls of the content take
    /// the input unless an `IsDragRegion` of true makes them, or a container of them, drag the
    /// window; an `IsDragRegion` of false makes an element take the input.
    fn find_interactable_elements(element: &Ref<Visual>, parent_is_drag_region: bool, list: &mut Vec<Ref<Visual>>) {
        if !element.is_visible() {
            return;
        }
        if let Some(input) = element.cast::<ferroui_base::input::InputElement>() {
            if !input.is_hit_test_visible() {
                return;
            }
        }
        let mut is_drag_region = parent_is_drag_region;
        match Self::get_is_drag_region(element) {
            Some(false) => {
                list.push(element.clone());
                return;
            }
            Some(true) => {
                if element.is::<TemplatedControl>() {
                    return;
                }
                is_drag_region = true;
            }
            None => {}
        }
        if !is_drag_region {
            if let Some(control) = element.cast::<TemplatedControl>() {
                if control.is_effectively_enabled() {
                    list.push(element.clone());
                    return;
                }
            }
        }
        for child in element.get_visual_children().iter() {
            Self::find_interactable_elements(child, is_drag_region, list);
        }
    }
}
