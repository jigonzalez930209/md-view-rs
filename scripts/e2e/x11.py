#!/usr/bin/env python3
"""Minimal X11/XTest driver for the MD View E2E run.

Usage: DISPLAY=:97 python3 x11.py click 120 200
"""
import ctypes
import sys
import time

X11 = ctypes.CDLL("libX11.so.6")
XTST = ctypes.CDLL("libXtst.so.6")
X11.XOpenDisplay.restype = ctypes.c_void_p
X11.XStringToKeysym.restype = ctypes.c_ulong
X11.XKeysymToKeycode.restype = ctypes.c_ubyte
XTST.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
XTST.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_ulong]
XTST.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
X11.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]

DISPLAY = __import__("os").environ.get("DISPLAY", ":97").encode()
dpy = X11.XOpenDisplay(DISPLAY)
if not dpy:
    raise SystemExit("cannot open display")

MODS = {"ctrl": "Control_L", "shift": "Shift_L", "alt": "Alt_L"}
SPECIAL = {
    " ": "space", "\n": "Return", "\t": "Tab", ".": "period", ",": "comma",
    "/": "slash", "-": "minus", "=": "equal", "_": "underscore", ":": "colon",
    ";": "semicolon", "'": "apostrophe", '"': "quotedbl", "[": "bracketleft",
    "]": "bracketright", "\\": "backslash", "`": "grave",
}
NEEDS_SHIFT = set('ABCDEFGHIJKLMNOPQRSTUVWXYZ_:{}"<>?~!@#$%^&*()+')
SHIFT_MAP = {
    "_": "minus", ":": "semicolon", '"': "apostrophe", "?": "slash",
    "+": "equal", "!": "1", "@": "2", "#": "3", "$": "4", "%": "5",
    "^": "6", "&": "7", "*": "8", "(": "9", ")": "0", "{": "bracketleft",
    "}": "bracketright", "<": "comma", ">": "period", "~": "grave",
}


def keycode(name):
    sym = X11.XStringToKeysym(name.encode())
    if not sym:
        raise SystemExit(f"unknown keysym {name}")
    code = X11.XKeysymToKeycode(dpy, sym)
    if not code:
        raise SystemExit(f"no keycode for {name}")
    return code


def press(code, down):
    XTST.XTestFakeKeyEvent(dpy, code, 1 if down else 0, 0)
    X11.XFlush(dpy)
    time.sleep(0.015)


def chord(combo):
    parts = combo.lower().split("+")
    mods = [MODS[p] for p in parts[:-1]]
    key = parts[-1]
    if key.startswith("f") and key[1:].isdigit():
        key = key.upper()
    for mod in mods:
        press(keycode(mod), True)
    if key in ("plus", "equal"):
        press(keycode("equal"), True)
        press(keycode("equal"), False)
    elif key == "minus":
        press(keycode("minus"), True)
        press(keycode("minus"), False)
    elif key == "comma":
        press(keycode("comma"), True)
        press(keycode("comma"), False)
    elif key == "0":
        press(keycode("0"), True)
        press(keycode("0"), False)
    elif key == "tab":
        press(keycode("Tab"), True)
        press(keycode("Tab"), False)
    elif key in ("return", "enter"):
        press(keycode("Return"), True)
        press(keycode("Return"), False)
    elif key == "escape":
        press(keycode("Escape"), True)
        press(keycode("Escape"), False)
    else:
        press(keycode(key), True)
        press(keycode(key), False)
    for mod in reversed(mods):
        press(keycode(mod), False)


def type_text(text):
    for char in text:
        shifted = char in NEEDS_SHIFT
        name = SHIFT_MAP.get(char, char if not char.isupper() else char.lower())
        name = SPECIAL.get(char, name)
        if shifted:
            press(keycode("Shift_L"), True)
        press(keycode(name), True)
        press(keycode(name), False)
        if shifted:
            press(keycode("Shift_L"), False)
        time.sleep(0.01)


def click(x, y):
    XTST.XTestFakeMotionEvent(dpy, -1, x, y, 0)
    XTST.XTestFakeButtonEvent(dpy, 1, 1, 0)
    XTST.XTestFakeButtonEvent(dpy, 1, 0, 0)
    X11.XFlush(dpy)
    time.sleep(0.2)


def focus(window):
    X11.XSetInputFocus(dpy, int(window, 16), 2, 0)
    X11.XFlush(dpy)
    time.sleep(0.1)


if __name__ == "__main__":
    command = sys.argv[1]
    if command == "key":
        chord(sys.argv[2])
    elif command == "type":
        type_text(" ".join(sys.argv[2:]))
    elif command == "text":
        type_text(sys.argv[2])
    elif command == "click":
        click(int(sys.argv[2]), int(sys.argv[3]))
    elif command == "focus":
        focus(sys.argv[2])
    elif command == "rclick":
        x, y = int(sys.argv[2]), int(sys.argv[3])
        XTST.XTestFakeMotionEvent(dpy, -1, x, y, 0)
        XTST.XTestFakeButtonEvent(dpy, 3, 1, 0)
        XTST.XTestFakeButtonEvent(dpy, 3, 0, 0)
        X11.XFlush(dpy)
        time.sleep(0.2)
    elif command == "wheel":
        x, y, count = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
        XTST.XTestFakeMotionEvent(dpy, -1, x, y, 0)
        for _ in range(count):
            XTST.XTestFakeButtonEvent(dpy, 5, 1, 0)
            XTST.XTestFakeButtonEvent(dpy, 5, 0, 0)
            X11.XFlush(dpy)
            time.sleep(0.02)
        time.sleep(0.2)
    elif command == "wheelup":
        x, y, count = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
        XTST.XTestFakeMotionEvent(dpy, -1, x, y, 0)
        for _ in range(count):
            XTST.XTestFakeButtonEvent(dpy, 4, 1, 0)
            XTST.XTestFakeButtonEvent(dpy, 4, 0, 0)
            X11.XFlush(dpy)
            time.sleep(0.02)
        time.sleep(0.2)
    elif command == "sleep":
        time.sleep(float(sys.argv[2]))
    else:
        raise SystemExit(f"unknown command {command}")
