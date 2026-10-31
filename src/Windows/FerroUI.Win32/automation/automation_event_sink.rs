// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root for license information.

//! The native public-peer event route mirrors CUIAWindow's lazy provider
//! creation. The portable peer has already applied EventsSource once.

use super::automation_node::AutomationNode;
use super::interop::{TextEditChangeType, UiaCoreProviderApi};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::automation::provider::TextEditTextChangedEventArgs;
use ferroui_controls::automation::IAutomationEventSink;
use std::rc::Rc;

pub(crate) struct Win32AutomationEventSink;

impl Win32AutomationEventSink {
    pub(crate) fn register() {
        FerroLocator::current_mutable()
            .bind::<dyn IAutomationEventSink>()
            .to_constant(Rc::new(Self));
    }
}

impl IAutomationEventSink for Win32AutomationEventSink {
    fn raise_text_edit_text_changed(
        &self,
        source: &Ref<AutomationPeer>,
        event: &TextEditTextChangedEventArgs,
    ) {
        Dispatcher::ui_thread().verify_access();
        if let Some(node) = AutomationNode::get_or_create(Some(source)) {
            // Native strings/SAFEARRAY are owned only for this synchronous call.
            // An ordinary peer needs no TextEdit pattern or existing COM node.
            UiaCoreProviderApi::uia_raise_text_edit_text_changed_event(
                Some(&node),
                TextEditChangeType(event.change_type as i32),
                &event.changed_data,
            );
        }
    }
}
