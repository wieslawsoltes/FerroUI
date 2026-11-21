// Copyright (c) 2026 Wiesław Šoltés. MIT. Native view services for the original CAD page.
use super::CadPage;
use crate::cad::{
    CadDrawingCanvas,
    cad_view_model::{CadEffect, CadRibbonSession, CadSearchHit},
};
use ferroribbon::controls::{IRibbonItem, Ribbon};
use ferroribbon_core::{RibbonString, search::RibbonSearchResult};
use ferroui_base::{
    Ref, WeakRef,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::{Canvas, Control, Grid, TextBlock};
use std::rc::Rc;

pub(super) struct NativeCadSession(WeakRef<CadPage>);
impl NativeCadSession {
    pub fn new(page: &Ref<CadPage>) -> Self {
        Self(page.downgrade())
    }
}
pub(super) fn ribbon_item(item: &Ref<Control>) -> Option<Rc<dyn IRibbonItem>> {
    ValueTypes::try_cast(
        &Control::boxed(item.clone()),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
}
struct NativeSearchHit {
    ribbon: WeakRef<Ribbon>,
    result: Rc<RibbonSearchResult>,
}
impl CadSearchHit for NativeSearchHit {
    fn score(&self) -> f64 {
        self.result.score()
    }
    fn execute(&self) {
        if let Some(ribbon) = self.ribbon.upgrade() {
            ribbon.execute_search_entry(self.result.entry());
        }
    }
}
impl CadRibbonSession for NativeCadSession {
    fn invoke_id(&self, id: &str) -> bool {
        let Some(page) = self.0.upgrade() else {
            return false;
        };
        let Some(item) = page
            .cad_ribbon()
            .find_item(Some(id.into()))
            .and_then(|item| ribbon_item(&item))
        else {
            return false;
        };
        item.invoke();
        true
    }
    fn search(&self, text: &str) -> Option<Box<dyn CadSearchHit>> {
        let page = self.0.upgrade()?;
        let ribbon = page.cad_ribbon();
        let result = ribbon
            .search(Some(&RibbonString::from(text)), 1)
            .iter()
            .next()
            .flatten()?;
        Some(Box::new(NativeSearchHit {
            ribbon: ribbon.downgrade(),
            result,
        }))
    }
    fn apply(&self, effect: CadEffect) {
        let Some(page) = self.0.upgrade() else { return };
        let ribbon = page.cad_ribbon();
        match effect {
            CadEffect::TabSelection(choice) => page.apply_document_tab_selection(choice),
            CadEffect::Scene(action) => page.apply_scene_action(action),
            CadEffect::WorkspaceText(value) => {
                let combo = page.named::<ferroribbon::controls::RibbonComboBox>("WorkspaceCombo");
                if combo.text() != value {
                    combo.set_text(value);
                }
            }
            CadEffect::SetLightTheme(light) => page.apply_light_theme(light),
            CadEffect::Contexts(ids) => ribbon.set_active_contextual_groups(
                &ids.iter().map(|id| Some(id.as_str())).collect::<Vec<_>>(),
            ),
            CadEffect::WorkspaceTabs(ids) => {
                for tab in ribbon
                    .tabs()
                    .iter()
                    .flatten()
                    .filter(|t| !t.is_contextual())
                {
                    tab.set_is_tab_visible(tab.id().as_ref().is_some_and(|id| ids.contains(id)));
                }
            }
            CadEffect::SelectTab(id) => {
                ribbon.select_tab_id(&id);
            }
            CadEffect::ZoomExtents => page.named::<CadDrawingCanvas>("Canvas").zoom_extents(),
            CadEffect::PlaceTextEditor => {
                let canvas = page.named::<CadDrawingCanvas>("Canvas");
                let top_left = canvas.model_to_screen(3700., 4450.);
                let bottom_right = canvas.model_to_screen(6300., 3750.);
                let editor = page.named::<Grid>("MTextEditor");
                Canvas::set_left(&editor, top_left.x);
                Canvas::set_top(&editor, top_left.y - 12.);
                let width = bottom_right.x - top_left.x;
                let height = bottom_right.y - top_left.y + 12.;
                // Math.Max propagates a NaN input; Rust f64::max would discard it.
                editor.set_width(if width.is_nan() {
                    width
                } else {
                    width.max(120.)
                });
                editor.set_height(if height.is_nan() {
                    height
                } else {
                    height.max(40.)
                });
                page.named::<TextBlock>("MTextText")
                    .set_font_size((250. * canvas.view_scale()).clamp(9., 18.));
            }
        }
    }
}
