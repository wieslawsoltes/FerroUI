//! The original RibbonGroup.cs collapsed-popup and launcher paths, split to keep
//! native popup ownership reviewable. Floating/CAD and full Ribbon keyboard
//! ownership are still missing and tracked separately.
use super::*;
use ferroui_base::{
    CornerRadius,
    input::{Key, KeyModifiers, NavigationMethod},
    layout::HorizontalAlignment,
    metadata::{MarkupDelegate, MarkupInvokeError},
};
use ferroui_controls::{Grid, StackPanel, TextBlock, TopLevel, primitives::Popup};
pub(super) struct GroupPopup {
    pub popup: Ref<Popup>,
    pub presenter: Ref<Border>,
    pub slide_out_host: Ref<Border>,
    header: Ref<TextBlock>,
    launcher: Ref<Button>,
}
impl GroupPopup {
    fn new(group: &RibbonGroup) -> Rc<Self> {
        let presenter = Border::new();
        let header = TextBlock::new();
        header.set_horizontal_alignment(HorizontalAlignment::Center);
        header.set_font_size(group.metrics().caption_font_size());
        header.set_margin(Thickness::new(0., 0., 0., 2.));
        header.set_text(group.header().as_deref());
        RibbonTheme::set_theme_brush(
            Some(&header),
            Some(TextBlock::foreground_property().as_property()),
            Some("RibbonSecondaryForegroundBrush"),
        )
        .expect("popup header brush");
        let launcher = Button::new();
        if let Some(source) = group.dialog_launcher() {
            launcher.set_theme(source.theme());
            launcher.set_is_visible(source.is_visible());
        } else {
            launcher.set_is_visible(false);
        }
        launcher.set_horizontal_alignment(HorizontalAlignment::Right);
        let weak = group.to_ref().downgrade();
        launcher.click(move |_, _| {
            if let Some(group) = weak.upgrade() {
                group
                    .open_dialog_launcher()
                    .expect("popup launcher handler");
            }
        });
        let caption = Grid::new();
        caption.set_height(group.metrics().group_caption_height());
        caption.children().add(header.clone());
        caption.children().add(launcher.clone());
        let content = StackPanel::new();
        content.children().add(presenter.clone());
        // The source moves the existing expanded panel into this host, before
        // the caption; controls keep their identity and bindings.
        let slide_out_host = Border::new();
        slide_out_host.set_margin(Thickness::new(0., 2., 0., 0.));
        content.children().add(slide_out_host.clone());
        content.children().add(caption);
        let padding = Border::new();
        padding.set_padding(Thickness::new(4., 3., 4., 0.));
        padding.set_child(content);
        let chrome = Border::new();
        chrome.set_child(padding);
        chrome.set_border_thickness(Thickness::uniform(1.));
        chrome.set_corner_radius(CornerRadius::uniform(6.));
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::background_property().as_property()),
            Some("RibbonCommandBarBackgroundBrush"),
        )
        .expect("popup background brush");
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonPopupBorderBrush"),
        )
        .expect("popup border brush");
        let weak = group.to_ref().downgrade();
        chrome.key_down(move |_, e| {
            if e.key == Key::Escape
                && !e.handled()
                && let Some(group) = weak.upgrade()
            {
                group.close_popup();
                if let Some(button) = group.collapsed_button() {
                    button.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
                }
                e.set_handled(true);
            }
        });
        let popup = Popup::new();
        // WinUI's owned XamlRoot popup maps to a native popup with the group as
        // logical/styling parent; placement can still be window-relative. This
        // preserves inherited binding/resource context as the same panel moves.
        popup.set_popup_parent(Some(&group.to_ref().upcast::<Control>()));
        popup.set_child(chrome.clone());
        super::super::RibbonItemHelper::set_owner(&chrome, group.ribbon().map(|r| r.item_owner()))
            .expect("collapsed popup owner");
        popup.set_is_light_dismiss_enabled(true);
        let weak = group.to_ref().downgrade();
        popup
            .closed(move || {
                if let Some(group) = weak.upgrade() {
                    group.restore_from_popup()
                }
            })
            .detach();
        Rc::new(Self {
            popup,
            slide_out_host,
            presenter,
            header,
            launcher,
        })
    }
}
impl RibbonGroup {
    pub fn add_dialog_launcher_click(&self, handler: Option<MarkupDelegate>) {
        let old = {
            let mut event = self.dialog_launcher_click.borrow_mut();
            let next = MarkupDelegate::combine(event.as_ref(), handler.as_ref());
            std::mem::replace(&mut *event, next)
        };
        drop(old);
    }
    pub fn remove_dialog_launcher_click(&self, handler: Option<MarkupDelegate>) {
        let old = {
            let mut event = self.dialog_launcher_click.borrow_mut();
            let next = MarkupDelegate::remove(event.as_ref(), handler.as_ref());
            std::mem::replace(&mut *event, next)
        };
        drop(old);
    }
    pub fn open_dialog_launcher(&self) -> Result<(), MarkupInvokeError> {
        self.close_popup();
        let handler = self.dialog_launcher_click.borrow().clone();
        if let Some(handler) = handler {
            let args = Rc::new(ferroui_base::interactivity::RoutedEventArgs::new());
            handler.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(args)])?;
        }
        if let Some(command) = self.dialog_launcher_command()
            && command.can_execute(self.dialog_launcher_command_parameter().as_ref())
        {
            command.execute(self.dialog_launcher_command_parameter().as_ref());
        }
        if let Some(ribbon) = self.ribbon() {
            ribbon.on_item_invoked(
                self.to_ref().upcast(),
                self.id().map(|id| format!("{id}.launcher").into()),
                self.dialog_launcher_command_parameter(),
            );
        }
        Ok(())
    }
    pub fn is_popup_open(&self) -> bool {
        self.popup
            .borrow()
            .as_ref()
            .is_some_and(|p| p.popup.is_open())
    }
    pub(super) fn panel_in_popup(&self) -> bool {
        self.popup.borrow().as_ref().is_some_and(|p| {
            p.presenter
                .child()
                .is_some_and(|child| child.ptr_eq(&self.items_panel))
        })
    }
    pub fn popup_launcher(&self) -> Option<Ref<Button>> {
        self.popup.borrow().as_ref().map(|p| p.launcher.clone())
    }
    pub fn open_popup(&self) {
        if TopLevel::get_top_level(Some(self)).is_none() {
            return;
        }
        let existing = self.popup.borrow().clone();
        let parts = existing.unwrap_or_else(|| {
            let parts = GroupPopup::new(self);
            self.popup.replace(Some(parts.clone()));
            parts
        });
        let presenter = self.parts.borrow().presenter.clone();
        if let Some(presenter) = presenter
            && presenter
                .child()
                .is_some_and(|child| child.ptr_eq(&self.items_panel))
        {
            presenter.set_child(None);
        }
        self.apply_item_layouts(RibbonGroupState::Large, false, self.metrics());
        self.items_panel.set_is_simplified(false);
        parts.presenter.set_child(self.items_panel.clone());
        self.move_slide_out_into_popup();
        if let Some(child) = parts.popup.child() {
            child.set_value(
                ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
                self.actual_theme_variant(),
            );
        }
        parts
            .header
            .set_text(Some(&self.header().unwrap_or_default()));
        let launcher = self.dialog_launcher();
        parts.launcher.set_is_visible(
            launcher
                .as_ref()
                .is_some_and(|launcher| launcher.is_visible()),
        );
        if let Some(launcher) = launcher {
            AutomationProperties::set_name(
                &parts.launcher,
                AutomationProperties::get_name(&launcher).as_deref(),
            );
            ToolTip::set_tip(
                &parts.launcher,
                self.create_launcher_tool_tip().map(Control::boxed),
            );
        }
        let anchor = self
            .collapsed_button()
            .map(|button| button.upcast())
            .unwrap_or_else(|| self.to_ref().upcast());
        crate::primitives::RibbonPopupPlacement::place_below(&parts.popup, &anchor);
        if let Some(root) = self.ribbon() {
            root.attach_popup_keyboard(parts.popup.child().as_deref());
        }
        parts.popup.set_is_open(true);
    }
    pub fn close_popup(&self) {
        let popup = self.popup.borrow().clone();
        if let Some(popup) = popup
            && popup.popup.is_open()
        {
            popup.popup.set_is_open(false);
        }
    }
    fn restore_from_popup(&self) {
        self.move_slide_out_out_of_popup();
        let popup = self.popup.borrow().clone();
        if let Some(popup) = popup
            && popup
                .presenter
                .child()
                .is_some_and(|child| child.ptr_eq(&self.items_panel))
        {
            popup.presenter.set_child(None);
        }
        let presenter = self.parts.borrow().presenter.clone();
        if let Some(presenter) = presenter {
            presenter.set_child(self.items_panel.clone());
        }
        let state = self.applied_state.replace(None);
        if let Some(state) = state {
            self.apply_state(state.state, state.simplified, state.metrics, state.mask);
        }
    }
}
