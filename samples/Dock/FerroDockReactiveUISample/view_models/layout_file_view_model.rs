//! Reactive file actions composed around the original layout model and storage services.
use super::MainWindowViewModel;
use crate::services::ILayoutFileStorage;
use ferrodock_model::core::SerializationResult;
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactiveui::{
    reactive_object, ICommand, IReactiveObject, IReactiveObjectExt, ReactiveCommand, ReactiveObject,
};
use ferroui_system::IObserver;
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
};

/// Coordinates layout file commands and feedback without referencing UI controls.
pub struct LayoutFileViewModel {
    base: ReactiveObject,
    owner: Weak<MainWindowViewModel>,
    storage: Rc<dyn ILayoutFileStorage>,
    status: RefCell<String>,
    busy: Cell<bool>,
    available: Rc<BehaviorSubject<bool>>,
    save_layout: OnceCell<Rc<dyn ICommand>>,
    load_layout: OnceCell<Rc<dyn ICommand>>,
}
reactive_object!(LayoutFileViewModel, base);
impl AsRef<ReactiveObject> for LayoutFileViewModel {
    fn as_ref(&self) -> &ReactiveObject {
        &self.base
    }
}
impl PartialEq for LayoutFileViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl LayoutFileViewModel {
    pub(crate) fn new(
        owner: &Rc<MainWindowViewModel>,
        storage: Rc<dyn ILayoutFileStorage>,
    ) -> Rc<Self> {
        let this = Rc::new(Self {
            base: ReactiveObject::new(),
            owner: Rc::downgrade(owner),
            storage,
            status: RefCell::new(String::new()),
            busy: Cell::new(false),
            available: BehaviorSubject::new(owner.can_persist_layout()),
            save_layout: OnceCell::new(),
            load_layout: OnceCell::new(),
        });
        for save in [true, false] {
            let weak = Rc::downgrade(&this);
            let command = Rc::new(ReactiveCommand::create_from_task_with(
                move || {
                    let weak = weak.clone();
                    async move {
                        if let Some(this) = weak.upgrade() {
                            this.execute_file_action(save).await;
                        }
                        Ok(())
                    }
                },
                Some(this.available.as_observable()),
                None,
            ));
            let slot = if save {
                &this.save_layout
            } else {
                &this.load_layout
            };
            let _ = slot.set(command);
        }
        this
    }
    /// Gets the command to select and save a JSON layout file.
    pub fn save_layout(&self) -> Rc<dyn ICommand> {
        self.save_layout
            .get()
            .expect("initialized file command")
            .clone()
    }
    /// Gets the command to select and load a JSON layout file.
    pub fn load_layout(&self) -> Rc<dyn ICommand> {
        self.load_layout
            .get()
            .expect("initialized file command")
            .clone()
    }
    /// Gets operation feedback, including cancellation and useful failure messages.
    pub fn status(&self) -> String {
        self.status.borrow().clone()
    }
    /// Gets whether a picker/read/write operation is in progress.
    pub fn is_busy(&self) -> bool {
        self.busy.get()
    }
    fn set_status(&self, value: String) {
        self.raise_and_set_if_changed(&self.status, value, "Status");
    }
    async fn execute_file_action(&self, save: bool) {
        if self.busy.replace(true) {
            return;
        }
        self.raise_property_changed("IsBusy");
        self.available.on_next(false);
        self.set_status(
            if save {
                "Saving layout…"
            } else {
                "Opening layout…"
            }
            .into(),
        );
        let result = self.file_action(save).await;
        self.set_status(match result {
            Ok(Some(name)) => format!("{} {name}", if save { "Saved" } else { "Loaded" }),
            Ok(None) => if save {
                "Save cancelled."
            } else {
                "Open cancelled."
            }
            .into(),
            Err(error) => format!(
                "{} layout failed: {error}",
                if save { "Saving" } else { "Loading" }
            ),
        });
        self.busy.set(false);
        self.raise_property_changed("IsBusy");
        self.available
            .on_next(self.owner.upgrade().is_some_and(|m| m.can_persist_layout()));
    }
    async fn file_action(&self, save: bool) -> SerializationResult<Option<String>> {
        let Some(owner) = self.owner.upgrade() else {
            return Ok(None);
        };
        if save {
            self.storage.save_layout(owner.save_layout_bytes()?).await
        } else if let Some(file) = self.storage.open_layout().await? {
            owner.load_layout_bytes(file.bytes)?;
            Ok(Some(file.name))
        } else {
            Ok(None)
        }
    }
}
