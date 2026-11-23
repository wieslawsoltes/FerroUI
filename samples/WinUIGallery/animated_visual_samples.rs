//! The handlers of the AnimatedVisualPlayer and AnimatedIcon pages.

use ferroui_animated_visuals::{
    AnimatedAcceptVisualSource, AnimatedBackVisualSource, AnimatedChevronDownSmallVisualSource,
    AnimatedChevronRightDownSmallVisualSource, AnimatedChevronUpDownSmallVisualSource, AnimatedFindVisualSource,
    AnimatedGlobalNavigationButtonVisualSource, AnimatedIcon, AnimatedSettingsVisualSource, AnimatedVisualPlayer,
    AnimatedVisualSource,
};
use ferroui_base::input::InputElement;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::{Button, ComboBox, Control};

pub(crate) fn wire(root: &Ref<Control>) {
    wire_player(root);
    wire_icon(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name).and_then(|control| control.cast())
}

fn wire_player(root: &Ref<Control>) {
    let Some(player) = find::<AnimatedVisualPlayer>(root, "Player") else {
        return;
    };
    let actions: [(&str, fn(&AnimatedVisualPlayer)); 4] = [
        ("PlayButton", |player| drop(player.play_async(0.0, 1.0, false))),
        ("PauseButton", AnimatedVisualPlayer::pause),
        ("ResumeButton", AnimatedVisualPlayer::resume),
        ("StopButton", AnimatedVisualPlayer::stop),
    ];
    for (name, action) in actions {
        if let Some(button) = find::<Button>(root, name) {
            let player = player.downgrade();
            button.click(move |_, _| {
                if let Some(player) = player.upgrade() {
                    action(&player);
                }
            });
        }
    }
}

/// The source at `index` of the combo box of the page.
fn source_at(index: i32) -> Option<Ref<AnimatedVisualSource>> {
    Some(match index {
        0 => AnimatedAcceptVisualSource::new().upcast(),
        1 => AnimatedBackVisualSource::new().upcast(),
        2 => AnimatedChevronDownSmallVisualSource::new().upcast(),
        3 => AnimatedChevronRightDownSmallVisualSource::new().upcast(),
        4 => AnimatedChevronUpDownSmallVisualSource::new().upcast(),
        5 => AnimatedFindVisualSource::new().upcast(),
        6 => AnimatedGlobalNavigationButtonVisualSource::new().upcast(),
        7 => AnimatedSettingsVisualSource::new().upcast(),
        _ => return None,
    })
}

/// `Button_PointerEntered` and `Button_PointerExited` of the source page.
fn wire_icon(root: &Ref<Control>) {
    let (Some(button), Some(icon)) =
        (find::<Button>(root, "AnimatedIconButton"), find::<AnimatedIcon>(root, "SearchAnimatedIcon"))
    else {
        return;
    };
    for (event, state) in
        [(InputElement::pointer_entered_event(), "PointerOver"), (InputElement::pointer_exited_event(), "Normal")]
    {
        let icon = icon.downgrade();
        button.add_handler(event, move |_, _| {
            if let Some(icon) = icon.upgrade() {
                AnimatedIcon::set_state(&icon, Some(state.to_owned()));
            }
        });
    }
    if let Some(kind) = find::<ComboBox>(root, "AnimatedVisualSourceKind") {
        let icon = icon.downgrade();
        let combo = kind.downgrade();
        kind.selection_changed(move |_, _| {
            if let (Some(icon), Some(combo)) = (icon.upgrade(), combo.upgrade()) {
                if let Some(source) = source_at(combo.selected_index()) {
                    icon.set_source(Some(source));
                }
            }
        });
    }
}
