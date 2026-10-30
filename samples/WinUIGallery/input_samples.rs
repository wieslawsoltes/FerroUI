//! Original CheckBoxPage and PasswordBoxPage handlers adapted to Rust.
//! PasswordBox is still an explicit TextBox composition, as documented per page.

use ferroui_base::{ObjectType, Ref};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{CheckBox, Control, TextBlock, TextBox};
use std::cell::Cell;
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) {
    wire_password(root);
    wire_checkboxes(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

fn wire_password(root: &Ref<Control>) {
    let Some(password) = find::<TextBox>(root, "SimplePassword") else {
        return;
    };
    let weak_password = password.downgrade();
    let weak_output = root
        .get_control::<TextBlock>("PasswordValidation")
        .downgrade();
    password.property_changed(move |change| {
        if change.property() != TextBox::text_property().as_property() {
            return;
        }
        if let (Some(password), Some(output)) = (weak_password.upgrade(), weak_output.upgrade()) {
            let text = password.text().unwrap_or_default();
            if text.is_empty() || text == "Password" {
                output.set_text(Some("'Password' is not allowed."));
                output.set_is_visible(true);
                // A nonempty invalid value raises one further Text change when
                // cleared. Re-clearing the empty string would recurse forever.
                if !text.is_empty() {
                    password.set_text(Some(""));
                }
            } else {
                output.set_text(Some(""));
                output.set_is_visible(false);
            }
        }
    });
    let reveal = root.get_control::<CheckBox>("revealModeCheckBox");
    let weak_reveal = reveal.downgrade();
    let weak_password = root
        .get_control::<TextBox>("passworBoxWithRevealmode")
        .downgrade();
    reveal.property_changed(move |change| {
        if change.property() == ToggleButton::is_checked_property().as_property() {
            if let (Some(reveal), Some(password)) = (weak_reveal.upgrade(), weak_password.upgrade())
            {
                password.set_reveal_password(reveal.is_checked() == Some(true));
            }
        }
    });
}

fn wire_checkboxes(root: &Ref<Control>) {
    let Some(all) = find::<CheckBox>(root, "OptionsAllCheckBox") else {
        return;
    };
    for (name, output_name, three_state) in [
        ("TwoState", "TwoStateOutput", false),
        ("ThreeState", "ThreeStateOutput", true),
    ] {
        let checkbox = root.get_control::<CheckBox>(name);
        let weak_checkbox = checkbox.downgrade();
        let weak_output = root.get_control::<TextBlock>(output_name).downgrade();
        checkbox.property_changed(move |change| {
            if change.property() == ToggleButton::is_checked_property().as_property() {
                if let (Some(checkbox), Some(output)) =
                    (weak_checkbox.upgrade(), weak_output.upgrade())
                {
                    output.set_text(Some(match (three_state, checkbox.is_checked()) {
                        (true, Some(true)) => "CheckBox is checked.",
                        (true, Some(false)) => "CheckBox is unchecked.",
                        (true, None) => "CheckBox state is indeterminate.",
                        (false, Some(true)) => "You checked the box.",
                        (false, _) => "You unchecked the box.",
                    }));
                }
            }
        });
    }
    let options = ["Option1CheckBox", "Option2CheckBox", "Option3CheckBox"]
        .map(|name| root.get_control::<CheckBox>(name));
    // The source calls SetCheckedState at Loaded. All three children already
    // exist here, so initialize the aggregate after constructing the document.
    all.set_is_checked(aggregate(&options));
    let updating = Rc::new(Cell::new(false));
    for option in &options {
        let weak_all = all.downgrade();
        let weak_options = options.each_ref().map(|option| option.downgrade());
        let updating = updating.clone();
        option.property_changed(move |change| {
            if change.property() != ToggleButton::is_checked_property().as_property()
                || updating.get()
            {
                return;
            }
            if let (Some(all), Some(options)) = (weak_all.upgrade(), upgrade_all(&weak_options)) {
                updating.set(true);
                all.set_is_checked(aggregate(&options));
                updating.set(false);
            }
        });
    }
    let weak_all = all.downgrade();
    let weak_options = options.each_ref().map(|option| option.downgrade());
    all.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() || updating.get()
        {
            return;
        }
        if let (Some(all), Some(options)) = (weak_all.upgrade(), upgrade_all(&weak_options)) {
            let value = match all.is_checked() {
                Some(value) => Some(value),
                None if aggregate(&options) == Some(true) => Some(false),
                None => None,
            };
            if let Some(value) = value {
                updating.set(true);
                all.set_is_checked(Some(value));
                for option in options {
                    option.set_is_checked(Some(value));
                }
                updating.set(false);
            }
        }
    });
}

fn aggregate(options: &[Ref<CheckBox>; 3]) -> Option<bool> {
    if options
        .iter()
        .all(|option| option.is_checked() == Some(true))
    {
        Some(true)
    } else if options
        .iter()
        .all(|option| option.is_checked() == Some(false))
    {
        Some(false)
    } else {
        None
    }
}

fn upgrade_all(options: &[ferroui_base::WeakRef<CheckBox>; 3]) -> Option<[Ref<CheckBox>; 3]> {
    Some([
        options[0].upgrade()?,
        options[1].upgrade()?,
        options[2].upgrade()?,
    ])
}
