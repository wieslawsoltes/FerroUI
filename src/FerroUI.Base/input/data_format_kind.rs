/// Represents the kind of a [`DataFormat`](super::DataFormat).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DataFormatKind {
    /// The data format is specific to the application. The exact format
    /// name used internally will vary depending on the platform.
    ///
    /// Such a format is created using
    /// [`DataFormat::create_bytes_application_format`](super::DataFormat::create_bytes_application_format)
    /// or [`DataFormat::create_string_application_format`](super::DataFormat::create_string_application_format).
    Application = 0,

    /// The data format is specific to the current platform. Any other
    /// application using the same identifier will be able to access it.
    ///
    /// Such a format is created using
    /// [`DataFormat::create_bytes_platform_format`](super::DataFormat::create_bytes_platform_format)
    /// or [`DataFormat::create_string_platform_format`](super::DataFormat::create_string_platform_format).
    Platform = 1,

    /// The data format is cross-platform and supported directly by the
    /// framework. Such formats include [`DataFormat::text`](super::DataFormat::text)
    /// and [`DataFormat::bitmap`](super::DataFormat::bitmap).
    ///
    /// It is not possible to create such a format directly.
    Universal = 2,

    /// The data format is only usable within the current process. It never
    /// crosses process or serialization boundaries.
    ///
    /// Such a format is created using
    /// [`DataFormat::create_in_process_format`](super::DataFormat::create_in_process_format).
    InProcess = 3,
}

impl DataFormatKind {
    /// The name of the kind.
    pub fn name(self) -> &'static str {
        match self {
            DataFormatKind::Application => "Application",
            DataFormatKind::Platform => "Platform",
            DataFormatKind::Universal => "Universal",
            DataFormatKind::InProcess => "InProcess",
        }
    }
}

impl std::fmt::Display for DataFormatKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
