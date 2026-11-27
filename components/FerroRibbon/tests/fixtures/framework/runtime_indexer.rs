//! Pure SDK reproducer: declared read-only metadata, runtime-conditional notifications.
use ferroui_base::{
    BoxedValue,
    data::{
        CompiledBinding, CompiledBindingPathBuilder,
        core::ValueType,
        model::{BindableList, IBindableList, INotifyCollectionChanged, ModelTypes},
    },
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::{Border, Control};
use std::rc::Rc;
struct ConditionalList {
    items: Rc<BindableList<i32>>,
    notifying: bool,
}
impl PartialEq for ConditionalList {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
impl IBindableList for ConditionalList {
    fn count(&self) -> usize {
        self.items.items().count()
    }
    fn item_type(&self) -> ValueType {
        ValueType::of::<i32>()
    }
    fn get_item(&self, index: usize) -> Option<BoxedValue> {
        self.items.get_item(index)
    }
    fn set_item(&self, _: usize, _: &BoxedValue) -> bool {
        false
    }
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        self.notifying
            .then_some(self.items.as_ref() as &dyn INotifyCollectionChanged)
    }
}
ferroui_base::ferro_markup_type!(class ConditionalList{this:Rc<ConditionalList>,handles:[ConditionalList,Rc<ConditionalList>],indexers:[(i32)->i32{get:|v:&Rc<ConditionalList>,i|*v.items.get_item(usize::try_from(i).unwrap()).unwrap().downcast_ref::<i32>().unwrap()}]});
#[test]
fn compiled_indexer_uses_runtime_notifications_even_when_declared_type_does_not() {
    ferroui_controls::register_types();
    MarkupType::register(ConditionalList::MARKUP);
    ModelTypes::register::<ConditionalList>(|b| b.list());
    for notifying in [true, false] {
        let source = Rc::new(ConditionalList {
            items: BindableList::new([1]),
            notifying,
        });
        let builder = CompiledBindingPathBuilder::new();
        let path = ferroui_markup_xaml::xaml_il::runtime::compiled::path_indexer(
            &builder,
            ConditionalList::MARKUP,
            0,
            false,
        )
        .build();
        let binding = CompiledBinding::new(path).with_source(Some(source.clone()));
        let target = Border::new();
        let expression = target.bind_binding(Control::tag_property(), &binding);
        assert_eq!(target.tag().unwrap().downcast_ref::<i32>(), Some(&1));
        source.items.items().set(0, 2);
        assert_eq!(
            target.tag().unwrap().downcast_ref::<i32>(),
            Some(if notifying { &2 } else { &1 })
        );
        expression.dispose();
        assert!(!source.items.collection_changed().has_handlers());
    }
}
