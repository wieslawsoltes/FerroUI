//! Build and mount the same native component in the desktop sample and its
//! headless smoke test. The runtime never parses QML source at startup.
use ferroui_qml_controls::{ControlRegistry, QmlView};

include!(concat!(env!("OUT_DIR"), "/main_qml.rs"));

pub fn create() -> Result<QmlView, ferroui_qml::Error> {
    let mut registry = ControlRegistry::standard()?;
    registry.register_ferro_control("FerroUI.Controls", ferroui_controls::TextBlock::new)?;
    QmlView::new(&compiled_qml(), &registry, Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Button, TextBlock,
    };

    #[test]
    fn generated_component_updates_real_controls() {
        let _scope = UnitTestApplication::start(TestServices::new());
        let view = create().unwrap();
        let button = view.id("increment").unwrap();
        let label = view
            .control(view.id("countText").unwrap())
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert!(view.control(button).unwrap().cast::<Button>().is_some());
        assert_eq!(label.text().as_deref(), Some("Count: 0"));
        view.emit(button, "clicked", vec![]).unwrap();
        assert_eq!(label.text().as_deref(), Some("Count: 1"));
    }
}
