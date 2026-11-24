//! The Android widget class a control reports to accessibility services, from which TalkBack
//! takes the role it announces.

use ferroui_base::Ref;
use ferroui_controls::automation::peers::{AutomationControlType, AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::IToggleProvider;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{CalendarDatePicker, DatePicker, NumericUpDown, TimePicker, ToggleSwitch};

fn role(control_type: AutomationControlType) -> Option<&'static str> {
    Some(match control_type {
        AutomationControlType::Button | AutomationControlType::SplitButton => "android.widget.Button",
        AutomationControlType::Calendar => "android.widget.DatePicker",
        AutomationControlType::CheckBox => "android.widget.CheckBox",
        AutomationControlType::ComboBox => "android.widget.Spinner",
        AutomationControlType::Image => "android.widget.ImageView",
        AutomationControlType::List => "android.widget.ListView",
        AutomationControlType::ProgressBar => "android.widget.ProgressBar",
        AutomationControlType::RadioButton => "android.widget.RadioButton",
        AutomationControlType::ScrollViewer => "android.widget.ScrollView",
        AutomationControlType::Slider => "android.widget.SeekBar",
        AutomationControlType::Tab => "android.widget.TabWidget",
        AutomationControlType::Edit => "android.widget.EditText",
        AutomationControlType::Text => "android.widget.TextView",
        _ => return None,
    })
}

/// The class name of the node of `peer`: an override of the application, the widget of a control
/// that has a more specific one, the widget of its control type, or the class name of the peer.
pub(crate) fn get_class_name(peer: &Ref<AutomationPeer>) -> String {
    let Some(control_peer) = peer.clone().cast::<ControlAutomationPeer>() else {
        return peer.get_class_name();
    };
    let owner = control_peer.owner();

    if let Some(class_name) = AutomationProperties::get_class_name_override(&owner) {
        return class_name;
    }

    let control_type = peer.get_automation_control_type();
    let specific = match control_type {
        AutomationControlType::Button if owner.cast::<ToggleSwitch>().is_some() => Some("android.widget.Switch"),
        AutomationControlType::Button | AutomationControlType::SplitButton
            if peer.get_provider::<dyn IToggleProvider>().is_some() =>
        {
            Some("android.widget.ToggleButton")
        }
        AutomationControlType::Spinner if owner.cast::<NumericUpDown>().is_some() => {
            Some("android.widget.NumberPicker")
        }
        AutomationControlType::Custom if owner.cast::<DatePicker>().is_some() => Some("android.widget.DatePicker"),
        AutomationControlType::Button if owner.cast::<CalendarDatePicker>().is_some() => {
            Some("android.widget.DatePicker")
        }
        AutomationControlType::Custom if owner.cast::<TimePicker>().is_some() => Some("android.widget.TimePicker"),
        _ => None,
    };

    specific.or_else(|| role(control_type)).map_or_else(|| peer.get_class_name(), str::to_owned)
}
