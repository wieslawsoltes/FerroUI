//! XAML remains XAML. Named parts are wired to the corresponding Rust commands.
use ferroui::markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui::{controls::UserControl, prelude::*};
pub fn title_view() -> Ref<UserControl> {
    load(include_str!("TitleView.xaml"))
}
pub fn settings_view() -> Ref<UserControl> {
    load(include_str!("SettingsView.xaml"))
}
pub fn statistics_view() -> Ref<UserControl> {
    load(include_str!("StatisticsView.xaml"))
}
pub fn card_game_view() -> Ref<UserControl> {
    load(include_str!("CardGameView.xaml"))
}
fn load(xaml: &str) -> Ref<UserControl> {
    FerroRuntimeXamlLoader::parse_as(xaml, None).expect("valid embedded view XAML")
}
