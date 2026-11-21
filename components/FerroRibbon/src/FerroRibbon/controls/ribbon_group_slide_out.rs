//! Expanded panel paths from RibbonGroup.Panels.cs. Floating/dragging is in
//! ribbon_group_floating.rs.
use super::*;
use ferroribbon_core::collections::RibbonFixedItems;
use ferroui_base::{
    CornerRadius,
    input::{Key, KeyModifiers, NavigationMethod},
    layout::{HorizontalAlignment, VerticalAlignment},
    media::Brushes,
    styling::{ControlTheme, Styles},
};
use ferroui_controls::{
    Grid, RowDefinitions, TextBlock, TopLevel,
    primitives::{Popup, ToggleButton},
};
pub(super) struct SlideOut {
    popup: Ref<Popup>,
    host: Ref<Border>,
    pin: Ref<ToggleButton>,
    header: Ref<TextBlock>,
}
impl SlideOut {
    fn new(group: &RibbonGroup) -> Rc<Self> {
        let host = Border::new();
        host.set_padding(Thickness::new(4., 3., 4., 3.));
        let pin = ToggleButton::new();
        if let Some(styles) = group
            .try_find_resource(
                &"RibbonSlideOutPinStyles".into(),
                group.actual_theme_variant().as_ref(),
            )
            .flatten()
            .and_then(|value| ValueTypes::as_object(&*value))
            .and_then(|object| object.cast::<Styles>())
        {
            // Resource providers read their visual owner's platform settings when attached.
            let weak = pin.downgrade();
            let styles = Cell::new(Some(styles));
            pin.attached_to_visual_tree(move |_| {
                if let Some(pin) = weak.upgrade()
                    && let Some(styles) = styles.take()
                {
                    pin.resources().add_merged_dictionary(styles);
                    if let Some(theme) = pin
                        .try_get_resource(&ToggleButton::TYPE.into(), None)
                        .flatten()
                        .and_then(|value| ValueTypes::as_object(&*value))
                        .and_then(|object| object.cast::<ControlTheme>())
                    {
                        pin.set_theme(theme);
                    }
                }
            })
            .detach();
        }
        let icon = crate::compat::FontIcon::new();
        icon.set_glyph(Some("\u{e718}".into()));
        icon.set_font_size(11.);
        pin.set_content(Some(Control::boxed(icon)));
        pin.set_padding(Thickness::uniform(0.));
        pin.set_width(22.);
        pin.set_height(18.);
        pin.set_min_width(0.);
        pin.set_min_height(0.);
        pin.set_border_thickness(Thickness::uniform(0.));
        pin.set_horizontal_alignment(HorizontalAlignment::Left);
        pin.set_vertical_alignment(VerticalAlignment::Center);
        pin.set_background(Some(Brushes::transparent()));
        RibbonTheme::set_theme_brush(
            Some(&pin),
            Some(TemplatedControl::foreground_property().as_property()),
            Some("RibbonSecondaryForegroundBrush"),
        )
        .expect("pin foreground");
        let weak = group.to_ref().downgrade();
        pin.click(move |_, _| {
            if let Some(group) = weak.upgrade() {
                let parts = group
                    .slide_out
                    .borrow()
                    .clone()
                    .expect("slide-out pin before parts installed");
                group.set_is_slide_out_pinned(parts.pin.is_checked() == Some(true));
            }
        });
        let header = TextBlock::new();
        header.set_font_size(group.metrics().caption_font_size());
        header.set_horizontal_alignment(HorizontalAlignment::Center);
        header.set_vertical_alignment(VerticalAlignment::Center);
        header.set_margin(Thickness::new(24., 0., 24., 1.));
        RibbonTheme::set_theme_brush(
            Some(&header),
            Some(TextBlock::foreground_property().as_property()),
            Some("RibbonGroupCaptionForegroundBrush"),
        )
        .expect("slide-out caption foreground");
        let bar = Grid::new();
        bar.set_height(group.metrics().group_caption_height() + 2.);
        bar.children().add(header.clone());
        bar.children().add(pin.clone());
        RibbonTheme::set_theme_brush(
            Some(&bar),
            Some(Panel::background_property().as_property()),
            Some("RibbonGroupCaptionBackgroundBrush"),
        )
        .expect("slide-out caption background");
        let layout = Grid::new();
        layout.set_row_definitions(
            RowDefinitions::parse("Auto,Auto").expect("fixed source grid rows"),
        );
        layout.children().add(host.clone());
        Grid::set_row(&bar, 1);
        layout.children().add(bar);
        let chrome = Border::new();
        chrome.set_child(layout);
        chrome.set_border_thickness(Thickness::uniform(1.));
        chrome.set_corner_radius(CornerRadius::new(0., 0., 3., 3.));
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::background_property().as_property()),
            Some("RibbonCommandBarBackgroundBrush"),
        )
        .expect("slide-out background");
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonPopupBorderBrush"),
        )
        .expect("slide-out border");
        let weak = group.to_ref().downgrade();
        chrome.key_down(move |_, e| {
            if e.key == Key::Escape
                && !e.handled()
                && let Some(group) = weak.upgrade()
            {
                group.set_is_slide_out_pinned(false);
                group.close_slide_out();
                if let Some(button) = group.slide_out_button() {
                    button.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
                }
                e.set_handled(true);
            }
        });
        let popup = Popup::new();
        #[cfg(windows)]
        popup.set_use_winui_light_dismiss_overlay(true);
        // WinUI's owned XamlRoot popup maps to a native popup with the group as
        // logical/styling parent; placement can still be window-relative. This
        // preserves inherited binding/resource context as the same panel moves.
        popup.set_popup_parent(Some(&group.to_ref().upcast::<Control>()));
        popup.set_child(chrome.clone());
        super::super::RibbonItemHelper::set_owner(&chrome, group.ribbon().map(|r| r.item_owner()))
            .expect("slide-out chrome owner");
        let weak = group.to_ref().downgrade();
        popup
            .closed(move || {
                if let Some(group) = weak.upgrade() {
                    let parts = group.slide_out.borrow().clone();
                    if let Some(parts) = parts
                        && parts
                            .host
                            .child()
                            .is_some_and(|p| p.ptr_eq(&group.slide_out_panel))
                    {
                        parts.host.set_child(None);
                    }
                    group.update_slide_out_button();
                }
            })
            .detach();
        Rc::new(Self {
            popup,
            host,
            pin,
            header,
        })
    }
}
impl RibbonGroup {
    pub fn is_slide_out_open(&self) -> bool {
        self.slide_out
            .borrow()
            .as_ref()
            .is_some_and(|s| s.popup.is_open())
    }
    pub fn slide_out_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().slide_out_button.clone()
    }
    pub fn is_slide_out_pinned(&self) -> bool {
        self.get_value(Self::is_slide_out_pinned_property())
    }
    pub fn set_is_slide_out_pinned(&self, value: bool) {
        self.set_value(Self::is_slide_out_pinned_property(), value);
    }
    pub fn open_slide_out(&self) {
        if self.slide_out_items.count() == 0
            || TopLevel::get_top_level(Some(self)).is_none()
            || !self.is_loaded()
            || self.is_popup_open()
            || self.is_floating()
            || !self.is_visible()
        {
            return;
        }
        let existing = self.slide_out.borrow().clone();
        let parts = existing.unwrap_or_else(|| {
            let p = SlideOut::new(self);
            self.slide_out.replace(Some(p.clone()));
            p
        });
        self.detach_slide_out_panel();
        parts.host.set_child(self.slide_out_panel.clone());
        self.apply_slide_out_layouts();
        parts
            .header
            .set_text(Some(&self.header().unwrap_or_default()));
        parts.pin.set_is_checked(Some(self.is_slide_out_pinned()));
        self.update_pin_button();
        parts
            .popup
            .set_is_light_dismiss_enabled(!self.is_slide_out_pinned());
        if let Some(child) = parts.popup.child() {
            child.set_value(
                ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
                self.actual_theme_variant(),
            );
            child.set_min_width(self.bounds().width);
        }
        if let Some(root) = self.ribbon() {
            root.attach_popup_keyboard(parts.popup.child().as_deref());
        }
        crate::primitives::RibbonPopupPlacement::place_below(&parts.popup, &self.to_ref().upcast());
        parts.popup.set_is_open(true);
        self.update_slide_out_button();
    }
    pub fn close_slide_out(&self) {
        let parts = self.slide_out.borrow().clone();
        if let Some(parts) = parts
            && parts.popup.is_open()
        {
            parts.popup.set_is_open(false);
        }
    }
    pub(super) fn on_slide_out_pinned_changed(&self) {
        let parts = self.slide_out.borrow().clone();
        if let Some(parts) = parts {
            parts
                .popup
                .set_is_light_dismiss_enabled(!self.is_slide_out_pinned());
            parts.pin.set_is_checked(Some(self.is_slide_out_pinned()));
        }
        self.update_pin_button();
        if let Some(r) = self.ribbon() {
            r.raise_state_changed();
        }
    }
    fn update_pin_button(&self) {
        let Some(parts) = self.slide_out.borrow().clone() else {
            return;
        };
        let strings = ferroribbon_core::localization::RibbonStrings::current();
        let text = if self.is_slide_out_pinned() {
            strings.unpin_panel()
        } else {
            strings.pin_panel()
        }
        .map(|s| s.to_utf8().expect("native UTF-16 pin text"));
        AutomationProperties::set_name(&parts.pin, text.as_deref());
        ToolTip::set_tip(&parts.pin, text.map(|s| Rc::new(s) as BoxedValue));
        if let Some(icon) = parts
            .pin
            .content()
            .and_then(|v| ValueTypes::as_object(&*v))
            .and_then(|o| o.cast::<crate::compat::FontIcon>())
        {
            icon.set_glyph(Some(
                if self.is_slide_out_pinned() {
                    "\u{e841}"
                } else {
                    "\u{e718}"
                }
                .into(),
            ));
        }
    }
    pub(super) fn detach_slide_out_panel(&self) {
        if let Some(border) = self
            .slide_out_panel
            .get_logical_parent()
            .and_then(|p| p.cast::<Border>())
        {
            border.set_child(None);
        }
    }
    pub(super) fn update_slide_out_button(&self) {
        let Some(button) = self.slide_out_button() else {
            return;
        };
        let visible = self.slide_out_items.iter().any(|i| {
            i.expect("NullReference: slide-out item.Visibility")
                .is_visible()
        }) && !self.is_simplified()
            && self.state() != RibbonGroupState::Collapsed;
        button.set_is_visible(visible);
        button.set_content(Some(Rc::new(
            if self.is_slide_out_open() {
                "\u{e70e}"
            } else {
                "\u{e70d}"
            }
            .to_owned(),
        )));
        let text = ferroribbon_core::localization::RibbonStrings::current().format(
            Some(&"ExpandPanel".into()),
            Some(RibbonFixedItems::new(vec![
                ferroribbon_core::localization::RibbonFormatArgument::String(
                    self.header().unwrap_or_default().into(),
                ),
            ])),
        );
        let text = text
            .expect("slide-out label format")
            .to_utf8()
            .expect("native UTF-16 slide-out label");
        AutomationProperties::set_name(&button, Some(&text));
        ToolTip::set_tip(&button, Some(Rc::new(text)));
    }
    pub(super) fn move_slide_out_into_popup(&self) {
        let Some(popup) = self.popup.borrow().clone() else {
            return;
        };
        if self.slide_out_items.count() == 0 {
            popup.slide_out_host.set_child(None);
            popup
                .slide_out_host
                .set_border_thickness(Thickness::uniform(0.));
            return;
        }
        self.close_slide_out();
        self.detach_slide_out_panel();
        self.apply_slide_out_layouts();
        popup
            .slide_out_host
            .set_border_thickness(Thickness::new(0., 1., 0., 0.));
        popup
            .slide_out_host
            .set_padding(Thickness::new(0., 3., 0., 0.));
        RibbonTheme::set_theme_brush(
            Some(&popup.slide_out_host),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonSeparatorBrush"),
        )
        .expect("collapsed slide-out separator");
        popup.slide_out_host.set_child(self.slide_out_panel.clone());
    }
    pub(super) fn move_slide_out_out_of_popup(&self) {
        let popup = self.popup.borrow().clone();
        if let Some(popup) = popup
            && popup
                .slide_out_host
                .child()
                .is_some_and(|p| p.ptr_eq(&self.slide_out_panel))
        {
            popup.slide_out_host.set_child(None);
        }
    }
}
