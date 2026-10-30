#!/usr/bin/env python3
"""Generate typed factory override/forwarding declarations from the native contracts.

This generates dispatch only, never docking algorithms. Source nonvirtual members
are explicitly forwarded to FactoryBase; their list mirrors the frozen C# partials.
The compiler checks every generated signature against the complete contracts.
"""
from pathlib import Path
import argparse
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
TARGET = ROOT / 'src/Dock/FerroDock.Model.ReactiveUI/factory_impl.rs'
FIXED = {
    'global_dock_tracking_state', 'current_dockable', 'current_root_dock',
    'current_dock_window', 'current_host_window', 'activate_window',
    'find', 'find_in_dock', 'is_dockable_pinned', 'unpin_dockable',
    'preview_pinned_dockable', 'toggle_preview_pinned_dockable',
    'hide_previewing_dockables', 'hide_dockable_by_id', 'restore_dockable_by_id',
    'set_document_dock_tabs_layout_left', 'set_document_dock_tabs_layout_top',
    'set_document_dock_tabs_layout_right', 'set_document_dock_layout_mode_tabbed',
    'set_document_dock_layout_mode_mdi',
}
OWN = {
    'visible_dockable_controls', 'visible_root_controls', 'pinned_dockable_controls',
    'pinned_root_controls', 'tab_dockable_controls', 'tab_root_controls',
    'tool_controls', 'document_controls', 'dock_controls', 'host_windows',
    'create_list', 'create_root_dock', 'create_proportional_dock', 'create_dock_dock',
    'create_stack_dock', 'create_grid_dock', 'create_wrap_dock', 'create_uniform_grid_dock',
    'create_proportional_dock_splitter', 'create_grid_dock_splitter', 'create_tool_dock',
    'create_document_dock', 'create_split_view_dock', 'create_dock_window',
    'create_document', 'create_tool', 'create_layout',
}


def methods(filename):
    """Read required trait signatures, excluding the contract's default adapters."""
    source = (ROOT / 'src/Dock/FerroDock.Model/core' / filename).read_text()
    result = []
    for match in re.finditer(r'^    fn (\w+)', source, re.M):
        start = source.index('(', match.end())
        depth, end = 1, start + 1
        while depth:
            if source[end] == '(': depth += 1
            elif source[end] == ')': depth -= 1
            end += 1
        tail = re.search(r'[;{]', source[end:])
        stop = end + tail.start()
        if source[stop] != ';': continue
        name = match.group(1)
        generic = source[match.end():start].strip()
        args = source[start + 1:end - 1].strip()
        assert args.startswith('&self'), name
        args = args[len('&self'):].strip().lstrip(',').strip().rstrip(',')
        names = re.findall(r'(?:^|,)\s*(\w+)\s*:', args)
        returns = source[end:stop].strip().split('where')[0].strip()
        result.append((name, generic, args, names, returns))
    return result


def qualified(signature):
    standard = {'Self', 'String', 'Vec', 'Option', 'Clone', 'PartialEq', 'T', 'Sized', 'Fn'}
    return re.sub(r'\b[A-Z]\w*\b', lambda m: m[0] if m[0] in standard else '$crate::__private::' + m[0], signature.replace('ferrodock_model::', ''))


def validate_dispatch(factory, events):
    """Reject dispatch that disagrees with the frozen C# virtual/nonvirtual surface."""
    paths = list((ROOT.parent / 'Dock/src/Dock.Model').glob('FactoryBase*.cs'))
    paths.append(ROOT.parent / 'Dock/src/Dock.Model.ReactiveUI/Factory.cs')
    virtual = set()
    reactive_overrides = set()
    for path in paths:
        declarations = set(re.findall(r'public\s+(?:virtual|abstract|override)\s+[^\n{=;]+?\s+(\w+)(?:<[^>]+>)?\s*(?=[({])',path.read_text()))
        virtual.update(declarations)
        if path.name == 'Factory.cs': reactive_overrides.update(declarations)
    properties = {'hide_tools_on_close','hide_documents_on_close','default_context_locator',
                  'default_host_window_locator','context_locator','host_window_locator','dockable_locator'}
    aliases = {'find_in_dock':'find','init_dock_window_with_host':'init_dock_window',
               'move_dockable_between_docks':'move_dockable','swap_dockable_between_docks':'swap_dockable'}
    for contract, members in [('factory',factory),('events',events)]:
        for name, *_ in members:
            native = name[4:] if name.startswith('set_') and name[4:] in properties else name
            native = aliases.get(native,native.removesuffix('_with_options').removesuffix('_with_root'))
            source = ''.join(part[:1].upper()+part[1:] for part in native.split('_'))
            expected_fixed = source not in virtual or name in {'hide_dockable_by_id','restore_dockable_by_id'}
            actual_fixed = name in FIXED or contract=='events' and not name.startswith('on_')
            if expected_fixed != actual_fixed:
                raise ValueError(f'{name}: dispatch disagrees with C# source member {source}')
            if (name in OWN) != (source in reactive_overrides):
                raise ValueError(f'{name}: base-body target disagrees with ReactiveUI Factory override {source}')


def render():
    factory = [method for method in methods('i_factory.rs') if method[0] != 'create_dockable_list']
    events = methods('i_factory_events.rs')
    validate_dispatch(factory,events)
    text = '''// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
// Generated by scripts/dock/generate_factory.py. Dispatch declarations only.

use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::{BoxedValue, FactoryBaseImpl};
use std::rc::Rc;

/// Typed virtual overrides for a composed ReactiveUI Factory subclass.
/// Implement factory(), FactoryBaseImpl, and invoke impl_factory!(Type).
/// A base-method call uses factory().base() for inherited engine bodies and
/// factory() for the concrete ReactiveUI creation/registry bodies.
pub trait FactoryImpl: FactoryBaseImpl {
    /// Gets the concrete factory component for this enclosing runtime.
    fn factory(&self) -> &crate::Factory;
    /// Resolves optional item dragging/reordering implemented by this runtime.
    fn as_dock_item_drag_policy(_this: Rc<Self>) -> Option<Rc<dyn IDockItemDragPolicy>> where Self: Sized { None }
    /// Resolves optional tool split policy implemented by this runtime.
    fn as_dock_split_policy(_this: Rc<Self>) -> Option<Rc<dyn IDockSplitPolicy>> where Self: Sized { None }
    /// Resolves optional drop policy implemented by this runtime.
    fn as_dock_drop_policy(_this: Rc<Self>) -> Option<Rc<dyn IDockDropPolicy>> where Self: Sized { None }
    /// Resolves optional outer split policy implemented by this runtime.
    fn as_dock_outer_split_policy(_this: Rc<Self>) -> Option<Rc<dyn IDockOuterSplitPolicy>> where Self: Sized { None }
    /// Resolves optional operation observer implemented by this runtime.
    fn as_dock_operation_observer(_this: Rc<Self>) -> Option<Rc<dyn IDockOperationObserver>> where Self: Sized { None }
    /// Resolves optional full-window host creation implemented by this runtime.
    fn as_dock_window_host_factory(_this: Rc<Self>) -> Option<Rc<dyn IDockWindowHostFactory>> where Self: Sized { None }
    /// Resolves optional host-close preservation implemented by this runtime.
    fn as_dock_window_close_policy(_this: Rc<Self>) -> Option<Rc<dyn IDockWindowClosePolicy>> where Self: Sized { None }
'''
    for name, generic, args, names, returns in factory + events:
        if name in FIXED or (name, generic, args, names, returns) in events and not name.startswith('on_'):
            continue
        callargs = ', '.join(names)
        signature = f'fn {name}{generic}(this: &Self' + (', ' + args if args else '') + ') ' + returns + ' where Self: Sized'
        target = 'this.factory()' if name in OWN else 'this.factory().base()'
        text += f'    /// Source virtual base body for `{name}`.\n    #[allow(clippy::too_many_arguments)]\n    {signature} {{ {target}.{name}({callargs}) }}\n'
    text += '''}

/// Implements complete factory/event/state-sync contracts through FactoryImpl.
/// Source nonvirtual methods retain direct base dispatch. Public virtual methods
/// use typed overrides, while the engine's recursion uses the enclosing IFactory.
#[macro_export]
macro_rules! impl_factory {
    ($type:ty) => {
        impl PartialEq for $type {
            fn eq(&self, other: &Self) -> bool { std::ptr::eq(self,other) }
        }
        impl $crate::__private::IFactory for $type {
            fn as_factory_base(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::FactoryBaseImpl>> { Some(self) }
            fn create_dockable_list(&self, items: Vec<Option<std::rc::Rc<dyn $crate::__private::IDockable>>>) -> $crate::__private::ListHandle<Option<std::rc::Rc<dyn $crate::__private::IDockable>>> { <$type as $crate::__private::IFactory>::create_list(self,items) }
            fn as_dock_item_drag_policy(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockItemDragPolicy>> { <$type as $crate::FactoryImpl>::as_dock_item_drag_policy(self) }
            fn as_dock_split_policy(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockSplitPolicy>> { <$type as $crate::FactoryImpl>::as_dock_split_policy(self) }
            fn as_dock_drop_policy(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockDropPolicy>> { <$type as $crate::FactoryImpl>::as_dock_drop_policy(self) }
            fn as_dock_outer_split_policy(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockOuterSplitPolicy>> { <$type as $crate::FactoryImpl>::as_dock_outer_split_policy(self) }
            fn as_dock_operation_observer(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockOperationObserver>> { <$type as $crate::FactoryImpl>::as_dock_operation_observer(self) }
            fn as_dock_window_host_factory(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockWindowHostFactory>> {
                <$type as $crate::FactoryImpl>::as_dock_window_host_factory(self)
            }
            fn as_dock_window_close_policy(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockWindowClosePolicy>> {
                <$type as $crate::FactoryImpl>::as_dock_window_close_policy(self)
            }
            fn as_docking_window_state_sync(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn $crate::__private::IDockingWindowStateSync>> { Some(self) }
'''
    for contract, members in [('IFactory', factory), ('IFactoryEvents', events)]:
        if contract == 'IFactoryEvents':
            text += '        }\n        impl $crate::__private::IFactoryEvents for $type {\n'
        for name, generic, args, names, returns in members:
            sig = f'fn {name}{generic}(&self' + (', ' + args if args else '') + ') ' + returns
            if generic: sig += ' where Self: Sized'
            is_fixed = name in FIXED or contract == 'IFactoryEvents' and not name.startswith('on_')
            target = f'<$type as $crate::FactoryImpl>::factory(self).base().{name}' if is_fixed else f'<$type as $crate::FactoryImpl>::{name}'
            callargs = ', '.join(names) if is_fixed else ', '.join(['self'] + names)
            text += f'            #[allow(clippy::too_many_arguments)]\n            {qualified(sig)} {{ {target}({callargs}) }}\n'
    text += '''        }
        impl $crate::__private::IDockingWindowStateSync for $type {
            fn on_docking_window_state_property_changed(&self, dockable: std::rc::Rc<dyn $crate::__private::IDockable>, property: $crate::__private::DockingWindowStateProperty) {
                $crate::__private::FactoryBaseImpl::on_docking_window_state_property_changed(self, dockable, property);
            }
        }
    };
}
'''
    result = subprocess.run(['rustfmt', '--edition', '2021', '--emit', 'stdout'], input=text, text=True, capture_output=True, check=True)
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected = render()
    if args.check:
        if not TARGET.exists() or TARGET.read_text() != expected:
            raise SystemExit('factory_impl.rs differs; run scripts/dock/generate_factory.py')
        print('Factory dispatch declarations match the complete native contracts.')
    else:
        TARGET.write_text(expected)


if __name__ == '__main__': main()
