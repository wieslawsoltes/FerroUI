use crate::interop::navigation_helper;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{ILauncher, IStorageItem};
use ferroui_base::utilities::Uri;
use std::rc::Rc;

/// The call the launcher makes into the page; replaced by a recorder in the
/// tests.
trait ILauncherPage {
    fn window_open(&self, uri: &str, target: &str) -> bool;
}

struct LauncherPage;

impl ILauncherPage for LauncherPage {
    fn window_open(&self, uri: &str, target: &str) -> bool {
        navigation_helper::window_open(uri, target)
    }
}

/// The launcher of the page: opens URIs in a new browsing context. Files
/// cannot be launched.
pub struct BrowserLauncher {
    page: Box<dyn ILauncherPage>,
}

impl Default for BrowserLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserLauncher {
    /// Creates the launcher.
    pub fn new() -> Self {
        Self { page: Box::new(LauncherPage) }
    }
}

impl ILauncher for BrowserLauncher {
    fn launch_uri_async(&self, uri: &Uri) -> LocalBoxFuture<bool> {
        if uri.is_absolute_uri() {
            return Box::pin(std::future::ready(self.page.window_open(uri.absolute_uri(), "_blank")));
        }
        Box::pin(std::future::ready(false))
    }

    fn launch_file_async(&self, _storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::utilities::UriKind;
    use std::cell::RefCell;
    use std::task::{Context, Poll, Waker};

    struct RecordingPage {
        opened: Rc<RefCell<Vec<(String, String)>>>,
        answer: bool,
    }

    impl ILauncherPage for RecordingPage {
        fn window_open(&self, uri: &str, target: &str) -> bool {
            self.opened.borrow_mut().push((uri.to_string(), target.to_string()));
            self.answer
        }
    }

    fn run(future: LocalBoxFuture<bool>) -> bool {
        let mut future = future;
        match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the launcher waits"),
        }
    }

    fn launcher(answer: bool) -> (BrowserLauncher, Rc<RefCell<Vec<(String, String)>>>) {
        let opened = Rc::new(RefCell::new(Vec::new()));
        (BrowserLauncher { page: Box::new(RecordingPage { opened: opened.clone(), answer }) }, opened)
    }

    #[test]
    fn an_absolute_uri_is_opened_in_a_new_browsing_context() {
        let (launcher, opened) = launcher(true);
        let uri = Uri::new("https://example.com/a?b=c", UriKind::Absolute).unwrap();
        assert!(run(launcher.launch_uri_async(&uri)));
        assert_eq!(vec![(uri.absolute_uri().to_string(), "_blank".to_string())], *opened.borrow());
    }

    #[test]
    fn the_answer_is_whether_the_page_opened_a_context() {
        let (launcher, _) = launcher(false);
        assert!(!run(launcher.launch_uri_async(&Uri::new("https://example.com", UriKind::Absolute).unwrap())));
    }

    #[test]
    fn a_relative_uri_is_not_opened() {
        let (launcher, opened) = launcher(true);
        let uri = Uri::new("docs/index.html", UriKind::RelativeOrAbsolute).unwrap();
        assert!(!run(launcher.launch_uri_async(&uri)));
        assert!(opened.borrow().is_empty());
    }
}
