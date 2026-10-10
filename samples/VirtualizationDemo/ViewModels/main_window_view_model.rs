//! Port of `ViewModels/MainWindowViewModel.cs`.

use super::{ChatPageViewModel, ExpanderPageViewModel, PlaygroundPageViewModel};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::rc::Rc;

pub struct MainWindowViewModel {
    base: ViewModelBase,
    playground: Rc<PlaygroundPageViewModel>,
    chat: Rc<ChatPageViewModel>,
    expanders: Rc<ExpanderPageViewModel>,
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MainWindowViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl MainWindowViewModel {
    pub fn new() -> Rc<MainWindowViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            playground: PlaygroundPageViewModel::new(),
            chat: ChatPageViewModel::new(),
            expanders: ExpanderPageViewModel::new(),
        })
    }

    pub fn playground(&self) -> Rc<PlaygroundPageViewModel> {
        self.playground.clone()
    }

    pub fn chat(&self) -> Rc<ChatPageViewModel> {
        self.chat.clone()
    }

    pub fn expanders(&self) -> Rc<ExpanderPageViewModel> {
        self.expanders.clone()
    }
}

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    constructors: [() => MainWindowViewModel::new],
    properties: [
        Playground: Rc<PlaygroundPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.playground() },
        Chat: Rc<ChatPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.chat() },
        Expanders: Rc<ExpanderPageViewModel> { get: |this: &Rc<MainWindowViewModel>| this.expanders() },
    ],
    notify_property_changed: MainWindowViewModel,
});
