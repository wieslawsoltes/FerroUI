//! Port of `ViewModels/WikipediaPageViewModel.cs`.

use crate::models::{HttpClient, HttpRequestException, OnThisDayArticle, OnThisDayJsonSerializerContext, WikipediaHttpClient};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::media::{TextTrimming, TextWrapping};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::GridLength;
use ferroui_controls_treedatagrid::models::{TemplateColumn, TextColumn, TextColumnOptions};
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::ColumnRef;
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, Models};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The Wikipedia page: the articles of the events of today in the "On this day" feed of the
/// Wikimedia API, with their thumbnails, titles and extracts.
pub struct WikipediaPageViewModel {
    base: ViewModelBase,
    data: FerroList<Rc<OnThisDayArticle>>,
    http_client: Arc<HttpClient>,
    load_error: RefCell<Option<String>>,
    source: Rc<FlatTreeDataGridSource<OnThisDayArticle>>,
    loading_task: LoadingTask,
}

impl PartialEq for WikipediaPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for WikipediaPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

impl WikipediaPageViewModel {
    /// A page that loads the content with the shared client of the demo.
    pub fn new() -> Rc<Self> {
        Self::with_http_client(WikipediaHttpClient::shared())
    }

    /// A page that loads the content with `http_client` (the reference's internal constructor,
    /// which its tests call with a client over a stub handler).
    pub fn with_http_client(http_client: Arc<HttpClient>) -> Rc<Self> {
        let data = FerroList::new();

        let wrap = TextColumnOptions::<OnThisDayArticle>::new();
        wrap.set_text_trimming(<dyn TextTrimming>::none());
        wrap.set_text_wrapping(TextWrapping::Wrap);

        let source = FlatTreeDataGridSource::new(Rc::new(data.clone()) as Models<OnThisDayArticle>);
        let columns = source.columns();
        columns.add(TemplateColumn::<OnThisDayArticle>::from_resource_keys(
            header("Image"),
            ResourceKey::from("WikipediaImageCell"),
            None,
            None,
            None,
        ) as ColumnRef<OnThisDayArticle>);
        // `x => x.Titles!.Normalized`, with the setter the reference synthesizes from it.
        columns.add(TextColumn::new(
            header("Title"),
            Accessor::new(|x: &Rc<OnThisDayArticle>| x.titles().and_then(|titles| titles.normalized)).with_setter(
                |x: &Rc<OnThisDayArticle>, value: Option<String>| {
                    let mut titles = x.titles();
                    if let Some(titles) = titles.as_mut() {
                        titles.normalized = value;
                    }
                    x.set_titles(titles);
                },
            ),
            None,
            None,
        ) as ColumnRef<OnThisDayArticle>);
        columns.add(TextColumn::new(
            header("Extract"),
            Accessor::member("Extract", |x: &Rc<OnThisDayArticle>| x.extract())
                .with_setter(|x: &Rc<OnThisDayArticle>, value: Option<String>| x.set_extract(value)),
            Some(GridLength::STAR),
            Some(Rc::new(wrap)),
        ) as ColumnRef<OnThisDayArticle>);

        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            data,
            http_client,
            load_error: RefCell::new(None),
            source,
            loading_task: LoadingTask::new(),
        });
        this.load_content();
        this
    }

    /// `LoadError`: why the content could not be loaded, if it could not.
    pub fn load_error(&self) -> Option<String> {
        self.load_error.borrow().clone()
    }

    fn set_load_error(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.load_error, value, "LoadError");
    }

    pub fn source(&self) -> Rc<FlatTreeDataGridSource<OnThisDayArticle>> {
        self.source.clone()
    }

    /// `LoadingTask`: the load of the content the constructor started.
    pub fn loading_task(&self) -> LoadingTask {
        self.loading_task.clone()
    }

    fn load_content(self: &Rc<Self>) {
        let now = DateTimeOffset::now();
        let d = now.day();
        let m = now.month();
        let uri = format!("https://api.wikimedia.org/feed/v1/wikipedia/en/onthisday/all/{m:02}/{d:02}");

        let this = Rc::downgrade(self);
        let loading_task = self.loading_task.clone();
        self.http_client.get_string_async(uri, move |s| {
            if let Some(this) = this.upgrade() {
                this.content_loaded(s);
            }
            loading_task.complete();
        });
    }

    /// The continuation of the request of the content, on the UI thread.
    fn content_loaded(&self, s: Result<String, HttpRequestException>) {
        let data = s.map_err(|e| e.to_string()).and_then(|s| {
            OnThisDayJsonSerializerContext::deserialize_on_this_day(&s).map_err(|e| e.to_string())
        });

        match data {
            Ok(data) => {
                if let Some(selected) = data.and_then(|data| data.selected) {
                    self.data.add_range(selected.into_iter().flat_map(|x| x.pages.unwrap_or_default()).map(Rc::new));
                }
            }
            Err(message) => self.set_load_error(Some(format!("Unable to load Wikipedia content: {message}"))),
        }
    }
}

/// `Task LoadingTask`: the load of the content of a page, which completes on the UI thread when
/// the response has been read (or the request failed).
#[derive(Clone)]
pub struct LoadingTask {
    dispatcher: Arc<Dispatcher>,
    completed: Rc<Cell<bool>>,
}

impl LoadingTask {
    fn new() -> Self {
        Self { dispatcher: Dispatcher::current_dispatcher(), completed: Rc::new(Cell::new(false)) }
    }

    fn complete(&self) {
        self.completed.set(true);
    }

    /// `IsCompleted`.
    pub fn is_completed(&self) -> bool {
        self.completed.get()
    }

    /// `await LoadingTask` on the UI thread (the tests): runs the jobs of the dispatcher until
    /// the load has completed.
    ///
    /// # Panics
    /// Panics when the load has not completed within a minute.
    pub fn run_to_completion(&self) {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            self.dispatcher.run_jobs(None);
            if self.is_completed() {
                return;
            }
            assert!(Instant::now() < deadline, "the content of the Wikipedia page did not load");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

ferro_markup_type!(class WikipediaPageViewModel {
    this: Rc<WikipediaPageViewModel>,
    handles: [WikipediaPageViewModel, Rc<WikipediaPageViewModel>, Option<Rc<WikipediaPageViewModel>>],
    properties: [
        LoadError: Option<String> { get: |this: &Rc<WikipediaPageViewModel>| this.load_error() },
        Source: TreeDataGridSourceRef {
            get: |this: &Rc<WikipediaPageViewModel>| TreeDataGridSourceRef::of::<OnThisDayArticle, _>(this.source())
        },
    ],
    notify_property_changed: WikipediaPageViewModel,
});
