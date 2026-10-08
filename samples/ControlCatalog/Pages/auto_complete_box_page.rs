//! Port of `Pages/AutoCompleteBoxPage.xaml.cs`: the class of the document
//! `Pages/AutoCompleteBoxPage.xaml`.
//!
//! The delegates the page gives its boxes (the asynchronous populator, the
//! text filter and the text selector) are methods of the page in the managed
//! original; the boxes are descendants of the page, so the delegates hold
//! the page weakly here.

use crate::markup::{content_page_class, xaml_class};
use crate::models::StateData;
use ferroui_base::data::converters::FuncMultiValueConverter;
use ferroui_base::data::core::TypedClrPropertyInfo;
use ferroui_base::data::{BindingBase, CompiledBinding, CompiledBindingPathBuilder, MultiBinding};
use ferroui_base::threading::{CancellationToken, CancellationTokenRegistration, OperationCanceledError};
use ferroui_base::utilities::StringComparison;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    AssignedBinding, AutoCompleteAsyncPopulator, AutoCompleteBox, AutoCompleteFilterPredicate, AutoCompleteSelector,
    ContentPage, ItemsSource,
};
use mini_mvvm::Delay;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// `Task.Delay(duration, cancellationToken)`: completes when `duration` has
/// passed, or with the cancellation as soon as the token is cancelled.
struct CancellableDelay {
    delay: Delay,
    cancellation_token: CancellationToken,
    /// Who waits, for the callback of the token.
    waker: Arc<Mutex<Option<Waker>>>,
    registration: Option<CancellationTokenRegistration>,
}

impl CancellableDelay {
    fn new(duration: Duration, cancellation_token: CancellationToken) -> Self {
        Self { delay: mini_mvvm::delay(duration), cancellation_token, waker: Arc::default(), registration: None }
    }
}

impl Future for CancellableDelay {
    type Output = Result<(), OperationCanceledError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if this.cancellation_token.is_cancellation_requested() {
            return Poll::Ready(Err(OperationCanceledError));
        }
        if Pin::new(&mut this.delay).poll(cx).is_ready() {
            return Poll::Ready(Ok(()));
        }

        *this.waker.lock().unwrap_or_else(PoisonError::into_inner) = Some(cx.waker().clone());
        if this.registration.is_none() {
            let waker = this.waker.clone();
            this.registration = Some(this.cancellation_token.register(move || {
                let waker = waker.lock().unwrap_or_else(PoisonError::into_inner).take();
                if let Some(waker) = waker {
                    waker.wake();
                }
            }));
        }
        Poll::Pending
    }
}

impl Drop for CancellableDelay {
    fn drop(&mut self) {
        if let Some(registration) = &self.registration {
            registration.dispose();
        }
    }
}

#[repr(C)]
pub struct AutoCompleteBoxPage {
    base: ContentPage,
    states: Rc<[Rc<StateData>]>,
    /// The sentences, each as its words in order (`LinkedList<string>[]`).
    sentences: Rc<[Vec<String>]>,
}

content_page_class!(AutoCompleteBoxPage);
ferro_class_info!(AutoCompleteBoxPage { new: AutoCompleteBoxPage::new });
xaml_class!(AutoCompleteBoxPage, "/Pages/AutoCompleteBoxPage.xaml");

/// A word of a sentence (`LinkedListNode<string>`): the index of the
/// sentence and of the word in it.
type Word = (usize, usize);

impl AutoCompleteBoxPage {
    fn build_all_states() -> Rc<[Rc<StateData>]> {
        Rc::from(vec![
            StateData::new("Alabama", "AL", "Montgomery"),
            StateData::new("Alaska", "AK", "Juneau"),
            StateData::new("Arizona", "AZ", "Phoenix"),
            StateData::new("Arkansas", "AR", "Little Rock"),
            StateData::new("California", "CA", "Sacramento"),
            StateData::new("Colorado", "CO", "Denver"),
            StateData::new("Connecticut", "CT", "Hartford"),
            StateData::new("Delaware", "DE", "Dover"),
            StateData::new("Florida", "FL", "Tallahassee"),
            StateData::new("Georgia", "GA", "Atlanta"),
            StateData::new("Hawaii", "HI", "Honolulu"),
            StateData::new("Idaho", "ID", "Boise"),
            StateData::new("Illinois", "IL", "Springfield"),
            StateData::new("Indiana", "IN", "Indianapolis"),
            StateData::new("Iowa", "IA", "Des Moines"),
            StateData::new("Kansas", "KS", "Topeka"),
            StateData::new("Kentucky", "KY", "Frankfort"),
            StateData::new("Louisiana", "LA", "Baton Rouge"),
            StateData::new("Maine", "ME", "Augusta"),
            StateData::new("Maryland", "MD", "Annapolis"),
            StateData::new("Massachusetts", "MA", "Boston"),
            StateData::new("Michigan", "MI", "Lansing"),
            StateData::new("Minnesota", "MN", "St. Paul"),
            StateData::new("Mississippi", "MS", "Jackson"),
            StateData::new("Missouri", "MO", "Jefferson City"),
            StateData::new("Montana", "MT", "Helena"),
            StateData::new("Nebraska", "NE", "Lincoln"),
            StateData::new("Nevada", "NV", "Carson City"),
            StateData::new("New Hampshire", "NH", "Concord"),
            StateData::new("New Jersey", "NJ", "Trenton"),
            StateData::new("New Mexico", "NM", "Santa Fe"),
            StateData::new("New York", "NY", "Albany"),
            StateData::new("North Carolina", "NC", "Raleigh"),
            StateData::new("North Dakota", "ND", "Bismarck"),
            StateData::new("Ohio", "OH", "Columbus"),
            StateData::new("Oklahoma", "OK", "Oklahoma City"),
            StateData::new("Oregon", "OR", "Salem"),
            StateData::new("Pennsylvania", "PA", "Harrisburg"),
            StateData::new("Rhode Island", "RI", "Providence"),
            StateData::new("South Carolina", "SC", "Columbia"),
            StateData::new("South Dakota", "SD", "Pierre"),
            StateData::new("Tennessee", "TN", "Nashville"),
            StateData::new("Texas", "TX", "Austin"),
            StateData::new("Utah", "UT", "Salt Lake City"),
            StateData::new("Vermont", "VT", "Montpelier"),
            StateData::new("Virginia", "VA", "Richmond"),
            StateData::new("Washington", "WA", "Olympia"),
            StateData::new("West Virginia", "WV", "Charleston"),
            StateData::new("Wisconsin", "WI", "Madison"),
            StateData::new("Wyoming", "WY", "Cheyenne"),
        ])
    }

    pub fn states(&self) -> Rc<[Rc<StateData>]> {
        self.states.clone()
    }

    fn build_all_sentences() -> Rc<[Vec<String>]> {
        ["Hello world", "No this is Patrick", "Never gonna give you up", "How does one patch KDE2 under FreeBSD"]
            .iter()
            .map(|x| x.split(' ').map(str::to_string).collect())
            .collect()
    }

    pub fn sentences(&self) -> Rc<[Vec<String>]> {
        self.sentences.clone()
    }

    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            states: Self::build_all_states(),
            sentences: Self::build_all_sentences(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // One list for the boxes, as the array of the states is in the managed original.
        let states = ItemsSource::from_items(this.states.iter().map(|state| Some(state.clone() as BoxedValue)));
        for auto_complete_box in
            this.get_all_auto_complete_box().into_iter().filter(|x| x.name().as_deref() != Some("CustomAutocompleteBox"))
        {
            auto_complete_box.set_items_source(Some(states.clone()));
        }

        let converter =
            FuncMultiValueConverter::<String, String>::new(|parts| format!("{} ({})", parts[0], parts[1]));
        let binding = MultiBinding::new().with_converter(Rc::new(converter));
        // Deviation (DEVIATIONS.md, ControlCatalog sample): upstream builds the two bindings with
        // `CompiledBinding.Create<StateData, string>(s => s.Name)`, which derives the path from an
        // expression tree; there are no expression trees, so the same one-element paths are
        // written with the path builder.
        binding.bindings().add(Self::state_binding("Name", StateData::name));
        binding.bindings().add(Self::state_binding("Abbreviation", StateData::abbreviation));

        let binding: Rc<dyn BindingBase> = binding;
        this.multi_binding_box().set_value_member_binding(Some(AssignedBinding::new(binding)));

        // The boxes are descendants of the page: their delegates hold the page weakly.
        {
            let weak = this.downgrade();
            this.async_box().set_async_populator(Some(AutoCompleteAsyncPopulator::new(
                move |search_text, cancellation_token| {
                    let this = weak.upgrade();
                    Box::pin(async move {
                        match this {
                            Some(this) => this.populate_async(search_text, cancellation_token).await,
                            None => Err(OperationCanceledError),
                        }
                    })
                },
            )));
        }

        let custom_autocomplete_box = this.custom_autocomplete_box();
        custom_autocomplete_box
            .set_items_source(Some(ItemsSource::from_values(this.sentences.iter().flat_map(|x| x.iter().cloned()))));
        {
            let weak = this.downgrade();
            custom_autocomplete_box.set_text_filter(Some(AutoCompleteFilterPredicate::new(
                move |search_text: Option<&str>, item: &Option<String>| {
                    weak.upgrade().is_some_and(|this| this.last_word_contains(search_text, item.as_deref()))
                },
            )));
        }
        {
            let weak = this.downgrade();
            custom_autocomplete_box.set_text_selector(Some(AutoCompleteSelector::new(
                move |text: Option<&str>, item: &Option<String>| {
                    weak.upgrade().map(|this| this.append_word(text, item.as_deref())).unwrap_or_default()
                },
            )));
        }

        this
    }

    /// A binding to a property of a state.
    fn state_binding(name: &str, get: fn(&StateData) -> String) -> Rc<dyn BindingBase> {
        let info = TypedClrPropertyInfo::<StateData, String>::new(name, Some(Rc::new(get)), None);
        CompiledBinding::new(CompiledBindingPathBuilder::new().typed_plain_property_info_with(info, false).build())
    }

    fn multi_binding_box(&self) -> Ref<AutoCompleteBox> {
        self.get_control::<AutoCompleteBox>("MultiBindingBox")
    }

    fn async_box(&self) -> Ref<AutoCompleteBox> {
        self.get_control::<AutoCompleteBox>("AsyncBox")
    }

    fn custom_autocomplete_box(&self) -> Ref<AutoCompleteBox> {
        self.get_control::<AutoCompleteBox>("CustomAutocompleteBox")
    }

    fn get_all_auto_complete_box(&self) -> Vec<Ref<AutoCompleteBox>> {
        self.get_logical_descendants().filter_map(|x| x.cast::<AutoCompleteBox>()).collect()
    }

    fn string_contains(str: &str, query: Option<&str>) -> bool {
        let Some(query) = query else {
            return false;
        };
        StringComparison::OrdinalIgnoreCase.index_of(str, query) >= 0
    }

    async fn populate_async(
        &self,
        search_text: Option<String>,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Option<BoxedValue>>, OperationCanceledError> {
        CancellableDelay::new(Duration::from_secs_f64(1.5), cancellation_token).await?;

        Ok(self
            .states
            .iter()
            .filter(|data| {
                Self::string_contains(&data.name(), search_text.as_deref())
                    || Self::string_contains(&data.capital(), search_text.as_deref())
            })
            .map(|data| Some(data.clone() as BoxedValue))
            .collect())
    }

    /// The word after `word` in its sentence (`LinkedListNode<string>.Next`).
    fn next(&self, word: Word) -> Option<Word> {
        let (sentence, index) = word;
        (index + 1 < self.sentences[sentence].len()).then_some((sentence, index + 1))
    }

    /// # Panics
    /// Panics if the search text has more words than there are sentences (an
    /// index out of range in the managed original, which reads the option of
    /// the word index).
    fn last_word_contains(&self, search_text: Option<&str>, item: Option<&str>) -> bool {
        let words: Vec<&str> = search_text.map(|text| text.split(' ').collect()).unwrap_or_default();
        let mut options: Vec<Option<Word>> = (0..self.sentences.len()).map(|x| Some((x, 0))).collect();
        for i in 0..words.len() {
            let word = words[i];
            for j in 0..options.len() {
                if let Some(option) = options[i] {
                    let value = &self.sentences[option.0][option.1];
                    if i == words.len() - 1 {
                        options[j] = value.to_lowercase().contains(&word.to_lowercase()).then_some(option);
                    } else {
                        options[j] = if StringComparison::InvariantCultureIgnoreCase.equals(Some(value.as_str()), Some(word)) {
                            self.next(option)
                        } else {
                            None
                        };
                    }
                }
            }
        }

        options.iter().any(|x| x.is_some_and(|x| Some(self.sentences[x.0][x.1].as_str()) == item))
    }

    fn append_word(&self, text: Option<&str>, item: Option<&str>) -> String {
        if let Some(item) = item {
            let mut parts: Vec<&str> = text.map(|text| text.split(' ').collect()).unwrap_or_default();
            if parts.is_empty() {
                return item.to_string();
            }

            let last = parts.len() - 1;
            parts[last] = item;
            return parts.join(" ");
        }
        String::new()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_states_and_the_sentences_are_the_ones_of_the_page() {
        let states = AutoCompleteBoxPage::build_all_states();
        assert_eq!(50, states.len());
        assert_eq!("Alabama", states[0].name());
        assert_eq!("Cheyenne", states[49].capital());

        let sentences = AutoCompleteBoxPage::build_all_sentences();
        assert_eq!(4, sentences.len());
        assert_eq!(vec!["Hello", "world"], sentences[0]);
        assert_eq!(7, sentences[3].len());
    }

    #[test]
    fn a_text_contains_a_query_whatever_its_case() {
        assert!(AutoCompleteBoxPage::string_contains("Montgomery", Some("GOM")));
        assert!(!AutoCompleteBoxPage::string_contains("Montgomery", Some("x")));
        assert!(!AutoCompleteBoxPage::string_contains("Montgomery", None));
    }
}
