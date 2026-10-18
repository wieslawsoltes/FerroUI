//! `TreeSelectionModelBaseTests_Single.cs`: the tree selection model in single selection mode.
//!
//! The reference tests its classic copy of the tree selection model here; the port has one
//! selection model, the core's [`TreeSelectionModelBase`], which these tests exercise. The
//! collections of the tests (the framework's notifying list) are [`FerroList`]s.

use ferroui_base::collections::FerroList;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_treedatagrid_core::model_collection::{Models, ModelsChangedArgs};
use ferroui_treedatagrid_core::selection::{
    IReadOnlyList, TreeSelectionModelBase, TreeSelectionModelHooks, TreeSelectionModelIndexesChangedEventArgs,
    TreeSelectionModelSelectionChangedEventArgsOf,
};
use ferroui_treedatagrid_core::utils::{CollectionChangedEventManager, ICollectionChangedListener};
use ferroui_treedatagrid_core::{index_path, IndexPath};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

type Args = TreeSelectionModelSelectionChangedEventArgsOf<Node>;

mod source {
    use super::*;

    #[test]
    fn changing_source_to_non_null_first_clears_old_selection() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);

        {
            let raised = raised.clone();
            let model = target.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(vec![index_path![0, 2]], e.deselected_indexes().to_vec());
                assert_eq!("Node 0-2", single(e.deselected_items().to_vec()).unwrap().caption());
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                assert_eq!(0, model.count());
                bump(&raised);
            }));
        }

        target.set_source(Some(create_data(1) as Models<Node>));

        assert_eq!(0, target.count());
        // The reference compares with `-1`, which converts to the path of no index.
        assert_eq!(IndexPath::new(-1), target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn can_assign_value_type_collection_to_selection_model_of_object() {
        // The reference assigns an array of integers to the framework's untyped selection model
        // (`SelectionModel<object>` seen as `ISelectionModel`), which must not throw.
        // The framework's selection contract is imported here only: its members share their names
        // with the tree selection model's.
        use ferroui_base::BoxedValue;
        use ferroui_controls::selection::{ISelectionModel, SelectionModel};
        use ferroui_controls::ItemsSource;

        let target: Rc<dyn ISelectionModel> = SelectionModel::<BoxedValue>::new();

        target.set_source(Some(ItemsSource::from_values([1, 2, 3])));
    }
}

mod selected_index {
    use super::*;

    #[test]
    fn can_set_selected_index() {
        let target = create_target(None);
        let raised = counter();

        {
            let raised = raised.clone();
            let model = target.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                assert_eq!(1, model.count());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn can_set_grandchild_selected_index() {
        let data = create_data(3);
        let target = create_target(Some(data));
        let raised = counter();

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![0, 0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![0, 0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 0, 2], target.selected_index());
        assert_eq!(index_path![0, 0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn setting_selected_index_clears_old_selection() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 1]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(index_path![0, 1], single(e.deselected_indexes().to_vec()));
                assert_eq!("Node 0-1", single(e.deselected_items().to_vec()).unwrap().caption());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn can_set_selected_index_to_empty() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(index_path![0, 2], single(e.deselected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.deselected_items().to_vec()).unwrap().caption());
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.set_selected_index(IndexPath::UNSELECTED);

        assert_eq!(1, raised.get());
        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
    }

    #[test]
    fn out_of_range_selected_index_clears_selection() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(index_path![0, 2], single(e.deselected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.deselected_items().to_vec()).unwrap().caption());
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![5, 10, 250]);

        assert_eq!(1, raised.get());
        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
    }

    #[test]
    fn can_select_unexpanded_item() {
        let target = create_target(None);
        let raised = counter();

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![1, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 1-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![1, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 2], target.selected_index());
        assert_eq!(index_path![1, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 1-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 1-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn setting_selected_index_during_collection_changed_results_in_correct_selection() {
        let data: Rc<FerroList<Rc<Node>>> = Rc::new(FerroList::new());
        let target = create_target(Some(data.clone()));
        // The collection manager holds its listeners weakly: the binding is kept alive for the change
        // (the reference discards it and relies on it not being collected yet).
        let _binding = MockBinding::new(target.clone(), &data);

        data.add(Node::new(None));

        assert_eq!(IndexPath::new(0), target.selected_index());
    }

    #[test]
    fn property_changed_is_raised() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedIndex");

        target.set_selected_index(index_path![0, 2]);

        assert_eq!(1, raised.get());
    }

    struct MockBinding {
        target: Rc<TestTreeSelectionModel>,
    }

    impl MockBinding {
        fn new(target: Rc<TestTreeSelectionModel>, data: &Rc<FerroList<Rc<Node>>>) -> Rc<Self> {
            let this = Rc::new(Self { target });
            let listener: Weak<dyn ICollectionChangedListener<Node>> = Rc::downgrade(&this) as Weak<dyn ICollectionChangedListener<Node>>;
            CollectionChangedEventManager::add_listener(&(data.clone() as Models<Node>), listener);
            this
        }
    }

    impl ICollectionChangedListener<Node> for MockBinding {
        fn changed(&self, _collection: &Models<Node>, _e: &ModelsChangedArgs<'_, Node>) {
            self.target.select(&IndexPath::new(0));
        }

        fn post_changed(&self, _collection: &Models<Node>, _e: &ModelsChangedArgs<'_, Node>) {}

        fn pre_changed(&self, _collection: &Models<Node>, _e: &ModelsChangedArgs<'_, Node>) {}
    }
}

mod selected_item {
    use super::*;

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedItem");

        target.set_selected_index(IndexPath::new(1));

        assert_eq!(1, raised.get());
    }
}

mod selected_indexes {
    use super::*;

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedIndexes");

        target.set_selected_index(IndexPath::new(1));

        assert_eq!(1, raised.get());
    }
}

mod selected_items {
    use super::*;

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedItems");

        target.set_selected_index(IndexPath::new(1));

        assert_eq!(1, raised.get());
    }
}

mod select {
    use super::*;

    #[test]
    fn select_sets_selected_index() {
        let target = create_target(None);
        let raised = counter();

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.select(&index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn select_clears_old_selection() {
        let target = create_target(None);
        let raised = counter();

        target.select(&index_path![0, 1]);

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(index_path![0, 1], single(e.deselected_indexes().to_vec()));
                assert_eq!("Node 0-1", single(e.deselected_items().to_vec()).unwrap().caption());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.select(&index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn select_with_invalid_index_does_nothing() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        count_selection_changes(&target, &raised);

        target.select(&index_path![5, 10, 250]);

        assert_eq!(0, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn selecting_already_selected_item_doesnt_raise_selection_changed() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        count_selection_changes(&target, &raised);

        target.select(&index_path![0, 2]);

        assert_eq!(0, raised.get());
    }

    #[test]
    fn selecting_item_twice_results_in_correct_count() {
        let target = create_target(None);

        {
            let _update = target.batch_update();
            target.set_selected_index(IndexPath::new(1));
            target.select(&IndexPath::new(1));
        }

        assert_eq!(1, target.count());
    }
}

mod deselect {
    use super::*;

    #[test]
    fn deselect_clears_current_selection() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 1]);

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(vec![index_path![0, 1]], e.deselected_indexes().to_vec());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.deselect(&index_path![0, 1]);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn deselect_does_nothing_for_nonselected_item() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 1]);
        count_selection_changes(&target, &raised);
        target.deselect(&index_path![0, 0]);

        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 1], target.selected_index());
        assert_eq!(vec![index_path![0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(0, raised.get());
    }
}

mod clear {
    use super::*;

    #[test]
    fn clear_raises_selection_changed() {
        let target = create_target(None);
        let raised = counter();

        target.select(&index_path![0, 1]);

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(vec![index_path![0, 1]], e.deselected_indexes().to_vec());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.clear();

        assert_eq!(1, raised.get());
    }
}

mod anchor_index {
    use super::*;

    #[test]
    fn setting_selected_index_sets_anchor_index() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "AnchorIndex");

        target.set_selected_index(index_path![0, 1]);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn setting_selected_index_to_empty_doesnt_clear_anchor_index() {
        let target = create_target(None);

        target.set_selected_index(index_path![0, 1]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.set_selected_index(IndexPath::UNSELECTED);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn select_sets_anchor_index() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "AnchorIndex");

        target.select(&index_path![0, 1]);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn deselect_doesnt_clear_anchor_index() {
        let target = create_target(None);

        target.select(&index_path![0, 1]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.deselect(&index_path![0, 1]);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn raises_property_changed() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "AnchorIndex");

        target.set_selected_index(index_path![0, 1]);

        assert_eq!(1, raised.get());
    }
}

mod single_select {
    use super::*;

    #[test]
    fn converting_to_multiple_selection_preserves_selection() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 1]);

        count_selection_changes(&target, &raised);

        target.set_single_select(false);

        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 1], target.selected_index());
        assert_eq!(vec![index_path![0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(0, raised.get());
    }

    #[test]
    fn raises_property_changed() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SingleSelect");

        target.set_single_select(false);

        assert_eq!(1, raised.get());
    }
}

mod collection_changes {
    use super::*;

    #[test]
    fn adding_root_item_before_selected_root_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(IndexPath::new(1));

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(2), target.selected_index());
        assert_eq!(vec![IndexPath::new(2)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(2), target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_child_item_before_selected_child_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![0, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::new(0), *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.get(0).children().insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(vec![index_path![0, 2]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![0, 2], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_root_item_before_selected_child_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![0, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 1], target.selected_index());
        assert_eq!(vec![index_path![1, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![1, 1], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_root_item_before_selected_grandchild_item_updates_indexes() {
        let data = create_data(3);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![0, 0, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 0, 1], target.selected_index());
        assert_eq!(vec![index_path![1, 0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![1, 0, 1], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_root_item_after_selected_root_item_doesnt_raise_events() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let raised = counter();

        target.set_selected_index(IndexPath::new(1));

        {
            let raised = raised.clone();
            target.property_changed().add(Rc::new(move |_name: &str| bump(&raised)));
        }
        count_selection_changes(&target, &raised);
        {
            let raised = raised.clone();
            target.add_indexes_changed(Rc::new(move |_e: &TreeSelectionModelIndexesChangedEventArgs| bump(&raised)));
        }

        data.insert(2, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(1), target.selected_index());
        assert_eq!(vec![IndexPath::new(1)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(1), target.anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn removing_root_selected_item_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();

        target.select(&IndexPath::new(1));

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.remove_at(1);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(IndexPath::UNSELECTED, target.anchor_index());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
    }

    #[test]
    fn removing_child_selected_item_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();

        target.select(&index_path![0, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.get(0).children().remove_at(1);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(IndexPath::UNSELECTED, target.anchor_index());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
    }

    #[test]
    fn removing_parent_of_selected_item_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();

        target.select(&index_path![0, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.remove_at(0);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(IndexPath::UNSELECTED, target.anchor_index());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
    }

    #[test]
    fn removing_root_item_before_selected_root_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(IndexPath::new(1));

        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(0, e.start_index());
                assert_eq!(-1, e.delta());
                bump(&raised);
            }));
        }

        data.remove_at(0);

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(0), target.selected_index());
        assert_eq!(vec![IndexPath::new(0)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(0), target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn removing_root_item_before_selected_child_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![1, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(0, e.start_index());
                assert_eq!(-1, e.delta());
                bump(&raised);
            }));
        }

        data.remove_at(0);

        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 1], target.selected_index());
        assert_eq!(vec![index_path![0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 1-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn removing_child_item_before_selected_grandhild_item_updates_indexes() {
        let data = create_data(3);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![1, 1, 2]);

        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(0, e.start_index());
                assert_eq!(-1, e.delta());
                bump(&raised);
            }));
        }

        data.get(1).children().remove_at(0);

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 0, 2], target.selected_index());
        assert_eq!(vec![index_path![1, 0, 2]], target.selected_indexes().to_vec());
        assert_eq!("Node 1-1-2", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1-1-2"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![1, 0, 2], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn removing_root_item_after_selected_root_item_doesnt_raise_events() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let raised = counter();

        target.set_selected_index(IndexPath::new(1));

        {
            let raised = raised.clone();
            target.property_changed().add(Rc::new(move |_name: &str| bump(&raised)));
        }
        count_selection_changes(&target, &raised);
        {
            let raised = raised.clone();
            target.add_indexes_changed(Rc::new(move |_e: &TreeSelectionModelIndexesChangedEventArgs| bump(&raised)));
        }

        data.remove_at(2);

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(1), target.selected_index());
        assert_eq!(vec![IndexPath::new(1)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(1), target.anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn replacing_selected_root_item_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();

        target.select(&IndexPath::new(1));

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.set(1, Node::new(Some("new")));

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn replacing_selected_child_item_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();

        target.select(&index_path![1, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 1-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.get(1).children().set(1, Node::new(Some("new")));

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn moving_selected_root_item_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let indexes_changed_raised = counter();
        let selection_changed_raised = counter();

        target.select(&IndexPath::new(1));
        let moved_item = data.get(1);

        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |_e: &TreeSelectionModelIndexesChangedEventArgs| bump(&raised)));
        }
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);

        data.move_item(1, 3);

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(3), target.selected_index());
        assert_eq!(vec![IndexPath::new(3)], target.selected_indexes().to_vec());
        assert_same(&moved_item, &target.selected_item().unwrap());
        assert_same(&moved_item, &single(target.selected_items().to_vec()).unwrap());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selected_item_raised.get());
    }






    #[test]
    fn moving_selected_child_item_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let indexes_changed_raised = counter();
        let selection_changed_raised = counter();

        target.select(&index_path![1, 1]);
        let moved_child = data.get(1).children().get(1);

        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |_e: &TreeSelectionModelIndexesChangedEventArgs| bump(&raised)));
        }
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);

        data.get(1).children().move_item(1, 2);

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 2], target.selected_index());
        assert_eq!(vec![index_path![1, 2]], target.selected_indexes().to_vec());
        assert_same(&moved_child, &target.selected_item().unwrap());
        assert_same(&moved_child, &single(target.selected_items().to_vec()).unwrap());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selected_item_raised.get());
    }

    #[test]
    fn moving_unselected_child_item_from_before_selected_item_to_after_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let indexes_changed_raised = counter();
        let selection_changed_raised = counter();

        target.select(&index_path![1, 1]);

        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::new(1), *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(2, e.end_index());
                assert_eq!(-1, e.delta());
                bump(&raised);
            }));
        }
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);

        data.get(1).children().move_item(0, 2);

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 0], target.selected_index());
        assert_eq!(vec![index_path![1, 0]], target.selected_indexes().to_vec());
        assert_eq!(Some("Node 1-1".to_string()), target.selected_item().map(|x| x.caption()));
        assert_eq!(vec!["Node 1-1"], captions(target.selected_items().to_vec()));
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selected_item_raised.get());
    }

    #[test]
    fn moving_unselected_child_item_from_after_selected_item_to_before_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let indexes_changed_raised = counter();
        let selection_changed_raised = counter();

        target.select(&index_path![1, 1]);

        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::new(1), *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(2, e.end_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);

        data.get(1).children().move_item(2, 0);

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 2], target.selected_index());
        assert_eq!(vec![index_path![1, 2]], target.selected_indexes().to_vec());
        assert_eq!(Some("Node 1-1".to_string()), target.selected_item().map(|x| x.caption()));
        assert_eq!(vec!["Node 1-1"], captions(target.selected_items().to_vec()));
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selected_item_raised.get());
    }

    #[test]
    fn resetting_root_updates_state() {
        let data = create_data(2);
        let target = create_target(Some(data.clone()));
        let selection_changed_raised = counter();

        target.select(&IndexPath::new(1));

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        count_selection_changes(&target, &selection_changed_raised);

        data.clear();

        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(IndexPath::UNSELECTED, target.anchor_index());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
    }

    #[test]
    fn handles_selection_made_in_collection_changed() {
        // Tests the following scenario:
        //
        // - Items changes from empty to having 1 item
        // - ViewModel auto-selects item 0 in CollectionChanged
        // - SelectionModel receives CollectionChanged
        // - And so adjusts the selected item from 0 to 1, which is past the end of the items.
        //
        // There's not much we can do about this situation because the order in which
        // CollectionChanged handlers are called can't be known (the problem also exists with
        // WPF). The best we can do is not select an invalid index.
        let data: Rc<FerroList<Rc<Node>>> = Rc::new(FerroList::new());
        let target = create_target(Some(data.clone()));

        {
            let target = target.clone();
            data.add_collection_changed(Rc::new(move |_e: &ModelsChangedArgs<'_, Node>| {
                target.select(&IndexPath::new(0));
            }));
        }

        data.add(Node::new(Some("foo")));

        assert_eq!(IndexPath::new(0), target.selected_index());
        assert_eq!(vec![IndexPath::new(0)], target.selected_indexes().to_vec());
        assert_eq!("foo", target.selected_item().unwrap().caption());
        assert_eq!(vec!["foo"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(0), target.anchor_index());
    }
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn bump(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

/// `target.PropertyChanged += (s, e) => { if (e.PropertyName == name) ++raised; }`.
fn count_property_changes(target: &TestTreeSelectionModel, name: &'static str) -> Rc<Cell<i32>> {
    let raised = counter();
    let r = raised.clone();
    target.property_changed().add(Rc::new(move |property: &str| {
        if property == name {
            bump(&r);
        }
    }));
    raised
}

/// `target.SelectionChanged += (s, e) => ++raised`.
fn count_selection_changes(target: &TestTreeSelectionModel, raised: &Rc<Cell<i32>>) {
    let raised = raised.clone();
    target.add_selection_changed(Rc::new(move |_e: &Args| bump(&raised)));
}

/// The captions of selected items (`items.Select(x => x!.Caption)`).
fn captions(items: Vec<Option<Rc<Node>>>) -> Vec<String> {
    items.into_iter().map(|x| x.map(|x| x.caption()).unwrap_or_default()).collect()
}

fn create_nodes(parent_id: &IndexPath, depth: i32) -> Rc<FerroList<Rc<Node>>> {
    let result: Rc<FerroList<Rc<Node>>> = Rc::new(FerroList::new());

    for i in 0..5 {
        let id = parent_id.append(i);
        let caption = format!("Node {}", id.to_vec().iter().map(|x| x.to_string()).collect::<Vec<_>>().join("-"));

        let node = Rc::new(Node { id, caption: Some(caption), children: RefCell::new(None), target_depth: depth });

        result.add(node);
    }

    result
}

fn create_data(depth: i32) -> Rc<FerroList<Rc<Node>>> {
    create_nodes(&IndexPath::UNSELECTED, depth)
}

fn create_target(data: Option<Rc<FerroList<Rc<Node>>>>) -> Rc<TestTreeSelectionModel> {
    TestTreeSelectionModel::new(data.unwrap_or_else(|| create_data(2)))
}

struct Node {
    id: IndexPath,
    caption: Option<String>,
    children: RefCell<Option<Rc<FerroList<Rc<Node>>>>>,
    target_depth: i32,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Node {
    /// `new Node { Caption = caption }`.
    fn new(caption: Option<&str>) -> Rc<Self> {
        Rc::new(Self {
            id: IndexPath::UNSELECTED,
            caption: caption.map(str::to_string),
            children: RefCell::new(None),
            target_depth: 0,
        })
    }

    fn caption(&self) -> String {
        self.caption.clone().unwrap_or_default()
    }

    /// `Children!`.
    fn children(&self) -> Rc<FerroList<Rc<Node>>> {
        self.children.borrow().clone().expect("the children of the node")
    }
}

/// The test model; its `Source` setter (`public new IEnumerable? Source`) is the base's
/// [`TreeSelectionModelBase::set_source`], reached through `Deref`.
struct TestTreeSelectionModel {
    base: Rc<TreeSelectionModelBase<Node>>,
}

impl TestTreeSelectionModel {
    fn new(data: Rc<FerroList<Rc<Node>>>) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| {
            let base = TreeSelectionModelBase::with_source(Some(data as Models<Node>));
            base.set_hooks(this.clone() as Weak<dyn TreeSelectionModelHooks<Node>>);
            Self { base }
        })
    }
}

impl Deref for TestTreeSelectionModel {
    type Target = TreeSelectionModelBase<Node>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl TreeSelectionModelHooks<Node> for TestTreeSelectionModel {
    fn get_children(&self, _model: &TreeSelectionModelBase<Node>, node: &Rc<Node>) -> Option<Models<Node>> {
        if node.children.borrow().is_none() && (node.id.count() as i32) < node.target_depth {
            *node.children.borrow_mut() = Some(create_nodes(&node.id, node.target_depth));
        }
        node.children.borrow().clone().map(|children| children as Models<Node>)
    }
}

/// `Assert.Single(items)`: the only item.
fn single<T>(items: Vec<T>) -> T {
    assert_eq!(1, items.len(), "expected a single item");
    items.into_iter().next().expect("a single item")
}

/// `Assert.Same(expected, actual)`.
fn assert_same<T: ?Sized>(expected: &Rc<T>, actual: &Rc<T>) {
    assert!(Rc::ptr_eq(expected, actual), "expected the same object");
}
