//! `OaphTestFixture` (`ReactiveUI.Tests/ReactiveObjects/Mocks/OaphTestFixture.cs`).

use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::cell::OnceCell;
use std::rc::Rc;

/// The number of letters of the derived property (`PrefixLength`).
const PREFIX_LENGTH: usize = 3;

reactive_properties! {
    /// A view model with a property derived from `IsOnlyOneWord` through a helper
    /// (`OaphTestFixture`).
    ///
    /// Upstream derives it from `TestFixture`; Rust has no inheritance, so the fixture holds the
    /// one property of `TestFixture` that it derives from.
    #[derive(Default)]
    pub struct OaphTestFixture {
        reactive: ReactiveObject,
        #[reactive]
        is_only_one_word: Option<String>,
        first_three_letters_of_one_word: OnceCell<ObservableAsPropertyHelper<String>>,
    }
}
reactive_object!(OaphTestFixture);

impl OaphTestFixture {
    /// The fixture with its helper subscribed to `IsOnlyOneWord`.
    pub fn new() -> Rc<Self> {
        let fixture = Rc::new(Self::default());
        let helper = fixture
            .when_any_value(prop!(OaphTestFixture => is_only_one_word))
            .select(|word: Option<String>| word.unwrap_or_default())
            .select(|word: String| if word.chars().count() >= PREFIX_LENGTH { word.chars().take(PREFIX_LENGTH).collect() } else { word })
            .to_property(&fixture, prop!(OaphTestFixture => first_three_letters_of_one_word));
        let _ = fixture.first_three_letters_of_one_word.set(helper);
        fixture
    }

    /// `FirstThreeLettersOfOneWord`: the first three letters of `IsOnlyOneWord`.
    pub fn first_three_letters_of_one_word(&self) -> String {
        self.first_three_letters_of_one_word.get().map(|helper| helper.value()).unwrap_or_default()
    }
}
