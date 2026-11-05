use crate::interop::navigation_helper;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{ILauncher, IStorageItem};
use std::rc::Rc;
use url::Url;

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
    fn launch_uri_async(&self, uri: &Url) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(self.page.window_open(uri.as_str(), "_blank")))
    }

    fn launch_file_async(&self, _storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let uri = Url::parse("https://example.com/a?b=c").unwrap();
        assert!(run(launcher.launch_uri_async(&uri)));
        assert_eq!(vec![(uri.as_str().to_string(), "_blank".to_string())], *opened.borrow());
    }

    #[test]
    fn the_answer_is_whether_the_page_opened_a_context() {
        let (launcher, _) = launcher(false);
        assert!(!run(launcher.launch_uri_async(&Url::parse("https://example.com").unwrap())));
    }

    #[test]
    fn the_text_of_the_url_is_opened() {
        // The URL standard normalises the text the page gets (`https://example.com` has its
        // root path); a relative reference is no `Url`, so it cannot be launched.
        let (launcher, opened) = launcher(true);
        assert!(run(launcher.launch_uri_async(&Url::parse("HTTPS://Example.com").unwrap())));
        assert_eq!(vec![("https://example.com/".to_string(), "_blank".to_string())], *opened.borrow());
    }
}
