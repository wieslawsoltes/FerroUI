//! Port of `Pages/TabbedPage/FluidNavBar/FluidNavItem.cs`.

use std::rc::Rc;

/// An item of a `FluidNavBar`: the path data of its icon and its label.
pub struct FluidNavItem {
    svg_path: String,
    label: String,
}

impl PartialEq for FluidNavItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FluidNavItem {
    pub fn new(svg_path: &str, label: &str) -> Rc<FluidNavItem> {
        Rc::new(Self { svg_path: svg_path.to_string(), label: label.to_string() })
    }

    pub fn svg_path(&self) -> String {
        self.svg_path.clone()
    }

    pub fn label(&self) -> String {
        self.label.clone()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn an_item_keeps_its_path_and_label() {
        let item = FluidNavItem::new("M0,0 L1,1", "Home");
        assert_eq!("M0,0 L1,1", item.svg_path());
        assert_eq!("Home", item.label());
    }
}
