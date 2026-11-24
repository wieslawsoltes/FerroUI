//! The code behind of the SwipeControl page of the gallery (`SwipeControlPage.xaml.cs`), in Rust:
//! the handlers of the invoked items.

use ferroui_base::Ref;
use ferroui_controls::{Control, SwipeControl, SwipeItem, SwipeItems, TextBlock};

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id != "SwipeControl" {
        return;
    }
    // Accept_ItemInvoked and Flag_ItemInvoked.
    for (index, name) in ["Accept", "Flag"].into_iter().enumerate() {
        on_invoked(root, "RevealSwipe", false, index, "RevealOutput", name);
    }
    // DeleteOne_ItemInvoked.
    on_invoked(root, "ExecuteSwipe", true, 0, "ExecuteOutput", "Archive");
}

/// The items of a side of a swipe control of the page.
pub(crate) fn items(root: &Ref<Control>, name: &str, right: bool) -> Option<Ref<SwipeItems>> {
    let control = root.get_control::<SwipeControl>(name);
    if right { control.right_items() } else { control.left_items() }
}

/// An item of a side of a swipe control of the page.
pub(crate) fn item(root: &Ref<Control>, name: &str, right: bool, index: usize) -> Option<Ref<SwipeItem>> {
    let items = items(root, name, right)?;
    let list = items.items();
    (index < list.count()).then(|| list.get(index))
}

/// The handler of an item: the text below the example says what was invoked.
fn on_invoked(root: &Ref<Control>, control: &str, right: bool, index: usize, output: &str, name: &'static str) {
    let Some(item) = item(root, control, right, index) else { return };
    let output = root.get_control::<TextBlock>(output).downgrade();
    item.invoked(move |_, _| {
        if let Some(output) = output.upgrade() {
            output.set_text(Some(&format!("{name} was invoked.")));
        }
    })
    .detach();
}
