//! The letter spacing tests of the label tests of the reference, which were
//! deferred until the text properties of the templated control existed.

use crate::Label;

#[test]
fn label_letter_spacing_default_value_is_zero() {
    let label = Label::new();
    assert_eq!(0.0, label.letter_spacing());
}

#[test]
fn label_letter_spacing_can_be_set_and_retrieved() {
    let label = Label::new();
    label.set_letter_spacing(2.5);
    assert_eq!(2.5, label.letter_spacing());
}

#[test]
fn label_letter_spacing_inherits_from_templated_control() {
    let label = Label::new();
    label.set_letter_spacing(3.0);
    // The letter spacing is inherited from the templated control.
    assert_eq!(3.0, label.letter_spacing());
}

#[test]
fn label_letter_spacing_can_be_negative() {
    let label = Label::new();
    label.set_letter_spacing(-1.5);
    assert_eq!(-1.5, label.letter_spacing());
}
