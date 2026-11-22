use crate::templates::IDataTemplate;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, TypeInfo};
use std::{cell::RefCell, rc::Rc};

/// The native property used by a column to show or edit a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnTemplateMember {
    Display,
    Editing,
}

impl ColumnTemplateMember {
    pub fn name(self) -> &'static str {
        match self {
            Self::Display => "CellTemplate",
            Self::Editing => "CellEditingTemplate",
        }
    }
}

/// A template slot on a live column with a known declarative owner.
pub struct ColumnTemplate {
    pub index: usize,
    pub expander_inner: bool,
    pub declaration: BoxedValue,
    pub type_name: &'static str,
    pub member: ColumnTemplateMember,
    pub template: Option<Rc<dyn IDataTemplate>>,
}

type Reader = Rc<dyn Fn(&FerroObject, usize) -> Option<Vec<ColumnTemplate>>>;

ferroui_base::ferro_thread_local! {
    static READERS: RefCell<Vec<(&'static TypeInfo, Reader)>> = const { RefCell::new(Vec::new()) };
}

/// Registers a reader for a class and its derived classes. Call this from class initialization.
/// A reader returns `None` if it cannot enumerate the full catalog within the slot budget.
pub fn register_column_templates<T: ObjectType>(
    read: fn(&T, usize) -> Option<Vec<ColumnTemplate>>,
) {
    READERS.with(|readers| {
        let mut readers = readers.borrow_mut();
        if !readers.iter().any(|(type_, _)| *type_ == T::TYPE) {
            let reader: Reader =
                Rc::new(move |object, budget| read(object.downcast_ref::<T>()?, budget));
            readers.push((T::TYPE, reader));
        }
    });
}

/// Reads the templates already held by the control. Readers never create cells or execute templates.
pub fn column_templates(object: &FerroObject, budget: usize) -> Option<Vec<ColumnTemplate>> {
    let reader = READERS.with(|readers| {
        let readers = readers.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, read)) = readers.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(read.clone());
            }
            current = type_.base_type();
        }
        None
    });
    match reader {
        Some(read) => read(object, budget),
        None => Some(Vec::new()),
    }
}
