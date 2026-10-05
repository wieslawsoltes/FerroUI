/// The arguments of the notification that the application was asked to
/// open URLs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlOpenedEventArgs {
    urls: Vec<String>,
}

impl UrlOpenedEventArgs {
    /// Creates the arguments with the URLs to open.
    pub fn new(urls: Vec<String>) -> Self {
        Self { urls }
    }

    /// The URLs to open.
    pub fn urls(&self) -> &[String] {
        &self.urls
    }
}
