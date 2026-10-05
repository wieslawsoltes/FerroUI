use crate::{Panel, RadioButton};

#[test]
fn indeterminate_radio_button_is_not_unchecked_after_checking_other_radio_button() {
    for is_three_state in [false, true] {
        let panel = Panel::new();

        let radio_button1 = RadioButton::new();
        radio_button1.set_is_three_state(false);
        radio_button1.set_is_checked(Some(false));

        let radio_button2 = RadioButton::new();
        radio_button2.set_is_three_state(is_three_state);
        radio_button2.set_is_checked(None);

        panel.children().add(&radio_button1);
        panel.children().add(&radio_button2);

        assert_eq!(radio_button2.is_checked(), None);

        radio_button1.set_is_checked(Some(true));

        assert_eq!(radio_button1.is_checked(), Some(true));
        assert_eq!(radio_button2.is_checked(), None);
    }
}

#[test]
fn radio_button_in_same_group_is_unchecked() {
    let parent = Panel::new();

    let panel1 = Panel::new();
    let panel2 = Panel::new();

    parent.children().add(&panel1);
    parent.children().add(&panel2);

    let radio_button1 = RadioButton::new();
    radio_button1.set_group_name(Some("A".to_string()));
    radio_button1.set_is_checked(Some(false));

    let radio_button2 = RadioButton::new();
    radio_button2.set_group_name(Some("A".to_string()));
    radio_button2.set_is_checked(Some(true));

    let radio_button3 = RadioButton::new();
    radio_button3.set_group_name(Some("A".to_string()));
    radio_button3.set_is_checked(Some(false));

    panel1.children().add(&radio_button1);
    panel1.children().add(&radio_button2);
    panel2.children().add(&radio_button3);

    assert_eq!(radio_button1.is_checked(), Some(false));
    assert_eq!(radio_button2.is_checked(), Some(true));
    assert_eq!(radio_button3.is_checked(), Some(false));

    radio_button3.set_is_checked(Some(true));

    assert_eq!(radio_button1.is_checked(), Some(false));
    assert_eq!(radio_button2.is_checked(), Some(false));
    assert_eq!(radio_button3.is_checked(), Some(true));
}

#[test]
fn radio_button_empty_group_name_not_influence_other_groups() {
    let parent = Panel::new();

    let radio_button1 = RadioButton::new();
    radio_button1.set_group_name(Some("A".to_string()));
    radio_button1.set_is_checked(Some(true));
    let radio_button2 = RadioButton::new();
    radio_button2.set_group_name(Some("A".to_string()));
    radio_button2.set_is_checked(Some(false));

    let radio_button3 = RadioButton::new();
    radio_button3.set_group_name(None);
    radio_button3.set_is_checked(Some(false));
    let radio_button4 = RadioButton::new();
    radio_button4.set_group_name(None);
    radio_button4.set_is_checked(Some(true));

    parent.children().add(&radio_button1);
    parent.children().add(&radio_button2);
    parent.children().add(&radio_button3);
    parent.children().add(&radio_button4);

    assert_eq!(radio_button1.is_checked(), Some(true));
    assert_eq!(radio_button2.is_checked(), Some(false));
    assert_eq!(radio_button3.is_checked(), Some(false));
    assert_eq!(radio_button4.is_checked(), Some(true));

    radio_button3.set_is_checked(Some(true));

    assert_eq!(radio_button1.is_checked(), Some(true));
    assert_eq!(radio_button2.is_checked(), Some(false));
    assert_eq!(radio_button3.is_checked(), Some(true));
    assert_eq!(radio_button4.is_checked(), Some(false));
}
