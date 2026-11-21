// Copyright (c) 2026 Wiesław Šoltés. MIT. Original CAD file/layout tab state.
use super::CadPage;
use crate::cad::{
    CadDrawingCanvas,
    cad_view_model::{CadDocumentTab, CadPageAction},
};
use ferroribbon::{controls::RibbonToggleButton, theming::RibbonTheme};
use ferroui_base::{
    data::core::ValueTypes,
    media::{Colors, FontWeight, IBrush, SolidColorBrush},
    styling::ControlTheme,
};
use ferroui_controls::{Application, Border, Button, StackPanel, TextBlock};
use std::rc::Rc;

impl CadPage {
    pub(super) fn connect_document_tabs(&self) {
        let tabs: Vec<_> = CadDocumentTab::ALL
            .into_iter()
            .map(|choice| {
                let tab = self.named::<Border>(choice.control_name());
                let weak = Rc::downgrade(&self.view_model());
                tab.tapped(move |_, _| {
                    if let Some(model) = weak.upgrade() {
                        model
                            .page_action()
                            .execute(Some(&CadPageAction::TabSelected(choice)));
                    }
                });
                (tab, choice)
            })
            .collect();
        assert!(self.document_tabs.set(tabs).is_ok());
        self.apply_document_tab_add_button_theme();
    }
    pub(super) fn apply_document_tab_add_button_theme(&self) {
        // Source assigns a chrome style only when the application resource is a Style.
        if let Some(theme) = Application::current()
            .and_then(|app| {
                app.resources()
                    .try_get_resource(&"RibbonChromeButtonStyle".into(), None)
            })
            .flatten()
            .and_then(|v| ValueTypes::as_object(&*v))
            .and_then(|v| v.cast::<ControlTheme>())
        {
            for name in ["CadFileAdd", "CadLayoutAdd"] {
                self.named::<Button>(name).set_theme(Some(theme.clone()));
            }
        }
    }
    pub(super) fn apply_document_tab_selection(&self, choice: CadDocumentTab) {
        if !choice.is_file() {
            let model = choice == CadDocumentTab::Model;
            let toggle = self.named::<RibbonToggleButton>("ModelToggle");
            toggle.set_is_checked(Some(model));
            toggle.set_label(Some(if model { "MODEL" } else { "PAPER" }.into()));
        }
        self.update_document_tab_visuals();
    }
    pub(super) fn update_document_tab_visuals(&self) {
        let canvas: Rc<dyn IBrush> =
            SolidColorBrush::with_color(self.named::<CadDrawingCanvas>("Canvas").canvas_color())
                .into();
        let brush = |key| {
            RibbonTheme::get_brush(Some(self), Some(key))
                .expect("fixed CAD theme brush key")
                .map(Into::<Rc<dyn IBrush>>::into)
        };
        if let Some(tabs) = self.document_tabs.get() {
            let model = self.view_model();
            let selected_file = tabs
                .iter()
                .find(|(_, choice)| *choice == model.selected_file_tab())
                .map(|(tab, _)| tab);
            let selected_layout = tabs
                .iter()
                .find(|(_, choice)| *choice == model.selected_layout_tab())
                .map(|(tab, _)| tab);
            for (tab, choice) in tabs {
                let selected = if choice.is_file() {
                    selected_file
                } else {
                    selected_layout
                }
                .is_some_and(|current| tab.ptr_eq(current));
                tab.set_background(if selected {
                    if choice.is_file() {
                        Some(canvas.clone())
                    } else {
                        brush("RibbonCommandBarBackgroundBrush")
                    }
                } else {
                    Some(SolidColorBrush::with_color(Colors::TRANSPARENT).into())
                });
                // Preserve the source pattern guard if application code replaces a child.
                if let Some(label) = tab
                    .child()
                    .and_then(|child| child.cast::<StackPanel>())
                    .and_then(|content| {
                        let children = content.children();
                        (children.count() > 0).then(|| children.get(0))
                    })
                    .and_then(|child| child.cast::<TextBlock>())
                {
                    label.set_foreground(brush(if selected {
                        "RibbonForegroundBrush"
                    } else {
                        "RibbonSecondaryForegroundBrush"
                    }));
                    label.set_font_weight(if selected {
                        FontWeight::SemiBold
                    } else {
                        FontWeight::Normal
                    });
                }
            }
        }
        self.named::<Border>("FileTabsRow")
            .set_background(brush("RibbonWindowBackgroundBrush"));
    }
}
