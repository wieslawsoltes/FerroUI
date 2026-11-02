//! Export the same Rust contracts an application installs in its runtime.
use ferroui_qml::{Error, Parameter, TypeDefinition, TypeRegistry, Value};
use std::path::PathBuf;

fn parameter(name: &str, ty: &str) -> Parameter {
    Parameter {
        name: name.into(),
        ty: Some(ty.into()),
    }
}

fn registry() -> Result<TypeRegistry, Error> {
    let mut registry = TypeRegistry::new();
    let device = TypeDefinition::default()
        .inherits("QtQml", "QtObject")
        .property("level", "int", Value::Number(0.))
        .readonly_property("model", "QtQml.Models.ListModel", Value::Null)
        .readonly_property("serial", "string", Value::String("demo-1".into()))
        .property_group("settings")
        .property("settings.gain", "real", Value::Number(1.))
        .property("children", "list<QtObject>", Value::List(vec![]))
        .default_property("children")
        .enumeration("Mode", &[("Low", 2), ("High", 7)])?
        .signal("reported", vec![parameter("message", "string")])
        .method("sample", vec![parameter("factor", "int")], "int", |args| {
            Ok(args[0].clone())
        });
    registry.register("Host.Devices", "Device", device)?;
    registry.register(
        "Host.Devices",
        "Sensor",
        TypeDefinition::default()
            .inherits("Host.Devices", "Device")
            .property("settings.scale", "real", Value::Number(1.))
            .property("temperature", "real", Value::Number(0.)),
    )?;
    registry.set_module_version("Host.Devices", 1, 4);
    registry.set_module_version("Host.Devices", 2, 2);
    registry.register(
        "Host.Plant",
        "Station",
        TypeDefinition::default()
            .inherits("QtQml", "QtObject")
            .property("device", "Host.Devices.Device", Value::Null),
    )?;
    registry.reexport("Host.Shared", "Host.Devices")?;
    registry.set_module_version("Host.Shared", 2, 2);
    Ok(registry)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: export_qmltypes IMPORT_PATH")?,
    );
    let registry = registry()?;
    for uri in ["Host.Devices", "Host.Plant", "Host.Shared"] {
        let directory = output.join(uri.replace('.', "/"));
        let metadata = registry.tooling_metadata(uri)?;
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("plugins.qmltypes"), metadata.qmltypes)?;
        std::fs::write(directory.join("qmldir"), metadata.qmldir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_contract_includes_versions_enum_values_and_cross_module_types() {
        let registry = registry().unwrap();
        let first = registry.tooling_metadata("Host.Devices").unwrap();
        assert_eq!(
            first.qmltypes,
            registry.tooling_metadata("Host.Devices").unwrap().qmltypes
        );
        assert!(first.qmltypes.contains("Host.Devices/Device 2.0"));
        assert!(first.qmltypes.contains("\"High\": 7"));
        assert!(first.qmltypes.contains("defaultProperty: \"children\""));
        let client = registry.tooling_metadata("Host.Plant").unwrap();
        assert!(client.qmldir.contains("depends Host.Devices 2.0"));
        assert!(client.qmltypes.contains("Host.Plant/Station 0.0"));
        assert!(registry.tooling_metadata("Missing").is_err());
    }
}
