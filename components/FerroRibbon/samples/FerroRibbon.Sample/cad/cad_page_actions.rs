// Copyright (c) 2026 Wiesław Šoltés. MIT. Framework-independent CAD page actions.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CadDocumentTab {
    Start,
    Drawing,
    FloorPlan,
    Model,
    Layout1,
    Layout2,
}
impl CadDocumentTab {
    pub const ALL: [Self; 6] = [
        Self::Start,
        Self::Drawing,
        Self::FloorPlan,
        Self::Model,
        Self::Layout1,
        Self::Layout2,
    ];
    pub fn is_file(self) -> bool {
        matches!(self, Self::Start | Self::Drawing | Self::FloorPlan)
    }
    pub fn control_name(self) -> &'static str {
        match self {
            Self::Start => "CadFileTab0",
            Self::Drawing => "CadFileTab1",
            Self::FloorPlan => "CadFileTab2",
            Self::Model => "CadLayoutTab0",
            Self::Layout1 => "CadLayoutTab1",
            Self::Layout2 => "CadLayoutTab2",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CadSurface {
    Ucs,
    ViewCube,
    Navigation,
    CommandLine,
    FileTabs,
    LayoutTabs,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CadSceneAction {
    Visibility(CadSurface, bool),
    CloseCommandLine,
    Grid(bool),
    Lineweights(bool),
    Model(bool),
    CleanScreen(bool),
    Pan(bool),
    CycleMinimized,
    PanelTitles(bool),
    FloatLayers,
    ReturnPanels,
    CursorMoved(f64, f64),
}
#[derive(Clone, Debug, Default)]
pub enum CadPageAction {
    TabSelected(CadDocumentTab),
    Scene(CadSceneAction),
    LayerCommitted(Option<String>),
    ApplicationItem(Option<String>),
    RecentItem(Option<String>),
    Exit,
    #[default]
    Options,
}
impl CadViewModel {
    pub(super) fn on_page_action(&self, action: CadPageAction) {
        match action {
            CadPageAction::TabSelected(choice) => {
                if choice.is_file() {
                    self.selected_file_tab.set(choice)
                } else {
                    self.selected_layout_tab.set(choice)
                };
                self.session.apply(CadEffect::TabSelection(choice));
            }
            CadPageAction::Scene(scene) => self.session.apply(CadEffect::Scene(scene)),
            CadPageAction::LayerCommitted(name) => self.echo(
                &format!("Current layer: \"{}\"", name.as_deref().unwrap_or("")),
                true,
            ),
            CadPageAction::ApplicationItem(label) => self.echo(
                &format!("Application menu: {}", label.as_deref().unwrap_or("")),
                true,
            ),
            CadPageAction::RecentItem(title) => self.echo(
                &format!("Opening \"{}\"...", title.as_deref().unwrap_or("")),
                true,
            ),
            CadPageAction::Exit => self.echo("EXIT is disabled in the showcase.", true),
            CadPageAction::Options => self.echo("OPTIONS", true),
        }
    }
    pub fn selected_file_tab(&self) -> CadDocumentTab {
        self.selected_file_tab.get()
    }
    pub fn selected_layout_tab(&self) -> CadDocumentTab {
        self.selected_layout_tab.get()
    }
}
