#!/usr/bin/env python3
"""Generates the element builders of the declarative Rust API (`ferroui-view`).

For every class of the object model of FerroUI.Base and FerroUI.Controls the
script writes, into `src/FerroUI.View/generated/builders.rs`:

    - a function that starts an element of the class (`button()`), for a class
      that can be created (it declares `new` in `ferro_class_info!`);
    - a trait of the class (`ButtonProps`) with one setter per registered
      property (`click_mode(...)`) and one handler per routed event
      (`on_click(...)`), implemented for the element of the class and of every
      class derived from it;
    - one trait (`AttachedProps`) with a setter per attached property, named
      after its owner (`grid_row(...)`, `dock_panel_dock(...)`).

The control crates beside those two get their builders in a file of their
own, behind a feature of `ferroui-view` (`GROUPS` below): the TreeDataGrid,
the colour picker, the dialogs and the ReactiveUI controls. Their classes
derive from the classes of FerroUI.Controls, so their elements also have the
setters of their bases.

Everything comes from the Rust sources (the walk is `ferroui_metadata.py`, which
the API model of the bindings shares), so a property ported later gets its
setter by running the script again:

    class and base       `ferro_class! { Name: Base, ... }`
    can be created       `ferro_class_info!(Name { new: ...`
    properties           `pub fn x_property() -> StyledProperty<T>` (and
                         `DirectProperty<Owner, T>`, `AttachedProperty<T>`)
                         in the `impl` of the owner
    routed events        `pub fn x_event() -> RoutedEvent<Args>`
    paths of types       the `rust_paths.rs` of the two crates

Two public types with the same simple name in one crate (the upstream
`ferroui_controls::NavigationEventArgs` of the pages and WinUI's
`ferroui_controls::navigation::NavigationEventArgs` of `Frame`) are told apart
by `SCOPED_PATHS`: the script gives the second a name of its own in its model
(`FrameNavigationEventArgs`) wherever a source declares or imports that path,
and the generated code re-exports it under that name and names it by its
public path. The tests are `scripts/test_generate_view_builders.py`.

A property is left out (and named with --report) when its value type names a
type the script cannot give a public path to, when it is a direct property
without a setter, or when its setter would hide a method of the element.

How to run (from the repository root; Python 3, standard library only):

    python3 scripts/generate_view_builders.py            # writes the file
    python3 scripts/generate_view_builders.py --check    # exit 1 if stale
    python3 scripts/generate_view_builders.py --report   # also list what was left out
"""

import argparse
import os
import sys

from ferroui_metadata import (  # noqa: F401 (the tests of the script use these names)
    CRATES, EXTERNAL_TYPES, PRELUDE_TYPES, ROOT, RUST_KEYWORDS, SCOPED_PATHS, STD_TYPES, Model,
    ancestors, class_path, generic_argument, ident, matching, owner_at, qualify_scoped_names, read,
    read_classes, read_members, read_paths, resolve, rust_files, snake, split_top_level,
    strip_comments,
)

OUTPUT = os.path.join(ROOT, "src", "FerroUI.View", "generated", "builders.rs")

# The other crates of controls: (feature of ferroui-view, output file, the
# crates read for the group). Each file has the classes of its crates only;
# the classes of FerroUI.Base and FerroUI.Controls are read for the bases and
# the paths of types.
# The types of a group that its `rust_paths.rs` does not list (they are not
# registered for markup) are given here: name -> public path.
GROUPS = [
    ("treedatagrid", "treedatagrid.rs", [
        ("src/TreeDataGrid/FerroUI.Controls.TreeDataGrid", "ferroui_controls_treedatagrid"),
    ], {
        "TreeDataGridSourceRef": "ferroui_controls_treedatagrid::TreeDataGridSourceRef",
        "IColumns": "ferroui_controls_treedatagrid::models::IColumns",
        "ITreeDataGridRows": "ferroui_controls_treedatagrid::models::ITreeDataGridRows",
        "TreeDataGridElementFactory": "ferroui_controls_treedatagrid::primitives::TreeDataGridElementFactory",
        "ITreeDataGridPresentationOptions": "ferroui_controls_treedatagrid::presentation::ITreeDataGridPresentationOptions",
    }),
    ("color-picker", "color_picker.rs", [
        ("src/FerroUI.Controls.ColorPicker", "ferroui_controls_color_picker"),
    ], {}),
    ("dialogs", "dialogs.rs", [
        ("src/FerroUI.Dialogs", "ferroui_dialogs"),
    ], {}),
    ("inking", "inking.rs", [
        ("src/FerroUI.Controls.Inking", "ferroui_controls_inking"),
    ], {}),
    ("repeater", "repeater.rs", [
        ("src/FerroUI.Controls.Repeater", "ferroui_controls_repeater"),
    ], {}),
    ("reactiveui", "reactiveui.rs", [
        ("src/FerroUI.ReactiveUI.Controls", "ferroui_reactiveui_controls"),
    ], {
        "RoutingState": "ferroui_reactiveui::routing::RoutingState",
    }),
]


# Methods of the element that a generated setter must not hide; the element's
# own method does what the property would (or the property is set with
# `set(...)`).
ELEMENT_METHODS = {
    "child", "children", "key", "set", "bind", "on", "node_ref", "class", "classes",
    "name", "into_element", "build", "prop", "routed", "with", "on_changed", "mount",
    "content_element", "dyn_children", "items_dyn",
}

def generate(model, crates=None, title="FerroUI.Base and FerroUI.Controls", attached_trait="AttachedProps"):
    """The file of the classes of `crates` (all the classes read when None)."""
    lines = [
        "// @generated by scripts/generate_view_builders.py — do not edit",
        f"//! The element builders of the classes of {title}:",
        "//! a function that starts an element of each class that can be created, and a",
        "//! trait per class with a setter for each of its properties and a handler for",
        "//! each of its routed events, and the classes themselves (the names of the",
        "//! elements of `rsx!`). See `scripts/generate_view_builders.py`.",
        "",
        "#![allow(clippy::too_many_arguments, missing_docs)]",
        "",
        "use crate::builder::{El, ElementBuilder, IntoProp, ViewClass};",
        "use ferroui_base::{ObjectType, Upcast};",
        "",
    ]
    def ours(name):
        return crates is None or model.crate_of.get(name) in crates
    classes = sorted(c for c in model.bases if class_path(model, c) and ours(c))
    defined = {}  # class -> set of method names it or an ancestor defines

    def names_of(name):
        if name in defined:
            return defined[name]
        inherited = set()
        base = model.bases.get(name)
        if base:
            inherited = set(names_of(base))
        own = set()
        for kind, prop, value_type, target, holder in model.properties.get(name, []):
            own.add(prop)
        for event, _ in model.events.get(name, []):
            own.add(f"on_{event}")
        defined[name] = inherited | own
        return defined[name]

    functions, traits = [], []
    used_functions = set()
    for name in classes:
        path = class_path(model, name)
        is_control = "Control" in ancestors(model, name) or name == "Control"
        if name in model.creatable and is_control:
            function = ident(snake(name))
            if function not in used_functions:
                used_functions.add(function)
                alias = f" as {name}" if path.split("::")[-1] != name else ""
                functions.append(f"pub use {path}{alias};\n")
                functions.append(f"impl ViewClass for {path} {{\n    fn create() -> ferroui_base::Ref<Self> {{\n        <{path}>::new()\n    }}\n}}\n")
                functions.append(f"/// Starts an element of [`{name}`]({path}).\npub fn {function}() -> El<{path}> {{\n    El::new()\n}}\n")
        inherited = set(names_of(model.bases[name])) if model.bases.get(name) else set()
        members = []
        for kind, prop, value_type, target, holder in model.properties.get(name, []):
            # An attached property of the class is also set on the class
            # itself, as markup does (`ScrollViewer.HorizontalScrollBarVisibility`).
            if kind == "AttachedProperty" and (prop in inherited or prop in ELEMENT_METHODS):
                continue
            holder_path = class_path(model, holder)
            if holder_path is None:
                model.left_out.append(f"{name}.{prop}: no public path for `{holder}`, whose impl declares it")
                continue
            if prop in ELEMENT_METHODS:
                model.left_out.append(f"{name}.{prop}: the element has a method of that name")
                continue
            if prop in inherited:
                continue
            resolved = resolve(model, value_type)
            if resolved is None:
                model.left_out.append(f"{name}.{prop}: no public path for `{value_type}`")
                continue
            members.append(
                f"    /// Sets [`{holder}::{prop}_property`]({holder_path}::{prop}_property).\n"
                f"    fn {ident(prop)}<M>(self, value: impl IntoProp<{resolved}, M>) -> Self {{\n"
                f"        self.prop(<{holder_path}>::{prop}_property(), value)\n    }}\n"
            )
        for event, args in model.events.get(name, []):
            method = f"on_{event}"
            if method in inherited:
                continue
            resolved = resolve(model, args)
            if resolved is None:
                model.left_out.append(f"{name}.{event}: no public path for `{args}`")
                continue
            members.append(
                f"    /// Handles [`{name}::{event}_event`]({path}::{event}_event).\n"
                f"    fn {ident(method)}(self, handler: impl Fn(&{resolved}) + 'static) -> Self {{\n"
                f"        self.routed(<{path}>::{event}_event(), move |_, e| handler(e))\n    }}\n"
            )
        if members:
            trait = f"{name}Props"
            traits.append(
                f"/// The properties and routed events of [`{name}`]({path}).\n"
                f"pub trait {trait}: ElementBuilder {{\n" + "\n".join(members) + "}\n"
                f"impl<T: ObjectType + Upcast<{path}>> {trait} for El<T> {{}}\n"
            )

    attached = []
    for owner in sorted(model.properties):
        owner_path = class_path(model, owner)
        if owner_path is None or not ours(owner):
            continue
        for kind, prop, value_type, target, holder in model.properties[owner]:
            if kind != "AttachedProperty":
                continue
            holder_path = class_path(model, holder) or owner_path
            resolved = resolve(model, value_type)
            if resolved is None:
                model.left_out.append(f"{owner}.{prop} (attached): no public path for `{value_type}`")
                continue
            method = f"{snake(owner)}_{prop}"
            attached.append(
                f"    /// Sets the attached [`{holder}::{prop}_property`]({holder_path}::{prop}_property).\n"
                f"    fn {ident(method)}<M>(self, value: impl IntoProp<{resolved}, M>) -> Self {{\n"
                f"        self.prop(<{holder_path}>::{prop}_property(), value)\n    }}\n"
            )
    lines += functions
    lines += traits
    if attached or crates is None:
        lines.append(f"/// The attached properties of {title}, named after their owner.")
        lines.append(f"pub trait {attached_trait}: ElementBuilder {{")
        lines.append("\n".join(attached) + "}")
        lines.append(f"impl<T: ObjectType> {attached_trait} for El<T> {{}}")
    lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--report", action="store_true")
    options = parser.parse_args()

    outputs = []
    model = Model()
    for directory, crate in CRATES:
        read_paths(model, directory, crate)
    for directory, crate in CRATES:
        read_classes(model, directory, crate)
    outputs.append((OUTPUT, generate(model), model))

    for feature, file_name, group, extra_paths in GROUPS:
        group_model = Model()
        for directory, crate in CRATES + group:
            read_paths(group_model, directory, crate)
        for name, path in extra_paths.items():
            group_model.add_path(name, path)
        for directory, crate in CRATES + group:
            read_classes(group_model, directory, crate)
        # What the base file left out is reported there.
        group_model.left_out = []
        crates = {crate for _, crate in group}
        title = ", ".join(directory.split("/")[-1] for directory, _ in group) + f" (feature `{feature}`)"
        trait = "".join(part.capitalize() for part in feature.split("-")) + "AttachedProps"
        text = generate(group_model, crates, title, trait)
        outputs.append((os.path.join(os.path.dirname(OUTPUT), file_name), text, group_model))

    if options.report:
        lines = sorted({line for _, _, m in outputs for line in m.left_out})
        for line in lines:
            print(line)
        print(f"{len(lines)} left out", file=sys.stderr)

    stale = False
    for path, text, _ in outputs:
        current = read(path) if os.path.exists(path) else None
        if current == text:
            continue
        if options.check:
            print(f"stale: {os.path.relpath(path, ROOT)}")
            stale = True
            continue
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(f"wrote {os.path.relpath(path, ROOT)}")
    if stale:
        sys.exit(1)


if __name__ == "__main__":
    main()
