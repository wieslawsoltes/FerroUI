//! Port of `Cannot_Assign_Control_To_Value` of `Styling/SetterTests.cs` (base unit tests): the value is a
//! control, so the test lives with the controls. The other tests of the file are in `styling/style_tests.rs` of
//! `ferroui-base`.
//!
//! Upstream assigns the control to `Setter.Value`, whose setter initializes a value that is an `ISetterValue`;
//! the member of the port that does both is `Setter::set_setter_value`.

use crate::{Border, Control};
use ferroui_base::styling::{ISetterValue, Setter};
use ferroui_base::BoxedValue;
use std::rc::Rc;

#[test]
#[should_panic(expected = "Cannot use a control as a Setter value. Wrap the control in a <Template>.")]
fn cannot_assign_control_to_value() {
    let target = Setter::empty();

    let border = Border::new();
    let value: BoxedValue = Rc::new(Some(border.clone()));
    let control: &Control = &border;
    target.set_setter_value(control as &dyn ISetterValue, value);
}
