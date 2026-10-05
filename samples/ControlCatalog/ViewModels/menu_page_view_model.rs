//! Port of `ViewModels/MenuPageViewModel.cs`.

use super::MenuItemViewModel;
use ferroui_base::data::model::BindableList;
use ferroui_base::input::ICommand;
use ferroui_base::platform::storage::FilePickerOpenOptions;
use ferroui_base::{ferro_markup_type, ElementRef, Ref, Visual};
use ferroui_controls::{Control, ItemsSource, TopLevel, Window};
use mini_mvvm::MiniCommand;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The view model of the menu page.
pub struct MenuPageViewModel {
    /// The view is not owned by its view model: held weakly.
    view: RefCell<Option<ElementRef<Control>>>,
    menu_items: RefCell<Rc<BindableList<Rc<MenuItemViewModel>>>>,
    recent_items: RefCell<Rc<BindableList<Rc<MenuItemViewModel>>>>,
    open_command: Rc<MiniCommand>,
    save_command: Rc<MiniCommand>,
    open_recent_command: Rc<MiniCommand>,
}

impl PartialEq for MenuPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MenuPageViewModel {
    pub fn new() -> Rc<MenuPageViewModel> {
        Rc::new_cyclic(|this: &Weak<MenuPageViewModel>| {
            let open_command = {
                let this = this.clone();
                MiniCommand::create_from_task(move || {
                    let this = this.upgrade();
                    async move {
                        if let Some(this) = this {
                            this.open().await;
                        }
                    }
                })
            };
            let save_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.save();
                    }
                })
            };
            let open_recent_command = {
                let this = this.clone();
                MiniCommand::create_with::<String>(move |path| {
                    if let Some(this) = this.upgrade() {
                        this.open_recent(&path);
                    }
                })
            };

            let recent_items = BindableList::new([
                MenuItemViewModel::new()
                    .with_header("File1.txt")
                    .with_command(open_recent_command.as_command())
                    .with_command_parameter(Rc::new(String::from(r"c:\foo\File1.txt"))),
                MenuItemViewModel::new()
                    .with_header("File2.txt")
                    .with_command(open_recent_command.as_command())
                    .with_command_parameter(Rc::new(String::from(r"c:\foo\File2.txt"))),
            ]);

            let menu_items = BindableList::new([
                MenuItemViewModel::new().with_header("_File").with_items(BindableList::new([
                    MenuItemViewModel::new().with_header("O_pen...").with_command(open_command.as_command()),
                    MenuItemViewModel::new().with_header("Save").with_command(save_command.as_command()),
                    MenuItemViewModel::new().with_header("-"),
                    MenuItemViewModel::new().with_header("Recent").with_items(recent_items.clone()),
                ])),
                MenuItemViewModel::new().with_header("_Edit").with_items(BindableList::new([
                    MenuItemViewModel::new().with_header("_Copy"),
                    MenuItemViewModel::new().with_header("_Paste"),
                ])),
            ]);

            Self {
                view: RefCell::new(None),
                menu_items: RefCell::new(menu_items),
                recent_items: RefCell::new(recent_items),
                open_command,
                save_command,
                open_recent_command,
            }
        })
    }

    pub fn view(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.view.borrow())
    }

    pub fn set_view(&self, value: Option<Ref<Control>>) {
        *self.view.borrow_mut() = ElementRef::from_nullable(value);
    }

    pub fn menu_items(&self) -> Rc<BindableList<Rc<MenuItemViewModel>>> {
        self.menu_items.borrow().clone()
    }

    pub fn set_menu_items(&self, value: Rc<BindableList<Rc<MenuItemViewModel>>>) {
        *self.menu_items.borrow_mut() = value;
    }

    pub fn recent_items(&self) -> Rc<BindableList<Rc<MenuItemViewModel>>> {
        self.recent_items.borrow().clone()
    }

    pub fn set_recent_items(&self, value: Rc<BindableList<Rc<MenuItemViewModel>>>) {
        *self.recent_items.borrow_mut() = value;
    }

    pub fn open_command(&self) -> Rc<MiniCommand> {
        self.open_command.clone()
    }

    pub fn save_command(&self) -> Rc<MiniCommand> {
        self.save_command.clone()
    }

    pub fn open_recent_command(&self) -> Rc<MiniCommand> {
        self.open_recent_command.clone()
    }

    pub async fn open(&self) {
        let view = self.view();
        let window = TopLevel::get_top_level(view.as_ref().map(|view| -> &Visual { view }))
            .and_then(|top_level| top_level.cast::<Window>());
        let Some(window) = window else {
            return;
        };

        let result = window
            .storage_provider()
            .open_file_picker_async(FilePickerOpenOptions::new().with_allow_multiple(true))
            .await
            .unwrap_or_else(|error| panic!("{error}"));

        for file in result {
            super::debug_write_line(&format!("Opened: {}", file.name()));
        }
    }

    pub fn save(&self) {
        super::debug_write_line("Save");
    }

    pub fn open_recent(&self, path: &str) {
        super::debug_write_line(&format!("Open recent: {path}"));
    }
}

ferro_markup_type!(class MenuPageViewModel {
    this: Rc<MenuPageViewModel>,
    handles: [MenuPageViewModel, Rc<MenuPageViewModel>, Option<Rc<MenuPageViewModel>>],
    constructors: [() => MenuPageViewModel::new],
    properties: [
        View: Option<Ref<Control>> {
            get: |this: &Rc<MenuPageViewModel>| this.view(),
            set: |this: &Rc<MenuPageViewModel>, value: Option<Ref<Control>>| this.set_view(value)
        },
        // A list a binding delivers to an items source property.
        MenuItems: ItemsSource { get: |this: &Rc<MenuPageViewModel>| ItemsSource::from(this.menu_items()) },
        RecentItems: ItemsSource { get: |this: &Rc<MenuPageViewModel>| ItemsSource::from(this.recent_items()) },
        OpenCommand: Rc<dyn ICommand> { get: |this: &Rc<MenuPageViewModel>| this.open_command().as_command() },
        SaveCommand: Rc<dyn ICommand> { get: |this: &Rc<MenuPageViewModel>| this.save_command().as_command() },
        OpenRecentCommand: Rc<dyn ICommand> {
            get: |this: &Rc<MenuPageViewModel>| this.open_recent_command().as_command()
        },
    ],
    methods: [
        fn Save() => |this: &Rc<MenuPageViewModel>| this.save(),
        fn OpenRecent(String) => |this: &Rc<MenuPageViewModel>, path: String| this.open_recent(&path),
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_menu_has_the_items_of_the_page() {
        let view_model = MenuPageViewModel::new();
        let items = view_model.menu_items().items().to_vec();
        let headers: Vec<_> = items.iter().map(|item| item.header().unwrap()).collect();
        assert_eq!(vec!["_File", "_Edit"], headers);

        let file = items[0].items().expect("the file menu").items().to_vec();
        let headers: Vec<_> = file.iter().map(|item| item.header().unwrap()).collect();
        assert_eq!(vec!["O_pen...", "Save", "-", "Recent"], headers);
        // The recent files of the file menu are the list of the `RecentItems` property.
        assert!(Rc::ptr_eq(&file[3].items().expect("the recent files"), &view_model.recent_items()));

        let recent = view_model.recent_items().items().to_vec();
        assert_eq!(2, recent.len());
        let parameter = recent[0].command_parameter().expect("the path");
        assert_eq!(Some(&r"c:\foo\File1.txt".to_string()), parameter.downcast_ref::<String>());
        recent[0].command().expect("the command").execute(Some(&parameter));
    }

    #[test]
    fn the_view_is_not_set_until_a_page_sets_it() {
        let view_model = MenuPageViewModel::new();
        assert!(view_model.view().is_none());
        view_model.save_command().execute(None);
    }
}
