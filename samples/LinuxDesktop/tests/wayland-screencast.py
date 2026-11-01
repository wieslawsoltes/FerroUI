#!/usr/bin/python3
"""Capture a frame through the public ScreenCast portal on the isolated test bus."""
import os
import shutil
import subprocess
import sys
from gi.repository import Gio, GLib

DEST = "org.freedesktop.portal.Desktop"
PATH = "/org/freedesktop/portal/desktop"
IFACE = "org.freedesktop.portal.ScreenCast"


def main(directory, stream):
    if not os.environ.get("FERRO_TEST_PRIVATE_PORTAL"):
        raise RuntimeError("Run from tests/wayland-session.sh in its private session")
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)

    def request(method, parameters, interface=IFACE):
        request_path = None
        loop = GLib.MainLoop()
        received = {}

        def response(_bus, _sender, path, _iface, _name, args):
            received[path] = args.unpack()
            if path == request_path:
                loop.quit()

        sub = bus.signal_subscribe(DEST, "org.freedesktop.portal.Request", "Response",
                                   None, None, Gio.DBusSignalFlags.NONE, response)
        try:
            request_path, = bus.call_sync(DEST, PATH, interface, method, parameters,
                GLib.VariantType.new("(o)"), Gio.DBusCallFlags.NONE, 8000, None).unpack()
            if request_path not in received:
                def expired():
                    loop.quit()
                    return GLib.SOURCE_REMOVE
                clock = GLib.timeout_add_seconds(12, expired)
                loop.run()
                if request_path in received:
                    GLib.source_remove(clock)
            if request_path not in received:
                raise TimeoutError(method)
            code, result = received[request_path]
            if code:
                raise RuntimeError(f"{method} returned {code}: {result}")
            return result
        finally:
            bus.signal_unsubscribe(sub)

    v = GLib.Variant
    # Grant only this unconfined test identity in the throwaway permission store.
    # A real login inherits the user's normal Screenshot permission prompts.
    bus.call_sync("org.freedesktop.impl.portal.PermissionStore",
        "/org/freedesktop/impl/portal/PermissionStore", "org.freedesktop.impl.portal.PermissionStore",
        "SetPermission", v("(sbssas)", ("screenshot", True, "screenshot", "", ["yes"])),
        None, Gio.DBusCallFlags.NONE, 4000, None)
    screenshot = request("Screenshot", v("(sa{sv})", ("", {
        "handle_token": v("s", "ferro_still"), "interactive": v("b", False)})),
        "org.freedesktop.portal.Screenshot")
    shutil.copyfile(Gio.File.new_for_uri(screenshot["uri"]).get_path(),
                    os.path.join(directory, "portal-screenshot.png"))
    created = request("CreateSession", v("(a{sv})", ({
        "handle_token": v("s", "ferro_create"), "session_handle_token": v("s", "ferro_capture")},)))
    session = created["session_handle"]
    try:
        request("SelectSources", v("(oa{sv})", (session, {
            "handle_token": v("s", "ferro_select"), "types": v("u", 1), "multiple": v("b", False)})))
        if not stream:
            print("Verified Screenshot delivery and ScreenCast session/source negotiation.")
            return
        selected = request("Start", v("(osa{sv})", (session, "", {"handle_token": v("s", "ferro_start")})))
        streams = selected["streams"]
        if len(streams) != 1:
            raise RuntimeError(f"Expected one monitor, got {streams}")
        result, fds = bus.call_with_unix_fd_list_sync(DEST, PATH, IFACE, "OpenPipeWireRemote",
            v("(oa{sv})", (session, {})), GLib.VariantType.new("(h)"),
            Gio.DBusCallFlags.NONE, 8000, None, None)
        node = streams[0][0]
        fd = fds.get(result.unpack()[0])
        try:
            subprocess.run(["gst-launch-1.0", "-q", "pipewiresrc", f"fd={fd}",
                f"path={node}", "num-buffers=1", "!", "videoconvert", "!", "pngenc",
                "!", "filesink", "location=" + os.path.join(directory, "portal-capture.png")],
                pass_fds=(fd,), timeout=18, check=True)
        finally:
            os.close(fd)
        print(f"The public ScreenCast portal produced a frame from PipeWire node {node}.")
    finally:
        try:
            bus.call_sync(DEST, session, "org.freedesktop.portal.Session", "Close",
                          None, None, Gio.DBusCallFlags.NONE, 4000, None)
        except GLib.Error as error:
            if Gio.DBusError.get_remote_error(error) != "org.freedesktop.DBus.Error.UnknownMethod":
                raise


if __name__ == "__main__":
    main(sys.argv[1], "--stream" in sys.argv[2:])
