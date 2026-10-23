//! The task in a window of its own.

fn main() {
    seven_guis_model::run("macros-temperature", "Temperature Converter", 480.0, 110.0, seven_guis_temperature_macros::TemperatureConverter);
}
