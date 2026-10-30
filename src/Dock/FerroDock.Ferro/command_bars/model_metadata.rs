// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

//! UI-owned metadata over original command-bar models, without domain UI dependencies.
use ferrodock_controls_recycling::{from_recycling_value, to_recycling_value};
use ferrodock_model::{
    collections::ReadOnlyListHandle,
    command_bars::{
        DockCommandBarDefinition, DockCommandBarItem, DockCommandBarKind, DockCommandBarMergeMode,
        IDockCommandBarProvider,
    },
};
use ferroui_base::{
    metadata::MarkupDelegate,
    reactive::{Disposable, IDisposable},
    BoxedValue,
};
use ferroui_reactiveui::ICommand;
use std::rc::Rc;
/// Native descriptor carrier for the original definition model.
#[doc(hidden)]
pub trait DockCommandBarDefinitionMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockCommandBarDefinitionMetadata as "DockCommandBarDefinition" {
    namespace:"FerroDock.Model.CommandBars",this:Rc<DockCommandBarDefinition>,handles:[Rc<DockCommandBarDefinition>,Option<Rc<DockCommandBarDefinition>>],
    constructors:[(String,DockCommandBarKind)=>DockCommandBarDefinition::new],
    properties:[
        Id:String{get:|this:&Rc<DockCommandBarDefinition>|this.id()},
        Kind:DockCommandBarKind{get:|this:&Rc<DockCommandBarDefinition>|this.kind()},
        MergeMode:DockCommandBarMergeMode{get:|this:&Rc<DockCommandBarDefinition>|this.merge_mode(),set:|this:&Rc<DockCommandBarDefinition>,value|this.set_merge_mode(value)},
        Order:i32{get:|this:&Rc<DockCommandBarDefinition>|this.order(),set:|this:&Rc<DockCommandBarDefinition>,value|this.set_order(value)},
        GroupId:Option<String>{get:|this:&Rc<DockCommandBarDefinition>|this.group_id(),set:|this:&Rc<DockCommandBarDefinition>,value|this.set_group_id(value)},
        Items:Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>>{get:|this:&Rc<DockCommandBarDefinition>|this.items(),set:|this:&Rc<DockCommandBarDefinition>,value|this.set_items(value)},
        Content:Option<BoxedValue>{get:|this:&Rc<DockCommandBarDefinition>|this.content().as_ref().map(from_recycling_value),set:|this:&Rc<DockCommandBarDefinition>,value:Option<BoxedValue>|this.set_content(value.map(to_recycling_value))},
    ],
});
/// Native descriptor carrier for the original item model.
#[doc(hidden)]
pub trait DockCommandBarItemMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockCommandBarItemMetadata as "DockCommandBarItem" {
    namespace:"FerroDock.Model.CommandBars",this:Rc<DockCommandBarItem>,handles:[Rc<DockCommandBarItem>,Option<Rc<DockCommandBarItem>>],
    constructors:[(Option<BoxedValue>)=>(|header:Option<BoxedValue>|DockCommandBarItem::new(header.map(to_recycling_value)))],
    properties:[
        Header:Option<BoxedValue>{get:|this:&Rc<DockCommandBarItem>|this.header().as_ref().map(from_recycling_value),set:|this:&Rc<DockCommandBarItem>,value:Option<BoxedValue>|this.set_header(value.map(to_recycling_value))},
        Icon:Option<BoxedValue>{get:|this:&Rc<DockCommandBarItem>|this.icon().as_ref().map(from_recycling_value),set:|this:&Rc<DockCommandBarItem>,value:Option<BoxedValue>|this.set_icon(value.map(to_recycling_value))},
        Command:Option<Rc<dyn ICommand>>{get:|this:&Rc<DockCommandBarItem>|this.command(),set:|this:&Rc<DockCommandBarItem>,value|this.set_command(value)},
        CommandParameter:Option<BoxedValue>{get:|this:&Rc<DockCommandBarItem>|this.command_parameter().as_ref().map(from_recycling_value),set:|this:&Rc<DockCommandBarItem>,value:Option<BoxedValue>|this.set_command_parameter(value.map(to_recycling_value))},
        Items:Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>>{get:|this:&Rc<DockCommandBarItem>|this.items(),set:|this:&Rc<DockCommandBarItem>,value|this.set_items(value)},
        GroupId:Option<String>{get:|this:&Rc<DockCommandBarItem>|this.group_id(),set:|this:&Rc<DockCommandBarItem>,value|this.set_group_id(value)},
        Order:i32{get:|this:&Rc<DockCommandBarItem>|this.order(),set:|this:&Rc<DockCommandBarItem>,value|this.set_order(value)},
        IsSeparator:bool{get:|this:&Rc<DockCommandBarItem>|this.is_separator(),set:|this:&Rc<DockCommandBarItem>,value|this.set_is_separator(value)},
    ],
});
/// Native descriptor carrier for the original command-bar provider.
#[doc(hidden)]
pub trait IDockCommandBarProviderMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockCommandBarProviderMetadata as "IDockCommandBarProvider" {
    namespace:"FerroDock.Model.CommandBars",this:Rc<dyn IDockCommandBarProvider>,handles:[Rc<dyn IDockCommandBarProvider>,Option<Rc<dyn IDockCommandBarProvider>>],
    methods:[fn GetCommandBars() -> ReadOnlyListHandle<Rc<DockCommandBarDefinition>> => (|this:&Rc<dyn IDockCommandBarProvider>|this.get_command_bars())],
    events:[CommandBarsChanged(Option<BoxedValue>,()) => subscribe],
});
pub(super) fn subscribe(
    provider: &Rc<dyn IDockCommandBarProvider>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(provider);
    let token = provider.command_bars_changed().add(Rc::new(move |_| {
        let sender = sender.upgrade().map(|sender| Rc::new(sender) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(()))]);
    }));
    let provider = provider.clone();
    Disposable::create(move || {
        provider.command_bars_changed().remove(token);
    })
}
