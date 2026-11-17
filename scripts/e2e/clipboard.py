#!/usr/bin/env python3
"""System clipboard driver for the MD View E2E run (X11).

Usage:
  DISPLAY=:97 python3 clipboard.py read [timeout]
  DISPLAY=:97 python3 clipboard.py serve TEXT READY_FILE [timeout]

`read` asks the current CLIPBOARD owner for UTF-8 text and prints it. It exits
non-zero when nothing owns the selection, which is what a simulated in-process
clipboard reports to the rest of the desktop.

`serve` takes ownership of CLIPBOARD, writes READY_FILE, and answers the requests
a paste sends. It exits after delivering the text, so the shell can wait for it.
"""
import ctypes
import os
import select
import sys
import time

X11 = ctypes.CDLL("libX11.so.6")

Display = ctypes.c_void_p
Window = ctypes.c_ulong
Atom = ctypes.c_ulong
Time = ctypes.c_ulong

X11.XOpenDisplay.argtypes = [ctypes.c_char_p]
X11.XOpenDisplay.restype = Display
X11.XDefaultRootWindow.argtypes = [Display]
X11.XDefaultRootWindow.restype = Window
X11.XCreateSimpleWindow.argtypes = [
    Display, Window, ctypes.c_int, ctypes.c_int, ctypes.c_uint, ctypes.c_uint,
    ctypes.c_uint, ctypes.c_ulong, ctypes.c_ulong,
]
X11.XCreateSimpleWindow.restype = Window
X11.XInternAtom.argtypes = [Display, ctypes.c_char_p, ctypes.c_int]
X11.XInternAtom.restype = Atom
X11.XSetSelectionOwner.argtypes = [Display, Atom, Window, Time]
X11.XSetSelectionOwner.restype = ctypes.c_int
X11.XGetSelectionOwner.argtypes = [Display, Atom]
X11.XGetSelectionOwner.restype = Window
X11.XConvertSelection.argtypes = [Display, Atom, Atom, Atom, Window, Time]
X11.XConvertSelection.restype = ctypes.c_int
X11.XNextEvent.argtypes = [Display, ctypes.c_void_p]
X11.XNextEvent.restype = ctypes.c_int
X11.XPending.argtypes = [Display]
X11.XPending.restype = ctypes.c_int
X11.XFlush.argtypes = [Display]
X11.XFlush.restype = ctypes.c_int
X11.XSync.argtypes = [Display, ctypes.c_int]
X11.XSync.restype = ctypes.c_int
X11.XConnectionNumber.argtypes = [Display]
X11.XConnectionNumber.restype = ctypes.c_int
X11.XChangeProperty.argtypes = [
    Display, Window, Atom, Atom, ctypes.c_int, ctypes.c_int, ctypes.c_void_p,
    ctypes.c_int,
]
X11.XChangeProperty.restype = ctypes.c_int
X11.XGetWindowProperty.argtypes = [
    Display, Window, Atom, ctypes.c_long, ctypes.c_long, ctypes.c_int, Atom,
    ctypes.POINTER(Atom), ctypes.POINTER(ctypes.c_int),
    ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong),
    ctypes.POINTER(ctypes.c_void_p),
]
X11.XGetWindowProperty.restype = ctypes.c_int
X11.XFree.argtypes = [ctypes.c_void_p]
X11.XFree.restype = ctypes.c_int
X11.XSendEvent.argtypes = [Display, Window, ctypes.c_int, ctypes.c_long, ctypes.c_void_p]
X11.XSendEvent.restype = ctypes.c_int

SELECTION_NOTIFY = 31
SELECTION_REQUEST = 30
SELECTION_CLEAR = 29
ANY_PROPERTY_TYPE = 0
PROP_MODE_REPLACE = 0


class SelectionRequest(ctypes.Structure):
    _fields_ = [
        ("type", ctypes.c_int),
        ("serial", ctypes.c_ulong),
        ("send_event", ctypes.c_int),
        ("display", Display),
        ("owner", Window),
        ("requestor", Window),
        ("selection", Atom),
        ("target", Atom),
        ("property", Atom),
        ("time", Time),
    ]


class SelectionEvent(ctypes.Structure):
    _fields_ = [
        ("type", ctypes.c_int),
        ("serial", ctypes.c_ulong),
        ("send_event", ctypes.c_int),
        ("display", Display),
        ("requestor", Window),
        ("selection", Atom),
        ("target", Atom),
        ("property", Atom),
        ("time", Time),
    ]


EVENT_BYTES = 24 * ctypes.sizeof(ctypes.c_long)


def open_display():
    display = X11.XOpenDisplay(os.environ.get("DISPLAY", ":97").encode())
    if not display:
        raise SystemExit("cannot open display")
    return display


def atom(display, name):
    return X11.XInternAtom(display, name.encode(), 0)


def fail(message):
    print(f"clipboard: {message}", file=sys.stderr)
    raise SystemExit(1)


def wait_for_event(display, timeout):
    connection = X11.XConnectionNumber(display)
    ready, _, _ = select.select([connection], [], [], timeout)
    return bool(ready)


def new_window(display):
    root = X11.XDefaultRootWindow(display)
    return X11.XCreateSimpleWindow(display, root, -10, -10, 1, 1, 0, 0, 0)


def serve(text, ready_path, timeout):
    display = open_display()
    window = new_window(display)
    clipboard = atom(display, "CLIPBOARD")
    targets = atom(display, "TARGETS")
    utf8 = atom(display, "UTF8_STRING")
    text_atom = atom(display, "TEXT")
    string = atom(display, "STRING")
    mime = atom(display, "text/plain;charset=utf-8")
    mime_plain = atom(display, "text/plain")
    offered = (targets, utf8, text_atom, string, mime, mime_plain)
    string_targets = {utf8, text_atom, string, mime, mime_plain}
    payload = text.encode()
    xa_atom = atom(display, "ATOM")

    X11.XSetSelectionOwner(display, clipboard, window, 0)
    X11.XSync(display, 0)
    if X11.XGetSelectionOwner(display, clipboard) != window:
        fail("could not take ownership of CLIPBOARD")
    with open(ready_path, "w", encoding="utf-8"):
        pass

    event_buffer = ctypes.create_string_buffer(EVENT_BYTES)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not X11.XPending(display):
            wait_for_event(display, min(0.1, deadline - time.monotonic()))
            continue
        if X11.XNextEvent(display, event_buffer):
            continue
        event = ctypes.cast(
            event_buffer, ctypes.POINTER(SelectionRequest)
        ).contents
        if event.type == SELECTION_CLEAR:
            fail("lost CLIPBOARD ownership before the paste")
        if event.type != SELECTION_REQUEST:
            continue
        target = event.target
        prop = event.property or target
        delivered = False
        if target == targets:
            atoms = (Atom * len(offered))(*offered)
            X11.XChangeProperty(
                display, event.requestor, prop, xa_atom, 32, PROP_MODE_REPLACE,
                ctypes.cast(atoms, ctypes.c_void_p), len(offered),
            )
        elif target in string_targets:
            data = ctypes.create_string_buffer(payload, len(payload))
            X11.XChangeProperty(
                display, event.requestor, prop, target, 8, PROP_MODE_REPLACE,
                ctypes.cast(data, ctypes.c_void_p), len(payload),
            )
            delivered = True
        else:
            prop = 0
        notify = SelectionEvent(
            type=SELECTION_NOTIFY,
            serial=0,
            send_event=1,
            display=display,
            requestor=event.requestor,
            selection=event.selection,
            target=event.target,
            property=prop,
            time=event.time,
        )
        X11.XSendEvent(
            display, event.requestor, 0, 0,
            ctypes.cast(ctypes.byref(notify), ctypes.c_void_p),
        )
        X11.XSync(display, 0)
        if delivered:
            # Let the requester process the property before the owner window disappears.
            time.sleep(0.15)
            return
    fail(f"no paste request arrived within {timeout:g}s")


def read_clipboard(timeout):
    display = open_display()
    window = new_window(display)
    clipboard = atom(display, "CLIPBOARD")
    utf8 = atom(display, "UTF8_STRING")
    property_atom = atom(display, "MDVIEW_E2E_CLIPBOARD")

    if X11.XGetSelectionOwner(display, clipboard) == 0:
        fail("CLIPBOARD has no owner (the app is using a private clipboard)")
    X11.XConvertSelection(
        display, clipboard, utf8, property_atom, window, 0
    )
    X11.XFlush(display)

    event_buffer = ctypes.create_string_buffer(EVENT_BYTES)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not X11.XPending(display):
            wait_for_event(display, min(0.1, deadline - time.monotonic()))
            continue
        if X11.XNextEvent(display, event_buffer):
            continue
        event = ctypes.cast(
            event_buffer, ctypes.POINTER(SelectionEvent)
        ).contents
        if event.type != SELECTION_NOTIFY:
            continue
        if event.property == 0:
            fail("the CLIPBOARD owner refused UTF8_STRING")
        actual_type = Atom()
        actual_format = ctypes.c_int()
        items = ctypes.c_ulong()
        after = ctypes.c_ulong()
        data = ctypes.c_void_p()
        status = X11.XGetWindowProperty(
            display, window, property_atom, 0, 0x1FFFFFFF, 1, ANY_PROPERTY_TYPE,
            ctypes.byref(actual_type), ctypes.byref(actual_format),
            ctypes.byref(items), ctypes.byref(after), ctypes.byref(data),
        )
        if status != 0 or not data:
            fail(f"could not read the CLIPBOARD property (status {status})")
        try:
            if actual_format.value != 8:
                fail(f"CLIPBOARD returned format {actual_format.value}, not UTF-8 bytes")
            text = ctypes.string_at(data, items.value).decode("utf-8", "replace")
        finally:
            X11.XFree(data)
        print(text, flush=True)
        return
    fail(f"no clipboard answer within {timeout:g}s")


def main(argv):
    if len(argv) < 2:
        raise SystemExit(__doc__)
    if argv[1] == "read":
        read_clipboard(float(argv[2]) if len(argv) > 2 else 3.0)
    elif argv[1] == "serve" and len(argv) >= 4:
        serve(argv[2], argv[3], float(argv[4]) if len(argv) > 4 else 10.0)
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
