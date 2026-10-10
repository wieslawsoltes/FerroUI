//! The resource database of the server (the port of `XResources.cs`): the
//! `RESOURCE_MANAGER` property of the root window, which `xrdb` writes.

use crate::event::Event;
use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::xlib;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// The resources of the server, with a notification per resource that
/// changed.
pub struct XResources {
    resources: RefCell<HashMap<String, String>>,
    x11: Rc<X11Info>,
    pub resource_changed: Event<String>,
}

/// The resources of a resource string (`name: value` lines).
pub fn parse_resources(res: &str) -> HashMap<String, String> {
    let mut new_resources = HashMap::new();
    for item in res.split('\n') {
        let Some((key, value)) = item.split_once(':') else {
            continue;
        };
        new_resources.insert(key.to_string(), value.trim_start().to_string());
    }
    new_resources
}

/// The keys to notify when `old` is replaced by `new`: first the ones
/// that are gone, then the ones that are new or have another value. (The
/// reference enumerates two hash sets; the order within each group is not
/// defined there, and is sorted here.)
pub fn changed_resources(old: &HashMap<String, String>, new: &HashMap<String, String>) -> Vec<String> {
    let mut missing_resources: HashSet<&String> = old.keys().collect();
    let mut changed_resources = Vec::new();
    for (key, value) in new {
        if !missing_resources.remove(key) || old.get(key) != Some(value) {
            changed_resources.push(key.clone());
        }
    }
    let mut missing: Vec<String> = missing_resources.into_iter().cloned().collect();
    missing.sort();
    changed_resources.sort();
    missing.extend(changed_resources);
    missing
}

impl XResources {
    pub fn new(plat: &Rc<FerroX11Platform>) -> Rc<Self> {
        let this = Rc::new(Self {
            resources: RefCell::new(HashMap::new()),
            x11: plat.info().clone(),
            resource_changed: Event::new(),
        });
        let weak = Rc::downgrade(&this);
        plat.globals().root_property_changed.subscribe(move |atom| {
            if let Some(this) = weak.upgrade() {
                this.on_root_property_changed(atom);
            }
        });
        this.update_resources();
        this
    }

    fn update_resources(&self) {
        let res = self.read_resources_string().unwrap_or_default();
        let new_resources = parse_resources(&res);
        let changed = changed_resources(&self.resources.borrow(), &new_resources);
        *self.resources.borrow_mut() = new_resources;
        for key in changed {
            self.resource_changed.invoke(key);
        }
    }

    pub fn get_resource(&self, key: &str) -> Option<String> {
        self.resources.borrow().get(key).cloned()
    }

    fn read_resources_string(&self) -> Option<String> {
        let atoms = self.x11.atoms();
        let property = xlib::x_get_window_property(
            self.x11.display(),
            self.x11.root_window(),
            atoms.RESOURCE_MANAGER,
            0,
            0x7fffffff,
            false,
            atoms.STRING,
        );
        if property.actual_format != 8 {
            return None;
        }
        // The bytes are Latin-1 for the `STRING` type; the reference
        // reads them with the ANSI code page of the process, which is
        // UTF-8 on the systems it runs on.
        Some(String::from_utf8_lossy(&property.data).into_owned())
    }

    fn on_root_property_changed(&self, atom: xlib::Atom) {
        if atom == self.x11.atoms().RESOURCE_MANAGER {
            self.update_resources();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn resources_are_lines_of_name_and_value() {
        let resources = parse_resources("Xft.dpi:\t144\nXcursor.size: 24\n! a comment\n\nXft.rgba:rgb\nURL: http://x:1\n");
        assert_eq!(resources.get("Xft.dpi").map(String::as_str), Some("144"));
        assert_eq!(resources.get("Xcursor.size").map(String::as_str), Some("24"));
        assert_eq!(resources.get("Xft.rgba").map(String::as_str), Some("rgb"));
        // Only the first colon separates.
        assert_eq!(resources.get("URL").map(String::as_str), Some("http://x:1"));
        assert_eq!(resources.len(), 4);
        assert!(parse_resources("").is_empty());
    }

    #[test]
    fn the_last_line_of_a_name_wins() {
        let resources = parse_resources("Xft.dpi: 96\nXft.dpi: 192");
        assert_eq!(resources.get("Xft.dpi").map(String::as_str), Some("192"));
    }

    #[test]
    fn removed_new_and_changed_resources_are_notified() {
        let old = parse_resources("a: 1\nb: 2\nc: 3");
        let new = parse_resources("b: 2\nc: 4\nd: 5");
        assert_eq!(changed_resources(&old, &new), vec!["a".to_string(), "c".to_string(), "d".to_string()]);
        assert!(changed_resources(&new, &new).is_empty());
        assert_eq!(changed_resources(&HashMap::new(), &old), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
}
