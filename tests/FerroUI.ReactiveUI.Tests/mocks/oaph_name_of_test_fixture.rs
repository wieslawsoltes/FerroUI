//! `OaphNameOfTestFixture` (`ReactiveUI.Tests/ReactiveObjects/Mocks/OaphNameOfTestFixture.cs`).

use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::cell::OnceCell;
use std::rc::Rc;

/// The number of letters of the derived properties (`LetterCount`).
const LETTER_COUNT: usize = 3;

reactive_properties! {
    /// A view model with two properties derived from `IsOnlyOneWord` through helpers that name
    /// their property (`OaphNameOfTestFixture`).
    ///
    /// Upstream derives it from `TestFixture`; the fixture holds the one property it derives from.
    /// Upstream names the properties with `nameof`; here the path of the property names it.
    #[derive(Default)]
    pub struct OaphNameOfTestFixture {
        reactive: ReactiveObject,
        #[reactive]
        is_only_one_word: Option<String>,
        first_three_letters_of_one_word: OnceCell<ObservableAsPropertyHelper<String>>,
        last_three_letters_of_one_word: OnceCell<ObservableAsPropertyHelper<String>>,
    }
}
reactive_object!(OaphNameOfTestFixture);

impl OaphNameOfTestFixture {
    /// The fixture with its helpers subscribed to `IsOnlyOneWord`.
    pub fn new() -> Rc<Self> {
        let fixture = Rc::new(Self::default());
        let first = fixture
            .when_any_value(prop!(OaphNameOfTestFixture => is_only_one_word))
            .select(|word: Option<String>| word.unwrap_or_default())
            .select(|word: String| if word.chars().count() >= LETTER_COUNT { word.chars().take(LETTER_COUNT).collect() } else { word })
            .to_property(&fixture, prop!(OaphNameOfTestFixture => first_three_letters_of_one_word));
        let _ = fixture.first_three_letters_of_one_word.set(first);
        let last = fixture
            .when_any_value(prop!(OaphNameOfTestFixture => is_only_one_word))
            .select(|word: Option<String>| word.unwrap_or_default())
            .select(|word: String| {
                let count = word.chars().count();
                if count >= LETTER_COUNT { word.chars().skip(count - LETTER_COUNT).collect() } else { word }
            })
            .to_property(&fixture, prop!(OaphNameOfTestFixture => last_three_letters_of_one_word));
        let _ = fixture.last_three_letters_of_one_word.set(last);
        fixture
    }

    /// `FirstThreeLettersOfOneWord`.
    pub fn first_three_letters_of_one_word(&self) -> String {
        self.first_three_letters_of_one_word.get().map(|helper| helper.value()).unwrap_or_default()
    }

    /// `LastThreeLettersOfOneWord`.
    pub fn last_three_letters_of_one_word(&self) -> String {
        self.last_three_letters_of_one_word.get().map(|helper| helper.value()).unwrap_or_default()
    }
}
