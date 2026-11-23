//! Composing controls: a child goes where the parent holds its children, as the builders of the
//! declarative Rust API place them (`ferroui-view`, `mount.rs`): the `Children` of a panel, the
//! `Items` of an items control, the `Content` of a content control (a window included) or of a
//! content presenter, the `Child` of a decorator or of a popup.

use crate::error::{status, Error, Result};
use crate::handles;
use crate::thread;
use crate::{FerroHandle, FerroStatus, FERRO_INVALID_STATE, FERRO_NOT_FOUND, FERRO_NOT_SUPPORTED, FERRO_TYPE_MISMATCH};
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::Popup;
use ferroui_controls::{ContentControl, Control, Decorator, ItemsControl, Panel};

/// Where a parent holds its children.
enum Host {
    Panel(Ref<Panel>),
    Items(Ref<ItemsControl>),
    Content(Ref<ContentControl>),
    Presenter(Ref<ContentPresenter>),
    Decorator(Ref<Decorator>),
    Popup(Ref<Popup>),
}

impl Host {
    fn of(parent: &Ref<FerroObject>) -> Result<Host> {
        // A popup first: it shows its child in a root of its own.
        if let Some(popup) = parent.cast::<Popup>() {
            return Ok(Host::Popup(popup));
        }
        if let Some(panel) = parent.cast::<Panel>() {
            return Ok(Host::Panel(panel));
        }
        if let Some(items) = parent.cast::<ItemsControl>() {
            return Ok(Host::Items(items));
        }
        if let Some(content) = parent.cast::<ContentControl>() {
            return Ok(Host::Content(content));
        }
        if let Some(presenter) = parent.cast::<ContentPresenter>() {
            return Ok(Host::Presenter(presenter));
        }
        if let Some(decorator) = parent.cast::<Decorator>() {
            return Ok(Host::Decorator(decorator));
        }
        Err(Error::new(FERRO_NOT_SUPPORTED, format!("a {} holds no children", parent.get_type().name())))
    }
}

fn objects(parent: FerroHandle, child: FerroHandle) -> Result<(Ref<FerroObject>, Ref<Control>)> {
    thread::check()?;
    let parent = handles::get(parent)?;
    let child = handles::get(child)?;
    let child = child
        .cast::<Control>()
        .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, format!("a {} is not a control", child.get_type().name())))?;
    Ok((parent, child))
}

#[cold]
fn occupied(parent: &Ref<FerroObject>) -> Error {
    Error::new(FERRO_INVALID_STATE, format!("the {} already holds its one child", parent.get_type().name()))
}

/// Adds the control `child` to the children of `parent`: at the end of the `Children` of a panel
/// or of the `Items` of an items control; as the one `Content` of a content control (a window)
/// or of a content presenter, or the `Child` of a decorator or a popup, which must not hold one
/// yet ([`FERRO_INVALID_STATE`]). The parent holds its own reference: the handle of the child can
/// be released afterwards. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_add_child(parent: FerroHandle, child: FerroHandle) -> FerroStatus {
    status(|| {
        let (parent, child) = objects(parent, child)?;
        match Host::of(&parent)? {
            Host::Panel(panel) => panel.children().add(child),
            Host::Items(items) => {
                items.items().add(Some(Control::boxed(child)));
            }
            Host::Content(content) => {
                if content.content().is_some() {
                    return Err(occupied(&parent));
                }
                content.set_content(Some(Control::boxed(child)));
            }
            Host::Presenter(presenter) => {
                if presenter.content().is_some() {
                    return Err(occupied(&parent));
                }
                presenter.set_content(Some(Control::boxed(child)));
            }
            Host::Decorator(decorator) => {
                if decorator.child().is_some() {
                    return Err(occupied(&parent));
                }
                decorator.set_child(child);
            }
            Host::Popup(popup) => {
                if popup.child().is_some() {
                    return Err(occupied(&parent));
                }
                popup.set_child(child);
            }
        }
        Ok(())
    })
}

/// Removes the control `child` from the children of `parent` (see [`ferro_add_child`]);
/// [`FERRO_NOT_FOUND`] if it is not one of them. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_remove_child(parent: FerroHandle, child: FerroHandle) -> FerroStatus {
    status(|| {
        let (parent, child) = objects(parent, child)?;
        let holds = |held: Option<Ref<Control>>| held.is_some_and(|held| held.ptr_eq(&child));
        let removed = match Host::of(&parent)? {
            Host::Panel(panel) => panel.children().remove(child.clone()),
            // Items compare by value, and a control by identity.
            Host::Items(items) => items.items().remove(&Some(Control::boxed(child.clone()))),
            Host::Content(content) => {
                let held = content.content().and_then(|value| Control::from_boxed(&value));
                let removed = holds(held);
                if removed {
                    content.set_content(None);
                }
                removed
            }
            Host::Presenter(presenter) => {
                let held = presenter.content().and_then(|value| Control::from_boxed(&value));
                let removed = holds(held);
                if removed {
                    presenter.set_content(None);
                }
                removed
            }
            Host::Decorator(decorator) => {
                let removed = holds(decorator.child());
                if removed {
                    decorator.set_child(None::<Ref<Control>>);
                }
                removed
            }
            Host::Popup(popup) => {
                let removed = holds(popup.child());
                if removed {
                    popup.set_child(None::<Ref<Control>>);
                }
                removed
            }
        };
        if removed {
            Ok(())
        } else {
            Err(Error::new(FERRO_NOT_FOUND, "the control is not a child of the parent"))
        }
    })
}
