"""The selection of an application for the C ABI (docs/c-abi.md, section 17; #977): what a
manifest (`ferroui.json`) selects of the API model, shared by the generator of the per-application
library (`scripts/capi/app_library.py`) and the generators of bindings, so that a library and the
bindings of one manifest agree on its fingerprint.

A manifest names classes (`classes`) and, optionally, the groups of calls of the ABI it uses
(`abi`, a subset of `GROUPS`; all of them when it is left out). The core calls (handles, IDs,
values, events, the application and the thread) are always there. It can also list the
`properties` and `events` the application uses, which narrow its generated bindings. A manifest
can be collected from the sources of the application (`scripts/capi/collect_manifest.py`).
"""

# The groups of calls of the ABI beyond the core, by the prefix of their names.
GROUPS = {
    # Frames (docs/c-abi.md, section 12): views described again on every update.
    "frames": ("ferro_frame_", "ferro_subscribe_dispatch"),
    # Signals shared through memory (section 14).
    "signals": ("ferro_signal_", "ferro_signals_"),
    # Controls written in another language (section 15), with their drawing and layout calls.
    "controls": ("ferro_class_", "ferro_property_register", "ferro_draw_", "ferro_layout_", "ferro_object_",
                 "ferro_panel_", "ferro_template_part", "ferro_raise"),
}


def group_of(function):
    """The group of a call of the ABI (`core` for the calls every application has)."""
    for group, names in GROUPS.items():
        # A name that ends with `_` is a prefix, any other is one call.
        if any(function.startswith(name) if name.endswith("_") else function == name for name in names):
            return group
    return "core"


def fnv1a(text):
    """The FNV-1a hash of `text` as a signed 64-bit integer (the fingerprints of the model)."""
    value = 0xCBF29CE484222325
    for byte in text.encode("utf-8"):
        value = ((value ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value - (1 << 64) if value >= (1 << 63) else value


def selected_names(model, manifest, key):
    """The names a manifest lists under `properties` or `events` (`key`), or None when it lists
    none: then the bindings have everything the classes of the selection declare. They narrow the
    generated bindings alone. The names of the model stay in the library, whose IDs are their
    positions (all of them are about 37 kB), so they are not part of the fingerprint: one library
    serves the bindings of a manifest with or without these lists."""
    if not manifest or key not in manifest:
        return None
    known = {entry["name"] for entry in model[key]}
    unknown = sorted(set(manifest[key]) - known)
    if unknown:
        raise SystemExit(f"the manifest names {unknown} under `{key}`, not in the model")
    return set(manifest[key])


class AppSelection:
    """What a manifest selects: the classes it names (with their entries), the groups of calls,
    the exports of a library built for it, and its fingerprint."""

    def __init__(self, model, manifest):
        self.properties = selected_names(model, manifest, "properties")
        self.events = selected_names(model, manifest, "events")
        by_name = {cls["name"]: cls for cls in model["classes"]}
        self.classes = []
        for name in manifest.get("classes", []):
            if name not in by_name:
                raise SystemExit(f"the manifest names {name}, which is no class of the model")
            if name not in self.classes:
                self.classes.append(name)
        self.classes.sort()
        self.entries = [by_name[name]["entry"] for name in self.classes if by_name[name].get("entry")]
        groups = manifest.get("abi", sorted(GROUPS))
        unknown = set(groups) - set(GROUPS)
        if unknown:
            raise SystemExit(f"unknown groups of calls in the manifest: {sorted(unknown)} (known: {sorted(GROUPS)})")
        self.groups = sorted(set(groups))
        self.functions = sorted(function["name"] for function in model["abi"]["functions"]
                                if group_of(function["name"]) in ("core", *self.groups))
        self.exports = sorted(self.functions + self.entries)
        # The model the IDs come from, the classes and the calls: what a binding and a library of
        # the selection have to agree on.
        self.fingerprint = fnv1a("\n".join(["model", str(model["fingerprint"]), "classes", *self.classes,
                                            "groups", *self.groups]))
        if self.fingerprint == 0:
            self.fingerprint = 1  # 0 is the library of every class and call
