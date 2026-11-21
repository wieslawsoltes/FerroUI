//! Floating paths from RibbonGroup.Panels.cs. The original constructs this popup chrome in code;
//! the actual item panels are moved, never copied.
use super::*;
use ferroui_base::{
    Point, Rect, Size,
    input::{InputElement, PointerEventArgs},
    layout::{HorizontalAlignment, VerticalAlignment},
    metadata::MarkupDelegate,
    styling::ControlTheme,
    utilities::{
        EventArgs,
        math_utilities::{MathUtilities, max, min},
    },
};
use ferroui_controls::{
    Application, ColumnDefinitions, Grid, PlacementMode, RowDefinitions, StackPanel, TextBlock,
    TopLevel,
    primitives::{
        Popup,
        popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment},
    },
};

pub(super) struct FloatingState {
    panel: RefCell<Option<Rc<FloatingPanel>>>,
    position: Cell<Point>,
    caption_start: Cell<Option<Point>>,
    drag_start: Cell<Point>,
    dragging: Cell<bool>,
    changed: RefCell<Option<MarkupDelegate>>,
}
impl Default for FloatingState {
    fn default() -> Self {
        Self {
            panel: RefCell::default(),
            position: Cell::new(Point::new(120., 160.)),
            caption_start: Cell::new(None),
            drag_start: Cell::new(Point::default()),
            dragging: Cell::new(false),
            changed: RefCell::default(),
        }
    }
}
struct FloatingPanel {
    popup: Ref<Popup>,
    items: Ref<Border>,
    slide_out: Ref<Border>,
    header: Ref<TextBlock>,
}
impl FloatingPanel {
    fn new(group: &RibbonGroup) -> Rc<Self> {
        let strings = ferroribbon_core::localization::RibbonStrings::current();
        let return_button = Button::new();
        let icon = crate::compat::FontIcon::new();
        icon.set_glyph(Some("\u{e70e}".into()));
        icon.set_font_size(10.);
        return_button.set_content(Some(Control::boxed(icon)));
        return_button.set_padding(Thickness::uniform(0.));
        return_button.set_width(18.);
        return_button.set_height(20.);
        return_button.set_min_width(0.);
        return_button.set_min_height(0.);
        return_button.set_horizontal_alignment(HorizontalAlignment::Center);
        if let Some(value) = Application::current()
            .and_then(|a| {
                a.resources()
                    .try_get_resource(&"RibbonChromeButtonStyle".into(), None)
            })
            .flatten()
            && let Some(theme) =
                ValueTypes::as_object(&*value).and_then(|o| o.cast::<ControlTheme>())
        {
            return_button.set_theme(theme);
        }
        let text = strings
            .return_panel_to_ribbon()
            .map(|s| s.to_utf8())
            .transpose()
            .expect("native localized string");
        AutomationProperties::set_name(&return_button, text.as_deref());
        ToolTip::set_tip(&return_button, text.map(|s| Rc::new(s) as BoxedValue));
        let weak = group.to_ref().downgrade();
        return_button.click(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.return_to_ribbon();
            }
        });
        let grip = TextBlock::new();
        grip.set_text(Some("⋮⋮"));
        grip.set_font_size(10.);
        grip.set_horizontal_alignment(HorizontalAlignment::Center);
        grip.set_vertical_alignment(VerticalAlignment::Center);
        grip.set_is_hit_test_visible(false);
        RibbonTheme::set_theme_brush(
            Some(&grip),
            Some(TextBlock::foreground_property().as_property()),
            Some("RibbonSecondaryForegroundBrush"),
        )
        .expect("floating grip brush");
        let bar = Grid::new();
        bar.set_width(20.);
        bar.set_row_definitions(RowDefinitions::parse("Auto,*").expect("source floating rows"));
        bar.children().add(return_button);
        Grid::set_row(&grip, 1);
        bar.children().add(grip);
        RibbonTheme::set_theme_brush(
            Some(&bar),
            Some(Panel::background_property().as_property()),
            Some("RibbonFloatingPanelBarBrush"),
        )
        .expect("floating bar brush");
        group.attach_float_drag(&bar.clone().upcast());
        let items = Border::new();
        items.set_padding(Thickness::new(4., 3., 4., 0.));
        let slide_out = Border::new();
        slide_out.set_padding(Thickness::new(4., 3., 4., 0.));
        slide_out.set_border_thickness(Thickness::new(0., 1., 0., 0.));
        RibbonTheme::set_theme_brush(
            Some(&slide_out),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonSeparatorBrush"),
        )
        .expect("floating separator");
        let header = TextBlock::new();
        header.set_font_size(group.metrics().caption_font_size());
        header.set_horizontal_alignment(HorizontalAlignment::Center);
        header.set_vertical_alignment(VerticalAlignment::Center);
        header.set_margin(Thickness::new(8., 0., 8., 1.));
        RibbonTheme::set_theme_brush(
            Some(&header),
            Some(TextBlock::foreground_property().as_property()),
            Some("RibbonGroupCaptionForegroundBrush"),
        )
        .expect("floating caption brush");
        let caption = Grid::new();
        caption.set_height(group.metrics().group_caption_height() + 2.);
        caption.children().add(header.clone());
        RibbonTheme::set_theme_brush(
            Some(&caption),
            Some(Panel::background_property().as_property()),
            Some("RibbonGroupCaptionBackgroundBrush"),
        )
        .expect("floating caption background");
        group.attach_float_drag(&caption.clone().upcast());
        let content = StackPanel::new();
        content.children().add(items.clone());
        content.children().add(slide_out.clone());
        content.children().add(caption);
        let layout = Grid::new();
        layout.set_column_definitions(
            ColumnDefinitions::parse("Auto,Auto").expect("source floating columns"),
        );
        layout.children().add(bar);
        Grid::set_column(&content, 1);
        layout.children().add(content);
        let chrome = Border::new();
        chrome.set_child(layout);
        chrome.set_border_thickness(Thickness::uniform(1.));
        chrome.set_corner_radius(ferroui_base::CornerRadius::uniform(3.));
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::background_property().as_property()),
            Some("RibbonCommandBarBackgroundBrush"),
        )
        .expect("floating background");
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonPopupBorderBrush"),
        )
        .expect("floating border");
        super::super::RibbonItemHelper::set_owner(&chrome, group.ribbon().map(|r| r.item_owner()))
            .expect("floating chrome owner");
        AutomationProperties::set_name(&chrome, Some(&group.header().unwrap_or_default()));
        let weak_group = group.to_ref().downgrade();
        chrome.add_handler(InputElement::right_tapped_event(), move |_, e| {
            if e.handled() {
                return;
            }
            if let Some(group) = weak_group.upgrade()
                && let Some(root) = group.ribbon()
            {
                root.show_item_context_menu(group.clone().upcast(), e.get_position(Some(&group)))
                    .expect("floating chrome context menu");
                e.set_handled(true);
            }
        });
        let popup = Popup::new();
        popup.set_child(chrome);
        popup.set_is_light_dismiss_enabled(false);
        Rc::new(Self {
            popup,
            items,
            slide_out,
            header,
        })
    }
}
impl RibbonGroup {
    pub fn is_floating(&self) -> bool {
        self.get_value(Self::is_floating_property())
    }
    pub fn set_is_floating(&self, v: bool) {
        self.set_value(Self::is_floating_property(), v);
    }
    pub fn floating_position(&self) -> Point {
        self.floating.position.get()
    }
    pub fn set_floating_position(&self, p: Point) {
        self.floating.position.set(p);
    }
    pub fn is_floating_panel_open(&self) -> bool {
        self.floating
            .panel
            .borrow()
            .as_ref()
            .is_some_and(|p| p.popup.is_open())
    }
    pub fn float(&self, position: Option<Point>) {
        if let Some(p) = position {
            self.set_floating_position(p);
        } else if self.is_loaded()
            && let Some(root) = TopLevel::get_top_level(Some(self))
        {
            self.set_floating_position(
                self.translate_point(Point::new(0., self.bounds().height + 8.), &root)
                    .expect("source floating transform"),
            );
        }
        self.set_is_floating(true);
    }
    pub fn return_to_ribbon(&self) {
        self.set_is_floating(false);
    }
    pub(super) fn on_is_floating_changed(&self) {
        self.close_popup();
        self.close_slide_out();
        if self.is_floating() {
            self.show_float();
        } else {
            self.hide_float();
        }
        self.update_visibility();
        let h = self.floating.changed.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                Some(Rc::new(EventArgs::EMPTY)),
            ])
            .expect("FloatingChanged handler");
        }
        if let Some(root) = self.ribbon() {
            root.on_group_floating_changed(self.to_ref());
        }
    }
    pub fn add_floating_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.floating.changed.borrow().as_ref(), h.as_ref());
        self.floating.changed.replace(next);
    }
    pub fn remove_floating_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.floating.changed.borrow().as_ref(), h.as_ref());
        self.floating.changed.replace(next);
    }
    #[cfg(test)]
    pub(super) fn panel_in_float(&self) -> bool {
        self.floating
            .panel
            .borrow()
            .as_ref()
            .is_some_and(|p| p.items.child().is_some_and(|c| c.ptr_eq(&self.items_panel)))
    }
    pub(crate) fn show_float(&self) {
        let ribbon = self.ribbon();
        let root = ribbon
            .as_ref()
            .and_then(|r| TopLevel::get_top_level(Some(&**r)))
            .or_else(|| TopLevel::get_top_level(Some(self)));
        let Some(root) = root.filter(|_| self.is_floating()) else {
            return;
        };
        let existing = self.floating.panel.borrow().clone();
        let parts = existing.unwrap_or_else(|| {
            let p = FloatingPanel::new(self);
            self.floating.panel.replace(Some(p.clone()));
            p
        });
        let presenter = self.parts.borrow().presenter.clone();
        if let Some(p) = presenter
            && p.child().is_some_and(|c| c.ptr_eq(&self.items_panel))
        {
            p.set_child(None);
        }
        if let Some(previous) = self
            .items_panel
            .get_logical_parent()
            .and_then(|p| p.cast::<Border>())
            && !previous.ptr_eq(&parts.items)
        {
            previous.set_child(None);
        }
        self.applied_state.replace(None);
        self.items_panel.set_is_simplified(false);
        self.apply_item_layouts(RibbonGroupState::Large, false, self.metrics());
        parts.items.set_child(self.items_panel.clone());
        if self.slide_out_items.count() > 0 {
            self.detach_slide_out_panel();
            self.apply_slide_out_layouts();
            parts.slide_out.set_child(self.slide_out_panel.clone());
            parts.slide_out.set_is_visible(true);
        } else {
            parts.slide_out.set_is_visible(false);
        }
        parts
            .header
            .set_text(Some(&self.header().unwrap_or_default()));
        // XamlRoot ownership maps to the ribbon's live TopLevel, rather than the
        // group that is about to become invisible when it starts floating.
        parts.popup.set_popup_parent(Some(&root.clone().upcast()));
        if let Some(child) = parts.popup.child() {
            child.set_value(
                ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
                ribbon
                    .as_ref()
                    .map(|r| r.actual_theme_variant())
                    .unwrap_or_else(|| self.actual_theme_variant()),
            );
        }
        self.clamp_floating_position(root.client_size());
        parts.popup.set_placement_target(root.upcast::<Control>());
        parts.popup.set_placement(PlacementMode::AnchorAndGravity);
        parts.popup.set_placement_rect(Some(Rect::default()));
        parts
            .popup
            .set_placement_anchor(PopupAnchor::TOP | PopupAnchor::LEFT);
        parts
            .popup
            .set_placement_gravity(PopupGravity::BOTTOM | PopupGravity::RIGHT);
        parts
            .popup
            .set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        self.update_float_offsets(&parts);
        if let Some(root) = ribbon {
            root.attach_popup_keyboard(parts.popup.child().as_deref());
        }
        parts.popup.set_is_open(true);
    }
    pub(crate) fn suspend_float(&self) {
        if let Some(p) = self.floating.panel.borrow().clone()
            && p.popup.is_open()
        {
            p.popup.set_is_open(false);
        }
    }
    fn hide_float(&self) {
        self.suspend_float();
        if let Some(p) = self.floating.panel.borrow().clone() {
            if p.items.child().is_some_and(|c| c.ptr_eq(&self.items_panel)) {
                p.items.set_child(None);
            }
            if p.slide_out
                .child()
                .is_some_and(|c| c.ptr_eq(&self.slide_out_panel))
            {
                p.slide_out.set_child(None);
            }
        }
        if let Some(p) = self.parts.borrow().presenter.clone() {
            p.set_child(self.items_panel.clone());
        }
        self.applied_state.replace(None);
        self.invalidate_items_layout();
        if let Some(tab) = self.tab() {
            tab.invalidate_groups_layout();
        }
    }
    fn clamp_floating_position(&self, size: Size) {
        let width = self
            .floating
            .panel
            .borrow()
            .as_ref()
            .and_then(|p| p.popup.child())
            .map(|c| c.bounds().width)
            .filter(|w| *w > 0.)
            .unwrap_or(120.);
        self.set_floating_position(clamp_position(self.floating_position(), size, width));
    }
    fn update_float_offsets(&self, p: &FloatingPanel) {
        let pos = self.floating_position();
        p.popup.set_horizontal_offset(pos.x);
        p.popup.set_vertical_offset(pos.y);
    }
    fn pointer_in_window(&self, e: &PointerEventArgs) -> Point {
        // None is the popup presentation source in FerroUI; WinUI null is the
        // owning XamlRoot. The explicit precise service bridges native popup roots
        // through continuous desktop transforms without integer PixelPoint.
        let root = self
            .ribbon()
            .and_then(|r| TopLevel::get_top_level(Some(&*r)))
            .or_else(|| TopLevel::get_top_level(Some(self)));
        e.try_get_position_precise(root.as_deref().map(|r| &**r as &ferroui_base::Visual))
            .expect(
                "FerroUI backend does not support continuous popup-to-window pointer coordinates",
            )
    }
    fn attach_float_drag(&self, handle: &Ref<Control>) {
        let weak = self.to_ref().downgrade();
        let h = handle.downgrade();
        handle.pointer_pressed(move |_, e| {
            if let (Some(g), Some(h)) = (weak.upgrade(), h.upgrade()) {
                if g.floating.panel.borrow().is_none()
                    || !e.properties().is_left_button_pressed
                    || e.source().is_some_and(|s| s.cast::<Button>().is_some())
                {
                    return;
                }
                g.floating.dragging.set(true);
                g.floating.drag_start.set(g.pointer_in_window(e));
                h.capture_pointer(e.pointer());
                e.set_handled(true);
            }
        });
        let weak = self.to_ref().downgrade();
        handle.pointer_moved(move |_, e| {
            if let Some(g) = weak.upgrade() {
                if !g.floating.dragging.get() {
                    return;
                }
                let p = g.floating.panel.borrow().clone();
                let Some(p) = p else {
                    return;
                };
                let current = g.pointer_in_window(e);
                let start = g.floating.drag_start.replace(current);
                let pos = g.floating_position();
                g.set_floating_position(Point::new(
                    pos.x + current.x - start.x,
                    pos.y + current.y - start.y,
                ));
                if let Some(root) = p
                    .popup
                    .placement_target()
                    .and_then(|c| TopLevel::get_top_level(Some(&*c)))
                {
                    g.clamp_floating_position(root.client_size());
                }
                g.update_float_offsets(&p);
                e.set_handled(true);
            }
        });
        let weak = self.to_ref().downgrade();
        let h = handle.downgrade();
        handle.pointer_released(move |_, _| {
            if let (Some(g), Some(h)) = (weak.upgrade(), h.upgrade()) {
                g.end_float_drag(&h);
            }
        });
        let weak = self.to_ref().downgrade();
        let h = handle.downgrade();
        handle.pointer_capture_lost(move |_, _| {
            if let (Some(g), Some(h)) = (weak.upgrade(), h.upgrade()) {
                g.end_float_drag(&h);
            }
        });
    }
    fn end_float_drag(&self, handle: &Control) {
        if self.floating.dragging.replace(false) {
            handle.release_pointer_captures();
            if let Some(r) = self.ribbon() {
                r.raise_state_changed();
            }
        }
    }
    pub(super) fn attach_caption_drag(
        &self,
        caption: &Ref<Control>,
    ) -> Vec<RoutedEventHandlerToken> {
        // Source handlers are instance methods: a route captured before a
        // re-template still reads the current _captionRow, rather than sender.
        let weak = self.to_ref().downgrade();
        let pressed = caption.pointer_pressed(move |_, e| {
            if let Some(g) = weak.upgrade() {
                if !g.ribbon().is_some_and(|r| r.can_float_groups())
                    || !e.properties().is_left_button_pressed
                    || e.source().is_some_and(|s| s.cast::<Button>().is_some())
                {
                    return;
                }
                let current = g.parts.borrow().caption.clone();
                let Some(current) = current else {
                    return;
                };
                g.floating.caption_start.set(Some(g.pointer_in_window(e)));
                current.capture_pointer(e.pointer());
            }
        });
        let weak = self.to_ref().downgrade();
        let moved = caption.pointer_moved(move |_, e| {
            if let Some(g) = weak.upgrade() {
                let Some(start) = g.floating.caption_start.get() else {
                    return;
                };
                let current = g.pointer_in_window(e);
                if (current.y - start.y).abs() > 16. || (current.x - start.x).abs() > 24. {
                    g.floating.caption_start.set(None);
                    let caption = g.parts.borrow().caption.clone();
                    if let Some(caption) = caption {
                        caption.release_pointer_captures();
                    }
                    g.float(Some(Point::new(
                        current.x - g.bounds().width / 2.,
                        current.y - 10.,
                    )));
                }
            }
        });
        let weak = self.to_ref().downgrade();
        let released = caption.pointer_released(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.end_caption_drag();
            }
        });
        let weak = self.to_ref().downgrade();
        let lost = caption.pointer_capture_lost(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.end_caption_drag();
            }
        });
        vec![pressed, moved, released, lost]
    }
    fn end_caption_drag(&self) {
        if self.floating.caption_start.take().is_some() {
            let caption = self.parts.borrow().caption.clone();
            if let Some(caption) = caption {
                caption.release_pointer_captures();
            }
        }
    }
    pub(super) fn detach_caption_drag(
        caption: Option<&Control>,
        tokens: &[RoutedEventHandlerToken],
    ) {
        if let Some(caption) = caption
            && tokens.len() == 4
        {
            caption.remove_handler(InputElement::pointer_pressed_event(), tokens[0]);
            caption.remove_handler(InputElement::pointer_moved_event(), tokens[1]);
            caption.remove_handler(InputElement::pointer_released_event(), tokens[2]);
            caption.remove_handler(InputElement::pointer_capture_lost_event(), tokens[3]);
        }
    }
}
fn clamp_position(p: Point, size: Size, width: f64) -> Point {
    Point::new(
        MathUtilities::clamp(p.x, 0., max(0., size.width - min(width, size.width))),
        MathUtilities::clamp(p.y, 0., max(0., size.height - 40.)),
    )
}

#[cfg(test)]
#[path = "../floating_tests.rs"]
mod tests;
