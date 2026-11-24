use crate::interop::AutomationControlType as XamlControlType;
use ferroui_controls::automation::peers::AutomationControlType as FerroControlType;
use ferroui_controls::automation::peers::AutomationLandmarkType as FerroLandmarkType;

pub(crate) struct ControlTypeMap;

impl ControlTypeMap {
    pub(crate) fn to_xaml(type_: FerroControlType) -> XamlControlType {
        match type_ {
            FerroControlType::Button => XamlControlType::Button,
            FerroControlType::Calendar => XamlControlType::Calendar,
            FerroControlType::CheckBox => XamlControlType::CheckBox,
            FerroControlType::ComboBox => XamlControlType::ComboBox,
            FerroControlType::ComboBoxItem => XamlControlType::ListItem,
            FerroControlType::Edit => XamlControlType::Edit,
            FerroControlType::Hyperlink => XamlControlType::Hyperlink,
            FerroControlType::Image => XamlControlType::Image,
            FerroControlType::ListItem => XamlControlType::ListItem,
            FerroControlType::List => XamlControlType::List,
            FerroControlType::Menu => XamlControlType::Menu,
            FerroControlType::MenuBar => XamlControlType::MenuBar,
            FerroControlType::MenuItem => XamlControlType::MenuItem,
            FerroControlType::ProgressBar => XamlControlType::ProgressBar,
            FerroControlType::RadioButton => XamlControlType::RadioButton,
            FerroControlType::ScrollBar => XamlControlType::ScrollBar,
            FerroControlType::Slider => XamlControlType::Slider,
            FerroControlType::Spinner => XamlControlType::Spinner,
            FerroControlType::StatusBar => XamlControlType::StatusBar,
            FerroControlType::Tab => XamlControlType::Tab,
            FerroControlType::TabItem => XamlControlType::TabItem,
            FerroControlType::Text => XamlControlType::Text,
            FerroControlType::ToolBar => XamlControlType::ToolBar,
            FerroControlType::ToolTip => XamlControlType::ToolTip,
            FerroControlType::Tree => XamlControlType::Tree,
            FerroControlType::TreeItem => XamlControlType::TreeItem,
            FerroControlType::Group => XamlControlType::Group,
            FerroControlType::Thumb => XamlControlType::Thumb,
            FerroControlType::DataGrid => XamlControlType::DataGrid,
            FerroControlType::DataItem => XamlControlType::DataItem,
            FerroControlType::Document => XamlControlType::Document,
            FerroControlType::SplitButton => XamlControlType::SplitButton,
            FerroControlType::Window => XamlControlType::Window,
            FerroControlType::Pane => XamlControlType::Pane,
            FerroControlType::Header => XamlControlType::Header,
            FerroControlType::HeaderItem => XamlControlType::HeaderItem,
            FerroControlType::Table => XamlControlType::Table,
            FerroControlType::TitleBar => XamlControlType::TitleBar,
            FerroControlType::Separator => XamlControlType::Separator,
            FerroControlType::Expander => XamlControlType::Group,
            _ => XamlControlType::Custom,
        }
    }

    /// WinUI's `AutomationLandmarkType`: None 0, Custom 1, Form 2, Main 3, Navigation 4, Search 5.
    /// The landmark types WinUI does not have are its custom one, named by
    /// [`landmark_name`](Self::landmark_name).
    pub(crate) fn landmark_to_xaml(landmark: Option<FerroLandmarkType>) -> i32 {
        match landmark {
            None => 0,
            Some(
                FerroLandmarkType::Banner
                | FerroLandmarkType::Complementary
                | FerroLandmarkType::ContentInfo
                | FerroLandmarkType::Region
                | FerroLandmarkType::Custom,
            ) => 1,
            Some(FerroLandmarkType::Form) => 2,
            Some(FerroLandmarkType::Main) => 3,
            Some(FerroLandmarkType::Navigation) => 4,
            Some(FerroLandmarkType::Search) => 5,
        }
    }

    /// The name of a landmark type WinUI has no value for; `None` for one it names itself.
    pub(crate) fn landmark_name(landmark: Option<FerroLandmarkType>) -> Option<&'static str> {
        Some(match landmark? {
            FerroLandmarkType::Banner => "banner",
            FerroLandmarkType::Complementary => "complementary",
            FerroLandmarkType::ContentInfo => "content information",
            FerroLandmarkType::Region => "region",
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the integration.
    use super::*;

    #[test]
    fn the_control_types_of_xaml() {
        assert_eq!(XamlControlType::Button, ControlTypeMap::to_xaml(FerroControlType::Button));
        assert_eq!(XamlControlType::ListItem, ControlTypeMap::to_xaml(FerroControlType::ComboBoxItem));
        assert_eq!(XamlControlType::Group, ControlTypeMap::to_xaml(FerroControlType::Expander));
        assert_eq!(XamlControlType::Separator, ControlTypeMap::to_xaml(FerroControlType::Separator));
        assert_eq!(XamlControlType::Custom, ControlTypeMap::to_xaml(FerroControlType::None));
        assert_eq!(XamlControlType::Custom, ControlTypeMap::to_xaml(FerroControlType::ScrollViewer));
        // The values of the system (Microsoft.UI.Xaml.Automation.Peers.AutomationControlType).
        assert_eq!(0, XamlControlType::Button.0);
        assert_eq!(25, XamlControlType::Custom.0);
        assert_eq!(33, XamlControlType::Pane.0);
        assert_eq!(38, XamlControlType::Separator.0);
    }

    #[test]
    fn landmarks_are_winui_values_with_names_for_the_others() {
        use super::FerroLandmarkType as L;
        assert_eq!(ControlTypeMap::landmark_to_xaml(None), 0);
        assert_eq!(ControlTypeMap::landmark_to_xaml(Some(L::Custom)), 1);
        assert_eq!(ControlTypeMap::landmark_to_xaml(Some(L::Banner)), 1);
        assert_eq!(ControlTypeMap::landmark_to_xaml(Some(L::Form)), 2);
        assert_eq!(ControlTypeMap::landmark_to_xaml(Some(L::Main)), 3);
        assert_eq!(ControlTypeMap::landmark_to_xaml(Some(L::Navigation)), 4);
        assert_eq!(ControlTypeMap::landmark_to_xaml(Some(L::Search)), 5);
        assert_eq!(ControlTypeMap::landmark_name(Some(L::Banner)), Some("banner"));
        assert_eq!(ControlTypeMap::landmark_name(Some(L::Custom)), None);
        assert_eq!(ControlTypeMap::landmark_name(Some(L::Main)), None);
    }
}
