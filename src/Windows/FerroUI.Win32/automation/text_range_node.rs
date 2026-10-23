//! Native `ITextRangeProvider` over FerroUI's live text ranges. The semantic
//! methods map WinUI's `CUIATextRangeProviderWrapper` at
//! `fe244ca6ac4d482712b234a9de3cf771cc6481de` (Microsoft Corporation, MIT).
//!
//! The COM object contains only a dispatcher and an identifier. Its non-Send
//! range lives in a UI-thread table, which owns no COM reference and therefore
//! cannot keep a released native range alive. Final release removes the range
//! on that same UI thread. No UI object or arbitrary foreign COM pointer is
//! asserted to be Send.

use super::automation_node::{AutomationNode, ELEMENT_NOT_AVAILABLE, SendProvider};
use super::interop::{
    IRawElementProviderSimple, ITextRangeProvider, ITextRangeProviderImpl,
    TextPatternRangeEndpoint, TextUnit,
};
use super::marshalling::{
    ComVariant, SafeArrayMarshaller, SafeArrayRef, ptr_to_string_bstr, string_to_bstr,
};
use super::text_pattern_marshalling::{
    attribute, attribute_from_variant, attribute_to_variant, endpoint, identity_of, provider_error,
    unit,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::automation::provider as portable;
use ferroui_microcom::{ComPtr, E_ABORT, E_FAIL, E_INVALIDARG, HResult, make_com};
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

struct RangeEntry {
    identity: usize,
    range: Rc<dyn portable::ITextRangeProvider>,
}

#[derive(Default)]
struct RangeTable {
    by_id: HashMap<u64, RangeEntry>,
    by_identity: HashMap<usize, u64>,
}

ferroui_base::ferro_thread_local! {
    static RANGES: RefCell<RangeTable> = RefCell::new(RangeTable::default());
}

static NEXT_RANGE_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) struct TextRangeNode {
    id: u64,
    owner: i32,
    dispatcher: Arc<Dispatcher>,
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TextRangeNode>();
};

/// Only a native range created below may cross the dispatcher boundary in this
/// wrapper. Its immutable shell and atomic COM reference count are thread safe.
pub(crate) struct SendTextRange(ComPtr<ITextRangeProvider>);

// SAFETY: the private field is constructed exclusively by `TextRangeNode::new`,
// whose shell is Send + Sync and whose callbacks marshal all UI objects.
unsafe impl Send for SendTextRange {}

impl SendTextRange {
    pub(crate) fn into_inner(self) -> ComPtr<ITextRangeProvider> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::RichTextBlock;
    use ferroui_controls::automation::peers::ControlAutomationPeer;
    use ferroui_controls::documents::Paragraph;

    #[test]
    fn native_text_range_delayed_release_preserves_reused_com_identity() {
        let _scope = Dispatcher::unit_test_scope();
        let view = RichTextBlock::new();
        view.blocks().add(Paragraph::with_text("range"));
        let peer = ControlAutomationPeer::create_peer_for_element(&view);
        let provider = peer.get_provider::<dyn portable::ITextProvider>().unwrap();
        let first = TextRangeNode::new(0, provider.document_range().unwrap());
        let second = TextRangeNode::new(0, provider.document_range().unwrap());
        let old_identity = first.0.as_ptr() as usize;
        let new_identity = second.0.as_ptr() as usize;
        let (old_id, new_id) = RANGES.with(|ranges| {
            let mut ranges = ranges.borrow_mut();
            let old_id = ranges.by_identity[&old_identity];
            let new_id = ranges.by_identity[&new_identity];
            // Reproduce the interval after the COM allocator reuses an address
            // and before the earlier wrapper's posted removal is dispatched.
            ranges.by_identity.insert(old_identity, new_id);
            (old_id, new_id)
        });
        TextRangeNode::remove(old_id);
        assert!(Rc::ptr_eq(
            &TextRangeNode::target(old_identity).unwrap(),
            &TextRangeNode::range(new_id).unwrap()
        ));
        assert!(TextRangeNode::range(old_id).is_err());
        RANGES.with(|ranges| {
            ranges.borrow_mut().by_identity.remove(&old_identity);
        });
        // Both wrappers can still be released safely, including removal of an
        // id whose queued cleanup already ran.
        drop(first);
        drop(second);
    }
}

impl TextRangeNode {
    /// Called on the UI thread. The registry holds the portable range and an
    /// integer COM identity, but does not hold a reference to its native wrapper.
    pub(crate) fn new(owner: i32, range: Rc<dyn portable::ITextRangeProvider>) -> SendTextRange {
        let dispatcher = Dispatcher::ui_thread();
        dispatcher.verify_access();
        let id = NEXT_RANGE_ID.fetch_add(1, Ordering::Relaxed);
        let node: ComPtr<ITextRangeProvider> = make_com(Self {
            id,
            owner,
            dispatcher,
        });
        // This class has one interface, so it is also its canonical IUnknown.
        let identity = node.as_ptr() as usize;
        RANGES.with(|ranges| {
            let mut ranges = ranges.borrow_mut();
            ranges.by_identity.insert(identity, id);
            ranges.by_id.insert(id, RangeEntry { identity, range });
        });
        SendTextRange(node)
    }

    fn range(id: u64) -> Result<Rc<dyn portable::ITextRangeProvider>, HResult> {
        RANGES
            .with(|ranges| {
                ranges
                    .borrow()
                    .by_id
                    .get(&id)
                    .map(|entry| entry.range.clone())
            })
            .ok_or(ELEMENT_NOT_AVAILABLE)
    }

    fn target(identity: usize) -> Result<Rc<dyn portable::ITextRangeProvider>, HResult> {
        RANGES
            .with(|ranges| {
                let ranges = ranges.borrow();
                ranges
                    .by_identity
                    .get(&identity)
                    .and_then(|id| ranges.by_id.get(id))
                    .map(|entry| entry.range.clone())
            })
            .ok_or(HResult(E_INVALIDARG))
    }

    fn remove(id: u64) {
        let removed = RANGES.with(|ranges| {
            let mut ranges = ranges.borrow_mut();
            let entry = ranges.by_id.remove(&id)?;
            // Final native release on another thread queues this removal. A
            // new COM wrapper may reuse the released address before the UI
            // thread runs it; that newer binding belongs to the newer id.
            if ranges.by_identity.get(&entry.identity) == Some(&id) {
                ranges.by_identity.remove(&entry.identity);
            }
            Some(entry)
        });
        // A range can own subscriptions. Dispose outside the registry borrow so
        // callbacks and custom range destructors can reenter safely.
        drop(removed);
    }

    #[cfg(test)]
    pub(crate) fn count() -> usize {
        RANGES.with(|ranges| ranges.borrow().by_id.len())
    }

    fn invoke<R: Send + 'static>(
        &self,
        call: impl FnOnce(&Rc<dyn portable::ITextRangeProvider>) -> Result<R, HResult> + Send + 'static,
    ) -> Result<R, HResult> {
        let id = self.id;
        let owner = self.owner;
        let run = move || {
            catch_unwind(AssertUnwindSafe(|| {
                AutomationNode::peer_of(owner).ok_or(ELEMENT_NOT_AVAILABLE)?;
                let range = Self::range(id)?;
                // Validate availability even for a custom provider whose methods
                // only inspect their endpoint values.
                range.peer().map_err(provider_error)?;
                call(&range)
            }))
            .map_err(|_| HResult(E_FAIL))?
        };
        if self.dispatcher.check_access() {
            run()
        } else {
            self.dispatcher
                .invoke_async(run)
                .result()
                .map_err(|_| HResult(E_ABORT))?
        }
    }
}

impl Drop for TextRangeNode {
    fn drop(&mut self) {
        let id = self.id;
        if self.dispatcher.check_access() {
            Self::remove(id);
        } else {
            self.dispatcher
                .post(move || Self::remove(id), DispatcherPriority::SEND);
        }
    }
}

impl ITextRangeProviderImpl for TextRangeNode {
    fn clone(&self) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let owner = self.owner;
        let range = self.invoke(move |range| {
            range
                .clone_range()
                .map(|range| Self::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn compare(&self, range: Option<&ITextRangeProvider>) -> Result<i32, HResult> {
        let identity = identity_of(range.ok_or(HResult(E_INVALIDARG))?)?;
        let key = identity.as_ptr() as usize;
        self.invoke(move |range| {
            range
                .compare(&*Self::target(key)?)
                .map(i32::from)
                .map_err(provider_error)
        })
    }

    fn compare_endpoints(
        &self,
        source_endpoint: TextPatternRangeEndpoint,
        target_range: Option<&ITextRangeProvider>,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<i32, HResult> {
        let source_endpoint = endpoint(source_endpoint)?;
        let target_endpoint = endpoint(target_endpoint)?;
        let identity = identity_of(target_range.ok_or(HResult(E_INVALIDARG))?)?;
        let key = identity.as_ptr() as usize;
        self.invoke(move |range| {
            range
                .compare_endpoints(source_endpoint, &*Self::target(key)?, target_endpoint)
                .map_err(provider_error)
        })
    }

    fn expand_to_enclosing_unit(&self, value: TextUnit) -> Result<(), HResult> {
        let value = unit(value)?;
        self.invoke(move |range| {
            range
                .expand_to_enclosing_unit(value)
                .map_err(provider_error)
        })
    }

    fn find_attribute(
        &self,
        id: i32,
        value: ComVariant,
        backward: i32,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let id = attribute(id);
        // SAFETY: the COM caller supplies a live borrowed VARIANT. No ownership
        // of its string/interface/array is transferred to this input parameter.
        let value = unsafe { attribute_from_variant(id, &value) }?;
        let owner = self.owner;
        let found = self.invoke(move |range| {
            range
                .find_attribute(id, &value, backward != 0)
                .map(|found| found.map(|range| Self::new(owner, range)))
                .map_err(provider_error)
        })?;
        Ok(found.map(SendTextRange::into_inner))
    }

    fn find_text(
        &self,
        text: *mut u16,
        backward: i32,
        ignore_case: i32,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        // SAFETY: `text` is a borrowed BSTR according to the COM contract. Its
        // length prefix, including embedded null characters, is honored.
        let text = unsafe { ptr_to_string_bstr(text) }.ok_or(HResult(E_INVALIDARG))?;
        if text.is_empty() {
            return Err(HResult(E_INVALIDARG));
        }
        let owner = self.owner;
        let found = self.invoke(move |range| {
            range
                .find_text(&text, backward != 0, ignore_case != 0)
                .map(|found| found.map(|range| Self::new(owner, range)))
                .map_err(provider_error)
        })?;
        Ok(found.map(SendTextRange::into_inner))
    }

    fn get_attribute_value(&self, id: i32) -> Result<ComVariant, HResult> {
        attribute_to_variant(self.invoke(move |range| {
            range
                .get_attribute_value(attribute(id))
                .map_err(provider_error)
        })?)
    }

    fn get_bounding_rectangles(&self) -> Result<SafeArrayRef, HResult> {
        let values = self.invoke(|range| {
            let peer = range.peer().map_err(provider_error)?;
            let rectangles = range.get_bounding_rectangles().map_err(provider_error)?;
            let mut result = Vec::with_capacity(rectangles.len().saturating_mul(4));
            for rectangle in rectangles {
                let screen = super::automation_node_text::to_screen(&peer, rectangle)?;
                if ![screen.x, screen.y, screen.width, screen.height]
                    .iter()
                    .all(|x| x.is_finite())
                {
                    return Err(HResult(E_FAIL));
                }
                result.extend_from_slice(&[screen.x, screen.y, screen.width, screen.height]);
            }
            Ok(result)
        })?;
        SafeArrayMarshaller::convert_to_unmanaged(Some(&values)).map_err(|_| HResult(E_FAIL))
    }

    fn get_enclosing_element(&self) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        let node = self.invoke(|range| {
            let peer = range.get_enclosing_element().map_err(provider_error)?;
            AutomationNode::get_or_create(Some(&peer))
                .map(SendProvider)
                .ok_or(ELEMENT_NOT_AVAILABLE)
        })?;
        Ok(Some(node.into_inner()))
    }

    fn get_text(&self, max_length: i32) -> Result<*mut u16, HResult> {
        let max_length = match max_length {
            -1 => None,
            0.. => Some(max_length as usize),
            _ => return Err(HResult(E_INVALIDARG)),
        };
        let text = self.invoke(move |range| range.get_text(max_length).map_err(provider_error))?;
        let result = string_to_bstr(&text);
        if result.is_null() {
            Err(HResult(0x8007_000E))
        } else {
            Ok(result)
        }
    }

    fn move_(&self, value: TextUnit, count: i32) -> Result<i32, HResult> {
        let value = unit(value)?;
        self.invoke(move |range| range.move_by(value, count).map_err(provider_error))
    }

    fn move_endpoint_by_unit(
        &self,
        value: TextPatternRangeEndpoint,
        by: TextUnit,
        count: i32,
    ) -> Result<i32, HResult> {
        let value = endpoint(value)?;
        let by = unit(by)?;
        self.invoke(move |range| {
            range
                .move_endpoint_by_unit(value, by, count)
                .map_err(provider_error)
        })
    }

    fn move_endpoint_by_range(
        &self,
        source_endpoint: TextPatternRangeEndpoint,
        target_range: Option<&ITextRangeProvider>,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<(), HResult> {
        let source_endpoint = endpoint(source_endpoint)?;
        let target_endpoint = endpoint(target_endpoint)?;
        let identity = identity_of(target_range.ok_or(HResult(E_INVALIDARG))?)?;
        let key = identity.as_ptr() as usize;
        self.invoke(move |range| {
            range
                .move_endpoint_by_range(source_endpoint, &*Self::target(key)?, target_endpoint)
                .map_err(provider_error)
        })
    }

    fn select(&self) -> Result<(), HResult> {
        self.invoke(|range| range.select().map_err(provider_error))
    }

    fn add_to_selection(&self) -> Result<(), HResult> {
        self.invoke(|range| range.add_to_selection().map_err(provider_error))
    }

    fn remove_from_selection(&self) -> Result<(), HResult> {
        self.invoke(|range| range.remove_from_selection().map_err(provider_error))
    }

    fn scroll_into_view(&self, align_to_top: i32) -> Result<(), HResult> {
        self.invoke(move |range| {
            range
                .scroll_into_view(align_to_top != 0)
                .map_err(provider_error)
        })
    }

    fn get_children(&self) -> Result<SafeArrayRef, HResult> {
        let children = self.invoke(|range| {
            range
                .get_children()
                .map_err(provider_error)?
                .iter()
                .map(|peer| {
                    AutomationNode::get_or_create(Some(peer))
                        .map(SendProvider)
                        .ok_or(ELEMENT_NOT_AVAILABLE)
                })
                .collect::<Result<Vec<_>, _>>()
        })?;
        let children: Vec<_> = children.into_iter().map(SendProvider::into_inner).collect();
        SafeArrayMarshaller::convert_to_unmanaged(Some(&children)).map_err(|_| HResult(E_FAIL))
    }
}
