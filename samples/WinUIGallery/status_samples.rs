//! The code behind of the pinned ProgressBar, Expander and PipsPager pages in Rust: the handlers
//! of `ProgressBarPage.xaml.cs`, `ExpanderPage.xaml.cs` and `PipsPagerPage.xaml.cs`, and the image
//! of the ToolTip page (which has no code-behind upstream). The handlers of the InfoBar page are in
//! `control_samples.rs`. The controls own their states; the sample owns
//! only its options.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::Stretch;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    Carousel, ComboBox, Control, ExpandDirection, Expander, Image, NumberBox, PipsPager,
    PipsPagerButtonVisibility, PipsPagerWrapMode, ProgressBar, TextBlock,
};

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "ProgressBar" => wire_progress_bar(root),
        "Expander" => wire_expander(root),
        "PipsPager" => wire_pips_pager(root),
        "ToolTip" => wire_tool_tip(root),
        _ => {}
    }
}

/// Calls `handler` with the selected index of `combo_box` whenever its selection changes
/// (the source's `SelectionChanged` handlers, which read the added item).
fn on_selected_index<T: ObjectType>(
    combo_box: &Ref<ComboBox>,
    target: &Ref<T>,
    handler: impl Fn(&Ref<T>, i32) + 'static,
) {
    let weak_combo_box = combo_box.downgrade();
    let weak_target = target.downgrade();
    combo_box.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let (Some(combo_box), Some(target)) = (weak_combo_box.upgrade(), weak_target.upgrade()) {
            handler(&target, combo_box.selected_index());
        }
    });
}

/// `ProgressValue_ValueChanged` of `ProgressBarPage.xaml.cs`: the number box sets the value of the
/// determinate bar; NaN (an empty box) is not a valid value, so the box goes back to 0.
fn wire_progress_bar(root: &Ref<Control>) {
    let number_box = root.get_control::<NumberBox>("ProgressValue");
    let weak_number_box = number_box.downgrade();
    let weak_bar = root.get_control::<ProgressBar>("ProgressBar2").downgrade();
    number_box
        .value_changed(move |_| {
            let (Some(number_box), Some(bar)) = (weak_number_box.upgrade(), weak_bar.upgrade())
            else {
                return;
            };
            let value = number_box.value();
            if value.is_nan() {
                number_box.set_numeric_value(0.0);
            } else {
                bar.set_range_value(value);
            }
        })
        .detach();
}

/// `ExpandDirectionComboBox_SelectionChanged` of `ExpanderPage.xaml.cs`: Down expands down from
/// the top, Up expands up from the bottom.
fn wire_expander(root: &Ref<Control>) {
    let expander = root.get_control::<Expander>("Expander1");
    on_selected_index(
        &root.get_control::<ComboBox>("ExpandDirectionComboBox"),
        &expander,
        |expander, index| {
            if index == 1 {
                expander.set_expand_direction(ExpandDirection::Up);
                expander.set_vertical_alignment(VerticalAlignment::Bottom);
            } else {
                expander.set_expand_direction(ExpandDirection::Down);
                expander.set_vertical_alignment(VerticalAlignment::Top);
            }
        },
    );
}

/// The source of the ToolTip page's image: `/Assets/SampleMedia/cliff.jpg`.
pub(crate) const CLIFF: &str = "ferres://WinUIGallery/Assets/SampleMedia/cliff.jpg";

/// The ToolTip page: the source's `Source` of the image whose tooltip is placed beside it (the
/// markup has no bitmap converter for the gallery's assets, so the sample loads it).
fn wire_tool_tip(root: &Ref<Control>) {
    let image = root.get_control::<Image>("textBoxToPlace");
    image.set_stretch(Stretch::Uniform);
    if let Ok(bitmap) = crate::assets::load_bitmap(CLIFF) {
        image.set_source(Some(std::rc::Rc::new(bitmap)));
    }
}

/// The source's `Pictures`, the items of the gallery.
pub(crate) const PICTURES: [&str; 8] = [
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage1.jpg",
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage2.jpg",
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage3.jpg",
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage4.jpg",
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage5.jpg",
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage6.jpg",
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage7.jpg",
    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage8.jpg",
];

/// The announcement of `TestPipsPager2_SelectedIndexChanged`.
pub(crate) fn page_announcement(pager: &PipsPager) -> String {
    format!(
        "Page {} of {} selected",
        pager.selected_page_index() + 1,
        pager.number_of_pages()
    )
}

/// The code behind of `PipsPagerPage.xaml.cs`: the gallery shows the source's `Pictures` (the
/// item template's image of each picture), and the pager has `Pictures.Count` pages, before the
/// x:Bind of the selection is initialized. `TestPipsPager2_SelectedIndexChanged` announces the page
/// (`UIHelper.AnnounceActionForAccessibility`, a polite live region here), and the options set the
/// orientation and the visibility of the previous and next buttons.
fn wire_pips_pager(root: &Ref<Control>) {
    let gallery = root.get_control::<Carousel>("Gallery");
    let mut errors = Vec::new();
    for picture in PICTURES {
        let image = Image::new();
        image.set_stretch(Stretch::Uniform);
        match crate::assets::load_bitmap(picture) {
            Ok(bitmap) => image.set_source(Some(std::rc::Rc::new(bitmap))),
            Err(error) => errors.push(format!("{picture}: {error}")),
        }
        gallery.items().add(Some(Control::boxed(image)));
    }
    let output = root.get_control::<TextBlock>("GalleryLoadError");
    output.set_text(Some(&errors.join("\n")));
    output.set_is_visible(!errors.is_empty());
    // The source's AutomationProperties.AutomationControlType of the FlipView.
    AutomationProperties::set_control_type_override(&gallery, Some(AutomationControlType::List));
    root.get_control::<PipsPager>("FlipViewPipsPager")
        .set_number_of_pages(PICTURES.len() as i32);

    let pager = root.get_control::<PipsPager>("TestPipsPager2");
    let weak_announcement = root
        .get_control::<TextBlock>("PipsPagerAnnouncement")
        .downgrade();
    let weak_pager = pager.downgrade();
    pager.selected_index_changed(move |_, _| {
        if let (Some(pager), Some(announcement)) =
            (weak_pager.upgrade(), weak_announcement.upgrade())
        {
            announcement.set_text(Some(&page_announcement(&pager)));
        }
    });
    on_selected_index(
        &root.get_control::<ComboBox>("OrientationComboBox"),
        &pager,
        |pager, index| {
            pager.set_orientation(if index == 1 {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            });
        },
    );
    on_selected_index(
        &root.get_control::<ComboBox>("PrevButtonComboBox"),
        &pager,
        |pager, index| pager.set_previous_button_visibility(pips_button_visibility(index)),
    );
    on_selected_index(
        &root.get_control::<ComboBox>("NextButtonComboBox"),
        &pager,
        |pager, index| pager.set_next_button_visibility(pips_button_visibility(index)),
    );
    on_selected_index(
        &root.get_control::<ComboBox>("WrapModeComboBox"),
        &pager,
        |pager, index| {
            pager.set_wrap_mode(if index == 1 {
                PipsPagerWrapMode::Wrap
            } else {
                PipsPagerWrapMode::None
            });
        },
    );
}

/// The item of `PrevButtonComboBox` and `NextButtonComboBox` as a button visibility.
fn pips_button_visibility(index: i32) -> PipsPagerButtonVisibility {
    match index {
        1 => PipsPagerButtonVisibility::VisibleOnPointerOver,
        2 => PipsPagerButtonVisibility::Collapsed,
        _ => PipsPagerButtonVisibility::Visible,
    }
}
