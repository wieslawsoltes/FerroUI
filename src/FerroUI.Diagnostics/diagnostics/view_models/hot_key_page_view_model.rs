//! Port of `Diagnostics/ViewModels/HotKeyPageViewModel.cs`.

use super::ViewModelBase;
use crate::diagnostics::DevToolsOptions;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use std::cell::RefCell;
use std::rc::Rc;

/// A hot key of the developer tools, as the hot keys page lists it: the
/// gesture, what it does and a longer description.
///
/// A record upstream: equal when the members are equal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotKeyDescription {
    gesture: String,
    brief_description: String,
    detailed_description: Option<String>,
}

impl HotKeyDescription {
    /// C# `new HotKeyDescription(string Gesture, string BriefDescription, string? DetailedDescription = null)`.
    pub fn new(
        gesture: impl Into<String>,
        brief_description: impl Into<String>,
        detailed_description: Option<String>,
    ) -> Self {
        Self { gesture: gesture.into(), brief_description: brief_description.into(), detailed_description }
    }

    pub fn gesture(&self) -> String {
        self.gesture.clone()
    }

    pub fn brief_description(&self) -> String {
        self.brief_description.clone()
    }

    pub fn detailed_description(&self) -> Option<String> {
        self.detailed_description.clone()
    }
}

/// The view model of the hot keys page: the hot keys of the options the
/// developer tools were opened with.
pub struct HotKeyPageViewModel {
    base: ViewModelBase,
    hot_key_descriptions: RefCell<Option<FerroList<Rc<HotKeyDescription>>>>,
}

/// Reference equality, as the class of the original.
impl PartialEq for HotKeyPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for HotKeyPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl HotKeyPageViewModel {
    /// C# `new HotKeyPageViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { base: ViewModelBase::new(), hot_key_descriptions: RefCell::new(None) })
    }

    /// The hot keys; `None` until the options are set.
    pub fn hot_key_descriptions(&self) -> Option<FerroList<Rc<HotKeyDescription>>> {
        self.hot_key_descriptions.borrow().clone()
    }

    fn set_hot_key_descriptions(&self, value: Option<FerroList<Rc<HotKeyDescription>>>) {
        self.base.raise_and_set_if_changed(&self.hot_key_descriptions, value, "HotKeyDescriptions");
    }

    /// Lists the hot keys of `options`.
    pub fn set_options(&self, options: &DevToolsOptions) {
        let hot_keys = options.hot_keys();

        let descriptions = [
            HotKeyDescription::new(
                Self::create_description(&options.gesture()),
                "Launch DevTools",
                Some("Launches DevTools to inspect the TopLevel that received the hotkey input".to_string()),
            ),
            HotKeyDescription::new(
                Self::create_description(&hot_keys.value_frames_freeze()),
                "Freeze Value Frames",
                Some("Pauses refreshing the Value Frames inspector for the selected Control".to_string()),
            ),
            HotKeyDescription::new(
                Self::create_description(&hot_keys.value_frames_unfreeze()),
                "Unfreeze Value Frames",
                Some("Resumes refreshing the Value Frames inspector for the selected Control".to_string()),
            ),
            HotKeyDescription::new(
                Self::create_description(&hot_keys.inspect_hovered_control()),
                "Inspect Control Under Pointer",
                Some("Inspects the hovered Control in the Logical or Visual Tree Page".to_string()),
            ),
            HotKeyDescription::new(
                Self::create_description(&hot_keys.toggle_popup_freeze()),
                "Toggle Popup Freeze",
                Some("Prevents visible Popups from closing so they can be inspected".to_string()),
            ),
            HotKeyDescription::new(
                Self::create_description(&hot_keys.screenshot_selected_control()),
                "Screenshot Selected Control",
                Some("Saves a Screenshot of the Selected Control in the Logical or Visual Tree Page".to_string()),
            ),
        ];

        self.set_hot_key_descriptions(Some(FerroList::from_items(descriptions.into_iter().map(Rc::new))));
    }

    /// The text of a gesture; a gesture of modifiers only leaves out the key.
    fn create_description(gesture: &KeyGesture) -> String {
        if gesture.key() == Key::None && gesture.key_modifiers() != KeyModifiers::NONE {
            gesture.to_string().replace("+None", "")
        } else {
            gesture.to_string()
        }
    }
}

// The list of the hot keys, which the hot keys page shows.
ferroui_controls::ferro_markup_list!(pub HotKeyDescriptionList: Rc<HotKeyDescription>);

ferro_markup_type!(class HotKeyDescription {
    this: Rc<HotKeyDescription>,
    handles: [HotKeyDescription, Rc<HotKeyDescription>, Option<Rc<HotKeyDescription>>],
    properties: [
        Gesture: String { get: HotKeyDescription::gesture },
        BriefDescription: String { get: HotKeyDescription::brief_description },
        DetailedDescription: Option<String> { get: HotKeyDescription::detailed_description },
    ],
});

ferro_markup_type!(class HotKeyPageViewModel {
    this: Rc<HotKeyPageViewModel>,
    handles: [HotKeyPageViewModel, Rc<HotKeyPageViewModel>, Option<Rc<HotKeyPageViewModel>>],
    base: ViewModelBase,
    constructors: [() => HotKeyPageViewModel::new],
    properties: [
        HotKeyDescriptions: Option<FerroList<Rc<HotKeyDescription>>> {
            get: HotKeyPageViewModel::hot_key_descriptions
        },
    ],
    notify_property_changed: HotKeyPageViewModel,
});
