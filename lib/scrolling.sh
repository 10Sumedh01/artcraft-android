#!/usr/bin/env bash
# Two-finger scrolling — the app-side patches.
#
# A phone or tablet has no wheel, so a two-finger drag is the only way to scroll. The
# builder's vendored winit backend (see lib/build-common.sh) recognises the gesture the way
# a browser does — a two-finger slide scrolls, a two-finger pinch zooms — and takes it over
# from the touch path: the fingers' touches are withheld, and the first finger's touch is
# cancelled so the gesture cannot start a text selection.
#
# Two things are left for the app:
#
#  * the press that arrives with the *first* finger must not start a selection: the canvas
#    has to know a gesture is in progress, which it learns from the touch the backend
#    cancelled;
#  * the momentum. The web build leaves the flick's coast to the browser (Chromium
#    synthesises it and sends it as more wheel events), and so do macOS and Windows; Android
#    sends no momentum of its own for a two-finger pan, so the app has to make the coast
#    itself, exactly as it does on Linux.
#
# Every edit is guarded and idempotent: it applies only while the original text is still
# present, so it no-ops once upstream handles it, and is re-applied on a fresh checkout.
#
# The scroll synthesis itself (the vendored winit/egui-winit backends) lives in
# lib/build-common.sh and runs regardless of this script; this script adds the app side.
#
# To test WITHOUT the patches, comment out the call to this script in lib/build-common.sh.
#
# Usage: scrolling.sh <source-dir> <app> <title>
set -euo pipefail

SRC="${1:?source dir}"
APP="${2:?app}"
TITLE="${3:?title}"

python3 - "$SRC" "$APP" "$TITLE" <<'PY'
from pathlib import Path
import re
import sys

src, app, title = Path(sys.argv[1]), sys.argv[2], sys.argv[3]


def fix(rel: str, old: str, new: str) -> bool:
    """Replace the single `old` in `rel` with `new`, unless it is already gone or already fixed."""
    path = src / rel
    try:
        text = path.read_text()
    except OSError:
        return False
    if old not in text or new in text or text.count(old) != 1:
        return False
    path.write_text(text.replace(old, new, 1))
    return True


def sub(rel: str, pattern: str, new: str) -> bool:
    """Replace the first match of `pattern` in `rel` with `new`, unless it is already fixed."""
    path = src / rel
    try:
        text = path.read_text()
    except OSError:
        return False
    if new in text:
        return False
    updated, count = re.subn(pattern, new, text, count=1, flags=re.S)
    if count != 1:
        return False
    path.write_text(updated)
    return True


fixed = []
if app == "wordcraft":
    # A two-finger gesture scrolls the document (see the builder's patched winit backend).
    # The backend takes the gesture over from the touch path and cancels the first finger's
    # touch when the second finger lands; that cancellation is how the canvas learns a
    # gesture is in progress, so a press that belongs to the gesture does not start a text
    # selection, and a selection the first finger already began is dropped.
    #
    # egui's own multi-touch state cannot be used for this: the backend keeps egui's touch
    # state clear (it reports the gesture itself, so egui must not read the same two
    # fingers as a pinch of its own and zoom the document by accident).
    guard = (
        "/// A two-finger gesture (scroll or pinch) is in progress: a touch press during it is part\n"
        "/// of that gesture, not a click, and any text-selection drag it began must be dropped.\n"
        "///\n"
        "/// The backend takes the gesture over from the touch path and says so by cancelling the\n"
        "/// first finger's touch (see the builder's patched winit backend), so the gesture is\n"
        "/// recognised by that cancellation and not by egui's own multi-touch state (which the\n"
        "/// backend keeps clear, so that egui does not read the same fingers as a pinch).\n"
        "fn touch_gesture_active(ui: &Ui) -> bool {\n"
        "    ui.input(|i| {\n"
        "        i.events\n"
        "            .iter()\n"
        "            .any(|e| matches!(e, egui::Event::Touch { phase: egui::TouchPhase::Cancel, .. }))\n"
        "    })\n"
        "}\n"
    )
    mouse_line = (
        "fn mouse(app: &mut WordApp, ui: &Ui, resp: &egui::Response, rects: &[Rect], "
        "layout: &DocLayout, scale: f32) {"
    )
    # A source that has never been patched: add the guard. A source that already has one
    # (an earlier builder's, or an upstream that grew its own) gets it updated instead.
    if "fn touch_gesture_active" not in (src / "crates/ui-egui/src/canvas.rs").read_text():
        if fix(
            "crates/ui-egui/src/canvas.rs",
            mouse_line,
            guard + "\n" + mouse_line,
        ):
            fixed.append("crates/ui-egui/src/canvas.rs (touch gestures start no selection)")
    elif sub(
        "crates/ui-egui/src/canvas.rs",
        r"/// A two-finger gesture \(scroll or pinch\) is in progress.*?\nfn touch_gesture_active\(ui: &Ui\) -> bool \{\n.*?\n\}\n",
        guard,
    ):
        fixed.append("crates/ui-egui/src/canvas.rs (touch gestures start no selection)")

    # Do not begin a selection on a press that is part of a two-finger gesture.
    if fix(
        "crates/ui-egui/src/canvas.rs",
        "    let pressed = ui.input(|i| i.pointer.primary_pressed()) && resp.contains_pointer() && !over_balloon(app, p);\n",
        "    let pressed = ui.input(|i| i.pointer.primary_pressed())\n"
        "        && !touch_gesture_active(ui)\n"
        "        && resp.contains_pointer()\n"
        "        && !over_balloon(app, p);\n",
    ):
        fixed.append("crates/ui-egui/src/canvas.rs (two-finger press does not select)")

    # A second finger arriving after the press already started a drag: drop it, so the scroll
    # never carries a selection along. Runs before the drag-extension arms below.
    if fix(
        "crates/ui-egui/src/canvas.rs",
        "    } else if app.canvas.dragging\n"
        "        && ui.input(|i| i.pointer.primary_down())\n"
        "        && let Some(d) = app.canvas.unit_drag.as_mut()\n",
        "    } else if app.canvas.dragging && touch_gesture_active(ui) {\n"
        "        // A two-finger scroll took over: stop selecting and forget the drag.\n"
        "        app.canvas.cancel_text_drag();\n"
        "    } else if app.canvas.dragging\n"
        "        && ui.input(|i| i.pointer.primary_down())\n"
        "        && let Some(d) = app.canvas.unit_drag.as_mut()\n",
    ):
        fixed.append("crates/ui-egui/src/canvas.rs (scroll cancels a live selection drag)")

    # The flick's coast. The upstream constant leaves the momentum to the platform on the
    # platforms that send their own: macOS and Windows (they report the lift, and a touchpad
    # there as line deltas) and the web (Chromium synthesises the coast and sends it as more
    # wheel events, which is why the web build must not make one as well).
    #
    # Android sends no momentum of its own for a two-finger pan — the touchscreen's OS layer
    # has no inertia — so the app has to make the coast, exactly as it does on Linux. The
    # earlier builder patch excluded Android, which left a flick with no coast at all.
    #
    # A mouse's notches are line deltas and never reach the momentum, so they cannot coast:
    # only the smooth (point) deltas of a pan or a trackpad do.
    without_android = (
        "const SYNTH_MOMENTUM: bool = !cfg!(any(\n"
        '    target_os = "macos",\n'
        '    target_os = "ios",\n'
        '    target_os = "windows",\n'
        '    target_arch = "wasm32",\n'
        "));"
    )
    if fix(
        "crates/ui-egui/src/scroll.rs",
        'const SYNTH_MOMENTUM: bool = !cfg!(any(target_os = "macos", target_os = "ios", target_os = "windows", target_arch = "wasm32"));',
        without_android,
    ):
        fixed.append("crates/ui-egui/src/scroll.rs (the app makes the coast Android does not send)")
    elif sub(
        "crates/ui-egui/src/scroll.rs",
        r"const SYNTH_MOMENTUM: bool = !cfg!\(any\(.*?\n\)\);",
        without_android,
    ):
        fixed.append("crates/ui-egui/src/scroll.rs (the app makes the coast Android does not send)")

if fixed:
    print(f"  Added two-finger-scroll patches to {title}: {', '.join(fixed)}")
PY
